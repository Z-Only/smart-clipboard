use std::collections::HashMap;

use regex::Regex;

/// Extract unique placeholder names from content using `{{name}}` syntax.
/// Returns names in order of first appearance, with duplicates removed.
pub fn extract_placeholders(content: &str) -> Vec<String> {
    let re = Regex::new(r"\{\{(\w+)\}\}").unwrap();
    let mut seen = Vec::new();
    for cap in re.captures_iter(content) {
        let name = cap[1].to_string();
        if !seen.contains(&name) {
            seen.push(name);
        }
    }
    seen
}

/// Replace all `{{name}}` placeholders with values from the hashmap.
/// If a placeholder has no corresponding value in the map, leave it as-is.
pub fn render(content: &str, values: &HashMap<String, String>) -> String {
    let re = Regex::new(r"\{\{(\w+)\}\}").unwrap();
    re.replace_all(content, |caps: &regex::Captures| {
        let key = &caps[1];
        match values.get(key) {
            Some(val) => val.clone(),
            None => caps[0].to_string(),
        }
    })
    .into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- extract_placeholders tests ---

    #[test]
    fn test_extract_single_placeholder() {
        let result = extract_placeholders("Hello {{name}}!");
        assert_eq!(result, vec!["name"]);
    }

    #[test]
    fn test_extract_multiple_placeholders() {
        let result = extract_placeholders("Dear {{name}}, your order {{order_id}} is ready.");
        assert_eq!(result, vec!["name", "order_id"]);
    }

    #[test]
    fn test_extract_duplicate_placeholders() {
        let result = extract_placeholders("{{name}} said hello to {{name}} and {{other}}.");
        assert_eq!(result, vec!["name", "other"]);
    }

    #[test]
    fn test_extract_no_placeholders() {
        let result = extract_placeholders("No placeholders here.");
        assert!(result.is_empty());
    }

    #[test]
    fn test_extract_empty_string() {
        let result = extract_placeholders("");
        assert!(result.is_empty());
    }

    #[test]
    fn test_extract_preserves_first_appearance_order() {
        let result = extract_placeholders("{{c}} {{a}} {{b}} {{a}} {{c}}");
        assert_eq!(result, vec!["c", "a", "b"]);
    }

    #[test]
    fn test_extract_underscore_and_numbers() {
        let result = extract_placeholders("{{var_1}} and {{item2}}");
        assert_eq!(result, vec!["var_1", "item2"]);
    }

    #[test]
    fn test_extract_ignores_malformed_braces() {
        let result = extract_placeholders("{name} and {{}} and {{ name}} and {{name }}");
        // Only {{}} is empty which \w+ won't match; {{ name}} has a space before name
        assert!(result.is_empty());
    }

    // --- render tests ---

    #[test]
    fn test_render_single_placeholder() {
        let mut values = HashMap::new();
        values.insert("name".to_string(), "Alice".to_string());
        let result = render("Hello {{name}}!", &values);
        assert_eq!(result, "Hello Alice!");
    }

    #[test]
    fn test_render_multiple_placeholders() {
        let mut values = HashMap::new();
        values.insert("name".to_string(), "Bob".to_string());
        values.insert("order_id".to_string(), "12345".to_string());
        let result = render("Dear {{name}}, order {{order_id}} is ready.", &values);
        assert_eq!(result, "Dear Bob, order 12345 is ready.");
    }

    #[test]
    fn test_render_duplicate_placeholders() {
        let mut values = HashMap::new();
        values.insert("name".to_string(), "Charlie".to_string());
        let result = render("{{name}} met {{name}}", &values);
        assert_eq!(result, "Charlie met Charlie");
    }

    #[test]
    fn test_render_missing_value_leaves_placeholder() {
        let mut values = HashMap::new();
        values.insert("name".to_string(), "Dave".to_string());
        let result = render("Hello {{name}}, your id is {{id}}.", &values);
        assert_eq!(result, "Hello Dave, your id is {{id}}.");
    }

    #[test]
    fn test_render_empty_values() {
        let values = HashMap::new();
        let result = render("{{a}} and {{b}}", &values);
        assert_eq!(result, "{{a}} and {{b}}");
    }

    #[test]
    fn test_render_no_placeholders() {
        let mut values = HashMap::new();
        values.insert("name".to_string(), "Eve".to_string());
        let result = render("No placeholders here.", &values);
        assert_eq!(result, "No placeholders here.");
    }

    #[test]
    fn test_render_empty_string() {
        let values = HashMap::new();
        let result = render("", &values);
        assert_eq!(result, "");
    }

    #[test]
    fn test_render_value_with_special_characters() {
        let mut values = HashMap::new();
        values.insert("content".to_string(), "<b>bold</b> & \"quoted\"".to_string());
        let result = render("Output: {{content}}", &values);
        assert_eq!(result, "Output: <b>bold</b> & \"quoted\"");
    }

    #[test]
    fn test_render_multiline_template() {
        let mut values = HashMap::new();
        values.insert("name".to_string(), "Frank".to_string());
        values.insert("date".to_string(), "2026-04-20".to_string());
        let template = "Dear {{name}},\n\nThis is a reminder for {{date}}.\n\nBest,\n{{name}}";
        let result = render(template, &values);
        assert_eq!(
            result,
            "Dear Frank,\n\nThis is a reminder for 2026-04-20.\n\nBest,\nFrank"
        );
    }
}
