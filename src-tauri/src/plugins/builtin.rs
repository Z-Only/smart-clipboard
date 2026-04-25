use std::collections::HashMap;

use regex::Regex;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginClassification {
    pub plugin_id: String,
    pub classification: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginTransformAction {
    pub plugin_id: String,
    pub action_id: String,
    pub label: String,
}

pub trait PluginHandler: Send + Sync {
    fn classify(&self, _plugin_id: &str, _content: &str) -> Vec<PluginClassification> {
        Vec::new()
    }

    fn transforms(&self, _plugin_id: &str, _content: &str) -> Vec<PluginTransformAction> {
        Vec::new()
    }

    fn apply_transform(&self, _action_id: &str, _content: &str) -> Option<String> {
        None
    }
}

pub type HandlerRegistry = HashMap<String, Box<dyn PluginHandler>>;

pub fn builtin_handler_registry() -> HandlerRegistry {
    let mut handlers: HandlerRegistry = HashMap::new();
    handlers.insert(
        "builtin.markdown_tools".to_string(),
        Box::new(MarkdownToolsHandler),
    );
    handlers
}

struct MarkdownToolsHandler;

impl PluginHandler for MarkdownToolsHandler {
    fn classify(&self, plugin_id: &str, content: &str) -> Vec<PluginClassification> {
        let trimmed = content.trim_start();
        let is_checklist = trimmed.starts_with("- [x]")
            || trimmed.starts_with("- [X]")
            || trimmed.starts_with("- [ ]")
            || trimmed.starts_with("* [x]")
            || trimmed.starts_with("* [X]")
            || trimmed.starts_with("* [ ]");

        if is_checklist {
            vec![PluginClassification {
                plugin_id: plugin_id.to_string(),
                classification: "markdown_checklist".to_string(),
            }]
        } else {
            Vec::new()
        }
    }

    fn transforms(&self, plugin_id: &str, content: &str) -> Vec<PluginTransformAction> {
        if !looks_like_markdown(content) {
            return Vec::new();
        }

        vec![
            PluginTransformAction {
                plugin_id: plugin_id.to_string(),
                action_id: "strip_markdown_format".to_string(),
                label: "Strip Markdown Formatting".to_string(),
            },
            PluginTransformAction {
                plugin_id: plugin_id.to_string(),
                action_id: "extract_plain_text".to_string(),
                label: "Extract Plain Text".to_string(),
            },
        ]
    }

    fn apply_transform(&self, action_id: &str, content: &str) -> Option<String> {
        match action_id {
            "strip_markdown_format" | "extract_plain_text" => Some(strip_markdown(content)),
            _ => None,
        }
    }
}

fn looks_like_markdown(content: &str) -> bool {
    let trimmed = content.trim();
    trimmed.starts_with('#')
        || trimmed.contains("**")
        || trimmed.contains("`")
        || trimmed.contains("- [")
        || trimmed.contains("*")
}

fn strip_markdown(content: &str) -> String {
    let mut output = content.to_string();

    let heading_re = Regex::new(r"(?m)^#{1,6}\s*").unwrap();
    output = heading_re.replace_all(&output, "").into_owned();

    let emphasis_re = Regex::new(r"(\*\*|__|\*|_|`)").unwrap();
    output = emphasis_re.replace_all(&output, "").into_owned();

    let checklist_re = Regex::new(r"(?m)^\s*[-*]\s*\[(?: |x|X)\]\s*").unwrap();
    output = checklist_re.replace_all(&output, "").into_owned();

    output.trim().to_string()
}
