use minisign_verify::{PublicKey, Signature};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SignatureScheme {
    Sha256,
    Minisign,
    Unknown,
}

fn detect_signature_scheme(signature: &str) -> SignatureScheme {
    let trimmed = signature.trim();
    if trimmed.starts_with("sha256:") {
        SignatureScheme::Sha256
    } else if trimmed.starts_with("untrusted comment:") || trimmed.starts_with("minisign:") {
        SignatureScheme::Minisign
    } else {
        SignatureScheme::Unknown
    }
}

fn verify_sha256(bytes: &[u8], signature: &str) -> Result<(), String> {
    let expected_hex = format!("sha256:{:x}", Sha256::digest(bytes));
    if signature.trim() == expected_hex {
        return Ok(());
    }
    Err("signature verification failed".to_string())
}

fn normalize_minisign_signature(signature: &str) -> String {
    let trimmed = signature.trim();
    if let Some(rest) = trimmed.strip_prefix("minisign:") {
        rest.trim().to_string()
    } else {
        trimmed.to_string()
    }
}

fn verify_minisign(bytes: &[u8], signature: &str, updater_public_key: &str) -> Result<(), String> {
    let public_key = PublicKey::from_base64(updater_public_key.trim())
        .map_err(|e| format!("Invalid updater public key: {e}"))?;
    let signature = Signature::decode(&normalize_minisign_signature(signature))
        .map_err(|e| format!("Invalid minisign signature: {e}"))?;
    public_key
        .verify(bytes, &signature, false)
        .map_err(|e| format!("Minisign verification failed: {e}"))
}

pub fn verify_downloaded_artifact_with_public_key(
    bytes: &[u8],
    signature: &str,
    updater_public_key: Option<&str>,
) -> Result<(), String> {
    match detect_signature_scheme(signature) {
        SignatureScheme::Sha256 => verify_sha256(bytes, signature),
        SignatureScheme::Minisign => {
            let Some(public_key) = updater_public_key
                .map(str::trim)
                .filter(|value| !value.is_empty())
            else {
                return Err("Missing updater public key for minisign verification".to_string());
            };
            verify_minisign(bytes, signature, public_key)
        }
        SignatureScheme::Unknown => Err("Unsupported signature scheme".to_string()),
    }
}

pub fn verify_downloaded_artifact(bytes: &[u8], signature: &str) -> Result<(), String> {
    verify_downloaded_artifact_with_public_key(bytes, signature, None)
}
