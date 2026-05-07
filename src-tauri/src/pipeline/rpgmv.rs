use crate::error::AppError;
use crate::error_translate::translate;
use crate::pipeline::cordova::{
    add_android_platform, build_android_release, emit_log, patch_config_xml, scaffold_cordova,
};
use crate::pipeline::signing::{sign_apk, zipalign};
use crate::pipeline::types::{BuildOptions, BuildResult, PipelineStage, PipelineStageEvent, StageStatus};
use crate::settings::AppSettings;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};

fn emit_stage(app: &AppHandle, stage: PipelineStage, status: StageStatus) {
    let _ = app.emit("pipeline-stage", PipelineStageEvent { stage, status });
}

fn check_cancel(cancel: &AtomicBool) -> Result<(), AppError> {
    if cancel.load(Ordering::SeqCst) {
        Err(AppError::msg("Build cancelled by user"))
    } else {
        Ok(())
    }
}

/// Scan for NW.js-specific calls that won't work in Cordova WebView.
fn scan_nwjs_plugins(game_path: &Path) -> Vec<String> {
    let mut warnings = vec![];
    let plugins_dir = game_path.join("www/js/plugins");
    if !plugins_dir.exists() {
        return warnings;
    }
    let patterns = [
        "require('fs')",
        "require(\"fs\")",
        "require('path')",
        "require(\"path\")",
        "require('child_process')",
        "nw.require",
        "nw.App",
        "process.versions.nw",
    ];
    if let Ok(entries) = std::fs::read_dir(&plugins_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "js").unwrap_or(false) {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    for &pat in &patterns {
                        if content.contains(pat) {
                            let name = path.file_name().unwrap_or_default().to_string_lossy();
                            warnings.push(format!(
                                "Plugin '{}' uses '{}' (NW.js API, may not work in WebView)",
                                name, pat
                            ));
                            break;
                        }
                    }
                }
            }
        }
    }
    warnings
}

fn copy_game_assets(
    app: &AppHandle,
    game_path: &Path,
    cordova_dir: &Path,
    engine: &str,
) -> Result<(), AppError> {
    // TyranoBuilder exports place index.html at the game root (no www/ subdirectory).
    let src_www = if engine == "TyranoBuilder" {
        game_path.to_path_buf()
    } else {
        game_path.join("www")
    };
    let dst_www = cordova_dir.join("www");

    // Remove the default Cordova www/ that was created by scaffold
    if dst_www.exists() {
        std::fs::remove_dir_all(&dst_www)?;
    }

    emit_log(app, "info", &format!("Copying {} → {}", src_www.display(), dst_www.display()));

    let options = fs_extra::dir::CopyOptions {
        copy_inside: true,
        ..Default::default()
    };

    fs_extra::dir::copy(&src_www, &dst_www, &options)
        .map_err(|e| AppError::msg(format!("Asset copy failed: {e}")))?;

    emit_log(app, "info", "Assets copied successfully");
    Ok(())
}

// ── Filename sanitization ─────────────────────────────────────────────────────
//
// AGP 8.x's mergeReleaseAssets task fingerprints every input file using MD5.
// When a filename contains non-ASCII bytes, Java's File.exists() returns false on
// some JVM/OS combinations (path encoding mismatch), causing the build to abort.
//
// Fix: after copying www/ into the Cordova project, rename any file whose name
// contains non-ASCII characters to an ASCII-safe equivalent, and patch all
// JSON/JS data-file references to use the new names.

fn ascii_fold(c: char) -> char {
    match c {
        'À'|'Á'|'Â'|'Ã'|'Ä'|'Å'|'Æ' => 'A',
        'à'|'á'|'â'|'ã'|'ä'|'å'|'æ' => 'a',
        'È'|'É'|'Ê'|'Ë' => 'E',
        'è'|'é'|'ê'|'ë' => 'e',
        'Ì'|'Í'|'Î'|'Ï' => 'I',
        'ì'|'í'|'î'|'ï' => 'i',
        'Ò'|'Ó'|'Ô'|'Õ'|'Ö'|'Ø' => 'O',
        'ò'|'ó'|'ô'|'õ'|'ö'|'ø' => 'o',
        'Ù'|'Ú'|'Û'|'Ü' => 'U',
        'ù'|'ú'|'û'|'ü' => 'u',
        'Ý'|'Ÿ' => 'Y',
        'ý'|'ÿ' => 'y',
        'Ñ' => 'N',
        'ñ' => 'n',
        'Ç' => 'C',
        'ç' => 'c',
        'Ð' => 'D',
        'ð' => 'd',
        'Þ' => 'T',
        'þ' => 't',
        'ß' => 's',
        _ => '_',
    }
}

fn to_ascii_name(name: &str) -> String {
    name.chars().map(|c| if c.is_ascii() { c } else { ascii_fold(c) }).collect()
}

/// Pick a non-colliding target path, appending _1 _2 … if needed.
fn dedup_path(parent: &Path, new_name: &str) -> PathBuf {
    let candidate = parent.join(new_name);
    if !candidate.exists() {
        return candidate;
    }
    let (stem, ext) = match new_name.rfind('.') {
        Some(i) => (&new_name[..i], &new_name[i..]),
        None => (new_name, ""),
    };
    for n in 1u32..=9999 {
        let c = parent.join(format!("{}_{}{}", stem, n, ext));
        if !c.exists() {
            return c;
        }
    }
    candidate
}

/// Walk `dir`, collect (old_path, new_path) for every file whose name has non-ASCII chars.
fn collect_file_renames(dir: &Path, out: &mut Vec<(PathBuf, PathBuf)>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_file_renames(&path, out)?;
        } else if path.is_file() {
            let fname = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");
            if fname.chars().any(|c| !c.is_ascii()) {
                let new_name = to_ascii_name(fname);
                let new_path = dedup_path(path.parent().unwrap(), &new_name);
                out.push((path, new_path));
            }
        }
    }
    Ok(())
}

/// Replace `"old_stem"` with `"new_stem"` in every JSON/JS file under `www_dir`.
fn patch_data_references(www_dir: &Path, stem_map: &[(String, String)]) -> Result<(), AppError> {
    let exts = ["json", "js"];
    patch_text_files(www_dir, &exts, stem_map)
}

fn patch_text_files(dir: &Path, exts: &[&str], stem_map: &[(String, String)]) -> Result<(), AppError> {
    for entry in std::fs::read_dir(dir)
        .map_err(|e| AppError::msg(format!("read_dir {}: {e}", dir.display())))?
    {
        let entry = entry.map_err(|e| AppError::msg(format!("{e}")))?;
        let path = entry.path();
        if path.is_dir() {
            patch_text_files(&path, exts, stem_map)?;
        } else if path.is_file() {
            let has_ext = path.extension()
                .and_then(|e| e.to_str())
                .map(|e| exts.iter().any(|&x| x.eq_ignore_ascii_case(e)))
                .unwrap_or(false);
            if !has_ext {
                continue;
            }
            let Ok(content) = std::fs::read_to_string(&path) else { continue };
            let mut patched = content.clone();
            for (old_stem, new_stem) in stem_map {
                let old_pat = format!("\"{}\"", old_stem);
                let new_pat = format!("\"{}\"", new_stem);
                if patched.contains(&old_pat) {
                    patched = patched.replace(&old_pat, &new_pat);
                }
            }
            if patched != content {
                std::fs::write(&path, patched.as_bytes())
                    .map_err(|e| AppError::msg(format!("write {}: {e}", path.display())))?;
            }
        }
    }
    Ok(())
}

/// Sanitize non-ASCII filenames in the Cordova www/ directory and patch cross-references.
/// Called after copy_game_assets, before the Gradle build.
fn sanitize_www_filenames(app: &AppHandle, www_dir: &Path) -> Result<(), AppError> {
    let mut renames: Vec<(PathBuf, PathBuf)> = Vec::new();
    collect_file_renames(www_dir, &mut renames)
        .map_err(|e| AppError::msg(format!("Scanning www/ for non-ASCII names: {e}")))?;

    if renames.is_empty() {
        return Ok(());
    }

    emit_log(
        app,
        "info",
        &format!(
            "Sanitizing {} non-ASCII filename(s) for Gradle compatibility...",
            renames.len()
        ),
    );

    // Build stem-rename map for data file patching, then do the renames.
    let mut stem_map: Vec<(String, String)> = Vec::new();
    for (old_path, new_path) in &renames {
        let old_stem = old_path.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string();
        let new_stem = new_path.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string();
        if old_stem != new_stem {
            stem_map.push((old_stem, new_stem));
        }
        std::fs::rename(old_path, new_path).map_err(|e| {
            AppError::msg(format!(
                "Rename '{}' → '{}': {e}",
                old_path.display(),
                new_path.display()
            ))
        })?;
    }

    if !stem_map.is_empty() {
        emit_log(app, "info", "Patching data file references to renamed assets...");
        patch_data_references(www_dir, &stem_map)?;
    }

    Ok(())
}

fn copy_icon(
    game_path: &Path,
    cordova_dir: &Path,
    engine: &str,
) -> Option<String> {
    // TyranoBuilder: icon at tyrano/icon.png or tyrano/data/icon.png
    let src = if engine == "TyranoBuilder" {
        let a = game_path.join("tyrano/icon.png");
        let b = game_path.join("tyrano/data/icon.png");
        if a.exists() { a } else { b }
    } else {
        game_path.join("www/icon/icon.png")
    };
    if !src.exists() {
        return None;
    }
    let res_dir = cordova_dir.join("res");
    std::fs::create_dir_all(&res_dir).ok()?;
    let dst = res_dir.join("icon.png");
    std::fs::copy(&src, &dst).ok()?;
    Some("res/icon.png".to_string())
}

fn make_app_id(raw: &str) -> String {
    // Convert to valid Java package name: lowercase, replace spaces/dashes with _, strip invalid chars
    let sanitized: String = raw
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '_' { c.to_ascii_lowercase() } else { '_' })
        .collect();
    let sanitized = sanitized.trim_matches('_').to_string();
    let sanitized = if sanitized.is_empty() { "game".to_string() } else { sanitized };
    // Ensure it doesn't start with a digit
    let sanitized = if sanitized.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
        format!("app_{}", sanitized)
    } else {
        sanitized
    };
    format!("com.vn2apk.{}", sanitized)
}

fn work_dir_for(app_name: &str) -> PathBuf {
    let sanitized: String = app_name
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect();
    Path::new("/tmp").join(format!("vn2apk_{}", sanitized))
}

pub async fn run_rpgmv_pipeline(
    app: AppHandle,
    game_path: PathBuf,
    options: BuildOptions,
    settings: AppSettings,
    cancel: Arc<AtomicBool>,
) {
    match run_pipeline_inner(&app, &game_path, &options, &settings, &cancel).await {
        Ok(result) => {
            let _ = app.emit("pipeline-done", result);
        }
        Err(e) => {
            let _ = app.emit("pipeline-error", translate(&e.to_string()));
        }
    }
}

async fn run_pipeline_inner(
    app: &AppHandle,
    game_path: &Path,
    options: &BuildOptions,
    settings: &AppSettings,
    cancel: &AtomicBool,
) -> Result<BuildResult, AppError> {
    let mut warnings: Vec<String> = vec![];

    // ── Stage 1: Validate ────────────────────────────────────────────────────
    emit_stage(app, PipelineStage::Validate, StageStatus::Running);
    emit_log(app, "info", "Validating game folder...");

    if !game_path.exists() {
        emit_stage(app, PipelineStage::Validate, StageStatus::Failed);
        return Err(AppError::msg("Game folder does not exist"));
    }
    if !game_path.join("www").is_dir() {
        emit_stage(app, PipelineStage::Validate, StageStatus::Failed);
        return Err(AppError::msg("Missing www/ directory — is this an RPGMV game?"));
    }
    if !game_path.join("www/index.html").exists() {
        emit_stage(app, PipelineStage::Validate, StageStatus::Failed);
        return Err(AppError::msg("Missing www/index.html"));
    }

    // Check disk space: need ~3x the www/ size
    let nwjs_warnings = scan_nwjs_plugins(game_path);
    for w in &nwjs_warnings {
        emit_log(app, "warn", w);
        warnings.push(w.clone());
    }
    emit_stage(app, PipelineStage::Validate, StageStatus::Done);
    check_cancel(cancel)?;

    // ── Stage 2: Scaffold Cordova ────────────────────────────────────────────
    emit_stage(app, PipelineStage::Scaffold, StageStatus::Running);
    emit_log(app, "info", "Scaffolding Cordova project...");

    let work_dir = work_dir_for(&options.app_name);
    let cordova_dir = scaffold_cordova(app, &work_dir, &options.app_id, &options.app_name, settings)
        .await
        .map_err(|e| { emit_stage(app, PipelineStage::Scaffold, StageStatus::Failed); e })?;

    emit_stage(app, PipelineStage::Scaffold, StageStatus::Done);
    check_cancel(cancel)?;

    // ── Stage 3: Copy Assets ─────────────────────────────────────────────────
    emit_stage(app, PipelineStage::CopyAssets, StageStatus::Running);
    copy_game_assets(app, game_path, &cordova_dir, &options.engine)
        .map_err(|e| { emit_stage(app, PipelineStage::CopyAssets, StageStatus::Failed); e })?;
    // Sanitize non-ASCII filenames so AGP 8.x can fingerprint them without crashing.
    sanitize_www_filenames(app, &cordova_dir.join("www"))
        .map_err(|e| { emit_stage(app, PipelineStage::CopyAssets, StageStatus::Failed); e })?;
    emit_stage(app, PipelineStage::CopyAssets, StageStatus::Done);
    check_cancel(cancel)?;

    // ── Stage 4: Patch config.xml ────────────────────────────────────────────
    emit_stage(app, PipelineStage::PatchConfig, StageStatus::Running);
    emit_log(app, "info", "Patching config.xml...");
    let icon_src = copy_icon(game_path, &cordova_dir, &options.engine);
    patch_config_xml(
        &cordova_dir,
        &options.app_id,
        &options.app_name,
        &options.version_name,
        icon_src.as_deref(),
    )
    .map_err(|e| { emit_stage(app, PipelineStage::PatchConfig, StageStatus::Failed); e })?;
    emit_stage(app, PipelineStage::PatchConfig, StageStatus::Done);
    check_cancel(cancel)?;

    // ── Stage 5: Add Android Platform ───────────────────────────────────────
    emit_stage(app, PipelineStage::AddPlatform, StageStatus::Running);
    emit_log(app, "info", "Adding Android platform (this takes a while)...");
    add_android_platform(app, &cordova_dir, settings)
        .await
        .map_err(|e| { emit_stage(app, PipelineStage::AddPlatform, StageStatus::Failed); e })?;
    emit_stage(app, PipelineStage::AddPlatform, StageStatus::Done);
    check_cancel(cancel)?;

    // ── Stage 6: Build ───────────────────────────────────────────────────────
    emit_stage(app, PipelineStage::Build, StageStatus::Running);
    emit_log(app, "info", "Running cordova build android --release...");
    let unsigned_apk = build_android_release(app, &cordova_dir, settings)
        .await
        .map_err(|e| { emit_stage(app, PipelineStage::Build, StageStatus::Failed); e })?;
    emit_stage(app, PipelineStage::Build, StageStatus::Done);
    check_cancel(cancel)?;

    // ── Stage 7: Zipalign ────────────────────────────────────────────────────
    emit_stage(app, PipelineStage::Align, StageStatus::Running);
    let aligned_apk = work_dir.join("app-aligned.apk");
    zipalign(app, &unsigned_apk, &aligned_apk, settings)
        .await
        .map_err(|e| { emit_stage(app, PipelineStage::Align, StageStatus::Failed); e })?;
    emit_stage(app, PipelineStage::Align, StageStatus::Done);
    check_cancel(cancel)?;

    // ── Stage 8: Sign ────────────────────────────────────────────────────────
    emit_stage(app, PipelineStage::Sign, StageStatus::Running);
    let signed_apk = work_dir.join(format!("{}-signed.apk", options.app_name.replace(' ', "_")));
    sign_apk(
        app,
        &aligned_apk,
        &signed_apk,
        &options.storepass,
        &options.keypass,
        settings,
    )
    .await
    .map_err(|e| { emit_stage(app, PipelineStage::Sign, StageStatus::Failed); e })?;
    emit_stage(app, PipelineStage::Sign, StageStatus::Done);
    check_cancel(cancel)?;

    // ── Stage 9: Copy to output ──────────────────────────────────────────────
    emit_stage(app, PipelineStage::Output, StageStatus::Running);
    let output_dir = if settings.output_dir.is_empty() {
        // Default to ~/Downloads
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
        PathBuf::from(home).join("Downloads")
    } else {
        PathBuf::from(&settings.output_dir)
    };
    std::fs::create_dir_all(&output_dir)?;

    let final_apk = output_dir.join(format!(
        "{}-{}.apk",
        options.app_name.replace(' ', "_"),
        options.version_name
    ));
    std::fs::copy(&signed_apk, &final_apk)?;
    emit_log(app, "info", &format!("APK saved to: {}", final_apk.display()));
    emit_stage(app, PipelineStage::Output, StageStatus::Done);

    Ok(BuildResult {
        apk_path: final_apk.to_string_lossy().into_owned(),
        warnings,
    })
}

pub fn suggest_app_id(game_folder_name: &str) -> String {
    make_app_id(game_folder_name)
}
