use crate::error::AppError;
use crate::pipeline::types::LogLine;
use crate::settings::AppSettings;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

pub fn emit_log(app: &AppHandle, level: &str, msg: &str) {
    let _ = app.emit(
        "pipeline-log",
        LogLine {
            level: level.into(),
            message: msg.into(),
            timestamp: now_ms(),
        },
    );
}

/// Prefer Java 21 over Java 25 — Gradle 8.7 (bundled in cordova-android 13)
/// doesn't support Java 25 class files (major version 69). Java 21 is LTS and works.
fn best_java_home(settings: &AppSettings) -> String {
    // If the configured JAVA_HOME is Java 21 or lower, use it as-is
    if !settings.java_home.is_empty() {
        let java_bin = PathBuf::from(&settings.java_home).join("bin/java");
        if java_bin.exists() {
            // Check if it's Java 25 — if so, prefer Java 21 if available
            let j21 = Path::new("/usr/lib/jvm/java-21-openjdk");
            if settings.java_home.contains("java-25") && j21.join("bin/java").exists() {
                return j21.to_string_lossy().into_owned();
            }
            return settings.java_home.clone();
        }
    }
    // Auto-discover: prefer Java 21, fall back to whatever is available
    let preferred = [
        "/usr/lib/jvm/java-21-openjdk",
        "/usr/lib/jvm/java-17-openjdk",
        "/usr/lib/jvm/java-21",
        "/usr/lib/jvm/java-17",
    ];
    for p in &preferred {
        if Path::new(p).join("bin/java").exists() {
            return p.to_string();
        }
    }
    settings.java_home.clone()
}

pub fn build_env(settings: &AppSettings) -> HashMap<String, String> {
    let mut env: HashMap<String, String> = std::env::vars().collect();

    if !settings.android_sdk_path.is_empty() {
        env.insert("ANDROID_HOME".into(), settings.android_sdk_path.clone());
        env.insert("ANDROID_SDK_ROOT".into(), settings.android_sdk_path.clone());
    }

    let java_home = best_java_home(settings);
    if !java_home.is_empty() {
        env.insert("JAVA_HOME".into(), java_home.clone());
    }

    // Required for Java 17+ modules with Gradle; remove security.manager (dropped in Java 17+)
    env.insert(
        "JAVA_TOOL_OPTIONS".into(),
        "-Dfile.encoding=UTF-8 --enable-native-access=ALL-UNNAMED".into(),
    );
    env.insert("GRADLE_OPTS".into(), "-Dfile.encoding=UTF-8".into());

    // Ensure Unicode filenames work across the process
    env.insert("LANG".into(), "en_US.UTF-8".into());
    env.insert("LC_ALL".into(), "en_US.UTF-8".into());

    let mut path_extras: Vec<String> = vec![];
    if !java_home.is_empty() {
        path_extras.push(format!("{}/bin", java_home));
    }
    if !settings.gradle_home.is_empty() {
        path_extras.push(format!("{}/bin", settings.gradle_home));
    }
    if !settings.android_sdk_path.is_empty() {
        path_extras.push(format!("{}/platform-tools", settings.android_sdk_path));
        path_extras.push(format!("{}/cmdline-tools/latest/bin", settings.android_sdk_path));
        path_extras.push(format!(
            "{}/build-tools/{}",
            settings.android_sdk_path, settings.build_tools_version
        ));
    }
    if !settings.node_path.is_empty() {
        if let Some(parent) = Path::new(&settings.node_path).parent() {
            path_extras.push(parent.to_string_lossy().into_owned());
        }
    }
    let home = std::env::var("HOME").unwrap_or_default();
    path_extras.push(format!("{}/.npm-global/bin", home));
    path_extras.push(format!("{}/.local/bin", home));

    let existing = env.get("PATH").cloned().unwrap_or_default();
    path_extras.push(existing);
    env.insert("PATH".into(), path_extras.join(":"));

    env
}

pub async fn stream_cmd(
    app: &AppHandle,
    program: &str,
    args: &[&str],
    cwd: &Path,
    env: &HashMap<String, String>,
) -> Result<(), AppError> {
    emit_log(app, "info", &format!("$ {} {}", program, args.join(" ")));

    let mut cmd = Command::new(program);
    cmd.args(args)
        .current_dir(cwd)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .env_clear();
    for (k, v) in env {
        cmd.env(k, v);
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| AppError::msg(format!("Failed to spawn '{program}': {e}")))?;

    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    let mut out_lines = BufReader::new(stdout).lines();
    let mut err_lines = BufReader::new(stderr).lines();
    let mut stdout_done = false;

    loop {
        tokio::select! {
            line = out_lines.next_line(), if !stdout_done => match line {
                Ok(Some(l)) => emit_log(app, "info", &l),
                Ok(None) => { stdout_done = true; }
                Err(_) => { stdout_done = true; }
            },
            line = err_lines.next_line() => match line {
                Ok(Some(l)) => emit_log(app, "warn", &l),
                Ok(None) => break,
                Err(_) => break,
            }
        }
        if stdout_done {
            while let Ok(Some(l)) = err_lines.next_line().await {
                emit_log(app, "warn", &l);
            }
            break;
        }
    }

    let status = child
        .wait()
        .await
        .map_err(|e| AppError::msg(format!("Wait failed: {e}")))?;
    if !status.success() {
        return Err(AppError::msg(format!("'{program}' exited with {status}")));
    }
    Ok(())
}

pub fn find_cordova(settings: &AppSettings) -> Result<String, AppError> {
    if !settings.cordova_path.is_empty() && Path::new(&settings.cordova_path).exists() {
        return Ok(settings.cordova_path.clone());
    }
    let home = std::env::var("HOME").unwrap_or_default();
    let candidates = [
        format!("{}/.npm-global/bin/cordova", home),
        "/usr/local/bin/cordova".to_string(),
        "/usr/bin/cordova".to_string(),
    ];
    candidates
        .into_iter()
        .find(|p| Path::new(p).exists())
        .or_else(|| which::which("cordova").ok().map(|p| p.to_string_lossy().into_owned()))
        .ok_or_else(|| {
            AppError::msg(
                "cordova not found. Set Cordova Path in Settings or run: npm install -g cordova",
            )
        })
}

pub async fn scaffold_cordova(
    app: &AppHandle,
    work_dir: &Path,
    app_id: &str,
    app_name: &str,
    settings: &AppSettings,
) -> Result<PathBuf, AppError> {
    let cordova_dir = work_dir.join("cordova");
    if cordova_dir.exists() {
        std::fs::remove_dir_all(&cordova_dir)?;
    }
    std::fs::create_dir_all(work_dir)?;

    let env = build_env(settings);
    let cordova_bin = find_cordova(settings)?;
    let dir_str = cordova_dir.to_string_lossy().into_owned();

    stream_cmd(
        app,
        &cordova_bin,
        &["create", &dir_str, app_id, app_name],
        work_dir,
        &env,
    )
    .await?;

    Ok(cordova_dir)
}

pub fn patch_config_xml(
    cordova_dir: &Path,
    app_id: &str,
    app_name: &str,
    version: &str,
    icon_src: Option<&str>,
) -> Result<(), AppError> {
    let config_path = cordova_dir.join("config.xml");
    let content = std::fs::read_to_string(&config_path)?;

    let content = patch_widget_tag(&content, app_id, app_name, version);

    let icon_line = icon_src
        .map(|s| format!("    <icon src=\"{}\" />\n", s))
        .unwrap_or_default();

    let insert = format!(
        r#"    <access origin="*" />
    <allow-navigation href="*" />
    <allow-intent href="http://*/*" />
    <allow-intent href="https://*/*" />
    <preference name="Orientation" value="landscape" />
    <preference name="FullScreen" value="true" />
    <preference name="AndroidPersistentFileLocation" value="Compatibility" />
    <preference name="android-targetSdkVersion" value="34" />
    <preference name="android-minSdkVersion" value="21" />
{icon_line}</widget>"#
    );

    let content = content.replace("</widget>", &insert);
    std::fs::write(&config_path, content)?;
    Ok(())
}

fn patch_widget_tag(content: &str, id: &str, name: &str, version: &str) -> String {
    let mut out = content.to_string();

    if let (Some(s), Some(e)) = (out.find("<name>"), out.find("</name>")) {
        out = format!("{}<name>{}</name>{}", &out[..s], name, &out[e + 7..]);
    }

    if let Some(widget_start) = out.find("<widget") {
        if let Some(rel_end) = out[widget_start..].find('>') {
            let tag_end = widget_start + rel_end + 1;
            let old_tag = out[widget_start..tag_end].to_string();
            let mut new_tag = old_tag.clone();
            new_tag = set_attr(&new_tag, "id", id);
            new_tag = set_attr(&new_tag, "version", version);
            out = out.replacen(&old_tag, &new_tag, 1);
        }
    }
    out
}

fn set_attr(tag: &str, attr: &str, value: &str) -> String {
    for delim in ['"', '\''] {
        let pat = format!("{attr}={delim}");
        if let Some(start) = tag.find(&pat) {
            let val_start = start + pat.len();
            if let Some(end) = tag[val_start..].find(delim) {
                return format!(
                    "{}{}=\"{}\"{}",
                    &tag[..start],
                    attr,
                    value,
                    &tag[val_start + end + 1..]
                );
            }
        }
    }
    tag.replacen("<widget", &format!("<widget {}=\"{}\"", attr, value), 1)
}

pub async fn add_android_platform(
    app: &AppHandle,
    cordova_dir: &Path,
    settings: &AppSettings,
) -> Result<(), AppError> {
    let env = build_env(settings);
    let cordova_bin = find_cordova(settings)?;
    stream_cmd(
        app,
        &cordova_bin,
        &["platform", "add", "android@13"],
        cordova_dir,
        &env,
    )
    .await?;

    // Disable Gradle daemon + VFS watching — reduces stale-state issues across runs.
    patch_gradle_properties(cordova_dir);
    Ok(())
}

/// Patch the platform's gradle.properties to fix Unicode filename hashing bugs in AGP 8.x.
///
/// Root cause: Gradle's VFS watcher and daemon retain stale state between runs.  When a
/// filename contains non-ASCII bytes (e.g. accented chars in audio file names), the
/// fingerprinting step in :app:mergeReleaseAssets fails with "does not exist", even though
/// the file is present on disk.  Disabling the daemon (fresh JVM each build) and VFS
/// watching eliminates the stale-state path.  Also forcing file.encoding=UTF-8 in the
/// JVM args ensures Java's File APIs handle Unicode names correctly.
fn patch_gradle_properties(cordova_dir: &Path) {
    let props_path = cordova_dir.join("platforms/android/gradle.properties");
    if !props_path.exists() {
        return;
    }
    let Ok(existing) = std::fs::read_to_string(&props_path) else { return };
    if existing.contains("org.gradle.daemon=false") {
        return; // already patched
    }
    let patch = "\n\
        # vn2apk: disable daemon + VFS watching to fix Unicode asset filename hashing\n\
        org.gradle.daemon=false\n\
        org.gradle.vfs.watch=false\n\
        org.gradle.caching=false\n\
        org.gradle.jvmargs=-Xmx2048m -Dfile.encoding=UTF-8\n";
    let _ = std::fs::write(&props_path, format!("{}{}", existing, patch));
}

pub async fn build_android_release(
    app: &AppHandle,
    cordova_dir: &Path,
    settings: &AppSettings,
) -> Result<PathBuf, AppError> {
    let env = build_env(settings);
    let cordova_bin = find_cordova(settings)?;
    // -- packageType=apk forces assembleRelease (APK) instead of bundleRelease (AAB).
    // Skip Lint: lintVitalRelease pulls ~60 MB of lint-checks/intellij-core/kotlin-compiler
    // that have zero effect on the produced APK. Excluding it keeps builds light on slow
    // networks and lets a fully-cached toolchain build offline.
    //
    // IMPORTANT (cordova-android 13): all gradle args must go in ONE --gradleArg whose
    // value is a single space-separated string. cordova-android's build.js does
    // `parseArgsStringToArgv(options.argv.gradleArg[0])` — it reads only the *first*
    // --gradleArg and re-tokenizes it. Passing several --gradleArg=… flags keeps only
    // the first (`-x`), which then attaches to the appended build task as
    // `-x cdvBuildRelease` — excluding the build itself, so Gradle runs `:help` and
    // emits no APK. Keep this as a single argument.
    stream_cmd(
        app,
        &cordova_bin,
        &[
            "build",
            "android",
            "--release",
            "--",
            "--packageType=apk",
            "--gradleArg=-x lintVitalRelease -x lintVitalAnalyzeRelease",
        ],
        cordova_dir,
        &env,
    )
    .await?;

    let candidates = [
        "platforms/android/app/build/outputs/apk/release/app-release-unsigned.apk",
        "platforms/android/app/build/outputs/apk/release/app-release.apk",
    ];
    for rel in &candidates {
        let p = cordova_dir.join(rel);
        if p.exists() {
            return Ok(p);
        }
    }
    Err(AppError::msg(
        "Cordova build succeeded but APK not found at expected output path. Check build logs.",
    ))
}
