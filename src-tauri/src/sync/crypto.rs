use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use argon2::{Argon2, Params, Version, Algorithm};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use hkdf::Hkdf;
use rand::rngs::OsRng;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use x25519_dalek::{PublicKey, StaticSecret};

const KEY_LEN: usize = 32;
const NONCE_LEN: usize = 12;
const HKDF_INFO: &[u8] = b"smart-clipboard-phase3-sync-v1";
const ARGON2_SALT_LEN: usize = 16;
const ARGON2_MEMORY_KIB: u32 = 65536;
const ARGON2_ITERATIONS: u32 = 3;
const ARGON2_PARALLELISM: u32 = 4;
const FILE_VERSION: [u8; 4] = [0x53, 0x43, 0x01, 0x00]; // "SC" + version 1.0

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceKeyPair {
    pub private_key: Vec<u8>,
    pub public_key: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EncryptedPayload {
    pub nonce: Vec<u8>,
    pub ciphertext: Vec<u8>,
}

pub fn generate_keypair() -> DeviceKeyPair {
    let private = StaticSecret::random_from_rng(OsRng);
    let public = PublicKey::from(&private);
    DeviceKeyPair {
        private_key: private.to_bytes().to_vec(),
        public_key: public.as_bytes().to_vec(),
    }
}

pub fn generate_salt() -> Vec<u8> {
    let mut salt = vec![0u8; ARGON2_SALT_LEN];
    OsRng.fill_bytes(&mut salt);
    salt
}

pub fn derive_key_from_password(password: &str, salt: &[u8]) -> Result<Vec<u8>, String> {
    let params = Params::new(
        ARGON2_MEMORY_KIB,
        ARGON2_ITERATIONS,
        ARGON2_PARALLELISM,
        Some(KEY_LEN),
    )
    .map_err(|e| format!("Invalid Argon2 params: {e}"))?;

    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut key = vec![0u8; KEY_LEN];
    argon2
        .hash_password_into(password.as_bytes(), salt, &mut key)
        .map_err(|e| format!("Argon2 key derivation failed: {e}"))?;
    Ok(key)
}

pub fn derive_shared_secret(private_key: &[u8], peer_public_key: &[u8]) -> Result<Vec<u8>, String> {
    let private_bytes: [u8; KEY_LEN] = private_key
        .try_into()
        .map_err(|_| format!("Invalid X25519 private key length: {}", private_key.len()))?;
    let public_bytes: [u8; KEY_LEN] = peer_public_key.try_into().map_err(|_| {
        format!(
            "Invalid X25519 public key length: {}",
            peer_public_key.len()
        )
    })?;

    let private = StaticSecret::from(private_bytes);
    let public = PublicKey::from(public_bytes);
    let shared = private.diffie_hellman(&public);

    let hk = Hkdf::<Sha256>::new(None, shared.as_bytes());
    let mut derived = [0u8; KEY_LEN];
    hk.expand(HKDF_INFO, &mut derived)
        .map_err(|e| format!("Failed to derive shared secret via HKDF: {e}"))?;
    Ok(derived.to_vec())
}

pub fn encrypt(plaintext: &[u8], shared_secret: &[u8]) -> Result<EncryptedPayload, String> {
    let key: [u8; KEY_LEN] = shared_secret
        .try_into()
        .map_err(|_| format!("Invalid AES-256 key length: {}", shared_secret.len()))?;
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|e| e.to_string())?;

    let mut nonce = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce);
    let ciphertext = cipher
        .encrypt(Nonce::from_slice(&nonce), plaintext)
        .map_err(|e| format!("AES-256-GCM encryption failed: {e}"))?;

    Ok(EncryptedPayload {
        nonce: nonce.to_vec(),
        ciphertext,
    })
}

pub fn decrypt(payload: &EncryptedPayload, shared_secret: &[u8]) -> Result<Vec<u8>, String> {
    let key: [u8; KEY_LEN] = shared_secret
        .try_into()
        .map_err(|_| format!("Invalid AES-256 key length: {}", shared_secret.len()))?;
    let nonce: [u8; NONCE_LEN] = payload
        .nonce
        .as_slice()
        .try_into()
        .map_err(|_| format!("Invalid AES-GCM nonce length: {}", payload.nonce.len()))?;

    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|e| e.to_string())?;
    cipher
        .decrypt(Nonce::from_slice(&nonce), payload.ciphertext.as_ref())
        .map_err(|e| format!("AES-256-GCM decryption failed: {e}"))
}

pub fn encode_key_material(bytes: &[u8]) -> String {
    BASE64.encode(bytes)
}

pub fn decode_key_material(value: &str) -> Result<Vec<u8>, String> {
    BASE64
        .decode(value)
        .map_err(|e| format!("Invalid base64 key material: {e}"))
}

/// Compute a human-readable fingerprint from a shared secret for UI verification.
/// Returns a string in the format "AB:CD:EF:01:23:45:67:89" (8 bytes, hex, colon-separated).
pub fn compute_fingerprint(shared_secret: &[u8]) -> String {
    use sha2::Digest;
    let mut hasher = Sha256::new();
    hasher.update(shared_secret);
    let hash = hasher.finalize();
    
    // Take first 8 bytes and format as hex with colon separators
    let bytes = &hash[..8];
    bytes
        .iter()
        .map(|b| format!("{:02X}", b))
        .collect::<Vec<_>>()
        .join(":")
}

// Encrypt data into a self-contained file format:
// [4 bytes version][16 bytes salt][12 bytes nonce][ciphertext+tag]
pub fn encrypt_file(plaintext: &[u8], master_key: &[u8], salt: &[u8]) -> Result<Vec<u8>, String> {
    let key: [u8; KEY_LEN] = master_key
        .try_into()
        .map_err(|_| format!("Invalid AES-256 key length: {}", master_key.len()))?;
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|e| e.to_string())?;

    let mut nonce = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce);
    let ciphertext = cipher
        .encrypt(Nonce::from_slice(&nonce), plaintext)
        .map_err(|e| format!("AES-256-GCM encryption failed: {e}"))?;

    let salt_padded = if salt.len() >= ARGON2_SALT_LEN {
        salt[..ARGON2_SALT_LEN].to_vec()
    } else {
        let mut padded = vec![0u8; ARGON2_SALT_LEN];
        padded[..salt.len()].copy_from_slice(salt);
        padded
    };

    let mut output = Vec::with_capacity(4 + ARGON2_SALT_LEN + NONCE_LEN + ciphertext.len());
    output.extend_from_slice(&FILE_VERSION);
    output.extend_from_slice(&salt_padded);
    output.extend_from_slice(&nonce);
    output.extend_from_slice(&ciphertext);
    Ok(output)
}

// Decrypt a file produced by encrypt_file.
// Returns (plaintext, salt).
pub fn decrypt_file(file_bytes: &[u8], master_key: &[u8]) -> Result<(Vec<u8>, Vec<u8>), String> {
    let header_len = 4 + ARGON2_SALT_LEN + NONCE_LEN;
    if file_bytes.len() < header_len + 16 {
        return Err("File too short to contain valid encrypted data".to_string());
    }

    let version = &file_bytes[..4];
    if version != FILE_VERSION {
        return Err(format!(
            "Unsupported file version: {:02x}{:02x}{:02x}{:02x}",
            version[0], version[1], version[2], version[3]
        ));
    }

    let salt = file_bytes[4..4 + ARGON2_SALT_LEN].to_vec();
    let nonce = &file_bytes[4 + ARGON2_SALT_LEN..header_len];
    let ciphertext = &file_bytes[header_len..];

    let key: [u8; KEY_LEN] = master_key
        .try_into()
        .map_err(|_| format!("Invalid AES-256 key length: {}", master_key.len()))?;
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|e| e.to_string())?;
    let plaintext = cipher
        .decrypt(Nonce::from_slice(nonce), ciphertext)
        .map_err(|_| "Decryption failed — wrong password or corrupted data".to_string())?;

    Ok((plaintext, salt))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_secret_derivation_matches() {
        let a = generate_keypair();
        let b = generate_keypair();
        let ab = derive_shared_secret(&a.private_key, &b.public_key).unwrap();
        let ba = derive_shared_secret(&b.private_key, &a.public_key).unwrap();
        assert_eq!(ab, ba);
        assert_eq!(ab.len(), 32);
    }

    #[test]
    fn aes_gcm_roundtrip() {
        let kp = generate_keypair();
        let secret = derive_shared_secret(&kp.private_key, &kp.public_key).unwrap();
        let plaintext = br#"{"type":"clipboardSync","value":"hello"}"#;
        let encrypted = encrypt(plaintext, &secret).unwrap();
        let decrypted = decrypt(&encrypted, &secret).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_compute_fingerprint_deterministic() {
        let secret = vec![1u8; 32];
        let fp1 = compute_fingerprint(&secret);
        let fp2 = compute_fingerprint(&secret);
        assert_eq!(fp1, fp2);
    }

    #[test]
    fn test_compute_fingerprint_format() {
        let secret = vec![0xAB; 32];
        let fp = compute_fingerprint(&secret);
        
        // Should be 8 groups of 2 hex chars separated by colons: XX:XX:XX:XX:XX:XX:XX:XX
        let parts: Vec<&str> = fp.split(':').collect();
        assert_eq!(parts.len(), 8, "Fingerprint should have 8 parts");
        
        for part in &parts {
            assert_eq!(part.len(), 2, "Each part should be 2 characters");
            assert!(part.chars().all(|c| c.is_ascii_hexdigit()), "Each part should be valid hex");
        }
    }

    #[test]
    fn test_different_secrets_different_fingerprints() {
        let secret1 = vec![1u8; 32];
        let secret2 = vec![2u8; 32];
        let fp1 = compute_fingerprint(&secret1);
        let fp2 = compute_fingerprint(&secret2);
        assert_ne!(fp1, fp2);
    }

    #[test]
    fn test_encrypt_decrypt_wrong_key_fails() {
        let kp1 = generate_keypair();
        let kp2 = generate_keypair();
        
        let secret1 = derive_shared_secret(&kp1.private_key, &kp1.public_key).unwrap();
        let secret2 = derive_shared_secret(&kp2.private_key, &kp2.public_key).unwrap();
        
        let plaintext = b"test message";
        let encrypted = encrypt(plaintext, &secret1).unwrap();
        
        // Decrypting with wrong key should fail
        let result = decrypt(&encrypted, &secret2);
        assert!(result.is_err());
    }

    #[test]
    fn test_derive_shared_secret_invalid_key_length() {
        let short_key = vec![1u8; 16];
        let long_key = vec![1u8; 64];
        let valid_key = vec![1u8; 32];
        
        // Short private key should fail
        assert!(derive_shared_secret(&short_key, &valid_key).is_err());
        
        // Long private key should fail
        assert!(derive_shared_secret(&long_key, &valid_key).is_err());
        
        // Short public key should fail
        assert!(derive_shared_secret(&valid_key, &short_key).is_err());
        
        // Long public key should fail
        assert!(derive_shared_secret(&valid_key, &long_key).is_err());
    }

    #[test]
    fn test_generate_salt_length() {
        let salt = generate_salt();
        assert_eq!(salt.len(), 16);
    }

    #[test]
    fn test_generate_salt_randomness() {
        let salt1 = generate_salt();
        let salt2 = generate_salt();
        assert_ne!(salt1, salt2);
    }

    #[test]
    fn test_derive_key_from_password_deterministic() {
        let salt = vec![42u8; 16];
        let key1 = derive_key_from_password("my-secret", &salt).unwrap();
        let key2 = derive_key_from_password("my-secret", &salt).unwrap();
        assert_eq!(key1, key2);
        assert_eq!(key1.len(), 32);
    }

    #[test]
    fn test_derive_key_from_password_different_passwords() {
        let salt = vec![42u8; 16];
        let key1 = derive_key_from_password("password-a", &salt).unwrap();
        let key2 = derive_key_from_password("password-b", &salt).unwrap();
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_derive_key_from_password_different_salts() {
        let salt1 = vec![1u8; 16];
        let salt2 = vec![2u8; 16];
        let key1 = derive_key_from_password("same-password", &salt1).unwrap();
        let key2 = derive_key_from_password("same-password", &salt2).unwrap();
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_encrypt_decrypt_file_roundtrip() {
        let salt = generate_salt();
        let key = derive_key_from_password("test-password", &salt).unwrap();
        let plaintext = b"hello world clipboard data";
        let encrypted = encrypt_file(plaintext, &key, &salt).unwrap();
        let (decrypted, recovered_salt) = decrypt_file(&encrypted, &key).unwrap();
        assert_eq!(decrypted, plaintext);
        assert_eq!(recovered_salt, salt);
    }

    #[test]
    fn test_encrypt_file_wrong_key_fails() {
        let salt = generate_salt();
        let key1 = derive_key_from_password("correct-password", &salt).unwrap();
        let key2 = derive_key_from_password("wrong-password", &salt).unwrap();
        let encrypted = encrypt_file(b"secret", &key1, &salt).unwrap();
        let result = decrypt_file(&encrypted, &key2);
        assert!(result.is_err());
    }

    #[test]
    fn test_decrypt_file_too_short() {
        let result = decrypt_file(&[0u8; 10], &[0u8; 32]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("too short"));
    }

    #[test]
    fn test_decrypt_file_wrong_version() {
        let mut bad_file = vec![0xFF, 0xFF, 0xFF, 0xFF];
        bad_file.extend_from_slice(&[0u8; 16 + 12 + 32]);
        let result = decrypt_file(&bad_file, &[0u8; 32]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unsupported file version"));
    }
}