use serde::{Deserialize, Serialize};

use super::patterns;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    Url,
    Email,
    Color,
    #[serde(rename = "filepath")]
    FilePath,
    Json,
    Xml,
    Code,
    Phone,
    Address,
    Text,
}

impl Category {
    pub fn as_str(&self) -> &'static str {
        match self {
            Category::Url => "url",
            Category::Email => "email",
            Category::Color => "color",
            Category::FilePath => "filepath",
            Category::Json => "json",
            Category::Xml => "xml",
            Category::Code => "code",
            Category::Phone => "phone",
            Category::Address => "address",
            Category::Text => "text",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "url" => Category::Url,
            "email" => Category::Email,
            "color" => Category::Color,
            "filepath" => Category::FilePath,
            "json" => Category::Json,
            "xml" => Category::Xml,
            "code" => Category::Code,
            "phone" => Category::Phone,
            "address" => Category::Address,
            _ => Category::Text,
        }
    }
}

/// Classify clipboard content into a category using a priority-ordered rule chain.
/// The classifier checks if the **entire content** matches a pattern (not substrings).
pub fn classify(content: &str) -> Category {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Category::Text;
    }

    // Single-line content checks (URL, Email, Color, FilePath, Phone)
    if !trimmed.contains('\n') {
        if patterns::is_url(trimmed) {
            return Category::Url;
        }
        if patterns::is_email(trimmed) {
            return Category::Email;
        }
        if patterns::is_color(trimmed) {
            return Category::Color;
        }
        if patterns::is_file_path(trimmed) {
            return Category::FilePath;
        }
        if patterns::is_phone(trimmed) {
            return Category::Phone;
        }
    }

    // Multi-line / structural checks
    if patterns::is_json_valid(trimmed) {
        return Category::Json;
    }
    if patterns::is_xml(trimmed) {
        return Category::Xml;
    }
    if patterns::is_code(trimmed) {
        return Category::Code;
    }
    if patterns::is_address(trimmed) {
        return Category::Address;
    }

    Category::Text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url() {
        assert_eq!(classify("https://github.com/tauri-apps/tauri"), Category::Url);
        assert_eq!(classify("http://localhost:3000/api"), Category::Url);
    }

    #[test]
    fn test_email() {
        assert_eq!(classify("user@example.com"), Category::Email);
        assert_eq!(classify("test.user+tag@domain.co.uk"), Category::Email);
    }

    #[test]
    fn test_hex_color() {
        assert_eq!(classify("#FF5733"), Category::Color);
        assert_eq!(classify("#fff"), Category::Color);
    }

    #[test]
    fn test_rgb_color() {
        assert_eq!(classify("rgb(255, 87, 51)"), Category::Color);
        assert_eq!(classify("hsl(120, 100%, 50%)"), Category::Color);
    }

    #[test]
    fn test_unix_path() {
        assert_eq!(classify("/usr/local/bin/rustc"), Category::FilePath);
        assert_eq!(classify("~/Documents/file.txt"), Category::FilePath);
    }

    #[test]
    fn test_windows_path() {
        assert_eq!(classify("C:\\Users\\test\\file.txt"), Category::FilePath);
    }

    #[test]
    fn test_json() {
        assert_eq!(classify(r#"{"key": "value", "num": 42}"#), Category::Json);
        assert_eq!(classify(r#"[1, 2, 3]"#), Category::Json);
    }

    #[test]
    fn test_xml() {
        assert_eq!(
            classify("<?xml version=\"1.0\"?><root/>"),
            Category::Xml
        );
        assert_eq!(classify("<div class=\"test\">hello</div>"), Category::Xml);
    }

    #[test]
    fn test_code_rust() {
        assert_eq!(
            classify("fn main() {\n    println!(\"hello\");\n}"),
            Category::Code
        );
    }

    #[test]
    fn test_code_javascript() {
        assert_eq!(
            classify("function hello() {\n    return 42;\n}"),
            Category::Code
        );
    }

    #[test]
    fn test_code_python() {
        assert_eq!(
            classify("def hello():\n    return 42"),
            Category::Code
        );
    }

    #[test]
    fn test_phone_cn() {
        assert_eq!(classify("13812345678"), Category::Phone);
    }

    #[test]
    fn test_phone_us() {
        assert_eq!(classify("(555) 123-4567"), Category::Phone);
        assert_eq!(classify("+1-555-123-4567"), Category::Phone);
    }

    #[test]
    fn test_address_en() {
        assert_eq!(
            classify("123 Main Street, Suite 100, New York"),
            Category::Address
        );
    }

    #[test]
    fn test_address_cn() {
        assert_eq!(
            classify("浙江省杭州市西湖区文三路"),
            Category::Address
        );
    }

    #[test]
    fn test_plain_text() {
        assert_eq!(classify("Hello, world!"), Category::Text);
        assert_eq!(classify("Just a random sentence."), Category::Text);
    }

    #[test]
    fn test_mixed_content_is_text() {
        // Paragraph with embedded URL should be text, not url
        assert_eq!(
            classify("Check out this link https://example.com for more info"),
            Category::Text
        );
    }

    #[test]
    fn test_empty() {
        assert_eq!(classify(""), Category::Text);
    }

    #[test]
    fn test_whitespace_only() {
        assert_eq!(classify("   \n\t  "), Category::Text);
    }
}
