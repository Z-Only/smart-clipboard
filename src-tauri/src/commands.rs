use std::sync::Arc;

use tauri::State;

use crate::config::{AppConfig, ConfigManager};
use crate::storage::{Database, SearchQuery, SearchResult};

#[tauri::command]
pub async fn get_entries(
    db: State<'_, Arc<Database>>,
    limit: i64,
    offset: i64,
    category: Option<String>,
) -> Result<SearchResult, String> {
    db.get_entries(limit, offset, category.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn search_entries(
    db: State<'_, Arc<Database>>,
    keyword: String,
    category: Option<String>,
    limit: i64,
    offset: i64,
) -> Result<SearchResult, String> {
    let query = SearchQuery {
        keyword: Some(keyword),
        category,
        is_favorite: None,
        limit,
        offset,
    };
    db.search(&query).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_entry(db: State<'_, Arc<Database>>, id: i64) -> Result<(), String> {
    db.delete_entry(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn toggle_favorite(db: State<'_, Arc<Database>>, id: i64) -> Result<bool, String> {
    db.toggle_favorite(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_entry_count(db: State<'_, Arc<Database>>) -> Result<i64, String> {
    db.get_entry_count().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn paste_entry(db: State<'_, Arc<Database>>, id: i64) -> Result<(), String> {
    let entry = db
        .get_entry_by_id(id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Entry not found".to_string())?;

    let mut clipboard =
        arboard::Clipboard::new().map_err(|e| format!("Clipboard error: {}", e))?;
    clipboard
        .set_text(&entry.content)
        .map_err(|e| format!("Clipboard error: {}", e))?;

    db.update_use_count(&entry.hash)
        .map_err(|e| e.to_string())?;

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
