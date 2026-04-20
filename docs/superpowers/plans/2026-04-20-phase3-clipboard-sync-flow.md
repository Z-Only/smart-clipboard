# Clipboard Sync Flow Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the complete clipboard sync business flow — when a new clipboard entry is created locally, it gets filtered, encrypted, sent to paired devices; receiving devices decrypt, deduplicate, store, and trigger frontend refresh.

**Architecture:** The sync pipeline hooks into the existing clipboard monitor → DB insert flow in `lib.rs`. After a successful local insert, the entry is passed to `SyncManager::broadcast_entry()` which checks sync config filters, serializes the entry, encrypts it per-device, and sends via existing WebSocket connections. On the receiving side, the real `ClipboardEntry` JSON is extracted from the new `ClipboardSync` message, deduplicated by hash + sync_log, inserted into the local DB with `source_device` metadata, and a `clipboard-changed` event is emitted to refresh the frontend.

**Tech Stack:** Rust (Tauri backend), SQLite (rusqlite), AES-256-GCM encryption, WebSocket (tokio-tungstenite), serde JSON serialization

---

### Task 1: Add `source_device` Column and Sync Log DB Methods

**Files:**
- Modify: `src-tauri/src/storage/migrations.rs`
- Modify: `src-tauri/src/storage/models.rs`
- Modify: `src-tauri/src/storage/database.rs`
- Modify: `src-tauri/src/storage/mod.rs`

- [ ] **Step 1: Add `source_device` column migration**

In `src-tauri/src/storage/migrations.rs`, after the existing `paired_columns` migration block (around line 140), add a migration to add `source_device` column to `clipboard_entries`:

```rust
// Add source_device column for sync origin tracking
let entry_columns: Vec<String> = {
    let mut stmt = conn.prepare("PRAGMA table_info(clipboard_entries)")?;
    let columns = stmt.query_map([], |row| row.get::<_, String>(1))?
        .filter_map(|r| r.ok())
        .collect();
    columns
};

if !entry_columns.iter().any(|c| c == "source_device") {
    conn.execute_batch(
        "ALTER TABLE clipboard_entries ADD COLUMN source_device TEXT;"
    )?;
}
```

- [ ] **Step 2: Add `source_device` to `ClipboardEntry` model**

In `src-tauri/src/storage/models.rs`, add to the `ClipboardEntry` struct:

```rust
pub struct ClipboardEntry {
    // ... existing fields ...
    pub expires_at: Option<NaiveDateTime>,
    pub source_device: Option<String>,  // NEW: device_id of origin, None = local
}
```

- [ ] **Step 3: Add `SyncLogEntry` model**

In `src-tauri/src/storage/models.rs`, add:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncLogEntry {
    pub id: Option<i64>,
    pub entry_hash: String,
    pub device_id: String,
    pub direction: String,  // "sent" or "received"
    pub synced_at: NaiveDateTime,
}
```

- [ ] **Step 4: Update `Database` to read/write `source_device`**

In `src-tauri/src/storage/database.rs`, update `insert_entry` to include `source_device`:

```rust
pub fn insert_entry(&self, entry: &ClipboardEntry) -> Result<i64> {
    let conn = self.conn.lock().unwrap();
    conn.execute(
        "INSERT INTO clipboard_entries (content, content_type, category, hash, source_app, is_favorite, is_sensitive, use_count, created_at, updated_at, expires_at, source_device)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            entry.content,
            entry.content_type,
            entry.category,
            entry.hash,
            entry.source_app,
            entry.is_favorite as i32,
            entry.is_sensitive as i32,
            entry.use_count,
            entry.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
            entry.updated_at.format("%Y-%m-%d %H:%M:%S").to_string(),
            entry.expires_at.map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string()),
            entry.source_device,
        ],
    )?;
    Ok(conn.last_insert_rowid())
}
```

Update ALL `row_to_entry` and query functions that read from `clipboard_entries` to include `source_device` (column index 12). The `row_to_entry` helper should read:

```rust
fn row_to_entry(row: &rusqlite::Row) -> rusqlite::Result<ClipboardEntry> {
    Ok(ClipboardEntry {
        // ... existing 12 fields ...
        source_device: row.get(12).ok().flatten(),
    })
}
```

- [ ] **Step 5: Add sync_log DB methods**

In `src-tauri/src/storage/database.rs`, add:

```rust
pub fn insert_sync_log(&self, entry_hash: &str, device_id: &str, direction: &str) -> Result<()> {
    let conn = self.conn.lock().unwrap();
    let now = Local::now().naive_local().format("%Y-%m-%d %H:%M:%S").to_string();
    conn.execute(
        "INSERT INTO sync_log (entry_hash, device_id, direction, synced_at) VALUES (?1, ?2, ?3, ?4)",
        params![entry_hash, device_id, direction, now],
    )?;
    Ok(())
}

pub fn has_sync_log(&self, entry_hash: &str, device_id: &str, direction: &str) -> Result<bool> {
    let conn = self.conn.lock().unwrap();
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sync_log WHERE entry_hash = ?1 AND device_id = ?2 AND direction = ?3",
        params![entry_hash, device_id, direction],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

pub fn has_received_entry(&self, entry_hash: &str) -> Result<bool> {
    let conn = self.conn.lock().unwrap();
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sync_log WHERE entry_hash = ?1 AND direction = 'received'",
        params![entry_hash],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}
```

- [ ] **Step 6: Export `SyncLogEntry` from `storage/mod.rs`**

```rust
pub use models::{
    CategoryCount, ClipboardEntry, DayCount, DiscoveredDevice, PairedDevice, SearchQuery,
    SearchResult, Statistics, SyncLogEntry, SyncStatus, Tag, Template,
};
```

- [ ] **Step 7: Verify compilation**

Run: `cd src-tauri && cargo check 2>&1 | head -50`
Expected: No errors (warnings OK)

- [ ] **Step 8: Commit**

```bash
git add -A && git commit -m "feat(sync): add source_device column and sync_log DB methods for clipboard sync flow"
```

---

### Task 2: Add Real `ClipboardSync` Protocol Message and Sync Entry Serialization

**Files:**
- Modify: `src-tauri/src/sync/protocol.rs`
- Modify: `src-tauri/src/sync/mod.rs`

- [ ] **Step 1: Add `SyncEntryPayload` and `ClipboardSync` message**

In `src-tauri/src/sync/protocol.rs`, add a serializable entry payload struct and replace the placeholder with a real sync message. Add before `SyncMessage`:

```rust
/// Payload for clipboard entry synchronization.
/// Contains only the fields needed for cross-device sync (no local-only fields like id, use_count).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncEntryPayload {
    pub content: String,
    pub content_type: String,
    pub category: String,
    pub hash: String,
    pub source_app: Option<String>,
    pub is_sensitive: bool,
    pub source_device: String,
    pub created_at: String,
}
```

Add a new variant to `SyncMessage` enum (keep `ClipboardSyncPlaceholder` for backward compat):

```rust
ClipboardSync {
    entry: SyncEntryPayload,
    sender_device_id: String,
    timestamp: i64,
},
```

- [ ] **Step 2: Add conversion helpers in `sync/mod.rs`**

Add a constant for max sync payload size and a method to convert `ClipboardEntry` to `SyncEntryPayload`:

```rust
const MAX_SYNC_PAYLOAD_BYTES: usize = 1_048_576; // 1 MB
```

Add to `SyncManager` impl:

```rust
/// Check if an entry should be synced based on current config filters.
pub fn should_sync_entry(&self, entry: &crate::storage::ClipboardEntry) -> bool {
    let config = self.get_config();
    if !config.enabled || !config.auto_sync {
        return false;
    }
    // Skip entries that came from another device (prevent loop)
    if entry.source_device.is_some() {
        return false;
    }
    // Skip images unless configured
    if entry.content_type == "image" && !config.sync_images {
        return false;
    }
    // Skip sensitive unless configured
    if entry.is_sensitive && !config.sync_sensitive {
        return false;
    }
    // Skip oversized payloads
    if entry.content.len() > MAX_SYNC_PAYLOAD_BYTES {
        return false;
    }
    true
}

/// Convert a local ClipboardEntry into a SyncEntryPayload for transmission.
pub fn entry_to_sync_payload(
    &self,
    entry: &crate::storage::ClipboardEntry,
) -> protocol::SyncEntryPayload {
    protocol::SyncEntryPayload {
        content: entry.content.clone(),
        content_type: entry.content_type.clone(),
        category: entry.category.clone(),
        hash: entry.hash.clone(),
        source_app: entry.source_app.clone(),
        is_sensitive: entry.is_sensitive,
        source_device: self.local_device.device_id.clone(),
        created_at: entry.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
    }
}
```

- [ ] **Step 3: Verify compilation**

Run: `cd src-tauri && cargo check 2>&1 | head -50`
Expected: No errors

- [ ] **Step 4: Commit**

```bash
git add -A && git commit -m "feat(sync): add ClipboardSync protocol message and entry serialization"
```

---

### Task 3: Implement Send Pipeline — Broadcast Entry to Connected Devices

**Files:**
- Modify: `src-tauri/src/sync/mod.rs`
- Modify: `src-tauri/src/sync/server.rs`
- Modify: `src-tauri/src/sync/client.rs`

- [ ] **Step 1: Add broadcast channel to SyncManager**

In `src-tauri/src/sync/mod.rs`, add a broadcast channel for outgoing sync entries. Add to `SyncManager` struct:

```rust
use tokio::sync::broadcast;

pub struct SyncManager {
    // ... existing fields ...
    outgoing_tx: broadcast::Sender<protocol::SyncEntryPayload>,
}
```

In `SyncManager::new()`, create the channel:

```rust
let (outgoing_tx, _) = broadcast::channel::<protocol::SyncEntryPayload>(64);
```

Store `outgoing_tx` in the struct.

Add a public method to broadcast:

```rust
/// Broadcast a clipboard entry to all connected paired devices.
/// Called from the clipboard monitor after a new local entry is inserted.
pub fn broadcast_entry(&self, entry: &crate::storage::ClipboardEntry) {
    if !self.should_sync_entry(entry) {
        return;
    }
    let payload = self.entry_to_sync_payload(entry);
    // Log send attempt
    log::info!("Broadcasting clipboard entry {} to paired devices", payload.hash);
    let _ = self.outgoing_tx.send(payload);
}

/// Subscribe to outgoing entry broadcasts (used by server/client connections).
pub fn subscribe_outgoing(&self) -> broadcast::Receiver<protocol::SyncEntryPayload> {
    self.outgoing_tx.subscribe()
}
```

- [ ] **Step 2: Wire outgoing broadcast into client connection loop**

In `src-tauri/src/sync/client.rs`, in the `connect_device_loop` function, after the handshake succeeds and before the main `loop`, subscribe to outgoing broadcasts:

```rust
let mut outgoing_rx = sync_manager.subscribe_outgoing();
```

Then in the `tokio::select!` loop, add a branch for outgoing entries:

```rust
tokio::select! {
    // ... existing heartbeat and read branches ...
    
    result = outgoing_rx.recv() => {
        match result {
            Ok(payload) => {
                let entry_hash = payload.hash.clone();
                // Check sync_log to avoid re-sending
                if sync_manager.db_ref().has_sync_log(&entry_hash, &device.id, "sent").unwrap_or(true) {
                    continue;
                }
                let sync_msg = SyncMessage::ClipboardSync {
                    entry: payload,
                    sender_device_id: sync_manager.local_device_info().device_id.clone(),
                    timestamp: chrono::Local::now().timestamp_millis(),
                };
                match sync_manager.encrypt_protocol_message(&device.id, &sync_msg) {
                    Ok(encrypted) => {
                        if let Err(e) = write.send(Message::Text(encrypted.to_text().unwrap_or_default().into())).await {
                            log::error!("Failed to send sync entry to {}: {}", device.id, e);
                            break;
                        }
                        let _ = sync_manager.db_ref().insert_sync_log(&entry_hash, &device.id, "sent");
                        sync_manager.touch_last_sync(format!("Sent clipboard entry to {}", device.name));
                    }
                    Err(e) => log::error!("Failed to encrypt sync entry for {}: {}", device.id, e),
                }
            }
            Err(broadcast::error::RecvError::Lagged(n)) => {
                log::warn!("Outgoing broadcast lagged by {} entries for device {}", n, device.id);
            }
            Err(_) => break,
        }
    }
}
```

- [ ] **Step 3: Wire outgoing broadcast into server connection handler**

In `src-tauri/src/sync/server.rs`, similarly subscribe and add outgoing broadcast handling in the message loop. After the handshake, subscribe:

```rust
let mut outgoing_rx = sync_manager.subscribe_outgoing();
```

Convert the `while let Some(message) = read.next().await` loop into a `tokio::select!` loop with both incoming messages and outgoing broadcasts.

- [ ] **Step 4: Add `db_ref()` accessor to SyncManager**

In `src-tauri/src/sync/mod.rs`, add:

```rust
pub fn db_ref(&self) -> &Database {
    &self.db
}
```

- [ ] **Step 5: Verify compilation**

Run: `cd src-tauri && cargo check 2>&1 | head -50`
Expected: No errors

- [ ] **Step 6: Commit**

```bash
git add -A && git commit -m "feat(sync): implement send pipeline with broadcast channel to connected devices"
```

---

### Task 4: Implement Receive Pipeline — Decrypt, Deduplicate, Store, Refresh

**Files:**
- Modify: `src-tauri/src/sync/mod.rs`
- Modify: `src-tauri/src/sync/server.rs`
- Modify: `src-tauri/src/sync/client.rs`

- [ ] **Step 1: Add `handle_incoming_sync` method to SyncManager**

In `src-tauri/src/sync/mod.rs`, add:

```rust
/// Handle an incoming ClipboardSync message from a remote device.
/// Performs dedup check, inserts into local DB, logs to sync_log, and returns
/// the stored entry (if accepted) for frontend notification.
pub fn handle_incoming_sync(
    &self,
    sender_device_id: &str,
    payload: &protocol::SyncEntryPayload,
) -> Result<Option<crate::storage::ClipboardEntry>, String> {
    let hash = &payload.hash;

    // 1. Check if we already have this entry (by hash in DB)
    match self.db.find_by_hash(hash) {
        Ok(Some(_)) => {
            log::info!("Skipping duplicate sync entry: {}", hash);
            // Still log it so we don't re-process
            let _ = self.db.insert_sync_log(hash, sender_device_id, "received");
            return Ok(None);
        }
        Ok(None) => {}
        Err(e) => return Err(format!("DB error during sync dedup: {}", e)),
    }

    // 2. Check sync_log for already-received entries
    if self.db.has_received_entry(hash).unwrap_or(false) {
        log::info!("Already received sync entry via sync_log: {}", hash);
        return Ok(None);
    }

    // 3. Parse created_at
    let created_at = chrono::NaiveDateTime::parse_from_str(&payload.created_at, "%Y-%m-%d %H:%M:%S")
        .unwrap_or_else(|_| chrono::Local::now().naive_local());
    let now = chrono::Local::now().naive_local();

    // 4. Build ClipboardEntry
    let entry = crate::storage::ClipboardEntry {
        id: None,
        content: payload.content.clone(),
        content_type: payload.content_type.clone(),
        category: payload.category.clone(),
        hash: hash.clone(),
        source_app: payload.source_app.clone(),
        is_favorite: false,
        is_sensitive: payload.is_sensitive,
        use_count: 1,
        created_at,
        updated_at: now,
        expires_at: None,
        source_device: Some(sender_device_id.to_string()),
    };

    // 5. Insert into DB
    match self.db.insert_entry(&entry) {
        Ok(id) => {
            let mut stored = entry;
            stored.id = Some(id);
            // 6. Log to sync_log
            let _ = self.db.insert_sync_log(hash, sender_device_id, "received");
            self.touch_last_sync(format!("Received clipboard entry from {}", sender_device_id));
            log::info!("Stored synced entry {} from device {}", hash, sender_device_id);
            Ok(Some(stored))
        }
        Err(e) => {
            // Could be a UNIQUE constraint violation (race condition) — treat as dedup
            if e.to_string().contains("UNIQUE") {
                log::info!("Sync entry {} already exists (race dedup)", hash);
                let _ = self.db.insert_sync_log(hash, sender_device_id, "received");
                Ok(None)
            } else {
                Err(format!("Failed to insert synced entry: {}", e))
            }
        }
    }
}
```

- [ ] **Step 2: Handle `ClipboardSync` in server.rs**

In `src-tauri/src/sync/server.rs`, in the message handling match, replace the `ClipboardSyncPlaceholder` handler and add `ClipboardSync` handling:

```rust
SyncMessage::ClipboardSync { entry, sender_device_id, .. } => {
    match sync_manager.handle_incoming_sync(&sender_device_id, &entry) {
        Ok(Some(stored_entry)) => {
            // Emit frontend event
            // (need app_handle passed to server — see Step 4)
            if let Some(ref app_handle) = *app_handle_ref {
                let _ = app_handle.emit("clipboard-changed", &stored_entry);
            }
            let ack = sync_manager.encrypt_protocol_message(
                &remote_device_id,
                &SyncMessage::SyncAck { entry_hash: entry.hash.clone(), accepted: true },
            )?;
            write.send(Message::Text(ack.to_text()?.into())).await.map_err(|e| e.to_string())?;
        }
        Ok(None) => {
            // Duplicate — still ack
            let ack = sync_manager.encrypt_protocol_message(
                &remote_device_id,
                &SyncMessage::SyncAck { entry_hash: entry.hash.clone(), accepted: true },
            )?;
            write.send(Message::Text(ack.to_text()?.into())).await.map_err(|e| e.to_string())?;
        }
        Err(e) => {
            log::error!("Failed to handle incoming sync: {}", e);
            let ack = sync_manager.encrypt_protocol_message(
                &remote_device_id,
                &SyncMessage::SyncAck { entry_hash: entry.hash.clone(), accepted: false },
            )?;
            write.send(Message::Text(ack.to_text()?.into())).await.map_err(|e| e.to_string())?;
        }
    }
}
```

- [ ] **Step 3: Handle `ClipboardSync` in client.rs**

Same pattern as server — handle `ClipboardSync` in the client's message loop.

- [ ] **Step 4: Pass `AppHandle` to SyncManager for event emission**

In `src-tauri/src/sync/mod.rs`, add an `AppHandle` field:

```rust
pub struct SyncManager {
    // ... existing fields ...
    app_handle: RwLock<Option<tauri::AppHandle>>,
}
```

Add setter and getter:

```rust
pub fn set_app_handle(&self, handle: tauri::AppHandle) {
    *self.app_handle.blocking_write() = Some(handle);
}

pub fn app_handle(&self) -> Option<tauri::AppHandle> {
    self.app_handle.blocking_read().clone()
}
```

In `lib.rs`, after creating SyncManager, call:

```rust
sync_manager.set_app_handle(app.handle().clone());
```

Update server.rs and client.rs to use `sync_manager.app_handle()` for emitting events.

- [ ] **Step 5: Verify compilation**

Run: `cd src-tauri && cargo check 2>&1 | head -50`
Expected: No errors

- [ ] **Step 6: Commit**

```bash
git add -A && git commit -m "feat(sync): implement receive pipeline with dedup, storage, and frontend refresh"
```

---

### Task 5: Hook Clipboard Monitor into Sync Pipeline

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Pass SyncManager to clipboard processing loop**

In `src-tauri/src/lib.rs`, in the `setup` closure, after creating `sync_manager`, clone it for the clipboard processing task:

```rust
let sync_for_rx = sync_manager.clone();
```

- [ ] **Step 2: Call `broadcast_entry` after successful insert**

In the `tauri::async_runtime::spawn` block that processes clipboard changes, after the successful `db_for_rx.insert_entry(&entry)` call, add:

```rust
Ok(id) => {
    let mut stored_entry = entry;
    stored_entry.id = Some(id);
    let _ = app_handle.emit("clipboard-changed", &stored_entry);
    // Broadcast to paired devices via sync pipeline
    sync_for_rx.broadcast_entry(&stored_entry);
}
```

- [ ] **Step 3: Set `source_device` to `None` for local entries**

Ensure the `ClipboardEntry` constructed in `lib.rs` has `source_device: None`:

```rust
let entry = ClipboardEntry {
    // ... existing fields ...
    expires_at,
    source_device: None,  // Local entry
};
```

- [ ] **Step 4: Verify compilation**

Run: `cd src-tauri && cargo check 2>&1 | head -50`
Expected: No errors

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "feat(sync): hook clipboard monitor into sync broadcast pipeline"
```

---

### Task 6: Update README and CHANGELOG, Final Commit

**Files:**
- Modify: `README.md`
- Modify: `README.zh-CN.md`
- Modify: `CHANGELOG.md`

- [ ] **Step 1: Update CHANGELOG.md**

Add a new entry at the top for the clipboard sync flow feature.

- [ ] **Step 2: Update README.md and README.zh-CN.md**

Update the sync section to reflect that clipboard entries are now actually synced between devices (not just discovery/pairing).

- [ ] **Step 3: Final commit**

```bash
git add -A && git commit -m "docs: update README and CHANGELOG for phase3 clipboard sync flow"
```
