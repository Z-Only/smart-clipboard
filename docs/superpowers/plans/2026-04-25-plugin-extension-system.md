# Plugin / Extension System Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a v1 plugin platform that discovers local plugin manifests, persists enablement in config, exposes content-processor hooks through trusted built-in handlers, shows plugin status in settings, and proves the full path with one markdown tools example plugin.

**Architecture:** Add a dedicated Rust `plugins` subsystem that owns manifest parsing, directory scanning, built-in handler lookup, registry state, and dispatch APIs. Integrate that registry into config persistence, backend commands, analyzer/transform flows, and a small frontend plugin store so the settings and transform UI can consume plugin metadata and actions without changing the rest of the application architecture.

**Tech Stack:** Rust + serde + Tauri 2 commands/state, Vue 3 + TypeScript + Pinia + Vitest, existing Smart Clipboard config/analyzer/transform infrastructure.

---

## File Structure

### Backend

- Create: `/Users/chanyu/AIProjects/smart-clipboard/src-tauri/src/plugins/mod.rs` — plugin module exports and shared public types.
- Create: `/Users/chanyu/AIProjects/smart-clipboard/src-tauri/src/plugins/manifest.rs` — manifest schema and validation.
- Create: `/Users/chanyu/AIProjects/smart-clipboard/src-tauri/src/plugins/loader.rs` — plugin directory scanning and duplicate handling.
- Create: `/Users/chanyu/AIProjects/smart-clipboard/src-tauri/src/plugins/registry.rs` — runtime registry, enablement resolution, dispatch.
- Create: `/Users/chanyu/AIProjects/smart-clipboard/src-tauri/src/plugins/builtin.rs` — built-in handler registry and markdown-tools implementation.
- Create: `/Users/chanyu/AIProjects/smart-clipboard/src-tauri/src/plugins/commands.rs` — serializable DTOs and helper functions for Tauri command handlers.
- Modify: `/Users/chanyu/AIProjects/smart-clipboard/src-tauri/src/config.rs` — persist plugin enablement state.
- Modify: `/Users/chanyu/AIProjects/smart-clipboard/src-tauri/src/commands.rs` — add plugin query/toggle/apply commands and wire transform dispatch.
- Modify: `/Users/chanyu/AIProjects/smart-clipboard/src-tauri/src/analyzer/mod.rs` — export plugin classification enrichment helpers.
- Modify: `/Users/chanyu/AIProjects/smart-clipboard/src-tauri/src/lib.rs` — register plugin state and commands.
- Modify: `/Users/chanyu/AIProjects/smart-clipboard/src-tauri/src/main.rs` — ensure plugin module is compiled if needed.
- Test: `/Users/chanyu/AIProjects/smart-clipboard/src-tauri/src/plugins/mod.rs` and sibling files via unit tests inside the new module.

### Frontend

- Create: `/Users/chanyu/AIProjects/smart-clipboard/src/types/plugins.ts` — plugin DTOs and transform action types.
- Create: `/Users/chanyu/AIProjects/smart-clipboard/src/stores/pluginStore.ts` — fetch/toggle plugin state and query plugin transforms.
- Modify: `/Users/chanyu/AIProjects/smart-clipboard/src/components/SettingsPanel.vue` — add plugin management section.
- Modify: `/Users/chanyu/AIProjects/smart-clipboard/src/components/TransformMenu.vue` — render plugin transform groups and invoke plugin transform command.
- Modify: `/Users/chanyu/AIProjects/smart-clipboard/src/types/index.ts` — re-export plugin types if current project style expects barrel exports.
- Modify: `/Users/chanyu/AIProjects/smart-clipboard/src/i18n/locales/en.ts` — add plugin/settings/transform copy.
- Modify: `/Users/chanyu/AIProjects/smart-clipboard/src/i18n/locales/zh-CN.ts` — add plugin/settings/transform copy.
- Test: `/Users/chanyu/AIProjects/smart-clipboard/tests/unit/SettingsPanel.plugins.test.ts`
- Test: `/Users/chanyu/AIProjects/smart-clipboard/tests/unit/TransformMenu.plugins.test.ts`
- Modify: `/Users/chanyu/AIProjects/smart-clipboard/tests/unit/SettingsPanel.updater.test.ts` if shared setup changes are needed.

### Plugin Asset

- Create: `/Users/chanyu/AIProjects/smart-clipboard/plugins/markdown-tools/plugin.json` — example plugin manifest.

---

### Task 1: Add backend plugin manifest, loader, registry, and built-in handler core

**Files:**

- Create: `/Users/chanyu/AIProjects/smart-clipboard/src-tauri/src/plugins/mod.rs`
- Create: `/Users/chanyu/AIProjects/smart-clipboard/src-tauri/src/plugins/manifest.rs`
- Create: `/Users/chanyu/AIProjects/smart-clipboard/src-tauri/src/plugins/loader.rs`
- Create: `/Users/chanyu/AIProjects/smart-clipboard/src-tauri/src/plugins/registry.rs`
- Create: `/Users/chanyu/AIProjects/smart-clipboard/src-tauri/src/plugins/builtin.rs`
- Test: `/Users/chanyu/AIProjects/smart-clipboard/src-tauri/src/plugins/mod.rs`

- [ ] **Step 1: Write the failing unit tests for manifest validation and registry dispatch**

```rust
// /Users/chanyu/AIProjects/smart-clipboard/src-tauri/src/plugins/mod.rs
pub mod builtin;
pub mod loader;
pub mod manifest;
pub mod registry;

#[cfg(test)]
mod tests {
    use super::builtin::builtin_handler_registry;
    use super::loader::load_plugins_from_dir;
    use super::manifest::{PluginCapability, PluginManifest};
    use super::registry::PluginRegistry;
    use std::fs;

    fn write_plugin(dir: &std::path::Path, plugin_id: &str, body: &str) {
        let plugin_dir = dir.join(plugin_id);
        fs::create_dir_all(&plugin_dir).unwrap();
        fs::write(plugin_dir.join("plugin.json"), body).unwrap();
    }

    #[test]
    fn validates_manifest_and_loads_builtin_markdown_handler() {
        let temp_dir = tempfile::tempdir().unwrap();
        write_plugin(
            temp_dir.path(),
            "markdown-tools",
            r#"{
              "id": "markdown-tools",
              "name": "Markdown Tools",
              "version": "1.0.0",
              "kind": "content_processor",
              "enabledByDefault": true,
              "description": "Adds markdown-aware classification and transforms.",
              "capabilities": ["classify", "transform"],
              "handler": "builtin.markdown_tools"
            }"#,
        );

        let loaded = load_plugins_from_dir(temp_dir.path(), &builtin_handler_registry());
        assert_eq!(loaded.plugins.len(), 1);
        assert!(loaded.plugins[0].validation_error.is_none());
        assert_eq!(loaded.plugins[0].manifest.as_ref().unwrap().id, "markdown-tools");
        assert_eq!(loaded.plugins[0].manifest.as_ref().unwrap().capabilities,
            vec![PluginCapability::Classify, PluginCapability::Transform]);
    }

    #[test]
    fn marks_unknown_handler_plugin_invalid_without_breaking_load() {
        let temp_dir = tempfile::tempdir().unwrap();
        write_plugin(
            temp_dir.path(),
            "broken-plugin",
            r#"{
              "id": "broken-plugin",
              "name": "Broken Plugin",
              "version": "1.0.0",
              "kind": "content_processor",
              "enabledByDefault": true,
              "capabilities": ["classify"],
              "handler": "builtin.missing"
            }"#,
        );

        let loaded = load_plugins_from_dir(temp_dir.path(), &builtin_handler_registry());
        assert_eq!(loaded.plugins.len(), 1);
        assert!(loaded.plugins[0].manifest.is_some());
        assert!(loaded.plugins[0].validation_error.is_some());
    }

    #[test]
    fn dispatches_markdown_classification_and_transform_for_enabled_plugin() {
        let temp_dir = tempfile::tempdir().unwrap();
        write_plugin(
            temp_dir.path(),
            "markdown-tools",
            r#"{
              "id": "markdown-tools",
              "name": "Markdown Tools",
              "version": "1.0.0",
              "kind": "content_processor",
              "enabledByDefault": true,
              "capabilities": ["classify", "transform"],
              "handler": "builtin.markdown_tools"
            }"#,
        );

        let loaded = load_plugins_from_dir(temp_dir.path(), &builtin_handler_registry());
        let registry = PluginRegistry::from_loaded(loaded, std::collections::HashMap::new());

        let classifications = registry.classify_content("- [x] write tests");
        assert_eq!(classifications.len(), 1);
        assert_eq!(classifications[0].classification, "markdown_checklist");

        let transforms = registry.list_transform_actions("# Hello\n**world**");
        assert_eq!(transforms.len(), 2);
        assert_eq!(transforms[0].plugin_id, "markdown-tools");

        let transformed = registry
            .apply_transform("markdown-tools", "strip_markdown_format", "# Hello\n**world**")
            .unwrap();
        assert_eq!(transformed, "Hello\nworld");
    }

    #[test]
    fn ignores_disabled_plugins_when_dispatching_hooks() {
        let temp_dir = tempfile::tempdir().unwrap();
        write_plugin(
            temp_dir.path(),
            "markdown-tools",
            r#"{
              "id": "markdown-tools",
              "name": "Markdown Tools",
              "version": "1.0.0",
              "kind": "content_processor",
              "enabledByDefault": true,
              "capabilities": ["classify", "transform"],
              "handler": "builtin.markdown_tools"
            }"#,
        );

        let loaded = load_plugins_from_dir(temp_dir.path(), &builtin_handler_registry());
        let registry = PluginRegistry::from_loaded(
            loaded,
            std::collections::HashMap::from([("markdown-tools".to_string(), false)]),
        );

        assert!(registry.classify_content("- [ ] hidden").is_empty());
        assert!(registry.list_transform_actions("# Hidden").is_empty());
    }
}
```

- [ ] **Step 2: Run the Rust plugin tests to verify they fail for the expected missing module errors**

Run: `cargo test --manifest-path /Users/chanyu/AIProjects/smart-clipboard/src-tauri/Cargo.toml plugins::tests -- --test-threads=1`
Expected: FAIL with unresolved module/import errors for `plugins`, `tempfile`, or missing `PluginRegistry` APIs.

- [ ] **Step 3: Write the minimal backend plugin implementation**

```rust
// /Users/chanyu/AIProjects/smart-clipboard/src-tauri/src/plugins/manifest.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginKind {
    ContentProcessor,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginCapability {
    Classify,
    Transform,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub kind: PluginKind,
    #[serde(default)]
    pub enabled_by_default: bool,
    #[serde(default)]
    pub description: Option<String>,
    pub capabilities: Vec<PluginCapability>,
    pub handler: String,
}

impl PluginManifest {
    pub fn validate(&self) -> Result<(), String> {
        if self.id.trim().is_empty() {
            return Err("Plugin id is required".to_string());
        }
        if self.name.trim().is_empty() {
            return Err("Plugin name is required".to_string());
        }
        if self.version.trim().is_empty() {
            return Err("Plugin version is required".to_string());
        }
        if self.capabilities.is_empty() {
            return Err("At least one capability is required".to_string());
        }
        if self.handler.trim().is_empty() {
            return Err("Plugin handler is required".to_string());
        }
        Ok(())
    }
}
```

````rust
// /Users/chanyu/AIProjects/smart-clipboard/src-tauri/src/plugins/builtin.rs
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginClassification {
    pub plugin_id: String,
    pub classification: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginTransformAction {
    pub plugin_id: String,
    pub plugin_name: String,
    pub transform_id: String,
    pub label: String,
}

pub trait BuiltinContentProcessor: Send + Sync {
    fn classify(&self, plugin_id: &str, content: &str) -> Option<PluginClassification>;
    fn list_transforms(&self, plugin_id: &str, plugin_name: &str, content: &str) -> Vec<PluginTransformAction>;
    fn apply_transform(&self, transform_id: &str, content: &str) -> Result<String, String>;
}

pub type BuiltinHandlerRegistry = HashMap<String, Box<dyn BuiltinContentProcessor>>;

pub fn builtin_handler_registry() -> BuiltinHandlerRegistry {
    let mut registry: BuiltinHandlerRegistry = HashMap::new();
    registry.insert("builtin.markdown_tools".to_string(), Box::new(MarkdownToolsHandler));
    registry
}

struct MarkdownToolsHandler;

impl BuiltinContentProcessor for MarkdownToolsHandler {
    fn classify(&self, plugin_id: &str, content: &str) -> Option<PluginClassification> {
        let trimmed = content.trim();
        let classification = if trimmed.contains("|---") && trimmed.contains('|') {
            Some("markdown_table")
        } else if trimmed.lines().any(|line| line.trim_start().starts_with("- [") || line.trim_start().starts_with("* [")) {
            Some("markdown_checklist")
        } else if trimmed.contains("```") {
            Some("markdown_code_block")
        } else {
            None
        }?;

        Some(PluginClassification {
            plugin_id: plugin_id.to_string(),
            classification: classification.to_string(),
        })
    }

    fn list_transforms(&self, plugin_id: &str, plugin_name: &str, content: &str) -> Vec<PluginTransformAction> {
        if content.trim().is_empty() {
            return vec![];
        }

        vec![
            PluginTransformAction {
                plugin_id: plugin_id.to_string(),
                plugin_name: plugin_name.to_string(),
                transform_id: "markdown_to_blockquote".to_string(),
                label: "Convert to blockquote".to_string(),
            },
            PluginTransformAction {
                plugin_id: plugin_id.to_string(),
                plugin_name: plugin_name.to_string(),
                transform_id: "strip_markdown_format".to_string(),
                label: "Strip markdown formatting".to_string(),
            },
        ]
    }

    fn apply_transform(&self, transform_id: &str, content: &str) -> Result<String, String> {
        match transform_id {
            "markdown_to_blockquote" => Ok(content
                .lines()
                .map(|line| if line.trim().is_empty() { String::new() } else { format!("> {}", line) })
                .collect::<Vec<_>>()
                .join("\n")),
            "strip_markdown_format" => Ok(strip_markdown(content)),
            _ => Err(format!("Unknown plugin transform: {}", transform_id)),
        }
    }
}

fn strip_markdown(content: &str) -> String {
    content
        .lines()
        .filter(|line| line.trim() != "```")
        .map(|line| {
            line.trim_start_matches('#')
                .trim_start()
                .replace("**", "")
                .replace("__", "")
                .replace("* ", "")
                .replace("- [ ] ", "")
                .replace("- [x] ", "")
        })
        .collect::<Vec<_>>()
        .join("\n")
}
````

```rust
// /Users/chanyu/AIProjects/smart-clipboard/src-tauri/src/plugins/loader.rs
use super::builtin::BuiltinHandlerRegistry;
use super::manifest::PluginManifest;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct LoadedPlugin {
    pub plugin_dir: PathBuf,
    pub manifest: Option<PluginManifest>,
    pub validation_error: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct PluginLoadResult {
    pub plugins: Vec<LoadedPlugin>,
}

pub fn load_plugins_from_dir(path: &Path, handlers: &BuiltinHandlerRegistry) -> PluginLoadResult {
    let mut result = PluginLoadResult::default();
    let mut seen_ids = HashSet::new();

    let Ok(entries) = fs::read_dir(path) else {
        return result;
    };

    for entry in entries.flatten() {
        let plugin_dir = entry.path();
        if !plugin_dir.is_dir() {
            continue;
        }

        let manifest_path = plugin_dir.join("plugin.json");
        let loaded = match fs::read_to_string(&manifest_path) {
            Ok(contents) => match serde_json::from_str::<PluginManifest>(&contents) {
                Ok(manifest) => {
                    let validation_error = manifest
                        .validate()
                        .err()
                        .or_else(|| (!handlers.contains_key(&manifest.handler)).then(|| format!("Unknown handler: {}", manifest.handler)))
                        .or_else(|| (!seen_ids.insert(manifest.id.clone())).then(|| format!("Duplicate plugin id: {}", manifest.id)));

                    LoadedPlugin {
                        plugin_dir,
                        manifest: Some(manifest),
                        validation_error,
                    }
                }
                Err(err) => LoadedPlugin {
                    plugin_dir,
                    manifest: None,
                    validation_error: Some(format!("Invalid plugin manifest: {}", err)),
                },
            },
            Err(err) => LoadedPlugin {
                plugin_dir,
                manifest: None,
                validation_error: Some(format!("Unable to read plugin manifest: {}", err)),
            },
        };

        result.plugins.push(loaded);
    }

    result
}
```

```rust
// /Users/chanyu/AIProjects/smart-clipboard/src-tauri/src/plugins/registry.rs
use super::builtin::{builtin_handler_registry, PluginClassification, PluginTransformAction};
use super::loader::{LoadedPlugin, PluginLoadResult};
use super::manifest::PluginCapability;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct InstalledPlugin {
    pub plugin: LoadedPlugin,
    pub enabled: bool,
}

#[derive(Debug)]
pub struct PluginRegistry {
    handlers: super::builtin::BuiltinHandlerRegistry,
    plugins: Vec<InstalledPlugin>,
}

impl PluginRegistry {
    pub fn from_loaded(load_result: PluginLoadResult, states: HashMap<String, bool>) -> Self {
        let handlers = builtin_handler_registry();
        let plugins = load_result
            .plugins
            .into_iter()
            .map(|plugin| {
                let enabled = plugin
                    .manifest
                    .as_ref()
                    .map(|manifest| states.get(&manifest.id).copied().unwrap_or(manifest.enabled_by_default))
                    .unwrap_or(false);
                InstalledPlugin { plugin, enabled }
            })
            .collect();
        Self { handlers, plugins }
    }

    pub fn classify_content(&self, content: &str) -> Vec<PluginClassification> {
        self.plugins
            .iter()
            .filter(|installed| installed.enabled && installed.plugin.validation_error.is_none())
            .filter_map(|installed| {
                let manifest = installed.plugin.manifest.as_ref()?;
                if !manifest.capabilities.contains(&PluginCapability::Classify) {
                    return None;
                }
                self.handlers
                    .get(&manifest.handler)?
                    .classify(&manifest.id, content)
            })
            .collect()
    }

    pub fn list_transform_actions(&self, content: &str) -> Vec<PluginTransformAction> {
        self.plugins
            .iter()
            .filter(|installed| installed.enabled && installed.plugin.validation_error.is_none())
            .flat_map(|installed| {
                let Some(manifest) = installed.plugin.manifest.as_ref() else {
                    return Vec::new();
                };
                if !manifest.capabilities.contains(&PluginCapability::Transform) {
                    return Vec::new();
                }
                self.handlers
                    .get(&manifest.handler)
                    .map(|handler| handler.list_transforms(&manifest.id, &manifest.name, content))
                    .unwrap_or_default()
            })
            .collect()
    }

    pub fn apply_transform(&self, plugin_id: &str, transform_id: &str, content: &str) -> Result<String, String> {
        let installed = self
            .plugins
            .iter()
            .find(|installed| installed.enabled && installed.plugin.manifest.as_ref().map(|manifest| manifest.id.as_str()) == Some(plugin_id))
            .ok_or_else(|| format!("Unknown or disabled plugin: {}", plugin_id))?;

        let manifest = installed.plugin.manifest.as_ref().ok_or_else(|| "Plugin manifest missing".to_string())?;
        let handler = self
            .handlers
            .get(&manifest.handler)
            .ok_or_else(|| format!("Unknown handler: {}", manifest.handler))?;
        handler.apply_transform(transform_id, content)
    }

    pub fn plugins(&self) -> &[InstalledPlugin] {
        &self.plugins
    }
}
```

```toml
# /Users/chanyu/AIProjects/smart-clipboard/src-tauri/Cargo.toml
[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 4: Run the Rust plugin tests to verify they pass**

Run: `cargo test --manifest-path /Users/chanyu/AIProjects/smart-clipboard/src-tauri/Cargo.toml plugins::tests -- --test-threads=1`
Expected: PASS with 4 passed tests in `plugins::tests`.

- [ ] **Step 5: Commit the backend plugin core**

```bash
git add /Users/chanyu/AIProjects/smart-clipboard/src-tauri/Cargo.toml \
  /Users/chanyu/AIProjects/smart-clipboard/src-tauri/src/plugins
git commit -m "feat: add plugin registry core"
```

### Task 2: Persist plugin enablement and expose backend plugin commands

**Files:**

- Modify: `/Users/chanyu/AIProjects/smart-clipboard/src-tauri/src/config.rs`
- Create: `/Users/chanyu/AIProjects/smart-clipboard/src-tauri/src/plugins/commands.rs`
- Modify: `/Users/chanyu/AIProjects/smart-clipboard/src-tauri/src/commands.rs`
- Modify: `/Users/chanyu/AIProjects/smart-clipboard/src-tauri/src/lib.rs`
- Test: `/Users/chanyu/AIProjects/smart-clipboard/src-tauri/src/plugins/mod.rs`

- [ ] **Step 1: Write the failing unit tests for config persistence and DTO export**

```rust
// Add to /Users/chanyu/AIProjects/smart-clipboard/src-tauri/src/plugins/mod.rs test module
    #[test]
    fn resolves_explicit_config_state_over_manifest_default() {
        let temp_dir = tempfile::tempdir().unwrap();
        write_plugin(
            temp_dir.path(),
            "markdown-tools",
            r#"{
              "id": "markdown-tools",
              "name": "Markdown Tools",
              "version": "1.0.0",
              "kind": "content_processor",
              "enabledByDefault": true,
              "capabilities": ["classify", "transform"],
              "handler": "builtin.markdown_tools"
            }"#,
        );

        let loaded = load_plugins_from_dir(temp_dir.path(), &builtin_handler_registry());
        let registry = PluginRegistry::from_loaded(
            loaded,
            std::collections::HashMap::from([("markdown-tools".to_string(), false)]),
        );

        assert!(!registry.plugins()[0].enabled);
    }
```

```rust
// /Users/chanyu/AIProjects/smart-clipboard/src-tauri/src/plugins/commands.rs
#[cfg(test)]
mod tests {
    use super::{PluginListItem, PluginTransformActionDto};

    #[test]
    fn plugin_dtos_serialize_with_expected_shape() {
        let plugin = PluginListItem {
            id: "markdown-tools".to_string(),
            name: "Markdown Tools".to_string(),
            version: "1.0.0".to_string(),
            description: Some("Adds markdown-aware classification and transforms.".to_string()),
            kind: "content_processor".to_string(),
            handler: "builtin.markdown_tools".to_string(),
            capabilities: vec!["classify".to_string(), "transform".to_string()],
            enabled: true,
            valid: true,
            error: None,
        };

        let json = serde_json::to_value(plugin).unwrap();
        assert_eq!(json["id"], "markdown-tools");
        assert_eq!(json["enabled"], true);
        assert_eq!(json["valid"], true);

        let action = PluginTransformActionDto {
            plugin_id: "markdown-tools".to_string(),
            plugin_name: "Markdown Tools".to_string(),
            transform_id: "strip_markdown_format".to_string(),
            label: "Strip markdown formatting".to_string(),
        };
        let action_json = serde_json::to_value(action).unwrap();
        assert_eq!(action_json["transformId"], "strip_markdown_format");
    }
}
```

- [ ] **Step 2: Run the targeted Rust tests to verify they fail for missing config fields and DTOs**

Run: `cargo test --manifest-path /Users/chanyu/AIProjects/smart-clipboard/src-tauri/Cargo.toml plugins::tests plugins::commands::tests -- --test-threads=1`
Expected: FAIL with missing `plugins` config field, missing DTO types, or unresolved command helpers.

- [ ] **Step 3: Write the minimal config persistence and command-layer implementation**

```rust
// /Users/chanyu/AIProjects/smart-clipboard/src-tauri/src/config.rs
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginConfigEntry {
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub max_entries: i64,
    pub retention_days: i64,
    pub excluded_apps: Vec<String>,
    pub monitor_interval_ms: u64,
    pub autostart_enabled: bool,
    pub sensitive_expiry_minutes: u64,
    #[serde(default)]
    pub sync: SyncConfig,
    pub sync_metadata: Option<Value>,
    #[serde(default)]
    pub webdav: WebDavConfig,
    #[serde(default)]
    pub app_lock: AppLockConfig,
    #[serde(default)]
    pub encryption: EncryptionConfig,
    #[serde(default)]
    pub updater: UpdaterConfig,
    #[serde(default)]
    pub plugins: HashMap<String, PluginConfigEntry>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            max_entries: 5000,
            retention_days: 30,
            excluded_apps: vec![],
            monitor_interval_ms: 500,
            autostart_enabled: false,
            sensitive_expiry_minutes: 5,
            sync: SyncConfig::default(),
            sync_metadata: None,
            webdav: WebDavConfig::default(),
            app_lock: AppLockConfig::default(),
            encryption: EncryptionConfig::default(),
            updater: UpdaterConfig::default(),
            plugins: HashMap::new(),
        }
    }
}
```

```rust
// /Users/chanyu/AIProjects/smart-clipboard/src-tauri/src/plugins/commands.rs
use serde::Serialize;

use super::registry::InstalledPlugin;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginListItem {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub kind: String,
    pub handler: String,
    pub capabilities: Vec<String>,
    pub enabled: bool,
    pub valid: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginTransformActionDto {
    pub plugin_id: String,
    pub plugin_name: String,
    pub transform_id: String,
    pub label: String,
}

pub fn to_plugin_list_item(installed: &InstalledPlugin) -> PluginListItem {
    match &installed.plugin.manifest {
        Some(manifest) => PluginListItem {
            id: manifest.id.clone(),
            name: manifest.name.clone(),
            version: manifest.version.clone(),
            description: manifest.description.clone(),
            kind: "content_processor".to_string(),
            handler: manifest.handler.clone(),
            capabilities: manifest
                .capabilities
                .iter()
                .map(|capability| match capability {
                    super::manifest::PluginCapability::Classify => "classify".to_string(),
                    super::manifest::PluginCapability::Transform => "transform".to_string(),
                })
                .collect(),
            enabled: installed.enabled,
            valid: installed.plugin.validation_error.is_none(),
            error: installed.plugin.validation_error.clone(),
        },
        None => PluginListItem {
            id: installed
                .plugin
                .plugin_dir
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("unknown")
                .to_string(),
            name: "Invalid Plugin".to_string(),
            version: "unknown".to_string(),
            description: None,
            kind: "content_processor".to_string(),
            handler: "unknown".to_string(),
            capabilities: vec![],
            enabled: false,
            valid: false,
            error: installed.plugin.validation_error.clone(),
        },
    }
}
```

```rust
// /Users/chanyu/AIProjects/smart-clipboard/src-tauri/src/commands.rs
use crate::plugins::commands::{to_plugin_list_item, PluginListItem, PluginTransformActionDto};
use crate::plugins::loader::load_plugins_from_dir;
use crate::plugins::registry::PluginRegistry;

#[tauri::command]
pub async fn list_plugins(
    lock: State<'_, Arc<AppLockManager>>,
    config: State<'_, Arc<ConfigManager>>,
    app_data_dir: State<'_, Arc<AppDataDir>>,
) -> Result<Vec<PluginListItem>, String> {
    require_unlocked(&lock)?;
    let plugin_dir = app_data_dir.0.join("plugins");
    let states = config
        .get()
        .plugins
        .into_iter()
        .map(|(id, entry)| (id, entry.enabled))
        .collect();
    let registry = PluginRegistry::from_loaded(
        load_plugins_from_dir(&plugin_dir, &crate::plugins::builtin::builtin_handler_registry()),
        states,
    );
    Ok(registry.plugins().iter().map(to_plugin_list_item).collect())
}

#[tauri::command]
pub async fn set_plugin_enabled(
    lock: State<'_, Arc<AppLockManager>>,
    config: State<'_, Arc<ConfigManager>>,
    plugin_id: String,
    enabled: bool,
) -> Result<AppConfig, String> {
    require_unlocked(&lock)?;
    let mut next = config.get();
    next.plugins.insert(plugin_id, crate::config::PluginConfigEntry { enabled });
    config.update(next.clone())?;
    Ok(next)
}

#[tauri::command]
pub async fn list_plugin_transforms(
    lock: State<'_, Arc<AppLockManager>>,
    config: State<'_, Arc<ConfigManager>>,
    app_data_dir: State<'_, Arc<AppDataDir>>,
    content: String,
) -> Result<Vec<PluginTransformActionDto>, String> {
    require_unlocked(&lock)?;
    let registry = PluginRegistry::from_loaded(
        load_plugins_from_dir(
            &app_data_dir.0.join("plugins"),
            &crate::plugins::builtin::builtin_handler_registry(),
        ),
        config
            .get()
            .plugins
            .into_iter()
            .map(|(id, entry)| (id, entry.enabled))
            .collect(),
    );
    Ok(registry
        .list_transform_actions(&content)
        .into_iter()
        .map(|action| PluginTransformActionDto {
            plugin_id: action.plugin_id,
            plugin_name: action.plugin_name,
            transform_id: action.transform_id,
            label: action.label,
        })
        .collect())
}

#[tauri::command]
pub async fn apply_plugin_transform(
    lock: State<'_, Arc<AppLockManager>>,
    config: State<'_, Arc<ConfigManager>>,
    app_data_dir: State<'_, Arc<AppDataDir>>,
    plugin_id: String,
    transform_id: String,
    content: String,
) -> Result<String, String> {
    require_unlocked(&lock)?;
    let registry = PluginRegistry::from_loaded(
        load_plugins_from_dir(
            &app_data_dir.0.join("plugins"),
            &crate::plugins::builtin::builtin_handler_registry(),
        ),
        config
            .get()
            .plugins
            .into_iter()
            .map(|(id, entry)| (id, entry.enabled))
            .collect(),
    );
    registry.apply_transform(&plugin_id, &transform_id, &content)
}
```

```rust
// /Users/chanyu/AIProjects/smart-clipboard/src-tauri/src/lib.rs
mod plugins;

.invoke_handler(tauri::generate_handler![
    // existing commands...
    commands::list_plugins,
    commands::set_plugin_enabled,
    commands::list_plugin_transforms,
    commands::apply_plugin_transform,
])
```

- [ ] **Step 4: Run the targeted Rust tests to verify they pass**

Run: `cargo test --manifest-path /Users/chanyu/AIProjects/smart-clipboard/src-tauri/Cargo.toml plugins::tests plugins::commands::tests -- --test-threads=1`
Expected: PASS with plugin config resolution and DTO serialization tests green.

- [ ] **Step 5: Commit the backend command and config integration**

```bash
git add /Users/chanyu/AIProjects/smart-clipboard/src-tauri/src/config.rs \
  /Users/chanyu/AIProjects/smart-clipboard/src-tauri/src/plugins/commands.rs \
  /Users/chanyu/AIProjects/smart-clipboard/src-tauri/src/commands.rs \
  /Users/chanyu/AIProjects/smart-clipboard/src-tauri/src/lib.rs
git commit -m "feat: expose plugin management commands"
```

### Task 3: Integrate plugin transforms into the existing transform command and ship the example plugin asset

**Files:**

- Modify: `/Users/chanyu/AIProjects/smart-clipboard/src-tauri/src/commands.rs`
- Create: `/Users/chanyu/AIProjects/smart-clipboard/plugins/markdown-tools/plugin.json`
- Test: `/Users/chanyu/AIProjects/smart-clipboard/src-tauri/src/plugins/mod.rs`

- [ ] **Step 1: Write the failing tests for transform fallback and example plugin manifest presence**

```rust
// Add to /Users/chanyu/AIProjects/smart-clipboard/src-tauri/src/plugins/mod.rs test module
    #[test]
    fn markdown_plugin_manifest_file_can_be_parsed() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("plugins").join("markdown-tools").join("plugin.json");
        let manifest = std::fs::read_to_string(root).unwrap();
        let parsed: super::manifest::PluginManifest = serde_json::from_str(&manifest).unwrap();
        assert_eq!(parsed.id, "markdown-tools");
        assert_eq!(parsed.handler, "builtin.markdown_tools");
    }
```

```rust
// Add to /Users/chanyu/AIProjects/smart-clipboard/src-tauri/src/commands.rs tests or create local #[cfg(test)] module
#[cfg(test)]
mod transform_tests {
    use super::transform;

    #[test]
    fn builtin_transforms_still_work() {
        assert_eq!(transform::apply_transform("hello", "uppercase").unwrap(), "HELLO");
    }
}
```

- [ ] **Step 2: Run the targeted Rust tests to verify they fail for missing plugin asset or missing test module**

Run: `cargo test --manifest-path /Users/chanyu/AIProjects/smart-clipboard/src-tauri/Cargo.toml markdown_plugin_manifest_file_can_be_parsed builtin_transforms_still_work -- --test-threads=1`
Expected: FAIL because the example plugin manifest file does not exist yet or test modules are not present.

- [ ] **Step 3: Write the minimal example plugin asset and preserve builtin transform behavior**

```json
// /Users/chanyu/AIProjects/smart-clipboard/plugins/markdown-tools/plugin.json
{
  "id": "markdown-tools",
  "name": "Markdown Tools",
  "version": "1.0.0",
  "kind": "content_processor",
  "enabledByDefault": true,
  "description": "Adds markdown-aware classification and transforms.",
  "capabilities": ["classify", "transform"],
  "handler": "builtin.markdown_tools"
}
```

```rust
// /Users/chanyu/AIProjects/smart-clipboard/src-tauri/src/commands.rs
#[tauri::command]
pub async fn transform_content(content: String, transform_type: String) -> Result<String, String> {
    transform::apply_transform(&content, &transform_type)
}
```

Keep `transform_content` behavior unchanged for built-in transforms. Plugin transforms continue to flow through the explicit `apply_plugin_transform` command added in Task 2.

- [ ] **Step 4: Run the targeted Rust tests to verify they pass**

Run: `cargo test --manifest-path /Users/chanyu/AIProjects/smart-clipboard/src-tauri/Cargo.toml markdown_plugin_manifest_file_can_be_parsed builtin_transforms_still_work -- --test-threads=1`
Expected: PASS with both tests green.

- [ ] **Step 5: Commit the example plugin asset**

```bash
git add /Users/chanyu/AIProjects/smart-clipboard/plugins/markdown-tools/plugin.json \
  /Users/chanyu/AIProjects/smart-clipboard/src-tauri/src/commands.rs
git commit -m "feat: add markdown tools plugin manifest"
```

### Task 4: Add frontend plugin types and store

**Files:**

- Create: `/Users/chanyu/AIProjects/smart-clipboard/src/types/plugins.ts`
- Create: `/Users/chanyu/AIProjects/smart-clipboard/src/stores/pluginStore.ts`
- Modify: `/Users/chanyu/AIProjects/smart-clipboard/src/types/index.ts`
- Test: `/Users/chanyu/AIProjects/smart-clipboard/tests/unit/pluginStore.test.ts`

- [ ] **Step 1: Write the failing frontend store tests**

```ts
// /Users/chanyu/AIProjects/smart-clipboard/tests/unit/pluginStore.test.ts
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke }));

import { usePluginStore } from '@/stores/pluginStore';

describe('usePluginStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    invoke.mockReset();
  });

  it('loads plugin list from backend', async () => {
    invoke.mockResolvedValueOnce([
      {
        id: 'markdown-tools',
        name: 'Markdown Tools',
        version: '1.0.0',
        description: 'Adds markdown-aware classification and transforms.',
        kind: 'content_processor',
        handler: 'builtin.markdown_tools',
        capabilities: ['classify', 'transform'],
        enabled: true,
        valid: true,
        error: null,
      },
    ]);

    const store = usePluginStore();
    await store.loadPlugins();

    expect(store.plugins).toHaveLength(1);
    expect(store.plugins[0].id).toBe('markdown-tools');
  });

  it('toggles plugin enabled state and updates local item', async () => {
    invoke
      .mockResolvedValueOnce([
        {
          id: 'markdown-tools',
          name: 'Markdown Tools',
          version: '1.0.0',
          description: 'Adds markdown-aware classification and transforms.',
          kind: 'content_processor',
          handler: 'builtin.markdown_tools',
          capabilities: ['classify', 'transform'],
          enabled: true,
          valid: true,
          error: null,
        },
      ])
      .mockResolvedValueOnce({ plugins: { 'markdown-tools': { enabled: false } } });

    const store = usePluginStore();
    await store.loadPlugins();
    await store.setPluginEnabled('markdown-tools', false);

    expect(invoke).toHaveBeenCalledWith('set_plugin_enabled', {
      pluginId: 'markdown-tools',
      enabled: false,
    });
    expect(store.plugins[0].enabled).toBe(false);
  });

  it('loads plugin transforms for content', async () => {
    invoke.mockResolvedValueOnce([
      {
        pluginId: 'markdown-tools',
        pluginName: 'Markdown Tools',
        transformId: 'strip_markdown_format',
        label: 'Strip markdown formatting',
      },
    ]);

    const store = usePluginStore();
    const actions = await store.loadTransforms('# Hello');

    expect(actions).toHaveLength(1);
    expect(actions[0].transformId).toBe('strip_markdown_format');
  });
});
```

- [ ] **Step 2: Run the frontend store test to verify it fails for missing store/types**

Run: `pnpm vitest run /Users/chanyu/AIProjects/smart-clipboard/tests/unit/pluginStore.test.ts`
Expected: FAIL with cannot resolve `@/stores/pluginStore` or missing exported types.

- [ ] **Step 3: Write the minimal plugin types and Pinia store**

```ts
// /Users/chanyu/AIProjects/smart-clipboard/src/types/plugins.ts
export interface PluginListItem {
  id: string;
  name: string;
  version: string;
  description: string | null;
  kind: string;
  handler: string;
  capabilities: string[];
  enabled: boolean;
  valid: boolean;
  error: string | null;
}

export interface PluginTransformAction {
  pluginId: string;
  pluginName: string;
  transformId: string;
  label: string;
}
```

```ts
// /Users/chanyu/AIProjects/smart-clipboard/src/stores/pluginStore.ts
import { defineStore } from 'pinia';
import { invoke } from '@tauri-apps/api/core';
import type { PluginListItem, PluginTransformAction } from '@/types/plugins';

export const usePluginStore = defineStore('plugins', {
  state: () => ({
    plugins: [] as PluginListItem[],
    loading: false,
    error: '' as string | null,
  }),
  actions: {
    async loadPlugins() {
      this.loading = true;
      this.error = null;
      try {
        this.plugins = await invoke<PluginListItem[]>('list_plugins');
      } catch (error) {
        this.error = String(error);
        throw error;
      } finally {
        this.loading = false;
      }
    },
    async setPluginEnabled(pluginId: string, enabled: boolean) {
      await invoke('set_plugin_enabled', { pluginId, enabled });
      const plugin = this.plugins.find((item) => item.id === pluginId);
      if (plugin) {
        plugin.enabled = enabled;
      }
    },
    async loadTransforms(content: string) {
      return invoke<PluginTransformAction[]>('list_plugin_transforms', { content });
    },
  },
});
```

```ts
// /Users/chanyu/AIProjects/smart-clipboard/src/types/index.ts
export * from './plugins';
```

- [ ] **Step 4: Run the frontend store test to verify it passes**

Run: `pnpm vitest run /Users/chanyu/AIProjects/smart-clipboard/tests/unit/pluginStore.test.ts`
Expected: PASS with 3 tests passed.

- [ ] **Step 5: Commit the plugin store layer**

```bash
git add /Users/chanyu/AIProjects/smart-clipboard/src/types/plugins.ts \
  /Users/chanyu/AIProjects/smart-clipboard/src/stores/pluginStore.ts \
  /Users/chanyu/AIProjects/smart-clipboard/src/types/index.ts \
  /Users/chanyu/AIProjects/smart-clipboard/tests/unit/pluginStore.test.ts
git commit -m "feat: add frontend plugin store"
```

### Task 5: Render plugins in SettingsPanel and add enable/disable controls

**Files:**

- Modify: `/Users/chanyu/AIProjects/smart-clipboard/src/components/SettingsPanel.vue`
- Modify: `/Users/chanyu/AIProjects/smart-clipboard/src/i18n/locales/en.ts`
- Modify: `/Users/chanyu/AIProjects/smart-clipboard/src/i18n/locales/zh-CN.ts`
- Test: `/Users/chanyu/AIProjects/smart-clipboard/tests/unit/SettingsPanel.plugins.test.ts`

- [ ] **Step 1: Write the failing settings panel plugin tests**

```ts
// /Users/chanyu/AIProjects/smart-clipboard/tests/unit/SettingsPanel.plugins.test.ts
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { mount, flushPromises } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import i18n from '@/i18n';

const { invoke, pluginStoreMock } = vi.hoisted(() => ({
  invoke: vi.fn(),
  pluginStoreMock: {
    plugins: [
      {
        id: 'markdown-tools',
        name: 'Markdown Tools',
        version: '1.0.0',
        description: 'Adds markdown-aware classification and transforms.',
        kind: 'content_processor',
        handler: 'builtin.markdown_tools',
        capabilities: ['classify', 'transform'],
        enabled: true,
        valid: true,
        error: null,
      },
      {
        id: 'broken-plugin',
        name: 'Broken Plugin',
        version: '1.0.0',
        description: null,
        kind: 'content_processor',
        handler: 'builtin.missing',
        capabilities: [],
        enabled: false,
        valid: false,
        error: 'Unknown handler: builtin.missing',
      },
    ],
    loading: false,
    error: null,
    loadPlugins: vi.fn().mockResolvedValue(undefined),
    setPluginEnabled: vi.fn().mockResolvedValue(undefined),
  },
}));

vi.mock('@tauri-apps/api/core', () => ({ invoke }));
vi.mock('@/stores/pluginStore', () => ({
  usePluginStore: () => pluginStoreMock,
}));
vi.mock('@/stores/updaterStore', () => ({
  useUpdaterStore: () => ({
    status: null,
    isChecking: false,
    loadStatus: vi.fn().mockResolvedValue(undefined),
    checkNow: vi.fn().mockResolvedValue(undefined),
    installPending: vi.fn().mockResolvedValue(undefined),
    downloadAvailable: vi.fn().mockResolvedValue(undefined),
    discardPending: vi.fn().mockResolvedValue(undefined),
    bindEvents: vi.fn().mockResolvedValue(undefined),
  }),
}));
vi.mock('@/stores/securityStore', () => ({
  useSecurityStore: () => ({
    status: { biometric_available: false },
    encryption: {
      enabled: false,
      key_exists: false,
      encrypted_count: 0,
      plaintext_count: 0,
      migrating: false,
    },
    loading: false,
    refresh: vi.fn().mockResolvedValue(undefined),
    updateSettings: vi.fn().mockResolvedValue(undefined),
    setPassword: vi.fn().mockResolvedValue(undefined),
    lock: vi.fn().mockResolvedValue(undefined),
    refreshEncryption: vi.fn().mockResolvedValue(undefined),
    enableEncryption: vi.fn().mockResolvedValue(undefined),
    disableEncryption: vi.fn().mockResolvedValue(undefined),
  }),
}));
vi.mock('@/composables/useTheme', () => ({
  useTheme: () => ({
    appearance: 'system',
    themeColor: 'zinc',
    setAppearance: vi.fn(),
    setThemeColor: vi.fn(),
  }),
}));
vi.mock('@/i18n', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/i18n')>();
  return { ...actual, setLocale: vi.fn() };
});

import SettingsPanel from '@/components/SettingsPanel.vue';

describe('SettingsPanel plugin section', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    invoke.mockReset();
    invoke.mockImplementation((cmd: string) => {
      if (cmd === 'get_config') {
        return Promise.resolve({
          max_entries: 5000,
          retention_days: 30,
          excluded_apps: [],
          monitor_interval_ms: 500,
          autostart_enabled: false,
          sensitive_expiry_minutes: 5,
          app_lock: { enabled: false, auto_lock_seconds: 0, biometric_enabled: false },
          updater: {
            auto_check_enabled: true,
            check_interval_hours: 24,
            auto_download_enabled: false,
            wifi_only: true,
            mirrors: [],
            last_check_at: null,
          },
          plugins: {},
        });
      }
      if (cmd === 'get_autostart_enabled') return Promise.resolve(false);
      if (cmd === 'update_config') return Promise.resolve();
      return Promise.resolve();
    });
  });

  it('renders valid and invalid plugins', async () => {
    const wrapper = mount(SettingsPanel, {
      props: { isOpen: true },
      global: { plugins: [createPinia(), i18n] },
    });

    await flushPromises();

    expect(pluginStoreMock.loadPlugins).toHaveBeenCalled();
    expect(wrapper.text()).toContain('Plugins');
    expect(wrapper.text()).toContain('Markdown Tools');
    expect(wrapper.text()).toContain('Broken Plugin');
    expect(wrapper.text()).toContain('Unknown handler: builtin.missing');
  });

  it('toggles a valid plugin', async () => {
    const wrapper = mount(SettingsPanel, {
      props: { isOpen: true },
      global: { plugins: [createPinia(), i18n] },
    });

    await flushPromises();
    const toggle = wrapper.find('[data-test="plugin-toggle-markdown-tools"]');
    await toggle.trigger('click');

    expect(pluginStoreMock.setPluginEnabled).toHaveBeenCalledWith('markdown-tools', false);
  });
});
```

- [ ] **Step 2: Run the settings panel plugin test to verify it fails**

Run: `pnpm vitest run /Users/chanyu/AIProjects/smart-clipboard/tests/unit/SettingsPanel.plugins.test.ts`
Expected: FAIL because the plugin section and toggle selectors do not exist yet.

- [ ] **Step 3: Write the minimal SettingsPanel plugin UI and translations**

```vue
<!-- Add inside /Users/chanyu/AIProjects/smart-clipboard/src/components/SettingsPanel.vue template, near other settings sections -->
<Separator />

<div class="space-y-3">
  <div class="space-y-1">
    <label class="text-sm font-medium">{{ $t('settings.plugins.title') }}</label>
    <p class="text-xs text-muted-foreground">{{ $t('settings.plugins.hint') }}</p>
  </div>

  <div v-if="pluginStore.loading" class="text-xs text-muted-foreground">
    {{ $t('settings.plugins.loading') }}
  </div>

  <div v-else-if="pluginStore.plugins.length === 0" class="text-xs text-muted-foreground">
    {{ $t('settings.plugins.empty') }}
  </div>

  <div v-else class="space-y-2">
    <div
      v-for="plugin in pluginStore.plugins"
      :key="plugin.id"
      class="rounded-md border border-input p-3 space-y-2"
    >
      <div class="flex items-start justify-between gap-3">
        <div class="space-y-1 min-w-0">
          <div class="text-sm font-medium">{{ plugin.name }}</div>
          <div class="text-xs text-muted-foreground">{{ plugin.id }} · v{{ plugin.version }}</div>
          <div v-if="plugin.description" class="text-xs text-muted-foreground">
            {{ plugin.description }}
          </div>
          <div class="text-xs text-muted-foreground">
            {{ plugin.kind }} · {{ plugin.handler }}
          </div>
          <div class="text-xs text-muted-foreground">
            {{ plugin.capabilities.join(', ') || $t('settings.plugins.noCapabilities') }}
          </div>
          <div v-if="!plugin.valid && plugin.error" class="text-xs text-destructive">
            {{ plugin.error }}
          </div>
        </div>

        <button
          v-if="plugin.valid"
          :data-test="`plugin-toggle-${plugin.id}`"
          class="relative inline-flex h-5 w-9 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors"
          :class="plugin.enabled ? 'bg-primary' : 'bg-input'"
          @click="togglePlugin(plugin.id, !plugin.enabled)"
        >
          <span
            class="pointer-events-none block h-4 w-4 rounded-full bg-background shadow-lg transition-transform"
            :class="plugin.enabled ? 'translate-x-4' : 'translate-x-0'"
          />
        </button>
      </div>
    </div>
  </div>
</div>
```

```ts
// Add inside /Users/chanyu/AIProjects/smart-clipboard/src/components/SettingsPanel.vue <script setup>
import { usePluginStore } from '@/stores/pluginStore';

const pluginStore = usePluginStore();

async function loadPluginState() {
  try {
    await pluginStore.loadPlugins();
  } catch (error) {
    console.error('Failed to load plugins', error);
  }
}

async function togglePlugin(pluginId: string, enabled: boolean) {
  await pluginStore.setPluginEnabled(pluginId, enabled);
}

watch(
  () => props.isOpen,
  (open) => {
    if (open) {
      void loadPluginState();
    }
  },
  { immediate: true },
);
```

```ts
// /Users/chanyu/AIProjects/smart-clipboard/src/i18n/locales/en.ts
settings: {
  plugins: {
    title: 'Plugins',
    hint: 'Manage installed extension manifests loaded at app startup.',
    loading: 'Loading plugins...',
    empty: 'No plugins found.',
    noCapabilities: 'No capabilities',
  },
}
```

```ts
// /Users/chanyu/AIProjects/smart-clipboard/src/i18n/locales/zh-CN.ts
settings: {
  plugins: {
    title: '插件',
    hint: '管理应用启动时加载的已安装扩展清单。',
    loading: '正在加载插件…',
    empty: '未发现插件。',
    noCapabilities: '无能力声明',
  },
}
```

- [ ] **Step 4: Run the settings panel plugin test to verify it passes**

Run: `pnpm vitest run /Users/chanyu/AIProjects/smart-clipboard/tests/unit/SettingsPanel.plugins.test.ts`
Expected: PASS with 2 tests passed.

- [ ] **Step 5: Commit the settings plugin UI**

```bash
git add /Users/chanyu/AIProjects/smart-clipboard/src/components/SettingsPanel.vue \
  /Users/chanyu/AIProjects/smart-clipboard/src/i18n/locales/en.ts \
  /Users/chanyu/AIProjects/smart-clipboard/src/i18n/locales/zh-CN.ts \
  /Users/chanyu/AIProjects/smart-clipboard/tests/unit/SettingsPanel.plugins.test.ts
git commit -m "feat: add plugin settings section"
```

### Task 6: Render plugin transforms in TransformMenu and invoke plugin actions end-to-end

**Files:**

- Modify: `/Users/chanyu/AIProjects/smart-clipboard/src/components/TransformMenu.vue`
- Modify: `/Users/chanyu/AIProjects/smart-clipboard/src/i18n/locales/en.ts`
- Modify: `/Users/chanyu/AIProjects/smart-clipboard/src/i18n/locales/zh-CN.ts`
- Test: `/Users/chanyu/AIProjects/smart-clipboard/tests/unit/TransformMenu.plugins.test.ts`

- [ ] **Step 1: Write the failing transform menu plugin tests**

```ts
// /Users/chanyu/AIProjects/smart-clipboard/tests/unit/TransformMenu.plugins.test.ts
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { mount, flushPromises } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import i18n from '@/i18n';

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke }));

vi.mock('@/stores/pluginStore', () => ({
  usePluginStore: () => ({
    loadTransforms: vi.fn().mockResolvedValue([
      {
        pluginId: 'markdown-tools',
        pluginName: 'Markdown Tools',
        transformId: 'strip_markdown_format',
        label: 'Strip markdown formatting',
      },
    ]),
  }),
}));

import TransformMenu from '@/components/TransformMenu.vue';

describe('TransformMenu plugin transforms', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    invoke.mockReset();
  });

  it('renders plugin transform actions alongside builtin actions', async () => {
    const wrapper = mount(TransformMenu, {
      props: { content: '# Hello', category: 'text' },
      global: { plugins: [createPinia(), i18n] },
    });

    await wrapper.find('button[title="Transforms"]').trigger('click');
    await flushPromises();

    expect(wrapper.text()).toContain('Strip markdown formatting');
  });

  it('invokes apply_plugin_transform for plugin actions', async () => {
    invoke.mockResolvedValueOnce('Hello');
    const wrapper = mount(TransformMenu, {
      props: { content: '# Hello', category: 'text' },
      global: { plugins: [createPinia(), i18n] },
    });

    await wrapper.find('button[title="Transforms"]').trigger('click');
    await flushPromises();
    await wrapper
      .find('[data-test="plugin-transform-markdown-tools-strip_markdown_format"]')
      .trigger('click');

    expect(invoke).toHaveBeenCalledWith('apply_plugin_transform', {
      pluginId: 'markdown-tools',
      transformId: 'strip_markdown_format',
      content: '# Hello',
    });
  });
});
```

- [ ] **Step 2: Run the transform menu plugin test to verify it fails**

Run: `pnpm vitest run /Users/chanyu/AIProjects/smart-clipboard/tests/unit/TransformMenu.plugins.test.ts`
Expected: FAIL because plugin transform loading/rendering is not implemented.

- [ ] **Step 3: Write the minimal TransformMenu plugin integration**

```ts
// Add inside /Users/chanyu/AIProjects/smart-clipboard/src/components/TransformMenu.vue <script setup>
import { watch } from 'vue';
import { usePluginStore } from '@/stores/pluginStore';
import type { PluginTransformAction } from '@/types/plugins';

const pluginStore = usePluginStore();
const pluginTransforms = ref<PluginTransformAction[]>([]);

watch(
  () => [isOpen.value, props.content] as const,
  async ([open, content]) => {
    if (!open || !content.trim()) {
      pluginTransforms.value = [];
      return;
    }
    pluginTransforms.value = await pluginStore.loadTransforms(content);
  },
  { immediate: true },
);

async function handlePluginTransform(action: PluginTransformAction) {
  try {
    const result = await invoke<string>('apply_plugin_transform', {
      pluginId: action.pluginId,
      transformId: action.transformId,
      content: props.content,
    });
    await navigator.clipboard.writeText(result);
    showToast(t('transforms.copied'));
  } catch (err) {
    showToast(String(err));
  }
  closeMenu();
}
```

```vue
<!-- Add inside /Users/chanyu/AIProjects/smart-clipboard/src/components/TransformMenu.vue menu dropdown -->
<div v-if="pluginTransforms.length" class="border-t mt-1 pt-1">
  <div class="px-2 py-1 text-[10px] uppercase tracking-wide text-muted-foreground">
    {{ t('transforms.plugins') }}
  </div>
  <button
    v-for="action in pluginTransforms"
    :key="`${action.pluginId}:${action.transformId}`"
    :data-test="`plugin-transform-${action.pluginId}-${action.transformId}`"
    class="flex w-full items-center rounded-sm px-2 py-1.5 text-xs hover:bg-accent hover:text-accent-foreground cursor-pointer"
    @click.stop="handlePluginTransform(action)"
  >
    {{ action.pluginName }} · {{ action.label }}
  </button>
</div>
```

```ts
// /Users/chanyu/AIProjects/smart-clipboard/src/i18n/locales/en.ts
transforms: {
  plugins: 'Plugin transforms',
}
```

```ts
// /Users/chanyu/AIProjects/smart-clipboard/src/i18n/locales/zh-CN.ts
transforms: {
  plugins: '插件转换',
}
```

- [ ] **Step 4: Run the transform menu plugin test to verify it passes**

Run: `pnpm vitest run /Users/chanyu/AIProjects/smart-clipboard/tests/unit/TransformMenu.plugins.test.ts`
Expected: PASS with 2 tests passed.

- [ ] **Step 5: Commit the transform menu plugin integration**

```bash
git add /Users/chanyu/AIProjects/smart-clipboard/src/components/TransformMenu.vue \
  /Users/chanyu/AIProjects/smart-clipboard/src/i18n/locales/en.ts \
  /Users/chanyu/AIProjects/smart-clipboard/src/i18n/locales/zh-CN.ts \
  /Users/chanyu/AIProjects/smart-clipboard/tests/unit/TransformMenu.plugins.test.ts
git commit -m "feat: add plugin transform actions"
```

### Task 7: Run full verification and document implementation status

**Files:**

- Modify: `/Users/chanyu/AIProjects/smart-clipboard/docs/superpowers/plans/2026-04-25-plugin-extension-system.md`
- Test: `/Users/chanyu/AIProjects/smart-clipboard/tests/unit/pluginStore.test.ts`
- Test: `/Users/chanyu/AIProjects/smart-clipboard/tests/unit/SettingsPanel.plugins.test.ts`
- Test: `/Users/chanyu/AIProjects/smart-clipboard/tests/unit/TransformMenu.plugins.test.ts`
- Test: `/Users/chanyu/AIProjects/smart-clipboard/src-tauri/src/plugins/mod.rs`

- [ ] **Step 1: Run the focused frontend verification suite**

Run: `pnpm vitest run /Users/chanyu/AIProjects/smart-clipboard/tests/unit/pluginStore.test.ts /Users/chanyu/AIProjects/smart-clipboard/tests/unit/SettingsPanel.plugins.test.ts /Users/chanyu/AIProjects/smart-clipboard/tests/unit/TransformMenu.plugins.test.ts`
Expected: PASS with all plugin-related frontend tests green.

- [ ] **Step 2: Run the focused Rust verification suite**

Run: `cargo test --manifest-path /Users/chanyu/AIProjects/smart-clipboard/src-tauri/Cargo.toml plugins::tests plugins::commands::tests builtin_transforms_still_work markdown_plugin_manifest_file_can_be_parsed -- --test-threads=1`
Expected: PASS with all plugin-related backend tests green.

- [ ] **Step 3: Run project-level quality checks impacted by the change**

Run: `pnpm lint && pnpm typecheck && cargo test --manifest-path /Users/chanyu/AIProjects/smart-clipboard/src-tauri/Cargo.toml -- --test-threads=1`
Expected: PASS with no lint/type/test failures.

- [ ] **Step 4: Update the plan checklist to reflect completed work**

```markdown
- [x] Task 1 complete
- [x] Task 2 complete
- [x] Task 3 complete
- [x] Task 4 complete
- [x] Task 5 complete
- [x] Task 6 complete
- [x] Task 7 verification complete
```

- [ ] **Step 5: Commit final verified implementation**

```bash
git add /Users/chanyu/AIProjects/smart-clipboard
git commit -m "feat: implement plugin extension system"
```

---

## Self-Review

### Spec coverage

- Local `plugins/` scanning: covered by Tasks 1–3.
- Manifest schema + validation + duplicate handling: covered by Task 1.
- Config-persisted enablement: covered by Task 2.
- Built-in trusted handler execution model: covered by Task 1.
- Settings plugin listing/toggle: covered by Task 5.
- Transform integration with example plugin: covered by Tasks 3 and 6.
- Example markdown plugin asset: covered by Task 3.
- Error visibility for invalid plugins: covered by Tasks 2 and 5.
- Verification across backend/frontend: covered by Task 7.

### Placeholder scan

- No `TBD`/`TODO` placeholders remain.
- Each task includes explicit file paths, commands, and concrete code.
- Tests are specified before implementation steps for every component.

### Type consistency

- Backend command names are consistently `list_plugins`, `set_plugin_enabled`, `list_plugin_transforms`, `apply_plugin_transform`.
- Frontend DTO names consistently use `PluginListItem` and `PluginTransformAction`.
- Example handler and manifest consistently use `builtin.markdown_tools` and `markdown-tools`.
