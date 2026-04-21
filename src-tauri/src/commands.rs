use std::borrow::Cow;
use std::sync::Arc;

use base64::{engine::general_purpose::STANDARD, Engine};
use tauri::State;

use crate::config::{AppConfig, ConfigManager};
use crate::storage::{Database, SearchQuery, SearchResult, Statistics, Tag};
use crate::sync::webdav::{WebDavConfig, WebDavSyncManager, WebDavSyncStatus};
use crate::sync::{SyncConfig, SyncManager};
use crate::AppDataDir;

#[tauri::command]
pub async fn get_entries(
    db: State<'_, Arc<Database>>,
    limit: i64,
    offset: i64,
    category: Option<String>,
    is_favorite: Option<bool>,
) -> Result<SearchResult, String> {
    db.get_entries(limit, offset, category.as_deref(), is_favorite)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn search_entries(
    db: State<'_, Arc<Database>>,
    keyword: String,
    category: Option<String>,
    is_favorite: Option<bool>,
    limit: i64,
    offset: i64,
) -> Result<SearchResult, String> {
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
    db: State<'_, Arc<Database>>,
    app_data_dir: State<'_, Arc<AppDataDir>>,
    id: i64,
) -> Result<(), String> {
    db.delete_entry_with_cleanup(id, &app_data_dir.0)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_entries(
    db: State<'_, Arc<Database>>,
    app_data_dir: State<'_, Arc<AppDataDir>>,
    ids: Vec<i64>,
) -> Result<i64, String> {
    db.delete_entries_with_cleanup(&ids, &app_data_dir.0)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn copy_entries(
    db: State<'_, Arc<Database>>,
    ids: Vec<i64>,
) -> Result<String, String> {
    let merged = db
        .merge_entries_content(&ids)
        .map_err(|e| e.to_string())?;

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
    db: State<'_, Arc<Database>>,
    ids: Vec<i64>,
    favorite: bool,
) -> Result<i64, String> {
    db.set_favorite_state_for_entries(&ids, favorite)
        .map_err(|e| e.to_string())
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
pub async fn get_statistics(
    db: State<'_, Arc<Database>>,
    app_data_dir: State<'_, Arc<AppDataDir>>,
) -> Result<Statistics, String> {
    let mut stats = db.get_statistics().map_err(|e| e.to_string())?;

    let db_path = app_data_dir.0.join("clipboard.db");
    stats.storage_size_bytes = std::fs::metadata(&db_path).map(|m| m.len()).unwrap_or(0);

    Ok(stats)
}

#[tauri::command]
pub async fn paste_entry(db: State<'_, Arc<Database>>, id: i64) -> Result<(), String> {
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
pub async fn create_tag(db: State<'_, Arc<Database>>, name: String) -> Result<Tag, String> {
    db.create_tag(&name).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_tag(db: State<'_, Arc<Database>>, id: i64) -> Result<(), String> {
    db.delete_tag(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_all_tags(db: State<'_, Arc<Database>>) -> Result<Vec<Tag>, String> {
    db.get_all_tags().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_tag_to_entry(
    db: State<'_, Arc<Database>>,
    entry_id: i64,
    tag_id: i64,
) -> Result<(), String> {
    db.add_tag_to_entry(entry_id, tag_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn remove_tag_from_entry(
    db: State<'_, Arc<Database>>,
    entry_id: i64,
    tag_id: i64,
) -> Result<(), String> {
    db.remove_tag_from_entry(entry_id, tag_id)
        .map_err(|e| e.to_string())
}


#[tauri::command]
pub async fn set_tags_for_entries(
    db: State<'_, Arc<Database>>,
    ids: Vec<i64>,
    tag_ids: Vec<i64>,
    mode: Option<String>,
) -> Result<(), String> {
    db.set_tags_for_entries(&ids, &tag_ids, mode.as_deref().unwrap_or("replace"))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_entry_tags(
    db: State<'_, Arc<Database>>,
    entry_id: i64,
) -> Result<Vec<Tag>, String> {
    db.get_entry_tags(entry_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_entries_by_tag(
    db: State<'_, Arc<Database>>,
    tag_id: i64,
) -> Result<Vec<crate::storage::ClipboardEntry>, String> {
    db.get_entries_by_tag(tag_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn transform_content(content: String, transform_type: String) -> Result<String, String> {
    transform::apply_transform(&content, &transform_type)
}

pub mod transform {
    use super::*;

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

    fn to_title_case(s: &str) -> String { s.split_whitespace().map(|word| {
        let mut chars = word.chars();
        match chars.next() {
            None => String::new(),
            Some(first) => first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase(),
        }
    }).collect::<Vec<_>>().join(" ") }

    fn url_encode(s: &str) -> String { urlencoding::encode(s).into_owned() }
    fn url_decode(s: &str) -> Result<String, String> { urlencoding::decode(s).map(|v| v.into_owned()).map_err(|e| e.to_string()) }
    fn json_format(s: &str) -> Result<String, String> { serde_json::from_str::<serde_json::Value>(s).and_then(|v| serde_json::to_string_pretty(&v)).map_err(|e| e.to_string()) }
    fn json_compact(s: &str) -> Result<String, String> { serde_json::from_str::<serde_json::Value>(s).and_then(|v| serde_json::to_string(&v)).map_err(|e| e.to_string()) }
    fn base64_decode(s: &str) -> Result<String, String> { STANDARD.decode(s).map_err(|e| e.to_string()).and_then(|bytes| String::from_utf8(bytes).map_err(|e| e.to_string())) }
    fn trim_whitespace(s: &str) -> String { s.trim().to_string() }
    fn html_escape(s: &str) -> String { html_escape::encode_safe(s).to_string() }
    fn html_unescape(s: &str) -> String { html_escape::decode_html_entities(s).to_string() }
}

include!(concat!(env!("OUT_DIR"), "/commands_sync_webdav.rs"));
