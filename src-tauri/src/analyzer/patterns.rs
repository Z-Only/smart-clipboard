use std::sync::LazyLock;

use regex::Regex;

static URL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?i)^https?://[^\s<>"']+$"#).unwrap());

static EMAIL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-zA-Z0-9._%+\-]+@[a-zA-Z0-9.\-]+\.[a-zA-Z]{2,}$").unwrap());

static HEX_COLOR_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^#(?:[0-9a-fA-F]{3}){1,2}$").unwrap());

static RGB_COLOR_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^(?:rgb|hsl)a?\(\s*[\d.,\s%]+\)$").unwrap());

static UNIX_PATH_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?:/|~/)(?:[a-zA-Z0-9._\-]+/)*[a-zA-Z0-9._\-]+/?$").unwrap());

static WIN_PATH_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^[A-Z]:\\(?:[^\s\\/:*?"<>|]+\\)*[^\s\\/:*?"<>|]+$"#).unwrap()
});

static PHONE_CN_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^1[3-9]\d{9}$").unwrap());

static PHONE_US_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?:\+?1[-.\s]?)?\(?[0-9]{3}\)?[-.\s]?[0-9]{3}[-.\s]?[0-9]{4}$").unwrap()
});

static PHONE_INTL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\+\d{1,3}[-.\s]?\d{4,14}$").unwrap());

static XML_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*<(\?xml|[a-zA-Z][\w\-]*)[\s>]").unwrap());

static CODE_KEYWORDS_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?m)(^|\s)(fn |function |def |class |import |from .+ import |#include|pub fn |pub struct |pub enum |const |let |var |SELECT |INSERT |UPDATE |DELETE |CREATE TABLE|impl )"
    ).unwrap()
});

static CODE_SYMBOLS_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[{};]\s*$|=>\s|->|&&|\|\||!=|==").unwrap());

pub fn is_url(text: &str) -> bool {
    URL_RE.is_match(text)
}

pub fn is_email(text: &str) -> bool {
    EMAIL_RE.is_match(text)
}

pub fn is_color(text: &str) -> bool {
    HEX_COLOR_RE.is_match(text) || RGB_COLOR_RE.is_match(text)
}

pub fn is_file_path(text: &str) -> bool {
    UNIX_PATH_RE.is_match(text) || WIN_PATH_RE.is_match(text)
}

pub fn is_json(text: &str) -> bool {
    let trimmed = text.trim();
    (trimmed.starts_with('{') && trimmed.ends_with('}'))
        || (trimmed.starts_with('[') && trimmed.ends_with(']'))
}

pub fn is_json_valid(text: &str) -> bool {
    is_json(text) && serde_json::from_str::<serde_json::Value>(text).is_ok()
}

pub fn is_xml(text: &str) -> bool {
    XML_RE.is_match(text)
}

pub fn is_code(text: &str) -> bool {
    let keyword_count = CODE_KEYWORDS_RE.find_iter(text).count();
    let symbol_count = CODE_SYMBOLS_RE.find_iter(text).count();
    // Need at least some code indicators
    keyword_count >= 1 || (symbol_count >= 2 && text.lines().count() >= 2)
}

pub fn is_phone(text: &str) -> bool {
    let cleaned: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    PHONE_CN_RE.is_match(&cleaned)
        || PHONE_US_RE.is_match(text.trim())
        || PHONE_INTL_RE.is_match(text.trim())
}

pub fn is_address(text: &str) -> bool {
    let lower = text.to_lowercase();
    // English address patterns
    let en_keywords = [
        "street", "avenue", "blvd", "road", "drive", "lane", "court", "suite", "apt", "floor",
    ];
    let en_count = en_keywords.iter().filter(|k| lower.contains(**k)).count();

    // Chinese address patterns
    let cn_keywords = ["省", "市", "区", "县", "街", "路", "号", "楼", "室", "镇", "村"];
    let cn_count = cn_keywords.iter().filter(|k| lower.contains(**k)).count();

    en_count >= 2 || cn_count >= 2
}
