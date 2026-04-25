use serde::Serialize;

use super::builtin::PluginTransformAction;
use super::registry::DiscoveredPlugin;

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
