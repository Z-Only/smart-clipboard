pub mod builtin;
pub mod commands;
pub mod loader;
pub mod manifest;
pub mod registry;

#[cfg(test)]
mod tests {
    use super::builtin::builtin_handler_registry;
    use super::commands::{PluginListItemDto, PluginTransformActionDto};
    use super::loader::load_plugins_from_dir;
    use super::manifest::{PluginCapability, PluginKind};
    use super::registry::PluginRegistry;
    use crate::config::AppConfig;
    use std::fs;

    fn write_plugin(dir: &std::path::Path, plugin_dir_name: &str, body: &str) {
        let plugin_dir = dir.join(plugin_dir_name);
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
        assert_eq!(
            loaded.plugins[0].manifest.as_ref().unwrap().id,
            "markdown-tools"
        );
        assert_eq!(
            loaded.plugins[0].manifest.as_ref().unwrap().kind,
            PluginKind::ContentProcessor
        );
        assert!(
            loaded.plugins[0]
                .manifest
                .as_ref()
                .unwrap()
                .enabled_by_default
        );
        assert_eq!(
            loaded.plugins[0].manifest.as_ref().unwrap().capabilities,
            vec![PluginCapability::Classify, PluginCapability::Transform]
        );
    }

    #[test]
    fn defaults_enabled_by_default_when_omitted() {
        let temp_dir = tempfile::tempdir().unwrap();
        write_plugin(
            temp_dir.path(),
            "markdown-default-enabled",
            r#"{
              "id": "markdown-default-enabled",
              "name": "Markdown Tools",
              "version": "1.0.0",
              "kind": "content_processor",
              "capabilities": ["classify"],
              "handler": "builtin.markdown_tools"
            }"#,
        );

        let loaded = load_plugins_from_dir(temp_dir.path(), &builtin_handler_registry());
        assert_eq!(loaded.plugins.len(), 1);
        assert!(loaded.plugins[0].validation_error.is_none());
        assert!(
            loaded.plugins[0]
                .manifest
                .as_ref()
                .unwrap()
                .enabled_by_default
        );
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
    fn marks_duplicate_plugin_ids_invalid_without_hiding_either_plugin() {
        let temp_dir = tempfile::tempdir().unwrap();
        write_plugin(
            temp_dir.path(),
            "markdown-tools-a",
            r#"{
              "id": "markdown-tools",
              "name": "Markdown Tools A",
              "version": "1.0.0",
              "kind": "content_processor",
              "enabledByDefault": true,
              "capabilities": ["classify"],
              "handler": "builtin.markdown_tools"
            }"#,
        );
        write_plugin(
            temp_dir.path(),
            "markdown-tools-b",
            r#"{
              "id": "markdown-tools",
              "name": "Markdown Tools B",
              "version": "1.0.0",
              "kind": "content_processor",
              "enabledByDefault": true,
              "capabilities": ["transform"],
              "handler": "builtin.markdown_tools"
            }"#,
        );

        let loaded = load_plugins_from_dir(temp_dir.path(), &builtin_handler_registry());
        assert_eq!(loaded.plugins.len(), 2);
        assert!(loaded
            .plugins
            .iter()
            .all(|plugin| plugin.manifest.is_some()));
        assert_eq!(
            loaded
                .plugins
                .iter()
                .filter(|plugin| plugin.validation_error.is_some())
                .count(),
            1
        );
        assert!(loaded.plugins.iter().any(|plugin| plugin
            .validation_error
            .as_deref()
            .unwrap_or("")
            .contains("duplicate plugin id")));
    }

    #[test]
    fn keeps_invalid_manifest_json_in_discovered_plugin_list() {
        let temp_dir = tempfile::tempdir().unwrap();
        write_plugin(
            temp_dir.path(),
            "bad-json",
            r#"{ "id": "broken", "name": "Broken" "version": "1.0.0" }"#,
        );

        let loaded = load_plugins_from_dir(temp_dir.path(), &builtin_handler_registry());
        assert_eq!(loaded.plugins.len(), 1);
        assert!(loaded.plugins[0].manifest.is_none());
        assert!(loaded.plugins[0]
            .validation_error
            .as_deref()
            .unwrap_or("")
            .contains("invalid plugin manifest JSON"));

        let registry = PluginRegistry::from_loaded(loaded, std::collections::HashMap::new());
        assert_eq!(registry.plugins().len(), 1);
        assert!(!registry.plugins()[0].is_valid);
        assert!(registry.plugins()[0].manifest.is_none());
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
            .apply_transform(
                "markdown-tools",
                "strip_markdown_format",
                "# Hello\n**world**",
            )
            .unwrap();
        assert_eq!(transformed, "Hello\nworld");
    }

    #[test]
    fn allows_multiple_plugins_to_share_same_handler() {
        let temp_dir = tempfile::tempdir().unwrap();
        write_plugin(
            temp_dir.path(),
            "markdown-tools-a",
            r#"{
              "id": "markdown-tools-a",
              "name": "Markdown Tools A",
              "version": "1.0.0",
              "kind": "content_processor",
              "enabledByDefault": true,
              "capabilities": ["classify"],
              "handler": "builtin.markdown_tools"
            }"#,
        );
        write_plugin(
            temp_dir.path(),
            "markdown-tools-b",
            r#"{
              "id": "markdown-tools-b",
              "name": "Markdown Tools B",
              "version": "1.0.0",
              "kind": "content_processor",
              "enabledByDefault": true,
              "capabilities": ["classify"],
              "handler": "builtin.markdown_tools"
            }"#,
        );

        let loaded = load_plugins_from_dir(temp_dir.path(), &builtin_handler_registry());
        let registry = PluginRegistry::from_loaded(loaded, std::collections::HashMap::new());

        let discovered = registry.plugins();
        assert_eq!(discovered.len(), 2);
        assert!(discovered.iter().all(|plugin| plugin.is_valid));

        let classifications = registry.classify_content("- [ ] one\n- [x] two");
        assert_eq!(classifications.len(), 2);
        assert_eq!(classifications[0].plugin_id, "markdown-tools-a");
        assert_eq!(classifications[1].plugin_id, "markdown-tools-b");
    }

    #[test]
    fn respects_config_enablement_overrides_for_registry_dispatch() {
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
        let mut config = AppConfig::default();
        config.plugin_enabled.insert("markdown-tools".into(), false);
        let registry = PluginRegistry::from_loaded(loaded, config.plugin_enabled.clone());

        assert!(registry.classify_content("- [x] write tests").is_empty());
        assert!(registry.list_transform_actions("# Hello").is_empty());
        assert_eq!(
            registry.apply_transform("markdown-tools", "strip_markdown_format", "# Hello"),
            None
        );

        let plugins = registry.plugins();
        assert_eq!(plugins.len(), 1);
        assert!(!plugins[0].enabled);
    }

    #[test]
    fn plugin_enablement_persists_in_app_config_json() {
        let mut config = AppConfig::default();
        config.plugin_enabled.insert("markdown-tools".into(), false);

        let json = serde_json::to_string(&config).unwrap();
        let restored: AppConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.plugin_enabled.get("markdown-tools"), Some(&false));
    }

    #[test]
    fn exports_plugin_dtos_with_manifest_and_error_metadata() {
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
        let registry = PluginRegistry::from_loaded(loaded, std::collections::HashMap::new());
        let plugins = registry.plugins();

        let valid = PluginListItemDto::from(plugins[1]);
        assert_eq!(valid.id.as_deref(), Some("markdown-tools"));
        assert_eq!(valid.name.as_deref(), Some("Markdown Tools"));
        assert!(valid.is_valid);
        assert_eq!(valid.validation_error, None);

        let invalid = PluginListItemDto::from(plugins[0]);
        assert_eq!(invalid.id.as_deref(), Some("broken-plugin"));
        assert!(!invalid.is_valid);
        assert!(invalid
            .validation_error
            .as_deref()
            .unwrap()
            .contains("unknown handler"));

        let actions = registry.list_transform_actions("# Hello\n**world**");
        let action_dto = PluginTransformActionDto::from(actions[0].clone());
        assert_eq!(action_dto.plugin_id, "markdown-tools");
        assert_eq!(action_dto.action_id, "strip_markdown_format");
        assert_eq!(action_dto.label, "Strip Markdown Formatting");
    }
}
