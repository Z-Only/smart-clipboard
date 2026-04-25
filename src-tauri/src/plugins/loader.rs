use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use super::builtin::HandlerRegistry;
use super::manifest::PluginManifest;

#[derive(Debug, Clone)]
pub struct LoadedPlugin {
    pub manifest: Option<PluginManifest>,
    pub validation_error: Option<String>,
    pub source_path: PathBuf,
}

#[derive(Debug, Clone, Default)]
pub struct LoadedPlugins {
    pub plugins: Vec<LoadedPlugin>,
}

pub fn load_plugins_from_dir(dir: &Path, handlers: &HandlerRegistry) -> LoadedPlugins {
    let mut plugins = Vec::new();
    let mut seen_ids = HashSet::new();

    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return LoadedPlugins { plugins },
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let manifest_path = path.join("plugin.json");
        if !manifest_path.exists() {
            continue;
        }

        let loaded = match fs::read_to_string(&manifest_path) {
            Ok(body) => match serde_json::from_str::<PluginManifest>(&body) {
                Ok(manifest) => {
                    let validation_error = manifest
                        .validate()
                        .err()
                        .or_else(|| {
                            if !seen_ids.insert(manifest.id.clone()) {
                                Some(format!("duplicate plugin id: {}", manifest.id))
                            } else {
                                None
                            }
                        })
                        .or_else(|| {
                            if handlers.contains_key(&manifest.handler) {
                                None
                            } else {
                                Some(format!("unknown handler: {}", manifest.handler))
                            }
                        });

                    LoadedPlugin {
                        manifest: Some(manifest),
                        validation_error,
                        source_path: path.clone(),
                    }
                }
                Err(err) => LoadedPlugin {
                    manifest: None,
                    validation_error: Some(format!("invalid plugin manifest JSON: {err}")),
                    source_path: path.clone(),
                },
            },
            Err(err) => LoadedPlugin {
                manifest: None,
                validation_error: Some(format!("failed to read plugin manifest: {err}")),
                source_path: path.clone(),
            },
        };

        plugins.push(loaded);
    }

    plugins.sort_by(|a, b| {
        let a_id = a.manifest.as_ref().map(|m| m.id.as_str()).unwrap_or("");
        let b_id = b.manifest.as_ref().map(|m| m.id.as_str()).unwrap_or("");
        a_id.cmp(b_id)
            .then_with(|| a.source_path.cmp(&b.source_path))
    });

    LoadedPlugins { plugins }
}
