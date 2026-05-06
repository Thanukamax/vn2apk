use crate::error::AppError;
use crate::pipeline::cordova::{emit_log, stream_cmd, build_env};
use crate::settings::AppSettings;
use std::path::{Path, PathBuf};
use tauri::AppHandle;

fn build_tools_bin(settings: &AppSettings) -> PathBuf {
    Path::new(&settings.android_sdk_path)
        .join("build-tools")
        .join(&settings.build_tools_version)
}

pub async fn zipalign(
    app: &AppHandle,
    unsigned_apk: &Path,
    aligned_apk: &Path,
    settings: &AppSettings,
) -> Result<(), AppError> {
    let bin = build_tools_bin(settings).join("zipalign");
    let bin_str = bin.to_string_lossy().into_owned();
    let unsigned_str = unsigned_apk.to_string_lossy().into_owned();
    let aligned_str = aligned_apk.to_string_lossy().into_owned();
    let env = build_env(settings);
    let cwd = unsigned_apk.parent().unwrap_or(Path::new("/tmp"));

    stream_cmd(
        app,
        &bin_str,
        &["-f", "4", &unsigned_str, &aligned_str],
        cwd,
        &env,
    )
    .await
}

pub async fn sign_apk(
    app: &AppHandle,
    aligned_apk: &Path,
    signed_apk: &Path,
    storepass: &str,
    keypass: &str,
    settings: &AppSettings,
) -> Result<(), AppError> {
    if settings.keystore_path.is_empty() {
        return Err(AppError::msg(
            "Keystore path not set. Configure it in Settings before building.",
        ));
    }
    if !Path::new(&settings.keystore_path).exists() {
        return Err(AppError::msg(format!(
            "Keystore not found: {}. Generate one in Settings.",
            settings.keystore_path
        )));
    }

    let bin = build_tools_bin(settings).join("apksigner");
    let bin_str = bin.to_string_lossy().into_owned();
    let aligned_str = aligned_apk.to_string_lossy().into_owned();
    let signed_str = signed_apk.to_string_lossy().into_owned();
    let sp = format!("pass:{storepass}");
    let kp = format!("pass:{keypass}");
    let env = build_env(settings);
    let cwd = aligned_apk.parent().unwrap_or(Path::new("/tmp"));

    emit_log(app, "info", "Signing APK...");
    stream_cmd(
        app,
        &bin_str,
        &[
            "sign",
            "--ks", &settings.keystore_path,
            "--ks-key-alias", &settings.keystore_alias,
            "--ks-pass", &sp,
            "--key-pass", &kp,
            "--out", &signed_str,
            &aligned_str,
        ],
        cwd,
        &env,
    )
    .await
}
