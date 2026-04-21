use pinyin::ToPinyin;

pub fn build_pinyin_fields(content: &str) -> (String, String) {
    let mut full_tokens: Vec<String> = Vec::new();
    let mut initials = String::new();

    for ch in content.chars() {
        if let Some(py) = ch.to_pinyin() {
            let plain = py.plain();
            full_tokens.push(plain.to_string());
            initials.push_str(py.first_letter());
        } else if ch.is_ascii_alphanumeric() {
            let lower = ch.to_ascii_lowercase();
            full_tokens.push(lower.to_string());
            initials.push(lower);
        }
    }

    let full = full_tokens.join(" ").replace(" ", "");
    (full, initials)
}

pub fn normalize_search_keyword(keyword: &str) -> String {
    keyword
        .chars()
        .filter_map(|ch| {
            if ch.is_ascii_alphanumeric() {
                Some(ch.to_ascii_lowercase())
            } else if ch.is_whitespace() {
                Some(' ')
            } else if ch.to_pinyin().is_some() {
                Some(ch)
            } else {
                None
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_pinyin_for_chinese_text() {
        let (full, initials) = build_pinyin_fields("智能剪贴板");
        assert_eq!(full, "zhinengjiantieban");
        assert_eq!(initials, "znjtb");
    }

    #[test]
    fn builds_mixed_ascii_and_chinese_fields() {
        let (full, initials) = build_pinyin_fields("Hello世界123");
        assert_eq!(full, "helloshijie123");
        assert_eq!(initials, "hellosj123");
    }

    #[test]
    fn normalizes_keyword_for_like_search() {
        assert_eq!(normalize_search_keyword(" ZN-JT!!  "), "znjt");
        assert_eq!(normalize_search_keyword("智能 JT"), "智能 jt");
    }
}
