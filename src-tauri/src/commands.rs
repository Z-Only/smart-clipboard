use std::borrow::Cow;
use std::sync::Arc;

use base64::{engine::general_purpose::STANDARD, Engine};
use tauri::State;

use crate::config::{AppConfig, ConfigManager};
use crate::storage::{Database, SearchQuery, SearchResult, Statistics, Tag};
use crate::sync::{SyncConfig, SyncManager};
use crate::AppDataDir;

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
pub async fn delete_entry(
    db: State<'_, Arc<Database>>,
    app_data_dir: State<'_, Arc<AppDataDir>>,
    id: i64,
) -> Result<(), String> {
    db.delete_entry_with_cleanup(id, &app_data_dir.0)
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

    // Compute storage size from the DB file
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
        // Load image from file and set to clipboard
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

// --- Tag management commands ---

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

    fn to_title_case(s: &str) -> String {
        s.split_whitespace()
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => {
                        let upper: String = first.to_uppercase().collect();
                        upper + &chars.as_str().to_lowercase()
                    }
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn url_encode(s: &str) -> String {
        s.chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || "-_.~".contains(c) {
                    c.to_string()
                } else {
                    let mut buf = [0u8; 4];
                    let encoded = c.encode_utf8(&mut buf);
                    encoded
                        .bytes()
                        .map(|b| format!("%{:02X}", b))
                        .collect::<String>()
                }
            })
            .collect()
    }

    fn url_decode(s: &str) -> Result<String, String> {
        let mut result = Vec::new();
        let bytes = s.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'%' && i + 2 < bytes.len() {
                let hex = &s[i + 1..i + 3];
                match u8::from_str_radix(hex, 16) {
                    Ok(byte) => {
                        result.push(byte);
                        i += 3;
                    }
                    Err(_) => {
                        result.push(bytes[i]);
                        i += 1;
                    }
                }
            } else if bytes[i] == b'+' {
                result.push(b' ');
                i += 1;
            } else {
                result.push(bytes[i]);
                i += 1;
            }
        }
        String::from_utf8(result).map_err(|e| format!("Invalid UTF-8 in URL decode: {}", e))
    }

    fn json_format(s: &str) -> Result<String, String> {
        let value: serde_json::Value =
            serde_json::from_str(s).map_err(|e| format!("Invalid JSON: {}", e))?;
        serde_json::to_string_pretty(&value).map_err(|e| format!("JSON format error: {}", e))
    }

    fn json_compact(s: &str) -> Result<String, String> {
        let value: serde_json::Value =
            serde_json::from_str(s).map_err(|e| format!("Invalid JSON: {}", e))?;
        serde_json::to_string(&value).map_err(|e| format!("JSON compact error: {}", e))
    }

    fn base64_decode(s: &str) -> Result<String, String> {
        let bytes = STANDARD
            .decode(s.trim())
            .map_err(|e| format!("Invalid Base64: {}", e))?;
        String::from_utf8(bytes).map_err(|e| format!("Invalid UTF-8 in Base64 decode: {}", e))
    }

    fn trim_whitespace(s: &str) -> String {
        // Trim leading/trailing whitespace and collapse internal whitespace sequences
        s.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    fn html_escape(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&#39;")
    }

    fn html_unescape(s: &str) -> String {
        s.replace("&#39;", "'")
            .replace("&quot;", "\"")
            .replace("&gt;", ">")
            .replace("&lt;", "<")
            .replace("&amp;", "&")
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_uppercase() {
            assert_eq!(
                apply_transform("hello world", "uppercase").unwrap(),
                "HELLO WORLD"
            );
        }

        #[test]
        fn test_lowercase() {
            assert_eq!(
                apply_transform("HELLO WORLD", "lowercase").unwrap(),
                "hello world"
            );
        }

        #[test]
        fn test_title_case() {
            assert_eq!(
                apply_transform("hello world", "title_case").unwrap(),
                "Hello World"
            );
            assert_eq!(
                apply_transform("HELLO WORLD", "title_case").unwrap(),
                "Hello World"
            );
            assert_eq!(
                apply_transform("hELLO wORLD", "title_case").unwrap(),
                "Hello World"
            );
        }

        #[test]
        fn test_url_encode() {
            assert_eq!(
                apply_transform("hello world", "url_encode").unwrap(),
                "hello%20world"
            );
            assert_eq!(
                apply_transform("foo=bar&baz=qux", "url_encode").unwrap(),
                "foo%3Dbar%26baz%3Dqux"
            );
            assert_eq!(
                apply_transform("a-b_c.d~e", "url_encode").unwrap(),
                "a-b_c.d~e"
            );
        }

        #[test]
        fn test_url_decode() {
            assert_eq!(
                apply_transform("hello%20world", "url_decode").unwrap(),
                "hello world"
            );
            assert_eq!(
                apply_transform("foo%3Dbar%26baz%3Dqux", "url_decode").unwrap(),
                "foo=bar&baz=qux"
            );
            assert_eq!(
                apply_transform("hello+world", "url_decode").unwrap(),
                "hello world"
            );
        }

        #[test]
        fn test_url_encode_decode_roundtrip() {
            let original = "hello world! foo@bar.com";
            let encoded = apply_transform(original, "url_encode").unwrap();
            let decoded = apply_transform(&encoded, "url_decode").unwrap();
            assert_eq!(decoded, original);
        }

        #[test]
        fn test_json_format() {
            let input = r#"{"name":"John","age":30}"#;
            let result = apply_transform(input, "json_format").unwrap();
            assert!(result.contains('\n'));
            assert!(result.contains("  \"name\": \"John\""));
        }

        #[test]
        fn test_json_compact() {
            let input = "{\n  \"name\": \"John\",\n  \"age\": 30\n}";
            let result = apply_transform(input, "json_compact").unwrap();
            // serde_json may reorder keys alphabetically
            assert!(!result.contains('\n'));
            assert!(result.contains("\"name\":\"John\""));
            assert!(result.contains("\"age\":30"));
        }

        #[test]
        fn test_json_format_invalid() {
            let result = apply_transform("not json", "json_format");
            assert!(result.is_err());
            assert!(result.unwrap_err().contains("Invalid JSON"));
        }

        #[test]
        fn test_json_format_compact_roundtrip() {
            let compact = r#"{"a":1,"b":[2,3]}"#;
            let formatted = apply_transform(compact, "json_format").unwrap();
            let recompacted = apply_transform(&formatted, "json_compact").unwrap();
            assert_eq!(recompacted, compact);
        }

        #[test]
        fn test_base64_encode() {
            assert_eq!(
                apply_transform("Hello, World!", "base64_encode").unwrap(),
                "SGVsbG8sIFdvcmxkIQ=="
            );
        }

        #[test]
        fn test_base64_decode() {
            assert_eq!(
                apply_transform("SGVsbG8sIFdvcmxkIQ==", "base64_decode").unwrap(),
                "Hello, World!"
            );
        }

        #[test]
        fn test_base64_decode_invalid() {
            let result = apply_transform("not-valid-base64!!!", "base64_decode");
            assert!(result.is_err());
        }

        #[test]
        fn test_base64_roundtrip() {
            let original = "The quick brown fox jumps over the lazy dog";
            let encoded = apply_transform(original, "base64_encode").unwrap();
            let decoded = apply_transform(&encoded, "base64_decode").unwrap();
            assert_eq!(decoded, original);
        }

        #[test]
        fn test_trim() {
            assert_eq!(
                apply_transform("  hello   world  ", "trim").unwrap(),
                "hello world"
            );
            assert_eq!(
                apply_transform("  \n  hello  \t  world  \n  ", "trim").unwrap(),
                "hello world"
            );
        }

        #[test]
        fn test_html_escape() {
            assert_eq!(
                apply_transform("<div class=\"test\">a & b</div>", "html_escape").unwrap(),
                "&lt;div class=&quot;test&quot;&gt;a &amp; b&lt;/div&gt;"
            );
            assert_eq!(
                apply_transform("it's a test", "html_escape").unwrap(),
                "it&#39;s a test"
            );
        }

        #[test]
        fn test_html_unescape() {
            assert_eq!(
                apply_transform(
                    "&lt;div class=&quot;test&quot;&gt;a &amp; b&lt;/div&gt;",
                    "html_unescape"
                )
                .unwrap(),
                "<div class=\"test\">a & b</div>"
            );
            assert_eq!(
                apply_transform("it&#39;s a test", "html_unescape").unwrap(),
                "it's a test"
            );
        }

        #[test]
        fn test_html_escape_unescape_roundtrip() {
            let original = "<p class=\"greeting\">Hello & welcome! It's great.</p>";
            let escaped = apply_transform(original, "html_escape").unwrap();
            let unescaped = apply_transform(&escaped, "html_unescape").unwrap();
            assert_eq!(unescaped, original);
        }

        #[test]
        fn test_unknown_transform() {
            let result = apply_transform("test", "unknown_type");
            assert!(result.is_err());
            assert!(result.unwrap_err().contains("Unknown transform type"));
        }

        #[test]
        fn test_url_encode_utf8() {
            let result = apply_transform("cafe\u{0301}", "url_encode").unwrap();
            let decoded = apply_transform(&result, "url_decode").unwrap();
            assert_eq!(decoded, "cafe\u{0301}");
        }

        #[test]
        fn test_empty_string_transforms() {
            assert_eq!(apply_transform("", "uppercase").unwrap(), "");
            assert_eq!(apply_transform("", "lowercase").unwrap(), "");
            assert_eq!(apply_transform("", "title_case").unwrap(), "");
            assert_eq!(apply_transform("", "url_encode").unwrap(), "");
            assert_eq!(apply_transform("", "url_decode").unwrap(), "");
            assert_eq!(apply_transform("", "base64_encode").unwrap(), "");
            assert_eq!(apply_transform("", "trim").unwrap(), "");
            assert_eq!(apply_transform("", "html_escape").unwrap(), "");
            assert_eq!(apply_transform("", "html_unescape").unwrap(), "");
        }
    }
}

#[tauri::command]
pub async fn get_sync_status(
    sync_manager: State<'_, Arc<SyncManager>>,
) -> Result<crate::storage::SyncStatus, String> {
    sync_manager.get_status()
}

#[tauri::command]
pub async fn get_sync_config(
    sync_manager: State<'_, Arc<SyncManager>>,
) -> Result<SyncConfig, String> {
    Ok(sync_manager.get_config())
}

#[tauri::command]
pub async fn update_sync_config(
    sync_manager: State<'_, Arc<SyncManager>>,
    new_config: SyncConfig,
) -> Result<(), String> {
    sync_manager.update_config(new_config)
}

#[tauri::command]
pub async fn get_discovered_devices(
    sync_manager: State<'_, Arc<SyncManager>>,
) -> Result<Vec<crate::storage::DiscoveredDevice>, String> {
    sync_manager.get_discovered_devices()
}

#[tauri::command]
pub async fn get_paired_devices(
    sync_manager: State<'_, Arc<SyncManager>>,
) -> Result<Vec<crate::storage::PairedDevice>, String> {
    sync_manager.get_paired_devices()
}

#[tauri::command]
pub async fn pair_device(
    sync_manager: State<'_, Arc<SyncManager>>,
    device_id: String,
) -> Result<(), String> {
    sync_manager.pair_device(&device_id)
}

#[tauri::command]
pub async fn unpair_device(
    sync_manager: State<'_, Arc<SyncManager>>,
    device_id: String,
) -> Result<(), String> {
    sync_manager.unpair_device(&device_id)
}

#[tauri::command]
pub async fn toggle_device_sync(
    sync_manager: State<'_, Arc<SyncManager>>,
    device_id: String,
    enabled: bool,
) -> Result<(), String> {
    sync_manager.toggle_device_sync(&device_id, enabled)
}
