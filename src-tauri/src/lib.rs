mod autodetect;
mod error;
mod pipeline;
mod settings;
mod state;
mod toolchain;

use error::AppError;
use pipeline::engine_detect::{detect_engine, validate_game_folder, EngineDetectResult};
use pipeline::renpy::run_renpy_pipeline;
use pipeline::rpgmv::{run_rpgmv_pipeline, suggest_app_id};
use pipeline::types::{BuildOptions, ValidationResult};
use settings::{load_settings, save_settings_to_disk, AppSettings};
use state::AppState;
use std::sync::{atomic::Ordering, Arc};
use tauri::State;
use toolchain::detect::check_all_tools;
use toolchain::install::{generate_keystore, install_tool};
use toolchain::types::ToolStatus;

// ── Auto-detect ───────────────────────────────────────────────────────────────

#[tauri::command]
async fn cmd_autodetect_settings() -> Result<AppSettings, AppError> {
    Ok(autodetect::autodetect())
}

// ── Settings ────────────────────────────────────────────────────────────────

#[tauri::command]
async fn cmd_get_settings(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<AppSettings, AppError> {
    let loaded = load_settings(&app_handle)?;
    *state.settings.lock().unwrap() = loaded.clone();
    Ok(loaded)
}

#[tauri::command]
async fn cmd_save_settings(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
    settings: AppSettings,
) -> Result<(), AppError> {
    save_settings_to_disk(&app_handle, &settings)?;
    *state.settings.lock().unwrap() = settings;
    Ok(())
}

// ── Toolchain ────────────────────────────────────────────────────────────────

#[tauri::command]
async fn cmd_check_toolchain(state: State<'_, AppState>) -> Result<Vec<ToolStatus>, AppError> {
    let settings = state.settings.lock().unwrap().clone();
    Ok(check_all_tools(&settings))
}

#[tauri::command]
async fn cmd_install_tool(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
    tool_name: String,
) -> Result<(), AppError> {
    let settings = state.settings.lock().unwrap().clone();
    if let Some(updated) = install_tool(&app_handle, &tool_name, &settings).await? {
        save_settings_to_disk(&app_handle, &updated)?;
        *state.settings.lock().unwrap() = updated;
    }
    Ok(())
}

#[tauri::command]
async fn cmd_generate_keystore(
    app_handle: tauri::AppHandle,
    keystore_path: String,
    alias: String,
    storepass: String,
    keypass: String,
    dname: String,
) -> Result<(), AppError> {
    generate_keystore(&app_handle, &keystore_path, &alias, &storepass, &keypass, &dname).await
}

// ── Pipeline ─────────────────────────────────────────────────────────────────

#[tauri::command]
async fn cmd_detect_engine(game_path: String) -> Result<EngineDetectResult, AppError> {
    detect_engine(&game_path)
}

#[tauri::command]
async fn cmd_validate_game_folder(game_path: String) -> Result<ValidationResult, AppError> {
    validate_game_folder(&game_path)
}

#[tauri::command]
async fn cmd_suggest_app_id(folder_name: String) -> Result<String, AppError> {
    Ok(suggest_app_id(&folder_name))
}

#[tauri::command]
async fn cmd_start_build(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
    game_path: String,
    build_options: BuildOptions,
) -> Result<(), AppError> {
    // Reset cancel flag
    state.build_cancel.store(false, Ordering::SeqCst);

    let settings = state.settings.lock().unwrap().clone();
    let cancel = Arc::clone(&state.build_cancel);
    let game_path = std::path::PathBuf::from(game_path);

    if build_options.engine == "RenPy" {
        tauri::async_runtime::spawn(run_renpy_pipeline(
            app_handle,
            game_path,
            build_options,
            settings,
            cancel,
        ));
    } else {
        tauri::async_runtime::spawn(run_rpgmv_pipeline(
            app_handle,
            game_path,
            build_options,
            settings,
            cancel,
        ));
    }

    Ok(())
}

#[tauri::command]
async fn cmd_cancel_build(state: State<'_, AppState>) -> Result<(), AppError> {
    state.build_cancel.store(true, Ordering::SeqCst);
    Ok(())
}

#[tauri::command]
async fn cmd_open_file_manager(path: String) -> Result<(), AppError> {
    #[cfg(target_os = "linux")]
    std::process::Command::new("xdg-open").arg(&path).spawn()?;
    #[cfg(target_os = "windows")]
    std::process::Command::new("explorer").arg(&path).spawn()?;
    #[cfg(target_os = "macos")]
    std::process::Command::new("open").arg(&path).spawn()?;
    Ok(())
}

// ── App builder ───────────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_fs::init())
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            cmd_get_settings,
            cmd_save_settings,
            cmd_check_toolchain,
            cmd_install_tool,
            cmd_generate_keystore,
            cmd_detect_engine,
            cmd_validate_game_folder,
            cmd_suggest_app_id,
            cmd_start_build,
            cmd_cancel_build,
            cmd_open_file_manager,
            cmd_autodetect_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
