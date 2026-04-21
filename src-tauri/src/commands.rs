use std::borrow::Cow;
use std::sync::Arc;

use tauri::State;

use crate::config::{AppConfig, ConfigManager};
use crate::security::{
    self, AppLockManager, AppLockStatus, SetPasswordPayload, UnlockPayload,
    UpdateAppLockSettingsPayload,
};
use crate::storage::{Database, SearchQuery, SearchResult, Statistics, Tag};
use crate::sync::webdav::{WebDavConfig, WebDavSyncManager, WebDavSyncStatus};
use crate::sync::{SyncConfig, SyncManager};
use crate::AppDataDir;

fn require_unlocked(lock: &State<'_, Arc<AppLockManager>>) -> Result<(), String> {
    lock.ensure_unlocked()
}

#[tauri::command]
pub async fn get_app_lock_status(
    lock: State<'_, Arc<AppLockManager>>,
) -> Result<AppLockStatus, String> {
    Ok(lock.status())
}

#[tauri::command]
pub async fn set_app_lock_password(
    app: tauri::AppHandle,
    lock: State<'_, Arc<AppLockManager>>,
    payload: SetPasswordPayload,
) -> Result<AppLockStatus, String> {
    let status = lock.set_password(payload)?;
    security::emit_lock_state(&app, &lock);
    Ok(status)
}

#[tauri::command]
pub async fn update_app_lock_settings(
    app: tauri::AppHandle,
    lock: State<'_, Arc<AppLockManager>>,
    payload: UpdateAppLockSettingsPayload,
) -> Result<AppLockStatus, String> {
    let status = lock.update_settings(payload)?;
    security::emit_lock_state(&app, &lock);
    Ok(status)
}

#[tauri::command]
pub async fn lock_app(
    app: tauri::AppHandle,
    lock: State<'_, Arc<AppLockManager>>,
) -> Result<AppLockStatus, String> {
    let status = lock.lock("manual");
    security::emit_lock_state(&app, &lock);
    Ok(status)
}

#[tauri::command]
pub async fn unlock_app(
    app: tauri::AppHandle,
    lock: State<'_, Arc<AppLockManager>>,
    payload: UnlockPayload,
) -> Result<AppLockStatus, String> {
    let prefer_biometric = payload.prefer_biometric.unwrap_or(false);
    if prefer_biometric && lock.status().biometric_enabled {
        match security::try_biometric_unlock() {
            Ok(true) => {
                let status = lock.mark_biometric_unlocked();
                security::emit_lock_state(&app, &lock);
                return Ok(status);
            }
            Ok(false) | Err(_) => {
                // Fall through to password unlock.
            }
        }
    }

    let password = payload
        .password
        .ok_or_else(|| "Password is required".to_string())?;
    match lock.verify_password(&password) {
        Ok(status) => {
            security::emit_lock_state(&app, &lock);
            Ok(status)
        }
        Err(err) => {
            let _ = lock.handle_failed_unlock();
            security::emit_lock_state(&app, &lock);
            Err(err)
        }
    }
}

#[tauri::command]
pub async fn get_entries(
    lock: State<'_, Arc<AppLockManager>>,
    db: State<'_, Arc<Database>>,
    limit: i64,
    offset: i64,
    category: Option<String>,
    is_favorite: Option<bool>,
) -> Result<SearchResult, String> {
    require_unlocked(&lock)?;
    db.get_entries(limit, offset, category.as_deref(), is_favorite)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn search_entries(
    lock: State<'_, Arc<AppLockManager>>,
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
    db.search(&query).map_err(|e| e.to_string())
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
    db: State<'_, Arc<Database>>,
    ids: Vec<i64>,
) -> Result<String, String> {
    require_unlocked(&lock)?;
    let merged = db.merge_entries_content(&ids).map_err(|e| e.to_string())?;

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
    db: State<'_, Arc<Database>>,
    app_data_dir: State<'_, Arc<AppDataDir>>,
) -> Result<Statistics, String> {
    require_unlocked(&lock)?;
    let mut stats = db.get_statistics().map_err(|e| e.to_string())?;

    let db_path = app_data_dir.0.join("clipboard.db");
    stats.storage_size_bytes = std::fs::metadata(&db_path).map(|m| m.len()).unwrap_or(0);

    Ok(stats)
}

#[tauri::command]
pub async fn paste_entry(
    lock: State<'_, Arc<AppLockManager>>,
    db: State<'_, Arc<Database>>,
    id: i64,
) -> Result<(), String> {
    require_unlocked(&lock)?;
    let entry = db
        .get_entry_by_id(id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Entry not found".to_string())?;

    let mut clipboard = arboard::Clipboard::new().map_err(|e| format!("Clipboard error: {}", e))?;

    if entry.content_type == "image" {
        let img = image::open(&entry.content)
            .map_err(|e| format!("Failed to open image: {}", e))?
            .to_rgba8();
        let (w, h) = img.dimensions();
        let arboard_img = arboard::ImageData {
            bytes: Cow::from(img.into_raw()),
            width: w as usize,
            height: h as usize,
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
    db: State<'_, Arc<Database>>,
    tag_id: i64,
) -> Result<Vec<crate::storage::ClipboardEntry>, String> {
    require_unlocked(&lock)?;
    db.get_entries_by_tag(tag_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn transform_content(content: String, transform_type: String) -> Result<String, String> {
    transform::apply_transform(&content, &transform_type)
}

pub mod transform {
    use base64::{engine::general_purpose::STANDARD, Engine};

    pub fn apply_transform(content: &str, transform_type: &str) -> Result<String, String> {
        match transform_type {
            "uppercase" => Ok(content.to_uppercase()),
            "lowercase" => Ok(content.to_lowercase()),
            "title_case" => Ok(to_title_case(content)),
            "url_encode" => Ok(url_encode(content)),
            "url_decode" => url_decode(content),
            "json_format" => json_format(content),
            "json_compact" => json_compact(content),
            "base64_encode" => Ok(STANDARD.encode(content.as_bytes())),
            "base64_decode" => base64_decode(content),
            "trim" => Ok(trim_whitespace(content)),
            "html_escape" => Ok(html_escape(content)),
            "html_unescape" => Ok(html_unescape(content)),
            _ => Err(format!("Unknown transform type: {}", transform_type)),
        }
    }

    fn to_title_case(s: &str) -> String {
        s.split_whitespace()
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => {
                        first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase()
                    }
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn url_encode(s: &str) -> String {
        urlencoding::encode(s).into_owned()
    }
    fn url_decode(s: &str) -> Result<String, String> {
        match urlencoding::decode(s) {
            Ok(v) => Ok(v.into_owned()),
            Err(e) => Err(e.to_string()),
        }
    }
    fn json_format(s: &str) -> Result<String, String> {
        serde_json::from_str::<serde_json::Value>(s)
            .and_then(|v| serde_json::to_string_pretty(&v))
            .map_err(|e| e.to_string())
    }
    fn json_compact(s: &str) -> Result<String, String> {
        serde_json::from_str::<serde_json::Value>(s)
            .and_then(|v| serde_json::to_string(&v))
            .map_err(|e| e.to_string())
    }
    fn base64_decode(s: &str) -> Result<String, String> {
        match STANDARD.decode(s) {
            Ok(bytes) => String::from_utf8(bytes).map_err(|e| e.to_string()),
            Err(e) => Err(e.to_string()),
        }
    }
    fn trim_whitespace(s: &str) -> String {
        s.trim().to_string()
    }
    fn html_escape(s: &str) -> String {
        html_escape::encode_safe(s).to_string()
    }
    fn html_unescape(s: &str) -> String {
        html_escape::decode_html_entities(s).to_string()
    }
}

#[tauri::command]
pub async fn get_sync_status(
    lock: State<'_, Arc<AppLockManager>>,
    sync_manager: State<'_, Arc<SyncManager>>,
) -> Result<crate::storage::SyncStatus, String> {
    require_unlocked(&lock)?;
    sync_manager.get_status()
}

#[tauri::command]
pub async fn get_sync_config(
    lock: State<'_, Arc<AppLockManager>>,
    config: State<'_, Arc<ConfigManager>>,
) -> Result<SyncConfig, String> {
    require_unlocked(&lock)?;
    Ok(config.get().sync)
}

#[tauri::command]
pub async fn update_sync_config(
    lock: State<'_, Arc<AppLockManager>>,
    sync_manager: State<'_, Arc<SyncManager>>,
    config: State<'_, Arc<ConfigManager>>,
    new_config: SyncConfig,
) -> Result<(), String> {
    require_unlocked(&lock)?;
    let mut app_config = config.get();
    app_config.sync = new_config.clone();
    config.update(app_config)?;
    sync_manager.update_config(new_config)
}

#[tauri::command]
pub async fn get_discovered_devices(
    lock: State<'_, Arc<AppLockManager>>,
    sync_manager: State<'_, Arc<SyncManager>>,
) -> Result<Vec<crate::storage::DiscoveredDevice>, String> {
    require_unlocked(&lock)?;
    sync_manager.get_discovered_devices()
}

#[tauri::command]
pub async fn get_paired_devices(
    lock: State<'_, Arc<AppLockManager>>,
    sync_manager: State<'_, Arc<SyncManager>>,
) -> Result<Vec<crate::storage::PairedDevice>, String> {
    require_unlocked(&lock)?;
    sync_manager.get_paired_devices()
}

#[tauri::command]
pub async fn pair_device(
    lock: State<'_, Arc<AppLockManager>>,
    sync_manager: State<'_, Arc<SyncManager>>,
    device_id: String,
) -> Result<(), String> {
    require_unlocked(&lock)?;
    sync_manager.pair_device(&device_id)
}

#[tauri::command]
pub async fn unpair_device(
    lock: State<'_, Arc<AppLockManager>>,
    sync_manager: State<'_, Arc<SyncManager>>,
    device_id: String,
) -> Result<(), String> {
    require_unlocked(&lock)?;
    sync_manager.unpair_device(&device_id)
}

#[tauri::command]
pub async fn toggle_device_sync(
    lock: State<'_, Arc<AppLockManager>>,
    sync_manager: State<'_, Arc<SyncManager>>,
    device_id: String,
    enabled: bool,
) -> Result<(), String> {
    require_unlocked(&lock)?;
    sync_manager.toggle_device_sync(&device_id, enabled)
}

#[tauri::command]
pub async fn webdav_connect(
    lock: State<'_, Arc<AppLockManager>>,
    manager: State<'_, Arc<WebDavSyncManager>>,
    config: WebDavConfig,
) -> Result<(), String> {
    require_unlocked(&lock)?;
    manager
        .connect(
            &config.server_url,
            &config.username,
            &config.password,
            &config.sync_password,
        )
        .await?;
    manager.update_config(config).await;
    Ok(())
}

#[tauri::command]
pub async fn webdav_disconnect(
    lock: State<'_, Arc<AppLockManager>>,
    manager: State<'_, Arc<WebDavSyncManager>>,
) -> Result<(), String> {
    require_unlocked(&lock)?;
    manager.disconnect().await;
    Ok(())
}

#[tauri::command]
pub async fn webdav_get_status(
    lock: State<'_, Arc<AppLockManager>>,
    manager: State<'_, Arc<WebDavSyncManager>>,
) -> Result<WebDavSyncStatus, String> {
    require_unlocked(&lock)?;
    Ok(manager.get_status().await)
}

#[tauri::command]
pub async fn webdav_get_config(
    lock: State<'_, Arc<AppLockManager>>,
    config: State<'_, Arc<ConfigManager>>,
) -> Result<WebDavConfig, String> {
    require_unlocked(&lock)?;
    Ok(config.get().webdav)
}

#[tauri::command]
pub async fn webdav_update_config(
    lock: State<'_, Arc<AppLockManager>>,
    manager: State<'_, Arc<WebDavSyncManager>>,
    config: State<'_, Arc<ConfigManager>>,
    new_config: WebDavConfig,
) -> Result<(), String> {
    require_unlocked(&lock)?;
    let mut app_config = config.get();
    app_config.webdav = new_config.clone();
    config.update(app_config)?;
    manager.update_config(new_config).await;
    Ok(())
}

#[tauri::command]
pub async fn webdav_trigger_sync(
    lock: State<'_, Arc<AppLockManager>>,
    manager: State<'_, Arc<WebDavSyncManager>>,
) -> Result<u32, String> {
    require_unlocked(&lock)?;
    manager.trigger_sync().await
}

#[tauri::command]
pub async fn webdav_remove_device(
    lock: State<'_, Arc<AppLockManager>>,
    manager: State<'_, Arc<WebDavSyncManager>>,
    device_id: String,
) -> Result<(), String> {
    require_unlocked(&lock)?;
    manager.remove_device(&device_id).await
}

#[cfg(test)]
mod command_guard_tests {
    use std::sync::Mutex;

    struct FakeLockManager {
        enabled: bool,
        locked: Mutex<bool>,
    }

    impl FakeLockManager {
        fn new(enabled: bool, locked: bool) -> Self {
            Self {
                enabled,
                locked: Mutex::new(locked),
            }
        }

        fn ensure_unlocked(&self) -> Result<(), String> {
            if self.enabled && *self.locked.lock().unwrap() {
                Err("App is locked".to_string())
            } else {
                Ok(())
            }
        }

        fn unlock(&self) {
            *self.locked.lock().unwrap() = false;
        }
    }

    #[test]
    fn guard_blocks_when_locked() {
        let lock = FakeLockManager::new(true, true);
        let result = lock.ensure_unlocked();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "App is locked");
    }

    #[test]
    fn guard_allows_when_unlocked() {
        let lock = FakeLockManager::new(true, false);
        assert!(lock.ensure_unlocked().is_ok());
    }

    #[test]
    fn guard_allows_after_unlock_transition() {
        let lock = FakeLockManager::new(true, true);
        assert!(lock.ensure_unlocked().is_err());
        lock.unlock();
        assert!(lock.ensure_unlocked().is_ok());
    }
}
