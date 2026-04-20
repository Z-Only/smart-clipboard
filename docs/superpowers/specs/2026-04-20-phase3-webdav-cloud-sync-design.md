# Phase 3: WebDAV Cloud Relay Sync Design Spec

## Overview

Add WebDAV-based cloud relay synchronization to Smart Clipboard, enabling clipboard sync between devices that are **not on the same local network**. WebDAV sync operates as a **fully independent channel** alongside the existing LAN sync (mDNS + WebSocket) — users can enable either or both.

Compatible with any standard WebDAV server: Jianguoyun (坚果云), Nextcloud, Synology, ownCloud, etc.

## Architecture

```
Device A                                    Device B
┌──────────────────┐                       ┌──────────────────┐
│ ClipboardMonitor │                       │ ClipboardMonitor │
│       ↓          │                       │       ↓          │
│ SyncManager      │                       │ SyncManager      │
│  ├─ LAN Sync     │                       │  ├─ LAN Sync     │
│  │  (existing)   │                       │  │  (existing)   │
│  │               │                       │  │               │
│  └─ WebDAV Sync  │                       │  └─ WebDAV Sync  │
│     ├─ Client    │──── HTTP PUT/GET ────►│     ├─ Client    │
│     ├─ Index Mgr │                       │     ├─ Index Mgr │
│     ├─ Poller    │                       │     ├─ Poller    │
│     └─ RateLimiter                       │     └─ RateLimiter
└──────────────────┘                       └──────────────────┘
            │                                       │
            ▼                                       ▼
┌──────────────────────────────────────────────────────────┐
│              WebDAV Server (坚果云 / Nextcloud / 群晖)    │
│                                                          │
│  /SmartClipboard/                                        │
│  ├── meta/                                               │
│  │   ├── index.enc       ← Encrypted index file          │
│  │   └── devices.enc     ← Encrypted device registry     │
│  └── entries/                                            │
│      ├── a1b2c3d4e5f6.enc  ← Encrypted entry files      │
│      ├── f7e8d9c0b1a2.enc                                │
│      └── ...                                             │
└──────────────────────────────────────────────────────────┘
```

## Relationship to LAN Sync

- **Fully independent**: WebDAV sync does not depend on mDNS, WebSocket, or any LAN sync component
- **Can coexist**: Users may enable LAN sync, WebDAV sync, or both simultaneously
- **Shared infrastructure**: Reuses `crypto.rs` (AES-256-GCM) with new password-derived key functions
- **Unified dispatch**: `SyncManager.broadcast_entry()` fans out to both channels

## Sync Trigger Mechanism

- **Push**: New local clipboard entries are uploaded to WebDAV immediately
- **Pull**: A configurable poller (default 30s, range 5s–300s) fetches the index file to discover new entries from other devices
- **Manual**: Users can trigger a sync manually via the UI

## WebDAV Data Layout

### Directory Structure

```
/SmartClipboard/                    ← Root directory (configurable)
├── meta/
│   ├── index.enc                   ← Encrypted index (all entry metadata)
│   └── devices.enc                 ← Encrypted device registry
└── entries/
    ├── <hash-prefix-12>.enc        ← Encrypted entry files (named by first 12 chars of content hash)
    └── ...
```

### Index File Format (decrypted)

```json
{
  "version": 1,
  "updated_at": "2026-04-20T20:30:00Z",
  "updated_by": "device-uuid-1",
  "entries": [
    {
      "hash": "a1b2c3d4e5f6...",
      "content_type": "text",
      "category": "url",
      "source_device": "device-uuid-1",
      "created_at": "2026-04-20T20:25:00Z",
      "is_sensitive": false,
      "size_bytes": 256
    }
  ]
}
```

**Design decisions**:
- Index file is the **sole discovery mechanism** — polling reads only `index.enc` (one HTTP GET), then compares against local known-hash set to find new entries
- **No PROPFIND dependency** — avoids directory listing entirely, maximizing WebDAV compatibility
- **Concurrent write conflicts** handled via **read-modify-write + ETag optimistic locking**: GET index → merge new entry → PUT with `If-Match: <etag>` → retry on 412 (max 3 attempts)
- **Entry files are immutable** — once uploaded, never modified; only the index file is updated

### Device Registry Format (decrypted)

```json
{
  "version": 1,
  "salt": "<base64-encoded 16-byte Argon2 salt>",
  "devices": [
    {
      "device_id": "uuid-1",
      "device_name": "MacBook Pro",
      "public_key": "<base64-encoded X25519 public key>",
      "registered_at": "2026-04-20T20:00:00Z",
      "last_sync_at": "2026-04-20T20:30:00Z"
    }
  ]
}
```

### Device Registration Flow

1. User enters WebDAV URL + credentials + sync password on a new device
2. Derive master key from sync password via Argon2id
3. Attempt to GET and decrypt `devices.enc`
4. If decryption succeeds → password correct → append this device's X25519 public key to device list → PUT updated `devices.enc`
5. If `devices.enc` does not exist → first-time setup → create directory structure (`MKCOL`), initialize empty index and device registry

## Encryption

### Key Hierarchy

```
User sync password (plaintext)
       │
       ▼  Argon2id (salt stored in devices.enc header)
   Master Key (32 bytes)
       │
       ├──→ Encrypt/decrypt index.enc
       ├──→ Encrypt/decrypt devices.enc
       └──→ Encrypt/decrypt entries/*.enc
```

### Password Derivation

- **Algorithm**: Argon2id (resistant to GPU/ASIC brute force, superior to PBKDF2)
- **Parameters**: `m=65536 (64MB), t=3, p=4` (OWASP recommended)
- **Salt**: Random 16 bytes, stored in `devices.enc` file header (plaintext — standard practice)

### Data Encryption

- Reuses existing `crypto.rs` **AES-256-GCM** implementation
- Each file gets an independent random 12-byte nonce
- **File format**: `[4 bytes version][16 bytes salt][12 bytes nonce][ciphertext][16 bytes auth tag]`
  - Salt field is only meaningful in `devices.enc`; other files use zero-filled salt (key already in memory)

### Device Identity

- Reuses existing X25519 keypair from LAN sync
- Public key written to `devices.enc` on registration
- MVP purpose: device identification only; future: device-level access control

### Extensions to crypto.rs

New functions added to existing `src-tauri/src/sync/crypto.rs` (no modifications to existing functions):

```rust
pub fn derive_key_from_password(password: &str, salt: &[u8]) -> Result<Vec<u8>, String>;
pub fn generate_salt() -> Vec<u8>;
pub fn encrypt_file(plaintext: &[u8], master_key: &[u8]) -> Result<Vec<u8>, String>;
pub fn decrypt_file(file_bytes: &[u8], master_key: &[u8]) -> Result<Vec<u8>, String>;
```

## Data Flow

### Upload (Push)

```
New local entry → SyncManager.broadcast_entry()
                       │
                       ├── LAN sync (existing): WebSocket push
                       │
                       └── WebDAV sync (new):
                           1. Check: WebDAV enabled? Master key available?
                           2. Check: Should sync? (sensitive/image/size filters)
                           3. Serialize entry → JSON → AES-256-GCM encrypt
                           4. RateLimiter.acquire(2)  ← 2 tokens (PUT entry + PUT index)
                           5. PUT /entries/<hash>.enc
                           6. GET index.enc → decrypt → append entry metadata → encrypt → PUT index.enc (If-Match)
                           7. On 412 conflict → retry read-modify-write (max 3)
```

### Download (Poll)

```
Poller (every N seconds)
    │
    1. RateLimiter.acquire(1)
    2. GET /meta/index.enc → decrypt
    3. Compare against local known-hash set → identify new entries
    4. For each new entry:
       a. RateLimiter.acquire(1)
       b. GET /entries/<hash>.enc → decrypt → deserialize
       c. Dedup check (local DB hash lookup)
       d. Insert into local DB with source_device tag
       e. emit("clipboard-changed") to notify frontend
    5. Update local "last_sync_at" timestamp
```

### Sync Filters

- Skip entries marked `is_sensitive` unless `sync_sensitive` is enabled
- Skip image entries unless `sync_images` is enabled
- Skip entries larger than 1MB
- Only sync entries created AFTER WebDAV sync was first enabled (no history dump)

## Token Bucket Rate Limiter

```
Configuration:
  - Default capacity: 150 tokens
  - Refill rate: 150 tokens / 30 min (坚果云 limit: 500 req/30min, with safety margin)
  - Each PUT/GET consumes 1 token
  - Each MKCOL consumes 1 token

Behavior:
  - When tokens insufficient, requests queue and wait (no drops)
  - Upload priority > poll priority (ensure local entries upload promptly)
  - User-configurable capacity and refill period (adapt to different WebDAV providers)
```

## Conflict Resolution

- **Index file conflicts**: ETag optimistic locking + read-modify-write retry (max 3)
- **Entry file conflicts**: Impossible (filename = content hash, same content)
- **Same entry from multiple devices**: Hash identical → natural dedup, single file
- **Concurrent uploads**: Index ETag conflict → retry merges both entries

## Backend (Rust)

### New Module: `src-tauri/src/sync/webdav/`

**`mod.rs`** — WebDavSyncManager
- Lifecycle management (start/stop/connect/disconnect)
- Push entry dispatch
- Holds master key in memory (`RwLock<Option<Vec<u8>>>`)

**`client.rs`** — WebDavClient
- HTTP operations via `reqwest`: PUT, GET, MKCOL, DELETE
- ETag-aware PUT (`If-Match` header)
- `exists()` via HEAD request
- `ensure_directory_structure()` — creates `/SmartClipboard/meta/` and `/entries/`
- Connection test endpoint

**`index.rs`** — IndexManager
- Read/write/merge encrypted index file
- Read/write encrypted device registry
- ETag-based conflict retry logic
- Device registration flow

**`poller.rs`** — SyncPoller
- Configurable interval timer (5s–300s)
- Calls IndexManager to discover new entries
- Downloads and inserts new entries into local DB
- Respects rate limiter

**`rate_limiter.rs`** — TokenBucketLimiter
- Async `acquire(count)` — blocks until tokens available
- `try_acquire(count)` — non-blocking attempt
- `available()` — current token count
- Configurable capacity and refill rate

### New Tauri IPC Commands

Added to `src-tauri/src/commands.rs`:

```rust
// WebDAV configuration
pub async fn get_webdav_config() -> Result<WebDavConfig, String>;
pub async fn update_webdav_config(config: WebDavConfig) -> Result<(), String>;
pub async fn test_webdav_connection(url: String, username: String, password: String) -> Result<(), String>;

// WebDAV sync control
pub async fn connect_webdav(url: String, username: String, password: String, sync_password: String) -> Result<(), String>;
pub async fn disconnect_webdav() -> Result<(), String>;
pub async fn get_webdav_status() -> Result<WebDavSyncStatus, String>;
pub async fn trigger_webdav_sync() -> Result<(), String>;
```

### Configuration

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebDavConfig {
    pub enabled: bool,                    // Default: false
    pub server_url: String,               // WebDAV server URL
    pub username: String,                  // WebDAV username
    pub password: String,                  // WebDAV password (encrypted at rest)
    pub sync_password: String,             // E2E encryption password (encrypted at rest)
    pub poll_interval_secs: u64,           // Default: 30 (range: 5–300)
    pub sync_images: bool,                 // Default: false
    pub sync_sensitive: bool,              // Default: false
    pub rate_limit_capacity: u32,          // Default: 150
    pub rate_limit_refill_minutes: u32,    // Default: 30
    pub remote_path: String,               // Default: "/SmartClipboard"
    pub max_cloud_entries: u32,            // Default: 2000
}
```

### New Cargo Dependencies

```toml
reqwest = { version = "0.12", features = ["rustls-tls"] }  # HTTP client for WebDAV
argon2 = "0.5"                                              # Password key derivation
```

## Frontend (Vue 3)

### SyncPanel Tab Extension

The existing `SyncPanel.vue` gains a **tab switcher** with two tabs:
- **局域网同步** (LAN Sync) — existing content
- **☁️ WebDAV 云同步** (WebDAV Cloud Sync) — new content

### WebDAV Tab Layout

```
┌─ WebDAV 连接 ──────────────────────────────────┐
│  启用 WebDAV 同步              [  开关  ]       │
│                                                │
│  服务器地址  [https://dav.jianguoyun.com/dav/]  │
│  用户名      [user@example.com              ]  │
│  密码        [••••••••••                    ]  │
│  同步密码    [••••••••••    ] ← E2E 加密密码   │
│  远程目录    [/SmartClipboard               ]  │
│                                                │
│  [测试连接]                                     │
└────────────────────────────────────────────────┘

┌─ 同步选项 ─────────────────────────────────────┐
│  轮询间隔    [30] 秒  (5-300)                   │
│  同步图片    [  关  ]                           │
│  同步敏感内容 [  关  ]                           │
└────────────────────────────────────────────────┘

┌─ 状态 ────────────────────────────────────────┐
│  状态: 🟢 已连接  ·  上次同步: 2 分钟前         │
│  已注册设备: 3  ·  云端条目: 1,234              │
│  限流: 142/150 tokens                          │
└────────────────────────────────────────────────┘

┌─ 已注册设备 ──────────────────────────────────┐
│  📱 MacBook Pro (本机)    上次同步: 刚刚        │
│  📱 iPhone 15             上次同步: 5 分钟前    │
│  📱 Windows Desktop       上次同步: 1 小时前    │
└────────────────────────────────────────────────┘
```

### New Store: `src/stores/webdavStore.ts`

```typescript
interface WebDavState {
  enabled: boolean;
  serverUrl: string;
  username: string;
  remoteDir: string;
  pollIntervalSecs: number;
  syncImages: boolean;
  syncSensitive: boolean;
  status: 'disconnected' | 'connecting' | 'connected' | 'syncing' | 'error';
  lastSyncAt: string | null;
  cloudEntryCount: number;
  registeredDevices: WebDavDevice[];
  rateLimitAvailable: number;
  rateLimitCapacity: number;
  error: string | null;
}
```

### Type Extensions (`src/types/index.ts`)

```typescript
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
  remoteDir: string;
  maxCloudEntries: number;
}

export interface WebDavDevice {
  deviceId: string;
  deviceName: string;
  registeredAt: string;
  lastSyncAt: string | null;
}

export type WebDavSyncStatusType = 'disconnected' | 'connecting' | 'connected' | 'syncing' | 'error';

export interface WebDavSyncStatus {
  status: WebDavSyncStatusType;
  lastSyncAt: string | null;
  cloudEntryCount: number;
  registeredDevices: WebDavDevice[];
  rateLimitAvailable: number;
  rateLimitCapacity: number;
  error: string | null;
}
```

### i18n Keys

```
webdav.title, webdav.subtitle
webdav.serverUrl, webdav.username, webdav.password, webdav.syncPassword
webdav.remoteDir, webdav.pollInterval
webdav.testConnection, webdav.testSuccess, webdav.testFailed
webdav.syncImages, webdav.syncSensitive
webdav.status.disconnected, webdav.status.connecting, webdav.status.connected
webdav.status.syncing, webdav.status.error
webdav.cloudEntries, webdav.registeredDevices, webdav.rateLimit
webdav.lastSync, webdav.triggerSync
webdav.tabs.lan, webdav.tabs.webdav
```

## Error Handling

| Scenario | Handling |
|----------|----------|
| WebDAV connection failure | Show error in UI, auto-retry with exponential backoff (5s/10s/30s/60s), does not affect LAN sync |
| Authentication failure (401/403) | Stop sync, prompt user to check credentials, no auto-retry |
| Wrong sync password | `devices.enc` decryption fails → show "Incorrect sync password" |
| Index ETag conflict (412) | Auto read-modify-write retry, max 3 attempts, defer to next poll on failure |
| Token exhaustion | Requests queue and wait for refill, UI shows "Rate limited, waiting N seconds" |
| Network timeout | 30s per-request timeout, poll-cycle failures don't block next cycle |
| Entry too large (>1MB) | Skip entry, log warning, other entries unaffected |
| Server disk full (507) | Catch error, pause uploads, prompt user to free space |
| Local DB vs cloud inconsistency | Content-hash dedup is authoritative; no full reconciliation in MVP |

## Security

- **Passwords not stored in plaintext**: WebDAV password and sync password encrypted at rest in config.json (AES with device-specific key or OS keychain)
- **Master key memory-only**: Argon2-derived master key exists only in runtime memory, never persisted
- **Sensitive entries excluded by default**: `sync_sensitive` defaults to false
- **Images excluded by default**: `sync_images` defaults to false (images can be large)
- **Server sees only ciphertext**: All files on WebDAV are E2E encrypted

## Capacity Management

- **Cloud entry limit**: Default 2000 entries (configurable via `max_cloud_entries`)
- **Index file size**: 2000 entries × ~150 bytes/entry ≈ 300KB encrypted — acceptable per-poll transfer
- **Cleanup strategy**: On upload, check cloud entry count; if over limit, delete oldest entry files (FIFO) and update index
- **Entry file size limit**: 1MB per entry (consistent with LAN sync)

## Constraints

- Max sync payload: 1MB per entry (skip larger content)
- Poll interval range: 5s–300s
- ETag retry: max 3 attempts per index update
- Rate limiter default: 150 tokens / 30 minutes
- Connection timeout: 30s per HTTP request
- Reconnect backoff: 5s, 10s, 30s, 60s (max)
