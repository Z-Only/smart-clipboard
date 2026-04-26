use std::sync::Arc;

use tauri::State;

use crate::encryption::EncryptionManager;
use crate::security::AppLockManager;
use crate::storage::{Database, Tag};

use super::{decrypt_entries, require_unlocked};

#[tauri::command]
pub async fn create_tag(
    lock: State<'_, Arc<AppLockManager>>,
    db: State<'_, Arc<Database>>,
    name: String,
) -> Result<Tag, String> {
    require_unlocked(&lock)?;
    db.create_tag(&name).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_tag(
    lock: State<'_, Arc<AppLockManager>>,
    db: State<'_, Arc<Database>>,
    id: i64,
) -> Result<(), String> {
    require_unlocked(&lock)?;
    db.delete_tag(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_all_tags(
    lock: State<'_, Arc<AppLockManager>>,
    db: State<'_, Arc<Database>>,
) -> Result<Vec<Tag>, String> {
    require_unlocked(&lock)?;
    db.get_all_tags().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_tag_to_entry(
    lock: State<'_, Arc<AppLockManager>>,
    db: State<'_, Arc<Database>>,
    entry_id: i64,
    tag_id: i64,
) -> Result<(), String> {
    require_unlocked(&lock)?;
    db.add_tag_to_entry(entry_id, tag_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn remove_tag_from_entry(
    lock: State<'_, Arc<AppLockManager>>,
    db: State<'_, Arc<Database>>,
    entry_id: i64,
    tag_id: i64,
) -> Result<(), String> {
    require_unlocked(&lock)?;
    db.remove_tag_from_entry(entry_id, tag_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_tags_for_entries(
    lock: State<'_, Arc<AppLockManager>>,
    db: State<'_, Arc<Database>>,
    ids: Vec<i64>,
    tag_ids: Vec<i64>,
    mode: Option<String>,
) -> Result<(), String> {
    require_unlocked(&lock)?;
    db.set_tags_for_entries(&ids, &tag_ids, mode.as_deref().unwrap_or("replace"))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_entry_tags(
    lock: State<'_, Arc<AppLockManager>>,
    db: State<'_, Arc<Database>>,
    entry_id: i64,
) -> Result<Vec<Tag>, String> {
    require_unlocked(&lock)?;
    db.get_entry_tags(entry_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_entries_by_tag(
    lock: State<'_, Arc<AppLockManager>>,
    encryption: State<'_, Arc<EncryptionManager>>,
    db: State<'_, Arc<Database>>,
    tag_id: i64,
) -> Result<Vec<crate::storage::ClipboardEntry>, String> {
    require_unlocked(&lock)?;
    let mut entries = db.get_entries_by_tag(tag_id).map_err(|e| e.to_string())?;
    decrypt_entries(&encryption, &mut entries);
    Ok(entries)
}
