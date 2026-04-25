pub mod builtin;
pub mod loader;
pub mod manifest;
pub mod registry;

#[cfg(test)]
mod tests {
    use super::builtin::builtin_handler_registry;
    use super::loader::load_plugins_from_dir;
    use super::manifest::{PluginCapability, PluginKind};
    use super::registry::PluginRegistry;
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

        let classifications = registry.classify_content("- [x] shared handler");
        assert_eq!(classifications.len(), 2);
        assert!(classifications
            .iter()
            .any(|result| result.plugin_id == "markdown-tools-a"));
        assert!(classifications
            .iter()
            .any(|result| result.plugin_id == "markdown-tools-b"));
    }

    #[test]
    fn preserves_invalid_plugins_in_registry_but_excludes_them_from_dispatch() {
        let temp_dir = tempfile::tempdir().unwrap();
        write_plugin(
            temp_dir.path(),
            "valid-plugin",
            r#"{
              "id": "valid-plugin",
              "name": "Valid Plugin",
              "version": "1.0.0",
              "kind": "content_processor",
              "enabledByDefault": true,
              "capabilities": ["classify"],
              "handler": "builtin.markdown_tools"
            }"#,
        );
        write_plugin(
            temp_dir.path(),
            "invalid-plugin",
            r#"{
              "id": "invalid-plugin",
              "name": "Invalid Plugin",
              "version": "1.0.0",
              "kind": "content_processor",
              "enabledByDefault": true,
              "capabilities": ["classify"],
              "handler": "builtin.missing"
            }"#,
        );

        let loaded = load_plugins_from_dir(temp_dir.path(), &builtin_handler_registry());
        let registry = PluginRegistry::from_loaded(loaded, std::collections::HashMap::new());

        let discovered = registry.plugins();
        assert_eq!(discovered.len(), 2);
        assert_eq!(
            discovered.iter().filter(|plugin| plugin.is_valid).count(),
            1
        );
        assert_eq!(
            discovered.iter().filter(|plugin| !plugin.is_valid).count(),
            1
        );
        assert!(discovered.iter().any(|plugin| plugin
            .validation_error
            .as_deref()
            .unwrap_or("")
            .contains("unknown handler")));

        let classifications = registry.classify_content("- [x] visible");
        assert_eq!(classifications.len(), 1);
        assert_eq!(classifications[0].plugin_id, "valid-plugin");
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

        let discovered = registry.plugins();
        assert_eq!(discovered.len(), 1);
        assert!(!discovered[0].enabled);
        assert!(discovered[0].is_valid);
    }
}
