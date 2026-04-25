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

pub(super) fn sanitize_ascii_token(token: &str) -> String {
    token
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

pub(super) fn sanitize_cjk_token(token: &str) -> String {
    token.chars().filter(|c| c.to_pinyin().is_some()).collect()
}

pub fn build_fts_match_expr(keyword: &str) -> String {
    let mut clauses = Vec::new();

    for word in keyword.split_whitespace() {
        let ascii_safe = sanitize_ascii_token(word);
        let han_safe = sanitize_cjk_token(word);

        if ascii_safe.chars().count() >= 2 {
            clauses.push(format!(
                "(content:\"{s}\"* OR pinyin_full:\"{s}\"* OR pinyin_initials:\"{s}\"*)",
                s = ascii_safe
            ));
        }

        if !han_safe.is_empty() {
            clauses.push(format!("content:\"{}\"", han_safe));
        }
    }

    clauses.join(" AND ")
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
    fn sanitize_ascii_strips_special_chars() {
        assert_eq!(sanitize_ascii_token("Zn*Jt\""), "znjt");
    }

    #[test]
    fn sanitize_ascii_keeps_alnum_lowercased() {
        assert_eq!(sanitize_ascii_token("Hello123"), "hello123");
    }

    #[test]
    fn sanitize_ascii_drops_cjk() {
        assert_eq!(sanitize_ascii_token("智能"), "");
    }

    #[test]
    fn sanitize_cjk_keeps_hanzi_only() {
        assert_eq!(sanitize_cjk_token("智能ABC123"), "智能");
    }

    #[test]
    fn build_match_expr_empty_when_only_punct() {
        assert_eq!(build_fts_match_expr("!!! ???"), "");
    }

    #[test]
    fn build_match_expr_ascii_multicol_prefix() {
        assert_eq!(
            build_fts_match_expr("znjtb"),
            "(content:\"znjtb\"* OR pinyin_full:\"znjtb\"* OR pinyin_initials:\"znjtb\"*)"
        );
    }

    #[test]
    fn build_match_expr_short_ascii_skipped() {
        assert_eq!(build_fts_match_expr("z"), "");
    }

    #[test]
    fn build_match_expr_cjk_token_uses_content_phrase() {
        assert_eq!(build_fts_match_expr("智能"), "content:\"智能\"");
    }

    #[test]
    fn build_match_expr_mixed_tokens_and_joined() {
        assert_eq!(
            build_fts_match_expr("hello 智能"),
            "(content:\"hello\"* OR pinyin_full:\"hello\"* OR pinyin_initials:\"hello\"*) AND content:\"智能\""
        );
    }

    #[test]
    fn build_match_expr_uppercase_ascii_normalized() {
        assert_eq!(
            build_fts_match_expr("ZNJTB"),
            "(content:\"znjtb\"* OR pinyin_full:\"znjtb\"* OR pinyin_initials:\"znjtb\"*)"
        );
    }

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
