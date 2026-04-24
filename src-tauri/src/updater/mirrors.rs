pub fn normalize_mirrors(input: &[String]) -> Vec<String> {
    input
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

pub fn validate_mirror_template(template: &str) -> Result<(), String> {
    let value = template.trim();
    if !value.starts_with("https://") {
        return Err("Mirror endpoint must start with https://".to_string());
    }
    if !value.contains("{url}") {
        return Err("Mirror endpoint must include {url}".to_string());
    }
    Ok(())
}

pub fn resolve_candidate_urls(canonical_url: &str, mirrors: &[String]) -> Vec<String> {
    let mut urls = normalize_mirrors(mirrors)
        .into_iter()
        .map(|mirror| mirror.replace("{url}", canonical_url))
        .collect::<Vec<_>>();
    urls.push(canonical_url.to_string());
    urls
}

#[cfg(test)]
mod tests {
    use super::{normalize_mirrors, resolve_candidate_urls, validate_mirror_template};

    #[test]
    fn valid_mirror_requires_https_and_placeholder() {
        assert!(validate_mirror_template("https://mirror.example/{url}").is_ok());
        assert!(validate_mirror_template("http://mirror.example/{url}").is_err());
        assert!(validate_mirror_template("https://mirror.example/path").is_err());
    }

    #[test]
    fn blank_mirror_lines_are_ignored() {
        let normalized = normalize_mirrors(&[
            "  ".to_string(),
            "https://mirror-a/{url}".to_string(),
            "\n".to_string(),
            "https://mirror-b/{url}  ".to_string(),
        ]);
        assert_eq!(
            normalized,
            vec![
                "https://mirror-a/{url}".to_string(),
                "https://mirror-b/{url}".to_string()
            ]
        );
    }

    #[test]
    fn candidate_urls_preserve_order_and_append_canonical() {
        let urls = resolve_candidate_urls(
            "https://github.com/org/repo/releases/latest/download/latest.json",
            &[
                "https://mirror-a/{url}".to_string(),
                "https://mirror-b/{url}".to_string(),
            ],
        );
        assert_eq!(
            urls,
            vec![
                "https://mirror-a/https://github.com/org/repo/releases/latest/download/latest.json",
                "https://mirror-b/https://github.com/org/repo/releases/latest/download/latest.json",
                "https://github.com/org/repo/releases/latest/download/latest.json"
            ]
        );
    }
}
