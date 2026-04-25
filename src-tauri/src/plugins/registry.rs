use std::collections::HashMap;

use super::builtin::{
    builtin_handler_registry, PluginClassification, PluginTransformAction, SharedPluginHandler,
};
use super::loader::{LoadedPlugin, LoadedPlugins};
use super::manifest::{PluginCapability, PluginManifest};

#[derive(Debug, Clone)]
pub struct DiscoveredPlugin {
    pub manifest: Option<PluginManifest>,
    pub enabled: bool,
    pub is_valid: bool,
    pub validation_error: Option<String>,
}

pub struct PluginRegistry {
    plugins: Vec<RegisteredPlugin>,
}

struct RegisteredPlugin {
    discovered: DiscoveredPlugin,
    handler: Option<SharedPluginHandler>,
}

impl PluginRegistry {
    pub fn from_loaded(overall: LoadedPlugins, overrides: HashMap<String, bool>) -> Self {
        let handlers = builtin_handler_registry();
        let plugins = overall
            .plugins
            .into_iter()
            .map(|loaded| Self::build_registered_plugin(loaded, &overrides, &handlers))
            .collect();

        Self { plugins }
    }

    pub fn plugins(&self) -> Vec<&DiscoveredPlugin> {
        self.plugins
            .iter()
            .map(|plugin| &plugin.discovered)
            .collect()
    }

    pub fn classify_content(&self, content: &str) -> Vec<PluginClassification> {
        self.plugins
            .iter()
            .filter_map(|plugin| plugin.dispatchable_handler(PluginCapability::Classify))
            .flat_map(|(manifest, handler)| handler.classify(&manifest.id, content))
            .collect()
    }

    pub fn list_transform_actions(&self, content: &str) -> Vec<PluginTransformAction> {
        self.plugins
            .iter()
            .filter_map(|plugin| plugin.dispatchable_handler(PluginCapability::Transform))
            .flat_map(|(manifest, handler)| handler.transforms(&manifest.id, content))
            .collect()
    }

    pub fn apply_transform(
        &self,
        plugin_id: &str,
        action_id: &str,
        content: &str,
    ) -> Option<String> {
        self.plugins
            .iter()
            .filter(|plugin| plugin.discovered.enabled && plugin.discovered.is_valid)
            .filter_map(|plugin| {
                let manifest = plugin.discovered.manifest.as_ref()?;
                let handler = plugin.handler.as_ref()?;
                Some((manifest, handler))
            })
            .find(|(manifest, _)| manifest.id == plugin_id)
            .and_then(|(_, handler)| handler.apply_transform(action_id, content))
    }

    fn build_registered_plugin(
        loaded: LoadedPlugin,
        overrides: &HashMap<String, bool>,
        handlers: &HashMap<String, SharedPluginHandler>,
    ) -> RegisteredPlugin {
        let enabled = loaded
            .manifest
            .as_ref()
            .map(|manifest| {
                overrides
                    .get(&manifest.id)
                    .copied()
                    .unwrap_or(manifest.enabled_by_default)
            })
            .unwrap_or(false);

        let is_valid = loaded.validation_error.is_none();
        let handler = loaded
            .manifest
            .as_ref()
            .and_then(|manifest| handlers.get(&manifest.handler).cloned())
            .filter(|_| is_valid);

        RegisteredPlugin {
            discovered: DiscoveredPlugin {
                manifest: loaded.manifest,
                enabled,
                is_valid,
                validation_error: loaded.validation_error,
            },
            handler,
        }
    }
}

impl RegisteredPlugin {
    fn dispatchable_handler(
        &self,
        capability: PluginCapability,
    ) -> Option<(&PluginManifest, &SharedPluginHandler)> {
        if !self.discovered.enabled || !self.discovered.is_valid {
            return None;
        }

        let manifest = self.discovered.manifest.as_ref()?;
        if !manifest.capabilities.contains(&capability) {
            return None;
        }

        Some((manifest, self.handler.as_ref()?))
    }
}
