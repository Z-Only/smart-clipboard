use std::sync::Arc;

use tauri::State;

use crate::config::{AppConfig, ConfigManager};

#[tauri::command]
pub async fn quit_app(app: tauri::AppHandle<impl tauri::Runtime>) -> Result<(), String> {
    app.exit(0);
    Ok(())
}

#[tauri::command]
pub async fn get_config(config: State<'_, Arc<ConfigManager>>) -> Result<AppConfig, String> {
    Ok(config.get())
}

#[tauri::command]
pub async fn update_config(
    config: State<'_, Arc<ConfigManager>>,
    new_config: AppConfig,
) -> Result<(), String> {
    config.update(new_config)
}

#[tauri::command]
pub async fn get_autostart_enabled(app: tauri::AppHandle) -> Result<bool, String> {
    use tauri_plugin_autostart::ManagerExt;
    app.autolaunch().is_enabled().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_autostart_enabled(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    use tauri_plugin_autostart::ManagerExt;
    if enabled {
        app.autolaunch().enable().map_err(|e| e.to_string())
    } else {
        app.autolaunch().disable().map_err(|e| e.to_string())
    }
}
