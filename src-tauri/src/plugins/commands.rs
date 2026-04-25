use std::sync::Arc;

use serde::Serialize;
use tauri::State;

use crate::config::ConfigManager;
use crate::AppDataDir;

use super::builtin::{builtin_handler_registry, PluginTransformAction};
use super::loader::load_plugins_from_dir;
use super::registry::{DiscoveredPlugin, PluginRegistry};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PluginListItemDto {
    pub id: Option<String>,
    pub name: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
    pub enabled: bool,
    pub enabled_by_default: Option<bool>,
    pub is_valid: bool,
    pub validation_error: Option<String>,
}

impl From<&DiscoveredPlugin> for PluginListItemDto {
    fn from(value: &DiscoveredPlugin) -> Self {
        let manifest = value.manifest.as_ref();
        Self {
            id: manifest.map(|m| m.id.clone()),
            name: manifest.map(|m| m.name.clone()),
            version: manifest.map(|m| m.version.clone()),
            description: manifest.and_then(|m| m.description.clone()),
            enabled: value.enabled,
            enabled_by_default: manifest.map(|m| m.enabled_by_default),
            is_valid: value.is_valid,
            validation_error: value.validation_error.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PluginTransformActionDto {
    pub plugin_id: String,
    pub action_id: String,
    pub label: String,
}

impl From<PluginTransformAction> for PluginTransformActionDto {
    fn from(value: PluginTransformAction) -> Self {
        Self {
            plugin_id: value.plugin_id,
            action_id: value.action_id,
            label: value.label,
        }
    }
}

pub fn load_plugin_registry(app_data_dir: &AppDataDir, config: &ConfigManager) -> PluginRegistry {
    let plugins_dir = app_data_dir.0.join("plugins");
    let loaded = load_plugins_from_dir(&plugins_dir, &builtin_handler_registry());
    PluginRegistry::from_loaded(loaded, config.get().plugin_enabled)
}

pub fn list_plugins_with_config(
    app_data_dir: &AppDataDir,
    config: &ConfigManager,
) -> Vec<PluginListItemDto> {
    load_plugin_registry(app_data_dir, config)
        .plugins()
        .into_iter()
        .map(PluginListItemDto::from)
        .collect()
}

pub fn persist_plugin_enabled(
    config: &ConfigManager,
    plugin_id: String,
    enabled: bool,
) -> Result<(), String> {
    let mut app_config = config.get();
    app_config.plugin_enabled.insert(plugin_id, enabled);
    config.update(app_config)
}

pub fn list_plugin_transform_dtos(
    app_data_dir: &AppDataDir,
    config: &ConfigManager,
    content: &str,
) -> Vec<PluginTransformActionDto> {
    load_plugin_registry(app_data_dir, config)
        .list_transform_actions(content)
        .into_iter()
        .map(PluginTransformActionDto::from)
        .collect()
}

pub fn apply_plugin_transform_with_config(
    app_data_dir: &AppDataDir,
    config: &ConfigManager,
    plugin_id: &str,
    action_id: &str,
    content: &str,
) -> Result<String, String> {
    load_plugin_registry(app_data_dir, config)
        .apply_transform(plugin_id, action_id, content)
        .ok_or_else(|| format!("Unknown plugin transform: {}:{}", plugin_id, action_id))
}

#[tauri::command]
pub async fn list_plugins(
    app_data_dir: State<'_, Arc<AppDataDir>>,
    config: State<'_, Arc<ConfigManager>>,
) -> Result<Vec<PluginListItemDto>, String> {
    Ok(list_plugins_with_config(
        app_data_dir.inner().as_ref(),
        config.inner().as_ref(),
    ))
}

#[tauri::command]
pub async fn set_plugin_enabled(
    config: State<'_, Arc<ConfigManager>>,
    plugin_id: String,
    enabled: bool,
) -> Result<(), String> {
    persist_plugin_enabled(config.inner().as_ref(), plugin_id, enabled)
}

#[tauri::command]
pub async fn list_plugin_transforms(
    app_data_dir: State<'_, Arc<AppDataDir>>,
    config: State<'_, Arc<ConfigManager>>,
    content: String,
) -> Result<Vec<PluginTransformActionDto>, String> {
    Ok(list_plugin_transform_dtos(
        app_data_dir.inner().as_ref(),
        config.inner().as_ref(),
        &content,
    ))
}

#[tauri::command]
pub async fn apply_plugin_transform(
    app_data_dir: State<'_, Arc<AppDataDir>>,
    config: State<'_, Arc<ConfigManager>>,
    plugin_id: String,
    action_id: String,
    content: String,
) -> Result<String, String> {
    apply_plugin_transform_with_config(
        app_data_dir.inner().as_ref(),
        config.inner().as_ref(),
        &plugin_id,
        &action_id,
        &content,
    )
}
