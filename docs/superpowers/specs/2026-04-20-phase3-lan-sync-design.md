# Phase 3: LAN Sync (mDNS + WebSocket) Design Spec

## Overview

Enable real-time clipboard synchronization between Smart Clipboard instances on the same local network. Devices discover each other via mDNS/DNS-SD, establish WebSocket connections, and exchange clipboard entries with end-to-end AES-256-GCM encryption.

## Architecture

```
Device A                              Device B
┌─────────────────┐                  ┌─────────────────┐
│ ClipboardMonitor│                  │ ClipboardMonitor│
│       ↓         │                  │       ↓         │
│ SyncManager     │◄── WebSocket ──►│ SyncManager     │
│  ├─ mDNS       │    (encrypted)   │  ├─ mDNS       │
│  ├─ WsServer   │                  │  ├─ WsServer   │
│  ├─ WsClient   │                  │  ├─ WsClient   │
│  └─ Crypto     │                  │  └─ Crypto     │
└─────────────────┘                  └─────────────────┘
```

## Device Discovery (mDNS)

- Service type: `_smartclip._tcp.local.`
- Each instance advertises itself with:
  - Instance name: `<device_name>._smartclip._tcp.local.`
  - TXT records: `device_id=<uuid>`, `version=1`
  - Port: configurable (default 23456)
- On discovery, devices appear in the "Devices" panel
- Pairing: devices must be mutually approved before syncing

## Sync Protocol (WebSocket)

### Connection Flow

1. Device A discovers Device B via mDNS
2. User on A clicks "Pair" → sends pairing request
3. User on B approves → shared secret exchanged (ECDH key agreement)
4. WebSocket connection established with message-level encryption

### Message Types

```rust
enum SyncMessage {
    // Pairing
    PairRequest { device_id: String, device_name: String, public_key: Vec<u8> },
    PairResponse { device_id: String, accepted: bool, public_key: Vec<u8> },

    // Sync
    ClipboardSync { entry: EncryptedEntry, timestamp: i64 },
    SyncAck { entry_hash: String },

    // Control
    Ping,
    Pong,
    Disconnect { reason: String },
}
```

### Encrypted Entry Format

```rust
struct EncryptedEntry {
    nonce: [u8; 12],        // AES-GCM nonce
    ciphertext: Vec<u8>,    // Encrypted JSON of ClipboardEntry
    hash: String,           // Content hash for dedup
}
```

## Encryption (E2E)

- **Key Exchange**: X25519 ECDH to derive shared secret
- **Encryption**: AES-256-GCM with random 12-byte nonce per message
- **Key Derivation**: HKDF-SHA256 from ECDH shared secret
- **Key Storage**: Paired device keys stored locally (encrypted with device master key)

## Data Model

### SQLite Schema Additions

```sql
-- Paired devices
CREATE TABLE paired_devices (
    id TEXT PRIMARY KEY,              -- UUID
    name TEXT NOT NULL,               -- Human-readable name
    public_key BLOB NOT NULL,        -- X25519 public key
    shared_secret BLOB,              -- Derived shared secret (encrypted)
    last_seen_at TEXT,               -- Last mDNS/connection time
    is_active INTEGER DEFAULT 1,     -- Whether to sync with this device
    paired_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Sync log for conflict resolution
CREATE TABLE sync_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    entry_hash TEXT NOT NULL,
    device_id TEXT NOT NULL,
    direction TEXT NOT NULL,          -- 'sent' or 'received'
    synced_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (device_id) REFERENCES paired_devices(id)
);

CREATE INDEX idx_sync_log_hash ON sync_log(entry_hash);
CREATE INDEX idx_sync_log_device ON sync_log(device_id);
```

## Backend (Rust)

### New Module: `src-tauri/src/sync/`

**`mod.rs`** - Module exports and SyncManager

**`mdns.rs`** - mDNS service advertisement and discovery

- `advertise(device_name, port)` - Register mDNS service
- `discover() -> Vec<DiscoveredDevice>` - Scan for peers
- Background task: continuous discovery with periodic refresh

**`server.rs`** - WebSocket server (listens for incoming connections)

- Accepts connections from paired devices
- Validates device identity via stored public key
- Routes messages to SyncManager

**`client.rs`** - WebSocket client (connects to discovered peers)

- Connects to paired devices
- Reconnects on disconnect with exponential backoff

**`crypto.rs`** - Encryption utilities

- `generate_keypair() -> (PrivateKey, PublicKey)`
- `derive_shared_secret(private_key, peer_public_key) -> SharedSecret`
- `encrypt( &[u8], shared_secret) -> EncryptedEntry`
- `decrypt(encrypted: &EncryptedEntry, shared_secret) -> Vec<u8>`

**`protocol.rs`** - Message serialization/deserialization

- SyncMessage enum with serde Serialize/Deserialize

**`commands.rs`** - Tauri IPC commands

- `get_sync_status() -> SyncStatus`
- `get_discovered_devices() -> Vec<DiscoveredDevice>`
- `get_paired_devices() -> Vec<PairedDevice>`
- `pair_device(device_id) -> Result<()>`
- `accept_pair_request(device_id, accept: bool) -> Result<()>`
- `unpair_device(device_id) -> Result<()>`
- `toggle_sync(enabled: bool) -> Result<()>`
- `get_sync_config() -> SyncConfig`
- `update_sync_config(config: SyncConfig) -> Result<()>`

### Rust Dependencies (new)

```toml
# mDNS
mdns-sd = "0.11"

# WebSocket
tokio-tungstenite = "0.26"
futures-util = "0.3"

# Crypto
x25519-dalek = "2"
aes-gcm = "0.10"
hkdf = "0.12"
rand = "0.8"
```

## Frontend (Vue 3)

### Components

**`SyncPanel.vue`** - Main sync management panel

- Sync toggle (enable/disable)
- Device name display/edit
- Paired devices list
- Discovered devices list with "Pair" buttons

**`DeviceCard.vue`** - Single device display

- Device name, status indicator (online/offline)
- Last seen timestamp
- Unpair button
- Sync active toggle

**`PairRequestDialog.vue`** - Incoming pair request notification

- Shows requesting device name
- Accept / Reject buttons
- Auto-dismiss after 60 seconds (reject)

### Store: `src/stores/syncStore.ts`

```typescript
interface SyncState {
  enabled: boolean;
  deviceName: string;
  pairedDevices: PairedDevice[];
  discoveredDevices: DiscoveredDevice[];
  pendingRequests: PairRequest[];
  syncStatus: 'idle' | 'syncing' | 'error';
}
```

### Navigation

Add "Sync" entry to top bar (between Templates and Statistics). Use `Wifi` icon from lucide-vue-next.

### i18n Keys

- `sync.title`, `sync.enable`, `sync.disable`, `sync.status`
- `sync.devices`, `sync.discovered`, `sync.paired`, `sync.pair`, `sync.unpair`
- `sync.pair_request`, `sync.accept`, `sync.reject`, `sync.pair_request_from`
- `sync.device_name`, `sync.last_seen`, `sync.online`, `sync.offline`
- `sync.no_devices`, `sync.scanning`, `sync.encrypted`

## Configuration

Add to existing config:

```rust
pub struct SyncConfig {
    pub enabled: bool,              // Default: false
    pub device_name: String,        // Default: hostname
    pub port: u16,                  // Default: 23456
    pub auto_sync: bool,            // Default: true (sync new entries automatically)
    pub sync_images: bool,          // Default: false (images can be large)
    pub sync_sensitive: bool,       // Default: false (don't sync sensitive content)
}
```

## Sync Behavior

- Only sync entries created AFTER pairing (no history dump)
- Skip entries marked `is_sensitive` unless `sync_sensitive` is enabled
- Skip image entries unless `sync_images` is enabled
- Dedup by content hash on receiving side
- Synced entries marked with `source_device` in metadata
- Conflict resolution: both sides keep their version, latest timestamp wins for display order

## Event Flow

1. New clipboard entry created locally
2. SyncManager checks: is sync enabled? Any paired+online devices?
3. For each paired device: encrypt entry → send via WebSocket
4. Receiving device: decrypt → dedup check → insert with source_device tag
5. Frontend receives `clipboard-changed` event as normal

## Constraints

- Max sync payload: 1MB per entry (skip larger content)
- WebSocket ping/pong every 30s to detect disconnection
- mDNS refresh every 10s
- Maximum 10 paired devices
- Reconnect backoff: 1s, 2s, 4s, 8s, 16s, 30s (max)
