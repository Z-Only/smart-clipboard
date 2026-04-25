pub mod builtin;
pub mod loader;
pub mod manifest;
pub mod registry;

#[cfg(test)]
mod tests {
    use super::builtin::builtin_handler_registry;
    use super::loader::load_plugins_from_dir;
    use super::manifest::PluginCapability;
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
        assert_eq!(
            loaded.plugins[0].manifest.as_ref().unwrap().id,
            "markdown-tools"
        );
        assert_eq!(
            loaded.plugins[0].manifest.as_ref().unwrap().capabilities,
            vec![PluginCapability::Classify, PluginCapability::Transform]
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
