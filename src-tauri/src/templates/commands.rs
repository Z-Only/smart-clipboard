use std::collections::HashMap;
use std::sync::Arc;

use tauri::State;

use crate::security::AppLockManager;
use crate::storage::{Database, Template};

use super::engine;

fn ensure_templates_unlocked(lock: &State<'_, Arc<AppLockManager>>) -> Result<(), String> {
    lock.ensure_unlocked()
}

#[tauri::command]
pub async fn create_template(
    lock: State<'_, Arc<AppLockManager>>,
    db: State<'_, Arc<Database>>,
    name: String,
    content: String,
    category: Option<String>,
) -> Result<Template, String> {
    ensure_templates_unlocked(&lock)?;
    let category = category.unwrap_or_else(|| "general".to_string());
    db.create_template(&name, &content, &category)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_template(
    lock: State<'_, Arc<AppLockManager>>,
    db: State<'_, Arc<Database>>,
    id: i64,
    name: String,
    content: String,
    category: Option<String>,
) -> Result<Template, String> {
    ensure_templates_unlocked(&lock)?;
    let category = category.unwrap_or_else(|| "general".to_string());
    db.update_template(id, &name, &content, &category)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_template(
    lock: State<'_, Arc<AppLockManager>>,
    db: State<'_, Arc<Database>>,
    id: i64,
) -> Result<(), String> {
    ensure_templates_unlocked(&lock)?;
    db.delete_template(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_templates(
    lock: State<'_, Arc<AppLockManager>>,
    db: State<'_, Arc<Database>>,
    category: Option<String>,
) -> Result<Vec<Template>, String> {
    ensure_templates_unlocked(&lock)?;
    db.get_templates(category.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_template(
    lock: State<'_, Arc<AppLockManager>>,
    db: State<'_, Arc<Database>>,
    id: i64,
) -> Result<Template, String> {
    ensure_templates_unlocked(&lock)?;
    db.get_template_by_id(id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Template not found".to_string())
}

#[tauri::command]
pub async fn use_template(
    lock: State<'_, Arc<AppLockManager>>,
    db: State<'_, Arc<Database>>,
    id: i64,
    values: HashMap<String, String>,
) -> Result<String, String> {
    ensure_templates_unlocked(&lock)?;
    let template = db
        .get_template_by_id(id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Template not found".to_string())?;

    let rendered = engine::render(&template.content, &values);

    // Copy rendered text to clipboard
    let mut clipboard = arboard::Clipboard::new().map_err(|e| format!("Clipboard error: {}", e))?;
    clipboard
        .set_text(&rendered)
        .map_err(|e| format!("Clipboard error: {}", e))?;

    // Increment use count
    db.increment_template_use_count(id)
        .map_err(|e| e.to_string())?;

    Ok(rendered)
}

#[tauri::command]
pub async fn get_template_categories(
    lock: State<'_, Arc<AppLockManager>>,
    db: State<'_, Arc<Database>>,
) -> Result<Vec<String>, String> {
    ensure_templates_unlocked(&lock)?;
    db.get_template_categories_list().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_template_placeholders(_content: String) -> Result<Vec<String>, String> {
    Ok(engine::extract_placeholders(&_content))
}

#[cfg(test)]
mod template_guard_tests {
    use std::sync::Mutex;

    struct FakeTemplateLock {
        enabled: bool,
        locked: Mutex<bool>,
    }

    impl FakeTemplateLock {
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
    fn template_guard_blocks_when_locked() {
        let lock = FakeTemplateLock::new(true, true);
        assert_eq!(lock.ensure_unlocked().unwrap_err(), "App is locked");
    }

    #[test]
    fn template_guard_recovers_after_unlock() {
        let lock = FakeTemplateLock::new(true, true);
        assert!(lock.ensure_unlocked().is_err());
        lock.unlock();
        assert!(lock.ensure_unlocked().is_ok());
    }
}
