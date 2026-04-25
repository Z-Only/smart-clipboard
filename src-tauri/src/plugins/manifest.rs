use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginCapability {
    Classify,
    Transform,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub kind: String,
    #[serde(rename = "enabledByDefault")]
    pub enabled_by_default: bool,
    #[serde(default)]
    pub description: Option<String>,
    pub capabilities: Vec<PluginCapability>,
    pub handler: String,
}

impl PluginManifest {
    pub fn validate(&self) -> Result<(), String> {
        if self.id.trim().is_empty() {
            return Err("plugin id cannot be empty".to_string());
        }
        if self.name.trim().is_empty() {
            return Err("plugin name cannot be empty".to_string());
        }
        if self.version.trim().is_empty() {
            return Err("plugin version cannot be empty".to_string());
        }
        if self.kind.trim().is_empty() {
            return Err("plugin kind cannot be empty".to_string());
        }
        if self.capabilities.is_empty() {
            return Err("plugin capabilities cannot be empty".to_string());
        }
        if self.handler.trim().is_empty() {
            return Err("plugin handler cannot be empty".to_string());
        }
        Ok(())
    }
}
