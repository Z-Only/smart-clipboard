use std::sync::{Arc, Mutex};

use aes_gcm::aead::rand_core::RngCore;
use aes_gcm::aead::{Aead, KeyInit, OsRng};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use serde::{Deserialize, Serialize};

use crate::config::ConfigManager;
use crate::storage::Database;

const KEYRING_SERVICE: &str = "smart-clipboard";
const KEYRING_ACCOUNT: &str = "db-encryption-key";
const ENCRYPTED_PREFIX: &str = "enc:v1:";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct EncryptionConfig {
    #[serde(default)]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionStatus {
    pub enabled: bool,
    pub key_exists: bool,
    pub encrypted_count: i64,
    pub plaintext_count: i64,
    pub migrating: bool,
}

#[derive(Debug)]
struct RuntimeState {
    cached_key: Option<Vec<u8>>,
    migrating: bool,
}

pub struct EncryptionManager {
    config: Arc<ConfigManager>,
    runtime: Mutex<RuntimeState>,
}

impl EncryptionManager {
    pub fn new(config: Arc<ConfigManager>) -> Self {
        Self {
            config,
            runtime: Mutex::new(RuntimeState {
                cached_key: None,
                migrating: false,
            }),
        }
    }

    /// Return current encryption status including counts from the database.
    pub fn status(&self, database: &Database) -> EncryptionStatus {
        let cfg = self.config.get().encryption;
        let runtime = self.runtime.lock().unwrap();
        let (encrypted_count, plaintext_count) =
            database.count_encrypted_entries().unwrap_or((0, 0));
        EncryptionStatus {
            enabled: cfg.enabled,
            key_exists: encryption_key_exists(),
            encrypted_count,
            plaintext_count,
            migrating: runtime.migrating,
        }
    }

    /// Enable encryption: generate key if absent, update config, migrate existing entries.
    pub fn enable(&self, database: &Database) -> Result<EncryptionStatus, String> {
        ensure_encryption_key()?;

        let mut cfg = self.config.get();
        cfg.encryption.enabled = true;
        self.config.update(cfg)?;

        self.migrate_to_encrypted(database)?;

        Ok(self.status(database))
    }

    /// Disable encryption: decrypt all entries, update config, remove key.
    pub fn disable(&self, database: &Database) -> Result<EncryptionStatus, String> {
        self.migrate_to_plaintext(database)?;

        let mut cfg = self.config.get();
        cfg.encryption.enabled = false;
        self.config.update(cfg)?;

        // Clear cached key
        let mut runtime = self.runtime.lock().unwrap();
        runtime.cached_key = None;

        Ok(self.status(database))
    }

    /// Encrypt plaintext content. Returns the encrypted string with prefix.
    pub fn encrypt_content(&self, plaintext: &str) -> Result<String, String> {
        let key_bytes = self.get_or_load_key()?;
        encrypt_string(plaintext, &key_bytes)
    }

    /// Decrypt content if encrypted, otherwise return as-is.
    pub fn decrypt_content(&self, content: &str) -> Result<String, String> {
        if !content.starts_with(ENCRYPTED_PREFIX) {
            return Ok(content.to_string());
        }
        let key_bytes = self.get_or_load_key()?;
        decrypt_string(content, &key_bytes)
    }

    /// Check if encryption is currently enabled in config.
    pub fn is_enabled(&self) -> bool {
        self.config.get().encryption.enabled
    }

    fn get_or_load_key(&self) -> Result<Vec<u8>, String> {
        let mut runtime = self.runtime.lock().unwrap();
        if let Some(ref key) = runtime.cached_key {
            return Ok(key.clone());
        }
        let key = load_encryption_key()?;
        runtime.cached_key = Some(key.clone());
        Ok(key)
    }

    fn migrate_to_encrypted(&self, database: &Database) -> Result<(), String> {
        {
            let mut runtime = self.runtime.lock().unwrap();
            runtime.migrating = true;
        }
        let result = self.do_migrate_to_encrypted(database);
        {
            let mut runtime = self.runtime.lock().unwrap();
            runtime.migrating = false;
        }
        result
    }

    fn do_migrate_to_encrypted(&self, database: &Database) -> Result<(), String> {
        let key_bytes = self.get_or_load_key()?;
        let entries = database
            .get_plaintext_entries_for_migration()
            .map_err(|e| format!("Failed to fetch entries for migration: {e}"))?;

        for (id, content) in entries {
            if content.starts_with(ENCRYPTED_PREFIX) {
                continue;
            }
            let encrypted = encrypt_string(&content, &key_bytes)?;
            database
                .update_entry_content_and_encrypted_flag(id, &encrypted, true)
                .map_err(|e| format!("Failed to encrypt entry {id}: {e}"))?;
        }
        Ok(())
    }

    fn migrate_to_plaintext(&self, database: &Database) -> Result<(), String> {
        {
            let mut runtime = self.runtime.lock().unwrap();
            runtime.migrating = true;
        }
        let result = self.do_migrate_to_plaintext(database);
        {
            let mut runtime = self.runtime.lock().unwrap();
            runtime.migrating = false;
        }
        result
    }

    fn do_migrate_to_plaintext(&self, database: &Database) -> Result<(), String> {
        let key_bytes = self.get_or_load_key()?;
        let entries = database
            .get_encrypted_entries_for_migration()
            .map_err(|e| format!("Failed to fetch encrypted entries: {e}"))?;

        for (id, content) in entries {
            if !content.starts_with(ENCRYPTED_PREFIX) {
                continue;
            }
            let decrypted = decrypt_string(&content, &key_bytes)?;
            database
                .update_entry_content_and_encrypted_flag(id, &decrypted, false)
                .map_err(|e| format!("Failed to decrypt entry {id}: {e}"))?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Keyring helpers
// ---------------------------------------------------------------------------

fn keyring_entry() -> Result<keyring_core::Entry, String> {
    init_keyring()?;
    keyring_core::Entry::new(KEYRING_SERVICE, KEYRING_ACCOUNT)
        .map_err(|e| format!("Failed to create keyring entry: {e}"))
}

pub fn encryption_key_exists() -> bool {
    load_encryption_key_raw()
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false)
}

fn load_encryption_key_raw() -> Result<String, String> {
    keyring_entry()?
        .get_password()
        .map_err(|e| format!("Failed to load encryption key: {e}"))
}

fn load_encryption_key() -> Result<Vec<u8>, String> {
    let encoded = load_encryption_key_raw()?;
    BASE64
        .decode(encoded.trim())
        .map_err(|e| format!("Failed to decode encryption key: {e}"))
}

fn save_encryption_key(key: &[u8]) -> Result<(), String> {
    let encoded = BASE64.encode(key);
    keyring_entry()?
        .set_password(&encoded)
        .map_err(|e| format!("Failed to store encryption key: {e}"))
}

fn ensure_encryption_key() -> Result<(), String> {
    if encryption_key_exists() {
        return Ok(());
    }
    let mut key = [0u8; 32]; // 256 bits
    OsRng.fill_bytes(&mut key);
    save_encryption_key(&key)
}

#[cfg(test)]
fn init_keyring() -> Result<(), String> {
    if keyring_core::get_default_store().is_none() {
        crate::security::install_test_keyring_store();
    }
    Ok(())
}

#[cfg(not(test))]
fn init_keyring() -> Result<(), String> {
    keyring::use_native_store(false).map_err(|e| format!("Failed to initialize keyring store: {e}"))
}

// ---------------------------------------------------------------------------
// AES-256-GCM encrypt / decrypt
// ---------------------------------------------------------------------------

fn encrypt_string(plaintext: &str, key_bytes: &[u8]) -> Result<String, String> {
    let key = Key::<Aes256Gcm>::from_slice(key_bytes);
    let cipher = Aes256Gcm::new(key);

    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|e| format!("Encryption failed: {e}"))?;

    // Format: enc:v1:<base64(nonce + ciphertext)>
    let mut combined = Vec::with_capacity(12 + ciphertext.len());
    combined.extend_from_slice(&nonce_bytes);
    combined.extend_from_slice(&ciphertext);
    Ok(format!("{}{}", ENCRYPTED_PREFIX, BASE64.encode(&combined)))
}

fn decrypt_string(content: &str, key_bytes: &[u8]) -> Result<String, String> {
    let encoded = content
        .strip_prefix(ENCRYPTED_PREFIX)
        .ok_or_else(|| "Content is not encrypted".to_string())?;

    let combined = BASE64
        .decode(encoded)
        .map_err(|e| format!("Failed to decode encrypted content: {e}"))?;

    if combined.len() < 12 {
        return Err("Invalid encrypted content: too short".to_string());
    }

    let (nonce_bytes, ciphertext) = combined.split_at(12);
    let key = Key::<Aes256Gcm>::from_slice(key_bytes);
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::from_slice(nonce_bytes);

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| "Decryption failed: invalid key or corrupted data".to_string())?;

    String::from_utf8(plaintext).map_err(|e| format!("Decrypted content is not valid UTF-8: {e}"))
}

/// Check whether a content string is encrypted (starts with the version prefix).
pub fn is_encrypted(content: &str) -> bool {
    content.starts_with(ENCRYPTED_PREFIX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let mut key = [0u8; 32];
        OsRng.fill_bytes(&mut key);

        let original = "Hello, clipboard encryption!";
        let encrypted = encrypt_string(original, &key).unwrap();

        assert!(encrypted.starts_with(ENCRYPTED_PREFIX));
        assert_ne!(encrypted, original);

        let decrypted = decrypt_string(&encrypted, &key).unwrap();
        assert_eq!(decrypted, original);
    }

    #[test]
    fn test_decrypt_plaintext_passthrough() {
        crate::security::install_test_keyring_store();
        ensure_encryption_key().unwrap();

        let config = Arc::new(ConfigManager::new(tempfile::tempdir().unwrap().keep()));
        let manager = EncryptionManager::new(config);

        let plaintext = "not encrypted";
        let result = manager.decrypt_content(plaintext).unwrap();
        assert_eq!(result, plaintext);

        crate::security::reset_test_keyring_store();
    }

    #[test]
    fn test_is_encrypted() {
        assert!(is_encrypted("enc:v1:abc123"));
        assert!(!is_encrypted("plain text"));
        assert!(!is_encrypted(""));
    }

    #[test]
    fn test_wrong_key_fails_decryption() {
        let mut key1 = [0u8; 32];
        let mut key2 = [0u8; 32];
        OsRng.fill_bytes(&mut key1);
        OsRng.fill_bytes(&mut key2);

        let encrypted = encrypt_string("secret", &key1).unwrap();
        let result = decrypt_string(&encrypted, &key2);
        assert!(result.is_err());
    }
}
