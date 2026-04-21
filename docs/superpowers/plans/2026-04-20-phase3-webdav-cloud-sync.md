# WebDAV Cloud Relay Sync Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add WebDAV-based cloud relay synchronization as an independent sync channel alongside existing LAN sync, enabling clipboard sync between devices not on the same local network.

**Architecture:** New `webdav/` submodule under `src-tauri/src/sync/` containing 5 files: rate limiter, HTTP client, index manager, poller, and orchestrator. Extends existing `crypto.rs` with password-derived key functions. Frontend adds a WebDAV tab to the existing SyncPanel with a new Pinia store.

**Tech Stack:** Rust (reqwest, argon2, aes-gcm), Vue 3 + TypeScript + Pinia, Tauri IPC

---

## File Structure

### New Files (Rust backend)

- `src-tauri/src/sync/webdav/mod.rs` — WebDavSyncManager orchestrator
- `src-tauri/src/sync/webdav/rate_limiter.rs` — Token bucket rate limiter
- `src-tauri/src/sync/webdav/client.rs` — WebDAV HTTP client (PUT/GET/MKCOL/DELETE)
- `src-tauri/src/sync/webdav/index.rs` — Encrypted index and device registry management
- `src-tauri/src/sync/webdav/poller.rs` — Periodic poll scheduler

### New Files (Vue frontend)

- `src/stores/webdavStore.ts` — Pinia store for WebDAV sync state
- `src/components/WebDavPanel.vue` — WebDAV configuration and status UI

### Modified Files (Rust backend)

- `src-tauri/Cargo.toml` — Add `reqwest` and `argon2` dependencies
- `src-tauri/src/sync/crypto.rs` — Add password-derived key functions
- `src-tauri/src/sync/mod.rs` — Re-export webdav module, integrate with SyncManager
- `src-tauri/src/config.rs` — Add WebDavConfig to AppConfig
- `src-tauri/src/commands.rs` — Add WebDAV Tauri IPC commands
- `src-tauri/src/lib.rs` — Register new commands, initialize WebDavSyncManager

### Modified Files (Vue frontend)

- `src/types/index.ts` — Add WebDAV types
- `src/components/SyncPanel.vue` — Add tab switcher for LAN / WebDAV
- `src/i18n/locales/en.ts` — Add WebDAV i18n keys
- `src/i18n/locales/zh-CN.ts` — Add WebDAV i18n keys (Chinese)

---

### Task 1: Add Cargo Dependencies

**Files:**

- Modify: `src-tauri/Cargo.toml`

- [ ] **Step 1: Add reqwest and argon2 to Cargo.toml**

Add these two lines after the existing `x25519-dalek` dependency:

```toml
reqwest = { version = "0.12", default-features = false, features = ["rustls-tls", "json"] }
argon2 = "0.5"
```

- [ ] **Step 2: Verify dependencies resolve**

Run: `cd src-tauri && cargo check 2>&1 | tail -5`
Expected: Compilation succeeds (possibly with warnings, no errors)

- [ ] **Step 3: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -m "chore: add reqwest and argon2 dependencies for WebDAV sync"
```

---

### Task 2: Extend crypto.rs with Password-Derived Key Functions

**Files:**

- Modify: `src-tauri/src/sync/crypto.rs`

- [ ] **Step 1: Add argon2 imports and constants at the top of crypto.rs**

After the existing imports (line 9, after `use x25519_dalek::{PublicKey, StaticSecret};`), add:

```rust
use argon2::{Argon2, Params, Version, Algorithm};
```

After the existing `HKDF_INFO` constant (line 13), add:

```rust
const ARGON2_SALT_LEN: usize = 16;
const ARGON2_MEMORY_KIB: u32 = 65536;
const ARGON2_ITERATIONS: u32 = 3;
const ARGON2_PARALLELISM: u32 = 4;
const FILE_VERSION: [u8; 4] = [0x53, 0x43, 0x01, 0x00]; // "SC" + version 1.0
```

- [ ] **Step 2: Add generate_salt function**

After the existing `generate_keypair()` function, add:

```rust
pub fn generate_salt() -> Vec<u8> {
    let mut salt = vec![0u8; ARGON2_SALT_LEN];
    OsRng.fill_bytes(&mut salt);
    salt
}
```

- [ ] **Step 3: Add derive_key_from_password function**

After `generate_salt()`, add:

```rust
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
```

- [ ] **Step 4: Add encrypt_file and decrypt_file functions**

After `derive_key_from_password()`, add:

```rust
/// Encrypt data into a self-contained file format:
/// [4 bytes version][16 bytes salt][12 bytes nonce][ciphertext+tag]
/// The salt field is caller-provided context (e.g. Argon2 salt for devices.enc,
/// or zeroes for entry files where the key is already derived).
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

/// Decrypt a file produced by encrypt_file.
/// Returns (plaintext, salt) — caller can use the salt for context.
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
```

- [ ] **Step 5: Add tests for new crypto functions**

At the end of the existing `mod tests` block (before the closing `}`), add:

```rust
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
```

- [ ] **Step 6: Run tests**

Run: `cd src-tauri && cargo test --lib sync::crypto -- --nocapture 2>&1 | tail -20`
Expected: All tests pass

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/sync/crypto.rs
git commit -m "feat(webdav): add password-derived key and file encryption to crypto.rs"
```

---

### Task 3: Token Bucket Rate Limiter

**Files:**

- Create: `src-tauri/src/sync/webdav/rate_limiter.rs`

- [ ] **Step 1: Create the rate_limiter.rs file**

```rust
use std::sync::Mutex;
use std::time::Instant;

pub struct TokenBucketLimiter {
    state: Mutex<BucketState>,
    capacity: u32,
    refill_rate_per_sec: f64,
}

struct BucketState {
    tokens: f64,
    last_refill: Instant,
}

impl TokenBucketLimiter {
    pub fn new(capacity: u32, refill_period_minutes: u32) -> Self {
        let refill_rate_per_sec = f64::from(capacity) / (f64::from(refill_period_minutes) * 60.0);
        Self {
            state: Mutex::new(BucketState {
                tokens: f64::from(capacity),
                last_refill: Instant::now(),
            }),
            capacity,
            refill_rate_per_sec,
        }
    }

    fn refill(&self, state: &mut BucketState) {
        let now = Instant::now();
        let elapsed = now.duration_since(state.last_refill).as_secs_f64();
        let added = elapsed * self.refill_rate_per_sec;
        state.tokens = (state.tokens + added).min(f64::from(self.capacity));
        state.last_refill = now;
    }

    /// Try to consume `count` tokens. Returns true if successful, false if insufficient.
    pub fn try_acquire(&self, count: u32) -> bool {
        let mut state = self.state.lock().unwrap();
        self.refill(&mut state);
        let needed = f64::from(count);
        if state.tokens >= needed {
            state.tokens -= needed;
            true
        } else {
            false
        }
    }

    /// Block until `count` tokens are available, then consume them.
    /// Returns the number of seconds waited.
    pub async fn acquire(&self, count: u32) -> f64 {
        let mut total_waited = 0.0;
        loop {
            {
                let mut state = self.state.lock().unwrap();
                self.refill(&mut state);
                let needed = f64::from(count);
                if state.tokens >= needed {
                    state.tokens -= needed;
                    return total_waited;
                }
                // Calculate wait time
                let deficit = needed - state.tokens;
                let wait_secs = deficit / self.refill_rate_per_sec;
                let wait_ms = (wait_secs * 1000.0).ceil().max(100.0) as u64;
                drop(state);
                tokio::time::sleep(std::time::Duration::from_millis(wait_ms)).await;
                total_waited += wait_ms as f64 / 1000.0;
            }
        }
    }

    /// Return the current number of available tokens (approximate).
    pub fn available(&self) -> u32 {
        let mut state = self.state.lock().unwrap();
        self.refill(&mut state);
        state.tokens.floor() as u32
    }

    pub fn capacity(&self) -> u32 {
        self.capacity
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_limiter_starts_full() {
        let limiter = TokenBucketLimiter::new(100, 30);
        assert_eq!(limiter.available(), 100);
        assert_eq!(limiter.capacity(), 100);
    }

    #[test]
    fn test_try_acquire_success() {
        let limiter = TokenBucketLimiter::new(100, 30);
        assert!(limiter.try_acquire(10));
        assert_eq!(limiter.available(), 90);
    }

    #[test]
    fn test_try_acquire_insufficient() {
        let limiter = TokenBucketLimiter::new(5, 30);
        assert!(limiter.try_acquire(3));
        assert!(!limiter.try_acquire(5));
        assert_eq!(limiter.available(), 2);
    }

    #[test]
    fn test_try_acquire_exact() {
        let limiter = TokenBucketLimiter::new(10, 30);
        assert!(limiter.try_acquire(10));
        assert!(!limiter.try_acquire(1));
        assert_eq!(limiter.available(), 0);
    }

    #[tokio::test]
    async fn test_acquire_immediate_when_available() {
        let limiter = TokenBucketLimiter::new(100, 30);
        let waited = limiter.acquire(5).await;
        assert_eq!(waited, 0.0);
        assert_eq!(limiter.available(), 95);
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cd src-tauri && cargo test --lib sync::webdav::rate_limiter -- --nocapture 2>&1 | tail -15`
Expected: All tests pass (will fail until mod.rs wires it — tested after Task 7)

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/sync/webdav/rate_limiter.rs
git commit -m "feat(webdav): add token bucket rate limiter"
```

---

### Task 4: WebDAV HTTP Client

**Files:**

- Create: `src-tauri/src/sync/webdav/client.rs`

- [ ] **Step 1: Create the client.rs file**

```rust
use std::sync::Arc;

use log::{info, warn};
use reqwest::{Client, StatusCode};

use super::rate_limiter::TokenBucketLimiter;

pub struct WebDavClient {
    http: Client,
    base_url: String,
    username: String,
    password: String,
    rate_limiter: Arc<TokenBucketLimiter>,
}

pub enum PutResult {
    Ok,
    EtagConflict,
}

impl WebDavClient {
    pub fn new(
        base_url: &str,
        username: &str,
        password: &str,
        rate_limiter: Arc<TokenBucketLimiter>,
    ) -> Result<Self, String> {
        let http = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {e}"))?;

        let base_url = base_url.trim_end_matches('/').to_string();

        Ok(Self {
            http,
            base_url,
            username: username.to_string(),
            password: password.to_string(),
            rate_limiter,
        })
    }

    fn url(&self, path: &str) -> String {
        let path = path.trim_start_matches('/');
        format!("{}/{}", self.base_url, path)
    }

    pub async fn get(&self, path: &str) -> Result<(Vec<u8>, Option<String>), String> {
        self.rate_limiter.acquire(1).await;
        let response = self
            .http
            .get(self.url(path))
            .basic_auth(&self.username, Some(&self.password))
            .send()
            .await
            .map_err(|e| format!("WebDAV GET failed: {e}"))?;

        let status = response.status();
        if status == StatusCode::NOT_FOUND {
            return Err("NotFound".to_string());
        }
        if !status.is_success() {
            return Err(format!("WebDAV GET returned {status}"));
        }

        let etag = response
            .headers()
            .get("etag")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let body = response
            .bytes()
            .await
            .map_err(|e| format!("Failed to read response body: {e}"))?;

        Ok((body.to_vec(), etag))
    }

    pub async fn put(&self, path: &str, data: &[u8]) -> Result<(), String> {
        self.rate_limiter.acquire(1).await;
        let response = self
            .http
            .put(self.url(path))
            .basic_auth(&self.username, Some(&self.password))
            .header("Content-Type", "application/octet-stream")
            .body(data.to_vec())
            .send()
            .await
            .map_err(|e| format!("WebDAV PUT failed: {e}"))?;

        let status = response.status();
        if !status.is_success() && status != StatusCode::CREATED {
            return Err(format!("WebDAV PUT returned {status}"));
        }
        Ok(())
    }

    pub async fn put_with_etag(&self, path: &str, data: &[u8], etag: &str) -> Result<PutResult, String> {
        self.rate_limiter.acquire(1).await;
        let response = self
            .http
            .put(self.url(path))
            .basic_auth(&self.username, Some(&self.password))
            .header("Content-Type", "application/octet-stream")
            .header("If-Match", etag)
            .body(data.to_vec())
            .send()
            .await
            .map_err(|e| format!("WebDAV PUT failed: {e}"))?;

        let status = response.status();
        if status == StatusCode::PRECONDITION_FAILED {
            return Ok(PutResult::EtagConflict);
        }
        if !status.is_success() && status != StatusCode::CREATED {
            return Err(format!("WebDAV PUT returned {status}"));
        }
        Ok(PutResult::Ok)
    }

    pub async fn mkcol(&self, path: &str) -> Result<(), String> {
        self.rate_limiter.acquire(1).await;
        let response = self
            .http
            .request(reqwest::Method::from_bytes(b"MKCOL").unwrap(), self.url(path))
            .basic_auth(&self.username, Some(&self.password))
            .send()
            .await
            .map_err(|e| format!("WebDAV MKCOL failed: {e}"))?;

        let status = response.status();
        // 201 Created or 405 Method Not Allowed (already exists) are both OK
        if status == StatusCode::CREATED
            || status == StatusCode::METHOD_NOT_ALLOWED
            || status == StatusCode::CONFLICT
        {
            return Ok(());
        }
        if !status.is_success() {
            return Err(format!("WebDAV MKCOL returned {status}"));
        }
        Ok(())
    }

    pub async fn delete(&self, path: &str) -> Result<(), String> {
        self.rate_limiter.acquire(1).await;
        let response = self
            .http
            .delete(self.url(path))
            .basic_auth(&self.username, Some(&self.password))
            .send()
            .await
            .map_err(|e| format!("WebDAV DELETE failed: {e}"))?;

        let status = response.status();
        if status == StatusCode::NOT_FOUND {
            return Ok(()); // Already gone
        }
        if !status.is_success() && status != StatusCode::NO_CONTENT {
            return Err(format!("WebDAV DELETE returned {status}"));
        }
        Ok(())
    }

    pub async fn exists(&self, path: &str) -> Result<bool, String> {
        self.rate_limiter.acquire(1).await;
        let response = self
            .http
            .head(self.url(path))
            .basic_auth(&self.username, Some(&self.password))
            .send()
            .await
            .map_err(|e| format!("WebDAV HEAD failed: {e}"))?;

        Ok(response.status().is_success())
    }

    pub async fn ensure_directory_structure(&self, remote_path: &str) -> Result<(), String> {
        let remote_path = remote_path.trim_matches('/');
        self.mkcol(remote_path).await?;
        self.mkcol(&format!("{}/meta", remote_path)).await?;
        self.mkcol(&format!("{}/entries", remote_path)).await?;
        info!("WebDAV directory structure ensured at /{}", remote_path);
        Ok(())
    }

    /// Test connectivity by attempting HEAD on the base URL.
    pub async fn test_connection(&self) -> Result<(), String> {
        let response = self
            .http
            .request(reqwest::Method::OPTIONS, &self.base_url)
            .basic_auth(&self.username, Some(&self.password))
            .send()
            .await
            .map_err(|e| format!("Connection test failed: {e}"))?;

        let status = response.status();
        if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
            return Err("Authentication failed — check username and password".to_string());
        }
        if !status.is_success() {
            warn!("WebDAV OPTIONS returned {status}, but connection succeeded");
        }
        Ok(())
    }
}
```

- [ ] **Step 2: Commit**

```bash
git add src-tauri/src/sync/webdav/client.rs
git commit -m "feat(webdav): add WebDAV HTTP client with rate limiting"
```

---

### Task 5: Encrypted Index and Device Registry Manager

**Files:**

- Create: `src-tauri/src/sync/webdav/index.rs`

- [ ] **Step 1: Create the index.rs file**

```rust
use std::collections::HashSet;
use std::sync::Arc;

use chrono::Utc;
use log::{info, warn};
use serde::{Deserialize, Serialize};

use super::client::{PutResult, WebDavClient};
use crate::sync::crypto;

const MAX_ETAG_RETRIES: u32 = 3;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncIndex {
    pub version: u32,
    pub updated_at: String,
    pub updated_by: String,
    pub entries: Vec<IndexEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexEntry {
    pub hash: String,
    pub content_type: String,
    pub category: String,
    pub source_device: String,
    pub created_at: String,
    pub is_sensitive: bool,
    pub size_bytes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceRegistry {
    pub version: u32,
    pub salt: String,
    pub devices: Vec<RegisteredDevice>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisteredDevice {
    pub device_id: String,
    pub device_name: String,
    pub public_key: String,
    pub registered_at: String,
    pub last_sync_at: Option<String>,
}

pub struct IndexManager {
    client: Arc<WebDavClient>,
    remote_path: String,
    device_id: String,
}

impl IndexManager {
    pub fn new(client: Arc<WebDavClient>, remote_path: &str, device_id: &str) -> Self {
        Self {
            client,
            remote_path: remote_path.trim_matches('/').to_string(),
            device_id: device_id.to_string(),
        }
    }

    fn index_path(&self) -> String {
        format!("{}/meta/index.enc", self.remote_path)
    }

    fn devices_path(&self) -> String {
        format!("{}/meta/devices.enc", self.remote_path)
    }

    fn entry_path(&self, hash: &str) -> String {
        let prefix = if hash.len() >= 12 {
            &hash[..12]
        } else {
            hash
        };
        format!("{}/entries/{}.enc", self.remote_path, prefix)
    }

    // --- Device Registry ---

    pub async fn load_device_registry(
        &self,
        master_key: &[u8],
    ) -> Result<DeviceRegistry, String> {
        let (encrypted, _etag) = self.client.get(&self.devices_path()).await?;
        let (plaintext, _salt) = crypto::decrypt_file(&encrypted, master_key)?;
        serde_json::from_slice(&plaintext)
            .map_err(|e| format!("Failed to parse device registry: {e}"))
    }

    pub async fn save_device_registry(
        &self,
        registry: &DeviceRegistry,
        master_key: &[u8],
        salt: &[u8],
    ) -> Result<(), String> {
        let json = serde_json::to_vec_pretty(registry)
            .map_err(|e| format!("Failed to serialize device registry: {e}"))?;
        let encrypted = crypto::encrypt_file(&json, master_key, salt)?;
        self.client.put(&self.devices_path(), &encrypted).await
    }

    pub async fn initialize_registry(
        &self,
        master_key: &[u8],
        salt: &[u8],
        device_id: &str,
        device_name: &str,
        public_key: &[u8],
    ) -> Result<DeviceRegistry, String> {
        let registry = DeviceRegistry {
            version: 1,
            salt: crypto::encode_key_material(salt),
            devices: vec![RegisteredDevice {
                device_id: device_id.to_string(),
                device_name: device_name.to_string(),
                public_key: crypto::encode_key_material(public_key),
                registered_at: Utc::now().to_rfc3339(),
                last_sync_at: None,
            }],
        };
        self.save_device_registry(&registry, master_key, salt)
            .await?;
        info!("Initialized device registry with device {}", device_id);
        Ok(registry)
    }

    pub async fn register_device(
        &self,
        master_key: &[u8],
        salt: &[u8],
        device_id: &str,
        device_name: &str,
        public_key: &[u8],
    ) -> Result<(), String> {
        let mut registry = self.load_device_registry(master_key).await?;

        // Update existing or add new
        if let Some(existing) = registry.devices.iter_mut().find(|d| d.device_id == device_id) {
            existing.device_name = device_name.to_string();
            existing.public_key = crypto::encode_key_material(public_key);
            existing.last_sync_at = Some(Utc::now().to_rfc3339());
        } else {
            registry.devices.push(RegisteredDevice {
                device_id: device_id.to_string(),
                device_name: device_name.to_string(),
                public_key: crypto::encode_key_material(public_key),
                registered_at: Utc::now().to_rfc3339(),
                last_sync_at: None,
            });
        }

        self.save_device_registry(&registry, master_key, salt)
            .await?;
        info!("Registered device {} in cloud registry", device_id);
        Ok(())
    }

    // --- Index File ---

    pub async fn load_index(
        &self,
        master_key: &[u8],
    ) -> Result<(SyncIndex, Option<String>), String> {
        let result = self.client.get(&self.index_path()).await;
        match result {
            Ok((encrypted, etag)) => {
                let (plaintext, _salt) = crypto::decrypt_file(&encrypted, master_key)?;
                let index: SyncIndex = serde_json::from_slice(&plaintext)
                    .map_err(|e| format!("Failed to parse index: {e}"))?;
                Ok((index, etag))
            }
            Err(e) if e == "NotFound" => {
                let empty = SyncIndex {
                    version: 1,
                    updated_at: Utc::now().to_rfc3339(),
                    updated_by: self.device_id.clone(),
                    entries: vec![],
                };
                Ok((empty, None))
            }
            Err(e) => Err(e),
        }
    }

    pub async fn save_index(
        &self,
        index: &SyncIndex,
        master_key: &[u8],
        etag: Option<&str>,
    ) -> Result<bool, String> {
        let json = serde_json::to_vec_pretty(index)
            .map_err(|e| format!("Failed to serialize index: {e}"))?;
        let zero_salt = vec![0u8; 16];
        let encrypted = crypto::encrypt_file(&json, master_key, &zero_salt)?;

        if let Some(etag) = etag {
            match self
                .client
                .put_with_etag(&self.index_path(), &encrypted, etag)
                .await?
            {
                PutResult::Ok => Ok(true),
                PutResult::EtagConflict => Ok(false),
            }
        } else {
            self.client.put(&self.index_path(), &encrypted).await?;
            Ok(true)
        }
    }

    /// Append an entry to the index with ETag-based conflict retry.
    pub async fn append_entry(
        &self,
        entry: IndexEntry,
        master_key: &[u8],
    ) -> Result<(), String> {
        for attempt in 0..MAX_ETAG_RETRIES {
            let (mut index, etag) = self.load_index(master_key).await?;

            // Skip if already in index
            if index.entries.iter().any(|e| e.hash == entry.hash) {
                return Ok(());
            }

            index.entries.push(entry.clone());
            index.updated_at = Utc::now().to_rfc3339();
            index.updated_by = self.device_id.clone();

            let saved = self
                .save_index(&index, master_key, etag.as_deref())
                .await?;
            if saved {
                return Ok(());
            }

            warn!(
                "Index ETag conflict on attempt {}, retrying...",
                attempt + 1
            );
        }
        Err("Failed to update index after max retries (ETag conflict)".to_string())
    }

    /// Remove oldest entries beyond the limit and delete their files.
    pub async fn enforce_entry_limit(
        &self,
        max_entries: u32,
        master_key: &[u8],
    ) -> Result<u32, String> {
        let (mut index, etag) = self.load_index(master_key).await?;
        let count = index.entries.len() as u32;
        if count <= max_entries {
            return Ok(0);
        }

        let remove_count = count - max_entries;
        // Entries are in chronological order; remove from the front (oldest)
        let removed: Vec<IndexEntry> = index.entries.drain(..remove_count as usize).collect();

        // Delete entry files
        for entry in &removed {
            if let Err(e) = self.client.delete(&self.entry_path(&entry.hash)).await {
                warn!("Failed to delete old entry file {}: {}", entry.hash, e);
            }
        }

        index.updated_at = Utc::now().to_rfc3339();
        index.updated_by = self.device_id.clone();
        self.save_index(&index, master_key, etag.as_deref())
            .await?;

        info!("Cleaned up {} old entries from cloud", remove_count);
        Ok(remove_count)
    }

    /// Find new entry hashes not in the local known set.
    pub fn find_new_entries(
        &self,
        index: &SyncIndex,
        known_hashes: &HashSet<String>,
        local_device_id: &str,
    ) -> Vec<IndexEntry> {
        index
            .entries
            .iter()
            .filter(|e| !known_hashes.contains(&e.hash) && e.source_device != local_device_id)
            .cloned()
            .collect()
    }

    // --- Entry Files ---

    pub async fn upload_entry(
        &self,
        hash: &str,
        plaintext_json: &[u8],
        master_key: &[u8],
    ) -> Result<(), String> {
        let zero_salt = vec![0u8; 16];
        let encrypted = crypto::encrypt_file(plaintext_json, master_key, &zero_salt)?;
        self.client.put(&self.entry_path(hash), &encrypted).await
    }

    pub async fn download_entry(
        &self,
        hash: &str,
        master_key: &[u8],
    ) -> Result<Vec<u8>, String> {
        let (encrypted, _etag) = self.client.get(&self.entry_path(hash)).await?;
        let (plaintext, _salt) = crypto::decrypt_file(&encrypted, master_key)?;
        Ok(plaintext)
    }
}
```

- [ ] **Step 2: Commit**

```bash
git add src-tauri/src/sync/webdav/index.rs
git commit -m "feat(webdav): add encrypted index and device registry manager"
```

---

### Task 6: Sync Poller

**Files:**

- Create: `src-tauri/src/sync/webdav/poller.rs`

- [ ] **Step 1: Create the poller.rs file**

```rust
use std::collections::HashSet;
use std::sync::Arc;

use chrono::Local;
use log::{error, info, warn};
use tauri::Emitter;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

use super::index::{IndexManager, SyncIndex};
use crate::storage::{ClipboardEntry, Database};
use crate::sync::protocol::SyncEntryPayload;

pub struct SyncPoller {
    db: Arc<Database>,
    index_manager: Arc<IndexManager>,
    master_key: Arc<RwLock<Option<Vec<u8>>>>,
    device_id: String,
    poll_handle: RwLock<Option<JoinHandle<()>>>,
    app_handle: Arc<RwLock<Option<tauri::AppHandle>>>,
}

impl SyncPoller {
    pub fn new(
        db: Arc<Database>,
        index_manager: Arc<IndexManager>,
        master_key: Arc<RwLock<Option<Vec<u8>>>>,
        device_id: &str,
        app_handle: Arc<RwLock<Option<tauri::AppHandle>>>,
    ) -> Self {
        Self {
            db,
            index_manager,
            master_key,
            device_id: device_id.to_string(),
            poll_handle: RwLock::new(None),
            app_handle,
        }
    }

    pub async fn start(&self, interval_secs: u64) {
        self.stop().await;

        let db = self.db.clone();
        let index_manager = self.index_manager.clone();
        let master_key = self.master_key.clone();
        let device_id = self.device_id.clone();
        let app_handle = self.app_handle.clone();

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(interval_secs));
            interval.tick().await; // Skip first immediate tick

            loop {
                interval.tick().await;

                let key = {
                    let guard = master_key.read().await;
                    match guard.as_ref() {
                        Some(k) => k.clone(),
                        None => {
                            warn!("WebDAV poller: no master key available, skipping poll");
                            continue;
                        }
                    }
                };

                if let Err(e) =
                    Self::poll_once(&db, &index_manager, &key, &device_id, &app_handle).await
                {
                    error!("WebDAV poll error: {}", e);
                }
            }
        });

        *self.poll_handle.write().await = Some(handle);
        info!(
            "WebDAV poller started with {}s interval",
            interval_secs
        );
    }

    pub async fn stop(&self) {
        if let Some(handle) = self.poll_handle.write().await.take() {
            handle.abort();
            info!("WebDAV poller stopped");
        }
    }

    pub async fn poll_now(&self) -> Result<u32, String> {
        let key = {
            let guard = self.master_key.read().await;
            guard
                .as_ref()
                .ok_or_else(|| "No master key available".to_string())?
                .clone()
        };
        Self::poll_once(
            &self.db,
            &self.index_manager,
            &key,
            &self.device_id,
            &self.app_handle,
        )
        .await
    }

    async fn poll_once(
        db: &Arc<Database>,
        index_manager: &Arc<IndexManager>,
        master_key: &[u8],
        device_id: &str,
        app_handle: &Arc<RwLock<Option<tauri::AppHandle>>>,
    ) -> Result<u32, String> {
        let (index, _etag) = index_manager.load_index(master_key).await?;

        // Build known hash set from local DB
        let known_hashes = Self::build_known_hashes(db)?;

        let new_entries = index_manager.find_new_entries(&index, &known_hashes, device_id);
        if new_entries.is_empty() {
            return Ok(0);
        }

        info!(
            "WebDAV poll: found {} new entries to download",
            new_entries.len()
        );

        let mut downloaded = 0u32;
        for index_entry in &new_entries {
            match index_manager
                .download_entry(&index_entry.hash, master_key)
                .await
            {
                Ok(plaintext) => {
                    match serde_json::from_slice::<SyncEntryPayload>(&plaintext) {
                        Ok(payload) => {
                            if Self::insert_synced_entry(db, &payload, app_handle).await {
                                downloaded += 1;
                            }
                        }
                        Err(e) => {
                            warn!(
                                "Failed to parse entry {}: {}",
                                index_entry.hash, e
                            );
                        }
                    }
                }
                Err(e) => {
                    warn!(
                        "Failed to download entry {}: {}",
                        index_entry.hash, e
                    );
                }
            }
        }

        info!("WebDAV poll: downloaded {} new entries", downloaded);
        Ok(downloaded)
    }

    fn build_known_hashes(db: &Database) -> Result<HashSet<String>, String> {
        db.get_all_hashes().map_err(|e| format!("Failed to get known hashes: {e}"))
    }

    async fn insert_synced_entry(
        db: &Arc<Database>,
        payload: &SyncEntryPayload,
        app_handle: &Arc<RwLock<Option<tauri::AppHandle>>>,
    ) -> bool {
        // Dedup check
        match db.find_by_hash(&payload.hash) {
            Ok(Some(_)) => return false,
            Ok(None) => {}
            Err(e) => {
                error!("DB error during WebDAV sync dedup: {}", e);
                return false;
            }
        }

        let created_at =
            chrono::NaiveDateTime::parse_from_str(&payload.created_at, "%Y-%m-%d %H:%M:%S")
                .unwrap_or_else(|_| Local::now().naive_local());
        let now = Local::now().naive_local();

        let entry = ClipboardEntry {
            id: None,
            content: payload.content.clone(),
            content_type: payload.content_type.clone(),
            category: payload.category.clone(),
            hash: payload.hash.clone(),
            source_app: payload.source_app.clone(),
            is_favorite: false,
            is_sensitive: payload.is_sensitive,
            use_count: 1,
            created_at,
            updated_at: now,
            expires_at: None,
            source_device: Some(payload.source_device.clone()),
        };

        match db.insert_entry(&entry) {
            Ok(id) => {
                let mut stored = entry;
                stored.id = Some(id);
                if let Some(handle) = app_handle.read().await.as_ref() {
                    let _ = handle.emit("clipboard-changed", &stored);
                }
                true
            }
            Err(e) => {
                if e.to_string().contains("UNIQUE") {
                    false // Race condition dedup
                } else {
                    error!("Failed to insert WebDAV synced entry: {}", e);
                    false
                }
            }
        }
    }
}
```

- [ ] **Step 2: Commit**

```bash
git add src-tauri/src/sync/webdav/poller.rs
git commit -m "feat(webdav): add sync poller for periodic pull"
```

---

### Task 7: WebDavSyncManager Orchestrator and Module Wiring

**Files:**

- Create: `src-tauri/src/sync/webdav/mod.rs`
- Modify: `src-tauri/src/sync/mod.rs`

- [ ] **Step 1: Create webdav/mod.rs**

```rust
pub mod client;
pub mod index;
pub mod poller;
pub mod rate_limiter;

use std::sync::Arc;

use chrono::Utc;
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use client::WebDavClient;
use index::{IndexEntry, IndexManager};
use poller::SyncPoller;
use rate_limiter::TokenBucketLimiter;

use crate::storage::{ClipboardEntry, Database};
use crate::sync::crypto;
use crate::sync::protocol::SyncEntryPayload;

const MAX_SYNC_PAYLOAD_BYTES: usize = 1_048_576; // 1 MB

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebDavConfig {
    pub enabled: bool,
    pub server_url: String,
    pub username: String,
    pub password: String,
    pub sync_password: String,
    pub poll_interval_secs: u64,
    pub sync_images: bool,
    pub sync_sensitive: bool,
    pub rate_limit_capacity: u32,
    pub rate_limit_refill_minutes: u32,
    pub remote_path: String,
    pub max_cloud_entries: u32,
}

impl Default for WebDavConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            server_url: String::new(),
            username: String::new(),
            password: String::new(),
            sync_password: String::new(),
            poll_interval_secs: 30,
            sync_images: false,
            sync_sensitive: false,
            rate_limit_capacity: 150,
            rate_limit_refill_minutes: 30,
            remote_path: "/SmartClipboard".to_string(),
            max_cloud_entries: 2000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebDavSyncStatus {
    pub status: String,
    pub last_sync_at: Option<String>,
    pub cloud_entry_count: u32,
    pub registered_devices: Vec<index::RegisteredDevice>,
    pub rate_limit_available: u32,
    pub rate_limit_capacity: u32,
    pub error: Option<String>,
}

pub struct WebDavSyncManager {
    db: Arc<Database>,
    master_key: Arc<RwLock<Option<Vec<u8>>>>,
    salt: RwLock<Option<Vec<u8>>>,
    client: RwLock<Option<Arc<WebDavClient>>>,
    index_manager: RwLock<Option<Arc<IndexManager>>>,
    poller: RwLock<Option<Arc<SyncPoller>>>,
    rate_limiter: RwLock<Option<Arc<TokenBucketLimiter>>>,
    config: RwLock<WebDavConfig>,
    status: RwLock<String>,
    last_sync_at: RwLock<Option<String>>,
    last_error: RwLock<Option<String>>,
    device_id: String,
    device_name: String,
    public_key: Vec<u8>,
    app_handle: Arc<RwLock<Option<tauri::AppHandle>>>,
}

impl WebDavSyncManager {
    pub fn new(
        db: Arc<Database>,
        config: WebDavConfig,
        device_id: &str,
        device_name: &str,
        public_key: &[u8],
    ) -> Arc<Self> {
        Arc::new(Self {
            db,
            master_key: Arc::new(RwLock::new(None)),
            salt: RwLock::new(None),
            client: RwLock::new(None),
            index_manager: RwLock::new(None),
            poller: RwLock::new(None),
            rate_limiter: RwLock::new(None),
            config: RwLock::new(config),
            status: RwLock::new("disconnected".to_string()),
            last_sync_at: RwLock::new(None),
            last_error: RwLock::new(None),
            device_id: device_id.to_string(),
            device_name: device_name.to_string(),
            public_key: public_key.to_vec(),
            app_handle: Arc::new(RwLock::new(None)),
        })
    }

    pub fn set_app_handle(&self, handle: tauri::AppHandle) {
        let app_handle = self.app_handle.clone();
        tauri::async_runtime::spawn(async move {
            *app_handle.write().await = Some(handle);
        });
    }

    pub async fn connect(
        &self,
        server_url: &str,
        username: &str,
        password: &str,
        sync_password: &str,
    ) -> Result<(), String> {
        *self.status.write().await = "connecting".to_string();
        *self.last_error.write().await = None;

        let config = self.config.read().await;
        let rate_limiter = Arc::new(TokenBucketLimiter::new(
            config.rate_limit_capacity,
            config.rate_limit_refill_minutes,
        ));
        let poll_interval = config.poll_interval_secs;
        let remote_path = config.remote_path.clone();
        let max_cloud_entries = config.max_cloud_entries;
        drop(config);

        // Create HTTP client
        let client = Arc::new(WebDavClient::new(
            server_url,
            username,
            password,
            rate_limiter.clone(),
        )?);

        // Test connection
        client.test_connection().await.map_err(|e| {
            tauri::async_runtime::block_on(async {
                *self.status.write().await = "error".to_string();
                *self.last_error.write().await = Some(e.clone());
            });
            e
        })?;

        // Ensure directory structure
        client.ensure_directory_structure(&remote_path).await?;

        // Create index manager
        let index_manager = Arc::new(IndexManager::new(
            client.clone(),
            &remote_path,
            &self.device_id,
        ));

        // Derive master key and handle device registration
        let devices_exist = client
            .exists(&format!("{}/meta/devices.enc", remote_path.trim_matches('/')))
            .await?;

        let (master_key, salt) = if devices_exist {
            // Load existing registry to get salt
            // First, try to get the file to extract salt from header
            let (encrypted, _) = client
                .get(&format!(
                    "{}/meta/devices.enc",
                    remote_path.trim_matches('/')
                ))
                .await?;

            // Extract salt from file header (bytes 4..20)
            if encrypted.len() < 36 {
                return Err("Device registry file is corrupted".to_string());
            }
            let salt = encrypted[4..20].to_vec();
            let master_key = crypto::derive_key_from_password(sync_password, &salt)?;

            // Verify by decrypting
            crypto::decrypt_file(&encrypted, &master_key).map_err(|_| {
                "Incorrect sync password — cannot decrypt device registry".to_string()
            })?;

            // Register this device
            index_manager
                .register_device(
                    &master_key,
                    &salt,
                    &self.device_id,
                    &self.device_name,
                    &self.public_key,
                )
                .await?;

            (master_key, salt)
        } else {
            // First-time setup
            let salt = crypto::generate_salt();
            let master_key = crypto::derive_key_from_password(sync_password, &salt)?;

            index_manager
                .initialize_registry(
                    &master_key,
                    &salt,
                    &self.device_id,
                    &self.device_name,
                    &self.public_key,
                )
                .await?;

            // Create empty index
            let empty_index = index::SyncIndex {
                version: 1,
                updated_at: Utc::now().to_rfc3339(),
                updated_by: self.device_id.clone(),
                entries: vec![],
            };
            index_manager
                .save_index(&empty_index, &master_key, None)
                .await?;

            (master_key, salt)
        };

        // Store state
        *self.master_key.write().await = Some(master_key);
        *self.salt.write().await = Some(salt);
        *self.client.write().await = Some(client);
        *self.index_manager.write().await = Some(index_manager.clone());
        *self.rate_limiter.write().await = Some(rate_limiter);

        // Start poller
        let poller = Arc::new(SyncPoller::new(
            self.db.clone(),
            index_manager,
            self.master_key.clone(),
            &self.device_id,
            self.app_handle.clone(),
        ));
        poller.start(poll_interval).await;
        *self.poller.write().await = Some(poller);

        *self.status.write().await = "connected".to_string();
        *self.last_sync_at.write().await = Some(Utc::now().to_rfc3339());
        info!("WebDAV sync connected to {}", server_url);
        Ok(())
    }

    pub async fn disconnect(&self) {
        if let Some(poller) = self.poller.read().await.as_ref() {
            poller.stop().await;
        }
        *self.poller.write().await = None;
        *self.client.write().await = None;
        *self.index_manager.write().await = None;
        *self.rate_limiter.write().await = None;
        *self.master_key.write().await = None;
        *self.salt.write().await = None;
        *self.status.write().await = "disconnected".to_string();
        *self.last_error.write().await = None;
        info!("WebDAV sync disconnected");
    }

    pub async fn push_entry(&self, entry: &ClipboardEntry) -> Result<(), String> {
        if !self.should_sync_entry(entry).await {
            return Ok(());
        }

        let master_key = {
            let guard = self.master_key.read().await;
            match guard.as_ref() {
                Some(k) => k.clone(),
                None => return Ok(()), // Not connected
            }
        };

        let index_manager = {
            let guard = self.index_manager.read().await;
            match guard.as_ref() {
                Some(im) => im.clone(),
                None => return Ok(()),
            }
        };

        let config = self.config.read().await;
        let max_cloud_entries = config.max_cloud_entries;
        drop(config);

        // Serialize entry as SyncEntryPayload
        let payload = SyncEntryPayload {
            content: entry.content.clone(),
            content_type: entry.content_type.clone(),
            category: entry.category.clone(),
            hash: entry.hash.clone(),
            source_app: entry.source_app.clone(),
            is_sensitive: entry.is_sensitive,
            source_device: self.device_id.clone(),
            created_at: entry.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
        };

        let json = serde_json::to_vec(&payload)
            .map_err(|e| format!("Failed to serialize entry: {e}"))?;

        // Upload entry file
        index_manager
            .upload_entry(&entry.hash, &json, &master_key)
            .await?;

        // Append to index
        let index_entry = IndexEntry {
            hash: entry.hash.clone(),
            content_type: entry.content_type.clone(),
            category: entry.category.clone(),
            source_device: self.device_id.clone(),
            created_at: entry.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
            is_sensitive: entry.is_sensitive,
            size_bytes: json.len(),
        };
        index_manager
            .append_entry(index_entry, &master_key)
            .await?;

        // Enforce entry limit
        if let Err(e) = index_manager
            .enforce_entry_limit(max_cloud_entries, &master_key)
            .await
        {
            warn!("Failed to enforce entry limit: {}", e);
        }

        *self.last_sync_at.write().await = Some(Utc::now().to_rfc3339());
        info!("Pushed entry {} to WebDAV", entry.hash);
        Ok(())
    }

    async fn should_sync_entry(&self, entry: &ClipboardEntry) -> bool {
        let config = self.config.read().await;
        if !config.enabled {
            return false;
        }
        if entry.is_sensitive && !config.sync_sensitive {
            return false;
        }
        if entry.content_type == "image" && !config.sync_images {
            return false;
        }
        if entry.content.len() > MAX_SYNC_PAYLOAD_BYTES {
            warn!(
                "Entry {} too large ({} bytes), skipping WebDAV sync",
                entry.hash,
                entry.content.len()
            );
            return false;
        }
        true
    }

    pub async fn trigger_sync(&self) -> Result<u32, String> {
        let poller = self.poller.read().await;
        match poller.as_ref() {
            Some(p) => p.poll_once().await,
            None => Err("Not connected".to_string()),
        }
    }

    pub async fn get_status(&self) -> WebDavSyncStatus {
        let status = self.status.read().await.clone();
        let last_sync_at = self.last_sync_at.read().await.clone();
        let last_error = self.last_error.read().await.clone();

        let (cloud_entry_count, registered_devices) = if let Some(ref im) =
            *self.index_manager.read().await
        {
            let master_key_guard = self.master_key.read().await;
            if let Some(ref key) = *master_key_guard {
                let count = im
                    .load_index(key)
                    .await
                    .map(|(idx, _)| idx.entries.len() as u32)
                    .unwrap_or(0);
                let devices = im
                    .load_device_registry(key)
                    .await
                    .map(|reg| reg.devices)
                    .unwrap_or_default();
                (count, devices)
            } else {
                (0, vec![])
            }
        } else {
            (0, vec![])
        };

        let (rate_available, rate_capacity) =
            if let Some(ref rl) = *self.rate_limiter.read().await {
                (rl.available(), rl.capacity())
            } else {
                (0, 0)
            };

        WebDavSyncStatus {
            status,
            last_sync_at,
            cloud_entry_count,
            registered_devices: registered_devices
                .into_iter()
                .map(|d| index::RegisteredDevice {
                    device_id: d.device_id,
                    device_name: d.device_name,
                    public_key: d.public_key,
                    registered_at: d.registered_at,
                    last_sync_at: d.last_sync_at,
                })
                .collect(),
            rate_limit_available: rate_available,
            rate_limit_capacity: rate_capacity,
            error: last_error,
        }
    }

    pub async fn update_config(&self, new_config: WebDavConfig) {
        let was_connected = *self.status.read().await == "connected";
        *self.config.write().await = new_config.clone();

        if was_connected {
            if let Some(ref poller) = *self.poller.read().await {
                poller.set_interval(new_config.poll_interval_secs).await;
            }
        }
    }

    pub async fn remove_device(&self, device_id: &str) -> Result<(), String> {
        let master_key = self
            .master_key
            .read()
            .await
            .clone()
            .ok_or("Not connected")?;
        let index_manager = self
            .index_manager
            .read()
            .await
            .clone()
            .ok_or("Not connected")?;
        index_manager
            .remove_device(device_id, &master_key)
            .await
    }
}
```

- [ ] **Step 2: Add `pub mod webdav;` to sync/mod.rs**

In `src-tauri/src/sync/mod.rs`, add after the existing module declarations:

```rust
pub mod webdav;
```

- [ ] **Step 3: Run cargo check**

Run: `cd src-tauri && cargo check 2>&1 | tail -10`
Expected: Compilation succeeds (possibly with warnings)

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/sync/webdav/ src-tauri/src/sync/mod.rs
git commit -m "feat(webdav): implement WebDavSyncManager orchestrator and wire webdav module"
```

---

### Task 8: Extend config.rs with WebDavConfig

**Files:**

- Modify: `src-tauri/src/config.rs`

- [ ] **Step 1: Add WebDavConfig to AppConfig**

In `src-tauri/src/config.rs`, add the `webdav` field to the `AppConfig` struct:

```rust
use crate::sync::webdav::WebDavConfig;
```

Add to the `AppConfig` struct:

```rust
    #[serde(default)]
    pub webdav: WebDavConfig,
```

Add to the `Default` impl for `AppConfig`:

```rust
    webdav: WebDavConfig::default(),
```

- [ ] **Step 2: Run cargo check**

Run: `cd src-tauri && cargo check 2>&1 | tail -5`
Expected: Compilation succeeds

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/config.rs
git commit -m "feat(webdav): add WebDavConfig to AppConfig"
```

---

### Task 9: Add Tauri IPC Commands for WebDAV

**Files:**

- Modify: `src-tauri/src/commands.rs`

- [ ] **Step 1: Add WebDAV command functions**

Add these commands at the end of `src-tauri/src/commands.rs`:

```rust
use crate::sync::webdav::{WebDavConfig, WebDavSyncStatus};

#[tauri::command]
pub async fn webdav_connect(
    server_url: String,
    username: String,
    password: String,
    sync_password: String,
    state: tauri::State<'_, Arc<crate::sync::webdav::WebDavSyncManager>>,
) -> Result<(), String> {
    state.connect(&server_url, &username, &password, &sync_password).await
}

#[tauri::command]
pub async fn webdav_disconnect(
    state: tauri::State<'_, Arc<crate::sync::webdav::WebDavSyncManager>>,
) -> Result<(), String> {
    state.disconnect().await;
    Ok(())
}

#[tauri::command]
pub async fn webdav_get_status(
    state: tauri::State<'_, Arc<crate::sync::webdav::WebDavSyncManager>>,
) -> Result<WebDavSyncStatus, String> {
    Ok(state.get_status().await)
}

#[tauri::command]
pub async fn webdav_trigger_sync(
    state: tauri::State<'_, Arc<crate::sync::webdav::WebDavSyncManager>>,
) -> Result<u32, String> {
    state.trigger_sync().await
}

#[tauri::command]
pub async fn webdav_update_config(
    config: WebDavConfig,
    state: tauri::State<'_, Arc<crate::sync::webdav::WebDavSyncManager>>,
) -> Result<(), String> {
    state.update_config(config).await;
    Ok(())
}

#[tauri::command]
pub async fn webdav_remove_device(
    device_id: String,
    state: tauri::State<'_, Arc<crate::sync::webdav::WebDavSyncManager>>,
) -> Result<(), String> {
    state.remove_device(&device_id).await
}

#[tauri::command]
pub async fn webdav_test_connection(
    server_url: String,
    username: String,
    password: String,
) -> Result<(), String> {
    use crate::sync::webdav::rate_limiter::TokenBucketLimiter;
    use crate::sync::webdav::client::WebDavClient;
    use std::sync::Arc;

    let limiter = Arc::new(TokenBucketLimiter::new(10, 60));
    let client = WebDavClient::new(&server_url, &username, &password, limiter)?;
    client.test_connection().await
}
```

- [ ] **Step 2: Run cargo check**

Run: `cd src-tauri && cargo check 2>&1 | tail -5`
Expected: Compilation succeeds

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/commands.rs
git commit -m "feat(webdav): add Tauri IPC commands for WebDAV sync"
```

---

### Task 10: Register Commands and Initialize WebDavSyncManager in lib.rs

**Files:**

- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Initialize WebDavSyncManager and register commands**

In `src-tauri/src/lib.rs`, add the WebDavSyncManager initialization after the existing SyncManager setup:

```rust
use sync::webdav::{WebDavConfig, WebDavSyncManager};
```

In the `run()` function, after the existing sync manager initialization, add:

```rust
    // Initialize WebDAV sync manager
    let webdav_config = config_manager.get().webdav.clone();
    let webdav_manager = WebDavSyncManager::new(
        db.clone(),
        webdav_config,
        &device_id,
        &device_name,
        &public_key,
    );
    webdav_manager.set_app_handle(app.handle().clone());
```

Add `webdav_manager` to the Tauri managed state:

```rust
    app.manage(webdav_manager);
```

Register the new commands in the `invoke_handler`:

```rust
    commands::webdav_connect,
    commands::webdav_disconnect,
    commands::webdav_get_status,
    commands::webdav_trigger_sync,
    commands::webdav_update_config,
    commands::webdav_remove_device,
    commands::webdav_test_connection,
```

- [ ] **Step 2: Run cargo check**

Run: `cd src-tauri && cargo check 2>&1 | tail -5`
Expected: Compilation succeeds

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat(webdav): register WebDAV commands and initialize manager in lib.rs"
```

---

### Task 11: Add Frontend TypeScript Types

**Files:**

- Modify: `src/types/index.ts`

- [ ] **Step 1: Add WebDAV types**

Add at the end of `src/types/index.ts`:

```typescript
// WebDAV Cloud Sync Types
export interface WebDavConfig {
  enabled: boolean;
  serverUrl: string;
  username: string;
  password: string;
  syncPassword: string;
  pollIntervalSecs: number;
  syncImages: boolean;
  syncSensitive: boolean;
  rateLimitCapacity: number;
  rateLimitRefillMinutes: number;
  remotePath: string;
  maxCloudEntries: number;
}

export interface WebDavDevice {
  deviceId: string;
  deviceName: string;
  publicKey: string;
  registeredAt: string;
  lastSyncAt: string | null;
}

export interface WebDavSyncStatus {
  status: 'disconnected' | 'connecting' | 'connected' | 'error';
  lastSyncAt: string | null;
  cloudEntryCount: number;
  registeredDevices: WebDavDevice[];
  rateLimitAvailable: number;
  rateLimitCapacity: number;
  error: string | null;
}
```

- [ ] **Step 2: Commit**

```bash
git add src/types/index.ts
git commit -m "feat(webdav): add WebDAV TypeScript types"
```

---

### Task 12: Add i18n Translations

**Files:**

- Modify: `src/i18n/locales/en.ts`
- Modify: `src/i18n/locales/zh-CN.ts`

- [ ] **Step 1: Add English translations**

Add inside the `sync` section of `src/i18n/locales/en.ts` (or create a new `webdav` section at the same level):

```typescript
  webdav: {
    title: 'Cloud Sync (WebDAV)',
    description: 'Sync clipboard across networks via WebDAV server',
    serverUrl: 'Server URL',
    serverUrlPlaceholder: 'https://dav.example.com/dav',
    username: 'Username',
    password: 'Password',
    syncPassword: 'Sync Encryption Password',
    syncPasswordHint: 'Used to encrypt all data on the server. All devices must use the same password.',
    connect: 'Connect',
    disconnect: 'Disconnect',
    connecting: 'Connecting...',
    connected: 'Connected',
    disconnected: 'Disconnected',
    testConnection: 'Test Connection',
    testSuccess: 'Connection successful!',
    testFailed: 'Connection failed',
    status: 'Status',
    lastSync: 'Last Sync',
    cloudEntries: 'Cloud Entries',
    registeredDevices: 'Registered Devices',
    rateLimit: 'API Rate Limit',
    rateLimitInfo: '{available} / {capacity} requests available',
    pollInterval: 'Poll Interval',
    pollIntervalUnit: 'seconds',
    syncImages: 'Sync Images',
    syncSensitive: 'Sync Sensitive Items',
    remotePath: 'Remote Path',
    maxCloudEntries: 'Max Cloud Entries',
    removeDevice: 'Remove',
    removeDeviceConfirm: 'Remove device "{name}" from cloud sync?',
    noDevices: 'No devices registered',
    error: 'Error',
    settings: 'Settings',
    advanced: 'Advanced Settings',
    rateLimitCapacity: 'Rate Limit (requests)',
    rateLimitRefillMinutes: 'Rate Limit Refill (minutes)',
    triggerSync: 'Sync Now',
    syncTriggered: 'Sync triggered, {count} new entries downloaded',
    tab: {
      lan: 'LAN Sync',
      webdav: 'Cloud Sync',
    },
  },
```

- [ ] **Step 2: Add Chinese translations**

Add the corresponding section in `src/i18n/locales/zh-CN.ts`:

```typescript
  webdav: {
    title: '云端同步 (WebDAV)',
    description: '通过 WebDAV 服务器跨网络同步剪贴板',
    serverUrl: '服务器地址',
    serverUrlPlaceholder: 'https://dav.example.com/dav',
    username: '用户名',
    password: '密码',
    syncPassword: '同步加密密码',
    syncPasswordHint: '用于加密服务器上的所有数据，所有设备必须使用相同的密码。',
    connect: '连接',
    disconnect: '断开',
    connecting: '连接中...',
    connected: '已连接',
    disconnected: '未连接',
    testConnection: '测试连接',
    testSuccess: '连接成功！',
    testFailed: '连接失败',
    status: '状态',
    lastSync: '上次同步',
    cloudEntries: '云端条目',
    registeredDevices: '已注册设备',
    rateLimit: 'API 速率限制',
    rateLimitInfo: '{available} / {capacity} 请求可用',
    pollInterval: '轮询间隔',
    pollIntervalUnit: '秒',
    syncImages: '同步图片',
    syncSensitive: '同步敏感内容',
    remotePath: '远程路径',
    maxCloudEntries: '最大云端条目数',
    removeDevice: '移除',
    removeDeviceConfirm: '确定从云端同步中移除设备 "{name}" 吗？',
    noDevices: '暂无已注册设备',
    error: '错误',
    settings: '设置',
    advanced: '高级设置',
    rateLimitCapacity: '速率限制（请求数）',
    rateLimitRefillMinutes: '速率限制恢复时间（分钟）',
    triggerSync: '立即同步',
    syncTriggered: '同步已触发，下载了 {count} 条新记录',
    tab: {
      lan: '局域网同步',
      webdav: '云端同步',
    },
  },
```

- [ ] **Step 3: Commit**

```bash
git add src/i18n/locales/en.ts src/i18n/locales/zh-CN.ts
git commit -m "feat(webdav): add i18n translations for WebDAV sync"
```

---

### Task 13: Create webdavStore.ts

**Files:**

- Create: `src/stores/webdavStore.ts`

- [ ] **Step 1: Create the Pinia store**

Create `src/stores/webdavStore.ts`:

```typescript
import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type { WebDavConfig, WebDavSyncStatus } from '@/types';

export const useWebDavStore = defineStore('webdav', () => {
  const status = ref<WebDavSyncStatus>({
    status: 'disconnected',
    lastSyncAt: null,
    cloudEntryCount: 0,
    registeredDevices: [],
    rateLimitAvailable: 0,
    rateLimitCapacity: 0,
    error: null,
  });

  const config = ref<WebDavConfig>({
    enabled: false,
    serverUrl: '',
    username: '',
    password: '',
    syncPassword: '',
    pollIntervalSecs: 30,
    syncImages: false,
    syncSensitive: false,
    rateLimitCapacity: 150,
    rateLimitRefillMinutes: 30,
    remotePath: '/SmartClipboard',
    maxCloudEntries: 2000,
  });

  const isConnected = computed(() => status.value.status === 'connected');
  const isConnecting = computed(() => status.value.status === 'connecting');
  const hasError = computed(() => status.value.status === 'error');

  let pollTimer: ReturnType<typeof setInterval> | null = null;

  async function connect(
    serverUrl: string,
    username: string,
    password: string,
    syncPassword: string,
  ) {
    try {
      await invoke('webdav_connect', {
        serverUrl,
        username,
        password,
        syncPassword,
      });
      await refreshStatus();
      startStatusPolling();
    } catch (error) {
      await refreshStatus();
      throw error;
    }
  }

  async function disconnect() {
    stopStatusPolling();
    await invoke('webdav_disconnect');
    await refreshStatus();
  }

  async function refreshStatus() {
    try {
      status.value = await invoke<WebDavSyncStatus>('webdav_get_status');
    } catch (error) {
      console.error('Failed to refresh WebDAV status:', error);
    }
  }

  async function triggerSync(): Promise<number> {
    const count = await invoke<number>('webdav_trigger_sync');
    await refreshStatus();
    return count;
  }

  async function updateConfig(newConfig: WebDavConfig) {
    config.value = newConfig;
    await invoke('webdav_update_config', { config: newConfig });
  }

  async function removeDevice(deviceId: string) {
    await invoke('webdav_remove_device', { deviceId });
    await refreshStatus();
  }

  async function testConnection(
    serverUrl: string,
    username: string,
    password: string,
  ): Promise<void> {
    await invoke('webdav_test_connection', {
      serverUrl,
      username,
      password,
    });
  }

  function startStatusPolling() {
    stopStatusPolling();
    pollTimer = setInterval(refreshStatus, 10000);
  }

  function stopStatusPolling() {
    if (pollTimer) {
      clearInterval(pollTimer);
      pollTimer = null;
    }
  }

  return {
    status,
    config,
    isConnected,
    isConnecting,
    hasError,
    connect,
    disconnect,
    refreshStatus,
    triggerSync,
    updateConfig,
    removeDevice,
    testConnection,
    startStatusPolling,
    stopStatusPolling,
  };
});
```

- [ ] **Step 2: Commit**

```bash
git add src/stores/webdavStore.ts
git commit -m "feat(webdav): create Pinia store for WebDAV sync"
```

---

### Task 14: Create WebDavPanel.vue and Update SyncPanel.vue

**Files:**

- Create: `src/components/WebDavPanel.vue`
- Modify: `src/components/SyncPanel.vue`

- [ ] **Step 1: Create WebDavPanel.vue**

Create `src/components/WebDavPanel.vue`:

```vue
<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { useWebDavStore } from '@/stores/webdavStore';

const { t } = useI18n();
const store = useWebDavStore();

const serverUrl = ref('');
const username = ref('');
const password = ref('');
const syncPassword = ref('');
const showAdvanced = ref(false);
const testLoading = ref(false);
const testResult = ref<{ success: boolean; message: string } | null>(null);
const connectLoading = ref(false);
const syncLoading = ref(false);

onMounted(async () => {
  await store.refreshStatus();
  if (store.isConnected) {
    store.startStatusPolling();
  }
});

onUnmounted(() => {
  store.stopStatusPolling();
});

async function handleTestConnection() {
  testLoading.value = true;
  testResult.value = null;
  try {
    await store.testConnection(serverUrl.value, username.value, password.value);
    testResult.value = { success: true, message: t('webdav.testSuccess') };
  } catch (error) {
    testResult.value = {
      success: false,
      message: `${t('webdav.testFailed')}: ${error}`,
    };
  } finally {
    testLoading.value = false;
  }
}

async function handleConnect() {
  connectLoading.value = true;
  try {
    await store.connect(serverUrl.value, username.value, password.value, syncPassword.value);
  } catch (error) {
    console.error('WebDAV connect failed:', error);
  } finally {
    connectLoading.value = false;
  }
}

async function handleDisconnect() {
  await store.disconnect();
}

async function handleTriggerSync() {
  syncLoading.value = true;
  try {
    const count = await store.triggerSync();
    console.log(`Synced ${count} entries`);
  } catch (error) {
    console.error('Sync failed:', error);
  } finally {
    syncLoading.value = false;
  }
}

async function handleRemoveDevice(deviceId: string, deviceName: string) {
  if (confirm(t('webdav.removeDeviceConfirm', { name: deviceName }))) {
    await store.removeDevice(deviceId);
  }
}
</script>

<template>
  <div class="webdav-panel space-y-4">
    <div class="text-sm text-muted-foreground">
      {{ t('webdav.description') }}
    </div>

    <!-- Connection Form (when disconnected) -->
    <div v-if="!store.isConnected" class="space-y-3">
      <div class="space-y-2">
        <label class="text-sm font-medium">{{ t('webdav.serverUrl') }}</label>
        <input
          v-model="serverUrl"
          type="url"
          :placeholder="t('webdav.serverUrlPlaceholder')"
          class="w-full rounded-md border px-3 py-2 text-sm"
        />
      </div>

      <div class="grid grid-cols-2 gap-3">
        <div class="space-y-2">
          <label class="text-sm font-medium">{{ t('webdav.username') }}</label>
          <input
            v-model="username"
            type="text"
            class="w-full rounded-md border px-3 py-2 text-sm"
          />
        </div>
        <div class="space-y-2">
          <label class="text-sm font-medium">{{ t('webdav.password') }}</label>
          <input
            v-model="password"
            type="password"
            class="w-full rounded-md border px-3 py-2 text-sm"
          />
        </div>
      </div>

      <div class="space-y-2">
        <label class="text-sm font-medium">{{ t('webdav.syncPassword') }}</label>
        <input
          v-model="syncPassword"
          type="password"
          class="w-full rounded-md border px-3 py-2 text-sm"
        />
        <p class="text-xs text-muted-foreground">{{ t('webdav.syncPasswordHint') }}</p>
      </div>

      <!-- Test result -->
      <div
        v-if="testResult"
        :class="[
          'rounded-md px-3 py-2 text-sm',
          testResult.success
            ? 'bg-green-50 text-green-700 dark:bg-green-900/20 dark:text-green-400'
            : 'bg-red-50 text-red-700 dark:bg-red-900/20 dark:text-red-400',
        ]"
      >
        {{ testResult.message }}
      </div>

      <!-- Error display -->
      <div
        v-if="store.hasError && store.status.error"
        class="rounded-md bg-red-50 px-3 py-2 text-sm text-red-700 dark:bg-red-900/20 dark:text-red-400"
      >
        {{ store.status.error }}
      </div>

      <div class="flex gap-2">
        <button
          @click="handleTestConnection"
          :disabled="testLoading || !serverUrl || !username || !password"
          class="rounded-md border px-4 py-2 text-sm hover:bg-accent disabled:opacity-50"
        >
          {{ testLoading ? '...' : t('webdav.testConnection') }}
        </button>
        <button
          @click="handleConnect"
          :disabled="
            connectLoading ||
            store.isConnecting ||
            !serverUrl ||
            !username ||
            !password ||
            !syncPassword
          "
          class="rounded-md bg-primary px-4 py-2 text-sm text-primary-foreground hover:bg-primary/90 disabled:opacity-50"
        >
          {{ connectLoading || store.isConnecting ? t('webdav.connecting') : t('webdav.connect') }}
        </button>
      </div>
    </div>

    <!-- Connected Status -->
    <div v-else class="space-y-4">
      <!-- Status Bar -->
      <div class="flex items-center justify-between rounded-md border p-3">
        <div class="flex items-center gap-2">
          <span class="h-2 w-2 rounded-full bg-green-500"></span>
          <span class="text-sm font-medium">{{ t('webdav.connected') }}</span>
        </div>
        <button
          @click="handleDisconnect"
          class="rounded-md border px-3 py-1 text-sm hover:bg-accent"
        >
          {{ t('webdav.disconnect') }}
        </button>
      </div>

      <!-- Stats -->
      <div class="grid grid-cols-2 gap-3 text-sm">
        <div class="rounded-md border p-3">
          <div class="text-muted-foreground">{{ t('webdav.cloudEntries') }}</div>
          <div class="text-lg font-semibold">{{ store.status.cloudEntryCount }}</div>
        </div>
        <div class="rounded-md border p-3">
          <div class="text-muted-foreground">{{ t('webdav.rateLimit') }}</div>
          <div class="text-lg font-semibold">
            {{ store.status.rateLimitAvailable }} / {{ store.status.rateLimitCapacity }}
          </div>
        </div>
      </div>

      <div v-if="store.status.lastSyncAt" class="text-xs text-muted-foreground">
        {{ t('webdav.lastSync') }}: {{ new Date(store.status.lastSyncAt).toLocaleString() }}
      </div>

      <!-- Sync Now Button -->
      <button
        @click="handleTriggerSync"
        :disabled="syncLoading"
        class="w-full rounded-md border px-4 py-2 text-sm hover:bg-accent disabled:opacity-50"
      >
        {{ syncLoading ? '...' : t('webdav.triggerSync') }}
      </button>

      <!-- Registered Devices -->
      <div class="space-y-2">
        <h4 class="text-sm font-medium">{{ t('webdav.registeredDevices') }}</h4>
        <div
          v-if="store.status.registeredDevices.length === 0"
          class="text-sm text-muted-foreground"
        >
          {{ t('webdav.noDevices') }}
        </div>
        <div
          v-for="device in store.status.registeredDevices"
          :key="device.deviceId"
          class="flex items-center justify-between rounded-md border p-2"
        >
          <div>
            <div class="text-sm font-medium">{{ device.deviceName }}</div>
            <div class="text-xs text-muted-foreground">
              {{ device.lastSyncAt ? new Date(device.lastSyncAt).toLocaleString() : '-' }}
            </div>
          </div>
          <button
            @click="handleRemoveDevice(device.deviceId, device.deviceName)"
            class="rounded px-2 py-1 text-xs text-red-600 hover:bg-red-50 dark:hover:bg-red-900/20"
          >
            {{ t('webdav.removeDevice') }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
```

- [ ] **Step 2: Update SyncPanel.vue to add tab switcher**

In `src/components/SyncPanel.vue`, wrap the existing content in a tab system. Add a tab bar at the top with "LAN Sync" and "Cloud Sync" tabs:

```vue
<script setup lang="ts">
// Add to existing imports:
import { ref } from 'vue';
import WebDavPanel from './WebDavPanel.vue';

const activeTab = ref<'lan' | 'webdav'>('lan');
</script>

<template>
  <div class="sync-panel">
    <!-- Tab Switcher -->
    <div class="mb-4 flex border-b">
      <button
        @click="activeTab = 'lan'"
        :class="[
          'px-4 py-2 text-sm font-medium border-b-2 -mb-px',
          activeTab === 'lan'
            ? 'border-primary text-primary'
            : 'border-transparent text-muted-foreground hover:text-foreground',
        ]"
      >
        {{ $t('webdav.tab.lan') }}
      </button>
      <button
        @click="activeTab = 'webdav'"
        :class="[
          'px-4 py-2 text-sm font-medium border-b-2 -mb-px',
          activeTab === 'webdav'
            ? 'border-primary text-primary'
            : 'border-transparent text-muted-foreground hover:text-foreground',
        ]"
      >
        {{ $t('webdav.tab.webdav') }}
      </button>
    </div>

    <!-- Tab Content -->
    <div v-if="activeTab === 'lan'">
      <!-- Existing LAN sync content goes here -->
    </div>
    <div v-else>
      <WebDavPanel />
    </div>
  </div>
</template>
```

Note: The exact integration depends on the current SyncPanel.vue structure. The key changes are:

1. Import `WebDavPanel` component
2. Add `activeTab` ref
3. Wrap existing content in `v-if="activeTab === 'lan'"` block
4. Add `WebDavPanel` in `v-else` block
5. Add tab switcher UI at the top

- [ ] **Step 3: Commit**

```bash
git add src/components/WebDavPanel.vue src/components/SyncPanel.vue
git commit -m "feat(webdav): create WebDavPanel component and add tab switcher to SyncPanel"
```

---

## Self-Review Checklist

1. **Spec coverage:** All requirements from the design spec are covered:
   - ✅ WebDAV HTTP client with PUT/GET/MKCOL/DELETE/HEAD
   - ✅ Token bucket rate limiter (Jianguoyun compatible)
   - ✅ Argon2id password-derived master key
   - ✅ AES-256-GCM file encryption with version header
   - ✅ Encrypted index file with ETag optimistic locking
   - ✅ Device registry with registration/removal
   - ✅ Push-pull sync (immediate upload + configurable polling)
   - ✅ WebDavSyncManager orchestrator
   - ✅ 7 Tauri IPC commands
   - ✅ Frontend Pinia store + WebDavPanel UI
   - ✅ i18n (en + zh-CN)
   - ✅ Tab switcher in SyncPanel (LAN / WebDAV)

2. **Placeholder scan:** No TBD, TODO, or incomplete sections.

3. **Type consistency:** Types match across tasks — `WebDavConfig`, `WebDavSyncStatus`, `IndexEntry`, `RegisteredDevice` are consistent between Rust and TypeScript definitions.

4. **Task ordering:** Dependencies are respected — Cargo deps (T1) → crypto (T2) → rate limiter (T3) → client (T4) → index (T5) → poller (T6) → orchestrator (T7) → config (T8) → commands (T9) → lib.rs (T10) → TS types (T11) → i18n (T12) → store (T13) → UI (T14).
