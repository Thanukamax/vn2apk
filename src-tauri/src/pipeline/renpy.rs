use crate::error::AppError;
use crate::pipeline::cordova::{build_env, emit_log, stream_cmd};
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

/// Find the renpy.sh launcher script inside the configured SDK path.
fn find_renpy_sh(settings: &AppSettings) -> Result<PathBuf, AppError> {
    if !settings.renpy_sdk_path.is_empty() {
        let sh = Path::new(&settings.renpy_sdk_path).join("renpy.sh");
        if sh.exists() {
            return Ok(sh);
        }
        let bat = Path::new(&settings.renpy_sdk_path).join("renpy.bat");
        if bat.exists() {
            return Ok(bat);
        }
    }
    // Fall back to PATH
    which::which("renpy")
        .map(PathBuf::from)
        .map_err(|_| AppError::msg(
            "renpy.sh not found. Set Ren'Py SDK Path in Settings, or install the Ren'Py SDK."
        ))
}

/// Write a minimal android.json into the game dir if one doesn't exist yet.
/// Ren'Py's android_build needs this to know the package name and app name.
fn ensure_android_json(game_path: &Path, options: &BuildOptions) -> Result<(), AppError> {
    let json_path = game_path.join("android.json");
    if json_path.exists() {
        return Ok(()); // user already configured it
    }
    let orientation = "sensorLandscape";
    let content = format!(
        r#"{{
  "package": "{}",
  "name": "{}",
  "icon_name": "{}",
  "version": "{}",
  "numeric_version": "{}",
  "orientation": "{}",
  "permissions": ["VIBRATE"],
  "include_pil": false,
  "include_sqlite": false,
  "layout": null
}}
"#,
        options.app_id,
        options.app_name,
        options.app_name,
        options.version_name,
        options.version_code,
        orientation,
    );
    std::fs::write(&json_path, content)?;
    Ok(())
}

/// Search all known Ren'Py output locations for the built APK.
fn find_renpy_apk(sdk_path: &str, game_path: &Path, app_name: &str) -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = vec![];

    let safe_name: String = app_name
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' { c } else { '_' })
        .collect();

    // Ren'Py 8.x launcher android_build output: <sdk>/rapt/bin/
    if !sdk_path.is_empty() {
        let rapt_bin = Path::new(sdk_path).join("rapt/bin");
        candidates.push(rapt_bin.join(format!("{}-release-unsigned.apk", safe_name)));
        candidates.push(rapt_bin.join(format!("{}-release.apk", safe_name)));
        candidates.push(rapt_bin.join(format!("{}-debug.apk", safe_name)));
        // Scan any .apk in rapt/bin/
        if let Ok(entries) = std::fs::read_dir(&rapt_bin) {
            for e in entries.flatten() {
                if e.path().extension().map(|x| x == "apk").unwrap_or(false) {
                    candidates.push(e.path());
                }
            }
        }
    }

    // Ren'Py android-project Gradle output inside game dir (some versions)
    let game_android = game_path.join("android-project/app/build/outputs/apk");
    for subdir in &["release", "debug"] {
        for name in &["app-release-unsigned.apk", "app-release.apk", "app-debug.apk"] {
            candidates.push(game_android.join(subdir).join(name));
        }
    }

    candidates.into_iter().find(|p| p.exists())
}

pub async fn run_renpy_pipeline(
    app: AppHandle,
    game_path: PathBuf,
    options: BuildOptions,
    settings: AppSettings,
    cancel: Arc<AtomicBool>,
) {
    match run_pipeline_inner(&app, &game_path, &options, &settings, &cancel).await {
        Ok(result) => { let _ = app.emit("pipeline-done", result); }
        Err(e)     => { let _ = app.emit("pipeline-error", e.to_string()); }
    }
}

async fn run_pipeline_inner(
    app: &AppHandle,
    game_path: &Path,
    options: &BuildOptions,
    settings: &AppSettings,
    cancel: &AtomicBool,
) -> Result<BuildResult, AppError> {
    let warnings: Vec<String> = vec![];

    // ── Stage 1: Validate ────────────────────────────────────────────────────
    emit_stage(app, PipelineStage::Validate, StageStatus::Running);
    emit_log(app, "info", "Validating Ren'Py game folder...");

    if !game_path.exists() {
        emit_stage(app, PipelineStage::Validate, StageStatus::Failed);
        return Err(AppError::msg("Game folder does not exist"));
    }
    if !game_path.join("game").is_dir() {
        emit_stage(app, PipelineStage::Validate, StageStatus::Failed);
        return Err(AppError::msg("Missing game/ directory — is this a Ren'Py project?"));
    }
    let renpy_sh = find_renpy_sh(settings).map_err(|e| {
        emit_stage(app, PipelineStage::Validate, StageStatus::Failed);
        e
    })?;
    emit_stage(app, PipelineStage::Validate, StageStatus::Done);
    check_cancel(cancel)?;

    // ── Stages 2-5: Cordova-specific stages are N/A; mark done quickly ───────
    for stage in [
        PipelineStage::Scaffold,
        PipelineStage::CopyAssets,
        PipelineStage::PatchConfig,
        PipelineStage::AddPlatform,
    ] {
        emit_stage(app, stage, StageStatus::Done);
    }

    // ── Stage 6: Build via Ren'Py android_build ──────────────────────────────
    emit_stage(app, PipelineStage::Build, StageStatus::Running);
    emit_log(app, "info", "Preparing Ren'Py Android configuration...");

    ensure_android_json(game_path, options).map_err(|e| {
        emit_stage(app, PipelineStage::Build, StageStatus::Failed);
        e
    })?;

    let env = build_env(settings);
    let renpy_str = renpy_sh.to_string_lossy().into_owned();
    let game_str = game_path.to_string_lossy().into_owned();
    // The android_build command is registered by the launcher game, not the base engine.
    // Correct invocation: renpy.sh <sdk>/launcher android_build <game_path>
    let launcher_str = Path::new(&settings.renpy_sdk_path)
        .join("launcher")
        .to_string_lossy()
        .into_owned();

    emit_log(app, "info", "Running Ren'Py android_build (this can take several minutes)...");
    stream_cmd(
        app,
        &renpy_str,
        &[&launcher_str, "android_build", &game_str],
        game_path,
        &env,
    )
    .await
    .map_err(|e| {
        emit_stage(app, PipelineStage::Build, StageStatus::Failed);
        e
    })?;
    check_cancel(cancel)?;

    let unsigned_apk = find_renpy_apk(&settings.renpy_sdk_path, game_path, &options.app_name)
        .ok_or_else(|| {
            emit_stage(app, PipelineStage::Build, StageStatus::Failed);
            AppError::msg(
                "Ren'Py build succeeded but APK not found. \
                 Check build logs for the output path.",
            )
        })?;
    emit_log(app, "info", &format!("Found APK: {}", unsigned_apk.display()));
    emit_stage(app, PipelineStage::Build, StageStatus::Done);
    check_cancel(cancel)?;

    // ── Stage 7: Zipalign ────────────────────────────────────────────────────
    emit_stage(app, PipelineStage::Align, StageStatus::Running);
    let work_dir = game_path.parent().unwrap_or(Path::new("/tmp"));
    let aligned_apk = work_dir.join("renpy-aligned.apk");
    zipalign(app, &unsigned_apk, &aligned_apk, settings)
        .await
        .map_err(|e| { emit_stage(app, PipelineStage::Align, StageStatus::Failed); e })?;
    emit_stage(app, PipelineStage::Align, StageStatus::Done);
    check_cancel(cancel)?;

    // ── Stage 8: Sign ────────────────────────────────────────────────────────
    emit_stage(app, PipelineStage::Sign, StageStatus::Running);
    let signed_apk = work_dir.join(format!("{}-signed.apk", options.app_name.replace(' ', "_")));
    sign_apk(app, &aligned_apk, &signed_apk, &options.storepass, &options.keypass, settings)
        .await
        .map_err(|e| { emit_stage(app, PipelineStage::Sign, StageStatus::Failed); e })?;
    emit_stage(app, PipelineStage::Sign, StageStatus::Done);
    check_cancel(cancel)?;

    // ── Stage 9: Copy to output ──────────────────────────────────────────────
    emit_stage(app, PipelineStage::Output, StageStatus::Running);
    let output_dir = if settings.output_dir.is_empty() {
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
