use std::collections::HashMap;

use super::builtin::{
    builtin_handler_registry, PluginClassification, PluginHandler, PluginTransformAction,
};
use super::loader::LoadedPlugins;
use super::manifest::PluginCapability;

pub struct PluginRegistry {
    plugins: Vec<RegisteredPlugin>,
}

struct RegisteredPlugin {
    manifest: super::manifest::PluginManifest,
    enabled: bool,
    handler: Box<dyn PluginHandler>,
}

impl PluginRegistry {
    pub fn from_loaded(overall: LoadedPlugins, overrides: HashMap<String, bool>) -> Self {
        let mut handlers = builtin_handler_registry();
        let plugins = overall
            .plugins
            .into_iter()
            .filter_map(|loaded| {
                if loaded.validation_error.is_some() {
                    return None;
                }
                let manifest = loaded.manifest?;
                let enabled = overrides
                    .get(&manifest.id)
                    .copied()
                    .unwrap_or(manifest.enabled_by_default);
                let handler = handlers.remove(&manifest.handler)?;
                Some(RegisteredPlugin {
                    manifest,
                    enabled,
                    handler,
                })
            })
            .collect();

        Self { plugins }
    }

    pub fn plugins(&self) -> Vec<&super::manifest::PluginManifest> {
        self.plugins.iter().map(|plugin| &plugin.manifest).collect()
    }

    pub fn classify_content(&self, content: &str) -> Vec<PluginClassification> {
        self.plugins
            .iter()
            .filter(|plugin| plugin.enabled)
            .filter(|plugin| {
                plugin
                    .manifest
                    .capabilities
                    .contains(&PluginCapability::Classify)
            })
            .flat_map(|plugin| plugin.handler.classify(&plugin.manifest.id, content))
            .collect()
    }

    pub fn list_transform_actions(&self, content: &str) -> Vec<PluginTransformAction> {
        self.plugins
            .iter()
            .filter(|plugin| plugin.enabled)
            .filter(|plugin| {
                plugin
                    .manifest
                    .capabilities
                    .contains(&PluginCapability::Transform)
            })
            .flat_map(|plugin| plugin.handler.transforms(&plugin.manifest.id, content))
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
            .find(|plugin| plugin.enabled && plugin.manifest.id == plugin_id)
            .and_then(|plugin| plugin.handler.apply_transform(action_id, content))
    }
}
