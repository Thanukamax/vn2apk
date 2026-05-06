use crate::error::AppError;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::Manager;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub android_sdk_path: String,
    pub gradle_home: String,
    pub java_home: String,
    pub node_path: String,
    pub cordova_path: String,
    pub keystore_path: String,
    pub keystore_alias: String,
    pub output_dir: String,
    pub build_tools_version: String,
    pub renpy_sdk_path: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            android_sdk_path: String::new(),
            gradle_home: String::new(),
            java_home: String::new(),
            node_path: String::new(),
            cordova_path: String::new(),
            keystore_path: String::new(),
            keystore_alias: "key0".into(),
            output_dir: String::new(),
            build_tools_version: "34.0.0".into(),
            renpy_sdk_path: String::new(),
        }
    }
}

fn settings_path(app_handle: &tauri::AppHandle) -> Result<PathBuf, AppError> {
    let data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| AppError::msg(format!("Could not resolve app data dir: {e}")))?;
    std::fs::create_dir_all(&data_dir)?;
    Ok(data_dir.join("settings.json"))
}

pub fn load_settings(app_handle: &tauri::AppHandle) -> Result<AppSettings, AppError> {
    let path = settings_path(app_handle)?;
    if !path.exists() {
        return Ok(AppSettings::default());
    }
    let content = std::fs::read_to_string(&path)?;
    let settings: AppSettings = serde_json::from_str(&content)?;
    Ok(settings)
}

pub fn save_settings_to_disk(
    app_handle: &tauri::AppHandle,
    settings: &AppSettings,
) -> Result<(), AppError> {
    let path = settings_path(app_handle)?;
    let content = serde_json::to_string_pretty(settings)?;
    std::fs::write(&path, content)?;
    Ok(())
}
