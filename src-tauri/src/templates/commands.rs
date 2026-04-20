use std::collections::HashMap;
use std::sync::Arc;

use tauri::State;

use crate::storage::{Database, Template};

use super::engine;

#[tauri::command]
pub async fn create_template(
    db: State<'_, Arc<Database>>,
    name: String,
    content: String,
    category: Option<String>,
) -> Result<Template, String> {
    let category = category.unwrap_or_else(|| "general".to_string());
    db.create_template(&name, &content, &category)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_template(
    db: State<'_, Arc<Database>>,
    id: i64,
    name: String,
    content: String,
    category: Option<String>,
) -> Result<Template, String> {
    let category = category.unwrap_or_else(|| "general".to_string());
    db.update_template(id, &name, &content, &category)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_template(db: State<'_, Arc<Database>>, id: i64) -> Result<(), String> {
    db.delete_template(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_templates(
    db: State<'_, Arc<Database>>,
    category: Option<String>,
) -> Result<Vec<Template>, String> {
    db.get_templates(category.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_template(db: State<'_, Arc<Database>>, id: i64) -> Result<Template, String> {
    db.get_template_by_id(id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Template not found".to_string())
}

#[tauri::command]
pub async fn use_template(
    db: State<'_, Arc<Database>>,
    id: i64,
    values: HashMap<String, String>,
) -> Result<String, String> {
    let template = db
        .get_template_by_id(id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Template not found".to_string())?;

    let rendered = engine::render(&template.content, &values);

    // Copy rendered text to clipboard
    let mut clipboard =
        arboard::Clipboard::new().map_err(|e| format!("Clipboard error: {}", e))?;
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
    db: State<'_, Arc<Database>>,
) -> Result<Vec<String>, String> {
    db.get_template_categories_list()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_template_placeholders(content: String) -> Result<Vec<String>, String> {
    Ok(engine::extract_placeholders(&content))
}
