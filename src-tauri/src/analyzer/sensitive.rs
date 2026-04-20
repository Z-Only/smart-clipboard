use std::sync::LazyLock;

use regex::Regex;

/// Compiled regex patterns for detecting sensitive information in clipboard content.
static SENSITIVE_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    let patterns = [
        // API Keys
        r#"(?i)(api[_\-]?key|apikey)\s*[:=]\s*['"]?[A-Za-z0-9_\-]{16,}"#,
        // AWS Access Key IDs
        r#"AKIA[0-9A-Z]{16}"#,
        // Tokens (bearer, auth tokens)
        r#"(?i)(token|bearer|auth)\s*[:=]\s*['"]?[A-Za-z0-9_\-\.]{20,}"#,
        // Private Keys (PEM format)
        r#"-----BEGIN\s+(RSA|EC|DSA|OPENSSH)?\s*PRIVATE KEY-----"#,
        // Generic secrets (password, credential, etc.)
        r#"(?i)(password|passwd|secret|credential)\s*[:=]\s*['"]?[^\s'"]{4,}"#,
        // JWT tokens
        r#"eyJ[A-Za-z0-9_-]{10,}\.eyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_\-]+"#,
        // Database connection strings
        r#"(?i)(mysql|postgres|mongodb|redis)://[^\s]+"#,
    ];
    patterns
        .iter()
        .map(|p| Regex::new(p).expect("Invalid sensitive pattern regex"))
        .collect()
});

/// Returns `true` if the content contains any sensitive information patterns
/// such as API keys, tokens, passwords, private keys, or connection strings.
pub fn detect_sensitive(content: &str) -> bool {
    SENSITIVE_PATTERNS.iter().any(|re| re.is_match(content))
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- API Key tests ---

    #[test]
    fn test_api_key_colon() {
        assert!(detect_sensitive("api_key: sk_live_abcdefghijklmnop"));
    }

    #[test]
    fn test_api_key_equals() {
        assert!(detect_sensitive("apikey=ABCDEF1234567890abcd"));
    }

    #[test]
    fn test_api_key_quoted() {
        assert!(detect_sensitive(r#"API-KEY: "mySecretKey12345678""#));
    }

    #[test]
    fn test_api_key_in_config() {
        assert!(detect_sensitive(
            "DATABASE_URL=postgres://localhost\nAPI_KEY=sk_live_abcdefghijklmnop"
        ));
    }

    // --- AWS Key tests ---

    #[test]
    fn test_aws_access_key() {
        assert!(detect_sensitive("AKIAIOSFODNN7EXAMPLE"));
    }

    #[test]
    fn test_aws_key_in_text() {
        assert!(detect_sensitive("aws_access_key_id = AKIAIOSFODNN7EXAMPLE"));
    }

    // --- Token tests ---

    #[test]
    fn test_bearer_token() {
        assert!(detect_sensitive(
            "bearer: eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.test"
        ));
    }

    #[test]
    fn test_auth_token_equals() {
        assert!(detect_sensitive("auth=abcdefghijklmnopqrstuvwxyz"));
    }

    #[test]
    fn test_token_with_quotes() {
        assert!(detect_sensitive(
            r#"token = "ghp_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx""#
        ));
    }

    // --- Private Key tests ---

    #[test]
    fn test_rsa_private_key() {
        assert!(detect_sensitive("-----BEGIN RSA PRIVATE KEY-----"));
    }

    #[test]
    fn test_ec_private_key() {
        assert!(detect_sensitive("-----BEGIN EC PRIVATE KEY-----"));
    }

    #[test]
    fn test_openssh_private_key() {
        assert!(detect_sensitive("-----BEGIN OPENSSH PRIVATE KEY-----"));
    }

    #[test]
    fn test_generic_private_key() {
        assert!(detect_sensitive("-----BEGIN PRIVATE KEY-----"));
    }

    #[test]
    fn test_private_key_in_block() {
        let pem =
            "-----BEGIN RSA PRIVATE KEY-----\nMIIEpAIBAAKCAQEA...\n-----END RSA PRIVATE KEY-----";
        assert!(detect_sensitive(pem));
    }

    // --- Generic secret tests ---

    #[test]
    fn test_password_equals() {
        assert!(detect_sensitive("password=MyS3cret!"));
    }

    #[test]
    fn test_passwd_colon() {
        assert!(detect_sensitive("passwd: hunter2"));
    }

    #[test]
    fn test_secret_in_config() {
        assert!(detect_sensitive(r#"secret = "my-secret-value-1234""#));
    }

    #[test]
    fn test_credential_field() {
        assert!(detect_sensitive("credential=supersecretcred123"));
    }

    // --- JWT tests ---

    #[test]
    fn test_jwt_token() {
        let jwt = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
        assert!(detect_sensitive(jwt));
    }

    #[test]
    fn test_jwt_in_header() {
        let header = "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U";
        assert!(detect_sensitive(header));
    }

    // --- Connection String tests ---

    #[test]
    fn test_postgres_url() {
        assert!(detect_sensitive(
            "postgres://user:password@localhost:5432/mydb"
        ));
    }

    #[test]
    fn test_mysql_url() {
        assert!(detect_sensitive("mysql://root:pass@127.0.0.1:3306/testdb"));
    }

    #[test]
    fn test_mongodb_url() {
        assert!(detect_sensitive(
            "mongodb://admin:pass@cluster0.example.mongodb.net/mydb"
        ));
    }

    #[test]
    fn test_redis_url() {
        assert!(detect_sensitive(
            "redis://default:password@redis.example.com:6379"
        ));
    }

    // --- Negative tests (should NOT be detected as sensitive) ---

    #[test]
    fn test_plain_text_not_sensitive() {
        assert!(!detect_sensitive("Hello, world!"));
    }

    #[test]
    fn test_url_not_sensitive() {
        assert!(!detect_sensitive("https://github.com/tauri-apps/tauri"));
    }

    #[test]
    fn test_code_not_sensitive() {
        assert!(!detect_sensitive(
            "fn main() {\n    println!(\"hello\");\n}"
        ));
    }

    #[test]
    fn test_short_password_not_sensitive() {
        // password value < 4 chars should not match
        assert!(!detect_sensitive("password=abc"));
    }

    #[test]
    fn test_email_not_sensitive() {
        assert!(!detect_sensitive("user@example.com"));
    }

    #[test]
    fn test_empty_not_sensitive() {
        assert!(!detect_sensitive(""));
    }

    #[test]
    fn test_json_not_sensitive() {
        assert!(!detect_sensitive(r#"{"name": "John", "age": 30}"#));
    }

    #[test]
    fn test_partial_aws_key_not_sensitive() {
        // AKIA followed by fewer than 16 chars should not match
        assert!(!detect_sensitive("AKIA12345"));
    }
}
