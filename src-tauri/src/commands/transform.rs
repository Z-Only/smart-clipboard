use base64::{engine::general_purpose::STANDARD, Engine};

#[tauri::command]
pub async fn transform_content(content: String, transform_type: String) -> Result<String, String> {
    apply_transform(&content, &transform_type)
}

pub fn apply_transform(content: &str, transform_type: &str) -> Result<String, String> {
    match transform_type {
        "uppercase" => Ok(content.to_uppercase()),
        "lowercase" => Ok(content.to_lowercase()),
        "title_case" => Ok(to_title_case(content)),
        "url_encode" => Ok(url_encode(content)),
        "url_decode" => url_decode(content),
        "json_format" => json_format(content),
        "json_compact" => json_compact(content),
        "base64_encode" => Ok(STANDARD.encode(content.as_bytes())),
        "base64_decode" => base64_decode(content),
        "trim" => Ok(trim_whitespace(content)),
        "html_escape" => Ok(html_escape(content)),
        "html_unescape" => Ok(html_unescape(content)),
        _ => Err(format!("Unknown transform type: {}", transform_type)),
    }
}

fn to_title_case(s: &str) -> String {
    s.split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => {
                    first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase()
                }
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn url_encode(s: &str) -> String {
    urlencoding::encode(s).into_owned()
}

fn url_decode(s: &str) -> Result<String, String> {
    match urlencoding::decode(s) {
        Ok(v) => Ok(v.into_owned()),
        Err(e) => Err(e.to_string()),
    }
}

fn json_format(s: &str) -> Result<String, String> {
    serde_json::from_str::<serde_json::Value>(s)
        .and_then(|v| serde_json::to_string_pretty(&v))
        .map_err(|e| e.to_string())
}

fn json_compact(s: &str) -> Result<String, String> {
    serde_json::from_str::<serde_json::Value>(s)
        .and_then(|v| serde_json::to_string(&v))
        .map_err(|e| e.to_string())
}

fn base64_decode(s: &str) -> Result<String, String> {
    match STANDARD.decode(s) {
        Ok(bytes) => String::from_utf8(bytes).map_err(|e| e.to_string()),
        Err(e) => Err(e.to_string()),
    }
}

fn trim_whitespace(s: &str) -> String {
    s.trim().to_string()
}

fn html_escape(s: &str) -> String {
    html_escape::encode_safe(s).to_string()
}

fn html_unescape(s: &str) -> String {
    html_escape::decode_html_entities(s).to_string()
}
