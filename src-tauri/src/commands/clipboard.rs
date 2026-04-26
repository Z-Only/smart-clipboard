use std::borrow::Cow;
use std::sync::Arc;

use tauri::State;

use crate::encryption::EncryptionManager;
use crate::security::AppLockManager;
use crate::storage::{Database, SearchQuery, SearchResult, Statistics};
use crate::AppDataDir;

use super::{decrypt_entries, decrypt_search_result, require_unlocked};

#[tauri::command]
pub async fn get_entries(
    lock: State<'_, Arc<AppLockManager>>,
    encryption: State<'_, Arc<EncryptionManager>>,
    db: State<'_, Arc<Database>>,
    limit: i64,
    offset: i64,
    category: Option<String>,
    is_favorite: Option<bool>,
) -> Result<SearchResult, String> {
    require_unlocked(&lock)?;
    let mut result = db
        .get_entries(limit, offset, category.as_deref(), is_favorite)
        .map_err(|e| e.to_string())?;
    decrypt_search_result(&encryption, &mut result);
    Ok(result)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn search_entries(
    lock: State<'_, Arc<AppLockManager>>,
    encryption: State<'_, Arc<EncryptionManager>>,
    db: State<'_, Arc<Database>>,
    keyword: String,
    category: Option<String>,
    is_favorite: Option<bool>,
    limit: i64,
    offset: i64,
) -> Result<SearchResult, String> {
    require_unlocked(&lock)?;
    let query = SearchQuery {
        keyword: Some(keyword),
        category,
        is_favorite,
        limit,
        offset,
    };
    let mut result = db.search(&query).map_err(|e| e.to_string())?;
    decrypt_search_result(&encryption, &mut result);
    Ok(result)
}

#[tauri::command]
pub async fn delete_entry(
    lock: State<'_, Arc<AppLockManager>>,
    db: State<'_, Arc<Database>>,
    app_data_dir: State<'_, Arc<AppDataDir>>,
    id: i64,
) -> Result<(), String> {
    require_unlocked(&lock)?;
    db.delete_entry_with_cleanup(id, &app_data_dir.0)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_entries(
    lock: State<'_, Arc<AppLockManager>>,
    db: State<'_, Arc<Database>>,
    app_data_dir: State<'_, Arc<AppDataDir>>,
    ids: Vec<i64>,
) -> Result<i64, String> {
    require_unlocked(&lock)?;
    db.delete_entries_with_cleanup(&ids, &app_data_dir.0)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn copy_entries(
    lock: State<'_, Arc<AppLockManager>>,
    encryption: State<'_, Arc<EncryptionManager>>,
    db: State<'_, Arc<Database>>,
    ids: Vec<i64>,
) -> Result<String, String> {
    require_unlocked(&lock)?;
    let mut entries = db.get_entries_by_ids(&ids).map_err(|e| e.to_string())?;
    decrypt_entries(&encryption, &mut entries);

    let merged = entries
        .into_iter()
        .filter(|entry| entry.content_type != "image")
        .map(|entry| entry.content.trim().to_string())
        .filter(|content| !content.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n");

    if merged.trim().is_empty() {
        return Ok(String::new());
    }

    let mut clipboard = arboard::Clipboard::new().map_err(|e| format!("Clipboard error: {}", e))?;
    clipboard
        .set_text(&merged)
        .map_err(|e| format!("Clipboard error: {}", e))?;

    Ok(merged)
}

#[tauri::command]
pub async fn set_favorite_state_for_entries(
    lock: State<'_, Arc<AppLockManager>>,
    db: State<'_, Arc<Database>>,
    ids: Vec<i64>,
    favorite: bool,
) -> Result<i64, String> {
    require_unlocked(&lock)?;
    db.set_favorite_state_for_entries(&ids, favorite)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn toggle_favorite(
    lock: State<'_, Arc<AppLockManager>>,
    db: State<'_, Arc<Database>>,
    id: i64,
) -> Result<bool, String> {
    require_unlocked(&lock)?;
    db.toggle_favorite(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_entry_count(
    lock: State<'_, Arc<AppLockManager>>,
    db: State<'_, Arc<Database>>,
) -> Result<i64, String> {
    require_unlocked(&lock)?;
    db.get_entry_count().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_statistics(
    lock: State<'_, Arc<AppLockManager>>,
    encryption: State<'_, Arc<EncryptionManager>>,
    db: State<'_, Arc<Database>>,
    app_data_dir: State<'_, Arc<AppDataDir>>,
) -> Result<Statistics, String> {
    require_unlocked(&lock)?;
    let mut stats = db.get_statistics().map_err(|e| e.to_string())?;
    decrypt_entries(&encryption, &mut stats.most_used);

    let db_path = app_data_dir.0.join("clipboard.db");
    stats.storage_size_bytes = std::fs::metadata(&db_path).map(|m| m.len()).unwrap_or(0);

    Ok(stats)
}

#[tauri::command]
pub async fn paste_entry(
    lock: State<'_, Arc<AppLockManager>>,
    encryption: State<'_, Arc<EncryptionManager>>,
    db: State<'_, Arc<Database>>,
    id: i64,
) -> Result<(), String> {
    require_unlocked(&lock)?;
    let mut entry = db
        .get_entry_by_id(id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Entry not found".to_string())?;

    if let Ok(decrypted) = encryption.decrypt_content(&entry.content) {
        entry.content = decrypted;
    }

    let mut clipboard = arboard::Clipboard::new().map_err(|e| format!("Clipboard error: {}", e))?;

    if entry.content_type == "image" {
        let img = image::open(&entry.content)
            .map_err(|e| format!("Failed to open image: {}", e))?
            .to_rgba8();
        let (width, height) = img.dimensions();
        let arboard_img = arboard::ImageData {
            bytes: Cow::from(img.into_raw()),
            width: width as usize,
            height: height as usize,
        };
        clipboard
            .set_image(arboard_img)
            .map_err(|e| format!("Clipboard error: {}", e))?;
    } else {
        clipboard
            .set_text(&entry.content)
            .map_err(|e| format!("Clipboard error: {}", e))?;
    }

    db.update_use_count(&entry.hash)
        .map_err(|e| e.to_string())?;

    Ok(())
}
