use crate::error::AppError;
use crate::settings::AppSettings;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogLine {
    pub level: String,
    pub message: String,
    pub timestamp: u64,
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn home() -> String {
    std::env::var("HOME").unwrap_or_else(|_| "/tmp".into())
}

fn emit_info(app: &AppHandle, msg: &str) {
    let _ = app.emit(
        "tool-install-log",
        LogLine {
            level: "info".into(),
            message: msg.into(),
            timestamp: now_ms(),
        },
    );
}

async fn stream_command(
    app_handle: &AppHandle,
    event: &str,
    program: &str,
    args: &[&str],
    env_overrides: &[(&str, &str)],
) -> Result<(), AppError> {
    let mut cmd = Command::new(program);
    cmd.args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    for (k, v) in env_overrides {
        cmd.env(k, v);
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| AppError::msg(format!("Failed to spawn {program}: {e}")))?;

    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    let mut stdout_lines = BufReader::new(stdout).lines();
    let mut stderr_lines = BufReader::new(stderr).lines();
    let mut stdout_done = false;
    let mut stderr_done = false;

    while !stdout_done || !stderr_done {
        tokio::select! {
            line = stdout_lines.next_line(), if !stdout_done => {
                match line {
                    Ok(Some(l)) => { let _ = app_handle.emit(event, LogLine { level: "info".into(), message: l, timestamp: now_ms() }); }
                    _ => { stdout_done = true; }
                }
            }
            line = stderr_lines.next_line(), if !stderr_done => {
                match line {
                    Ok(Some(l)) => { let _ = app_handle.emit(event, LogLine { level: "warn".into(), message: l, timestamp: now_ms() }); }
                    _ => { stderr_done = true; }
                }
            }
        }
    }

    let status = child
        .wait()
        .await
        .map_err(|e| AppError::msg(format!("Command wait failed: {e}")))?;
    if !status.success() {
        return Err(AppError::msg(format!(
            "{program} exited with status {status}"
        )));
    }
    Ok(())
}

async fn run_script(app: &AppHandle, script: &str) -> Result<(), AppError> {
    stream_command(app, "tool-install-log", "bash", &["-c", script], &[]).await
}

/// Install a tool and return updated AppSettings if any paths changed.
pub async fn install_tool(
    app: &AppHandle,
    tool_name: &str,
    settings: &AppSettings,
) -> Result<Option<AppSettings>, AppError> {
    let h = home();
    match tool_name {
        "cordova" => {
            stream_command(app, "tool-install-log", "npm", &["install", "-g", "cordova"], &[])
                .await?;
            // Locate the installed binary
            let path = std::process::Command::new("which")
                .arg("cordova")
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .unwrap_or_default();
            if !path.is_empty() {
                let mut updated = settings.clone();
                updated.cordova_path = path;
                Ok(Some(updated))
            } else {
                Ok(None)
            }
        }

        "gradle" => {
            let script = format!(
                r#"set -e
mkdir -p "{h}/gradle"
echo "Downloading Gradle 8.7..."
curl --silent --show-error -L -o /tmp/gradle-8.7-bin.zip "https://services.gradle.org/distributions/gradle-8.7-bin.zip"
echo "Extracting Gradle 8.7..."
unzip -q /tmp/gradle-8.7-bin.zip -d "{h}/gradle/"
rm -f /tmp/gradle-8.7-bin.zip
echo "Done: Gradle 8.7 installed to {h}/gradle/gradle-8.7"
"#
            );
            run_script(app, &script).await?;
            let mut updated = settings.clone();
            updated.gradle_home = format!("{h}/gradle/gradle-8.7");
            Ok(Some(updated))
        }

        "android-cmdline-tools" => {
            // If the user already has a java_home set, wire it in so sdkmanager can run
            let java_env = if !settings.java_home.is_empty() {
                format!(
                    "export JAVA_HOME='{jh}'; export PATH=\"$JAVA_HOME/bin:$PATH\";",
                    jh = settings.java_home
                )
            } else {
                String::new()
            };
            let script = format!(
                r#"set -e
{java_env}
mkdir -p "{h}/android-sdk/cmdline-tools"
echo "Downloading Android cmdline-tools..."
curl --silent --show-error -L -o /tmp/cmdline-tools.zip "https://dl.google.com/android/repository/commandlinetools-linux-11076708_latest.zip"
echo "Extracting cmdline-tools..."
unzip -q /tmp/cmdline-tools.zip -d "{h}/android-sdk/cmdline-tools/"
if [ -d "{h}/android-sdk/cmdline-tools/cmdline-tools" ]; then
  mv "{h}/android-sdk/cmdline-tools/cmdline-tools" "{h}/android-sdk/cmdline-tools/latest"
fi
rm -f /tmp/cmdline-tools.zip
SDKMANAGER="{h}/android-sdk/cmdline-tools/latest/bin/sdkmanager"
echo "Accepting SDK licenses..."
yes | "$SDKMANAGER" --sdk_root="{h}/android-sdk" --licenses >/dev/null 2>&1 || true
echo "Installing platform-tools, build-tools;34.0.0, platforms;android-34 (this may take a while)..."
"$SDKMANAGER" --sdk_root="{h}/android-sdk" "platform-tools" "build-tools;34.0.0" "platforms;android-34"
echo "Done: Android SDK installed to {h}/android-sdk"
"#
            );
            run_script(app, &script).await?;
            let mut updated = settings.clone();
            updated.android_sdk_path = format!("{h}/android-sdk");
            if updated.build_tools_version.is_empty() {
                updated.build_tools_version = "34.0.0".into();
            }
            Ok(Some(updated))
        }

        "jdk" => {
            let script = format!(
                r#"set -e
mkdir -p "{h}/jdk"
echo "Downloading JDK 21 (Eclipse Temurin)..."
curl --silent --show-error -L -o /tmp/jdk21.tar.gz "https://api.adoptium.net/v3/binary/latest/21/ga/linux/x64/jdk/hotspot/normal/eclipse"
echo "Extracting JDK 21..."
tar -xzf /tmp/jdk21.tar.gz --strip-components=1 -C "{h}/jdk/"
rm -f /tmp/jdk21.tar.gz
echo "Done: JDK 21 installed to {h}/jdk"
"#
            );
            run_script(app, &script).await?;
            let mut updated = settings.clone();
            updated.java_home = format!("{h}/jdk");
            Ok(Some(updated))
        }

        "node" => {
            let script = format!(
                r#"set -e
mkdir -p "{h}/nodejs"
echo "Downloading Node.js 20 LTS..."
curl --silent --show-error -L -o /tmp/node.tar.gz "https://nodejs.org/dist/v20.18.1/node-v20.18.1-linux-x64.tar.gz"
echo "Extracting Node.js..."
tar -xzf /tmp/node.tar.gz --strip-components=1 -C "{h}/nodejs/"
rm -f /tmp/node.tar.gz
echo "Done: Node.js installed to {h}/nodejs/bin/node"
"#
            );
            run_script(app, &script).await?;
            let mut updated = settings.clone();
            updated.node_path = format!("{h}/nodejs/bin/node");
            Ok(Some(updated))
        }

        "renpy-sdk" => {
            let script = format!(
                r#"set -e
echo "Downloading Ren'Py SDK 8.5.2 (~300 MB)..."
curl --silent --show-error -L -o /tmp/renpy-8.5.2-sdk.tar.bz2 "https://www.renpy.org/dl/8.5.2/renpy-8.5.2-sdk.tar.bz2"
echo "Extracting Ren'Py SDK..."
tar -xjf /tmp/renpy-8.5.2-sdk.tar.bz2 -C "{h}/"
rm -f /tmp/renpy-8.5.2-sdk.tar.bz2
chmod +x "{h}/renpy-8.5.2-sdk/renpy.sh" 2>/dev/null || true
echo "Done: Ren'Py SDK installed to {h}/renpy-8.5.2-sdk"
"#
            );
            run_script(app, &script).await?;
            let mut updated = settings.clone();
            updated.renpy_sdk_path = format!("{h}/renpy-8.5.2-sdk");
            Ok(Some(updated))
        }

        "rapt" => {
            Err(AppError::msg(
                "RAPT must be installed via the Ren'Py Launcher: open renpy.sh, go to Android → Install SDK.",
            ))
        }

        _ => {
            emit_info(app, &format!("No auto-install available for '{tool_name}'."));
            Ok(None)
        }
    }
}

pub async fn generate_keystore(
    app_handle: &AppHandle,
    keystore_path: &str,
    alias: &str,
    storepass: &str,
    keypass: &str,
    dname: &str,
) -> Result<(), AppError> {
    let expanded = if keystore_path.starts_with('~') {
        keystore_path.replacen('~', &home(), 1)
    } else {
        keystore_path.to_string()
    };
    let keystore_path = expanded.as_str();
    if std::path::Path::new(keystore_path).exists() {
        std::fs::remove_file(keystore_path)
            .map_err(|e| AppError::msg(format!("Failed to remove existing keystore: {e}")))?;
    }
    stream_command(
        app_handle,
        "tool-install-log",
        "keytool",
        &[
            "-genkeypair",
            "-keystore",
            keystore_path,
            "-alias",
            alias,
            "-keyalg",
            "RSA",
            "-keysize",
            "2048",
            "-validity",
            "10000",
            "-storepass",
            storepass,
            "-keypass",
            keypass,
            "-dname",
            dname,
            "-noprompt",
        ],
        &[],
    )
    .await
}
