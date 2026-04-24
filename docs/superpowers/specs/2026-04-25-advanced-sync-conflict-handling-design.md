# Advanced Sync Conflict Handling — Design Spec

- **Date:** 2026-04-25
- **Status:** Approved for implementation planning
- **Scope:** Smart Clipboard — cross-device sync (LAN + WebDAV)
- **Related roadmap item:** `README.md` → "Advanced sync conflict handling: Smarter merge / conflict-resolution behavior"

---

## 1. Context & Problem

Smart Clipboard already has two sync transports:

- **LAN sync** — encrypted WebSocket between paired devices (X25519 + AES-256-GCM); new entries broadcast and deduplicated by content `hash`.
- **WebDAV cloud sync** — end-to-end encrypted `index.enc` + per-entry `*.enc` files; ETag optimistic locking protects the index; the poller inserts new entries it does not yet have.

Content itself is **immutable** once copied (`hash = SHA-256(content)` is the unique key). What actually diverges across devices is **metadata and lifecycle state**:

| Immutable (no conflict possible)                                          | Mutable (can diverge)                                                              |
| ------------------------------------------------------------------------- | ---------------------------------------------------------------------------------- |
| `content`, `content_type`, `category`, `hash`, `source_app`, `created_at` | `is_favorite`, `is_sensitive`, `tags[]`, `expires_at`, `use_count`, deletion state |

Current behavior on metadata divergence is silent "local wins" — whichever device last wrote to its own DB keeps its state, and nothing propagates. This produces user-visible inconsistencies (favorites disappearing, tags vanishing on one device, deleted items reappearing).

## 2. Goals & Non-Goals

### Goals

1. Detect concurrent metadata mutations across paired/cloud-connected devices.
2. Apply a configurable automatic resolution strategy (default: **smart-merge**) that silently resolves the vast majority of cases.
3. Surface a manual-resolution UI only for cases where auto-merge could lose user intent (primarily security-relevant downgrades).
4. Persist a conflict log (queue + history) for user auditability.
5. Preserve backward compatibility — old clients without the new protocol fields must continue to sync without crashing.

### Non-Goals

- Content-level merge (three-way diff of clipboard text). Content is immutable by design.
- A full CRDT runtime (rejected as Approach C in brainstorming — too heavy for a 5MB app).
- Conflict handling for non-clipboard entities (templates, settings) — out of scope for this spec.
- Real-time presence (knowing "user B is currently editing tags on entry X").

## 3. Approach Summary (approved)

**Vector-clock based detection + pluggable strategy engine + conflict queue UI**, as chosen from three brainstormed approaches. Rationale:

- Vector clocks are the only option that can _detect concurrency_, which is required to ever surface a manual-resolution UI.
- Device count is bounded (`paired_devices` typically ≤ 10), so vector-clock size stays < 200 bytes.
- Strategy is pluggable so users can downgrade to simpler policies (LWW / prefer-local / prefer-remote / manual) if smart-merge surprises them.

## 4. Data Model Changes

### 4.1 `clipboard_entries` — new columns (ALTER TABLE, non-breaking)

| Column             | Type                         | Purpose                                                  |
| ------------------ | ---------------------------- | -------------------------------------------------------- |
| `metadata_version` | `TEXT NOT NULL DEFAULT '{}'` | Vector clock JSON, e.g. `{"device-A":3,"device-B":1}`    |
| `last_modified_by` | `TEXT`                       | `device_id` of the most recent metadata writer           |
| `last_modified_at` | `DATETIME`                   | Wall-clock timestamp of that writer (tiebreaker for LWW) |
| `is_deleted`       | `INTEGER NOT NULL DEFAULT 0` | Soft-delete tombstone flag                               |
| `deleted_at`       | `DATETIME`                   | When the tombstone was created (nullable)                |

**Why soft-delete:** Delete-vs-modify conflicts require the deletion event to be observable by peers. Hard DELETE loses that signal. Tombstones with `deleted_at` older than 30 days are hard-deleted by a background sweeper.

### 4.2 New table — `entry_metadata_changelog`

Records each local metadata change so it can be broadcast (LAN) or packed into the cloud index (WebDAV) on the next sync tick; rows are purged after they are acknowledged as synced.

```sql
CREATE TABLE entry_metadata_changelog (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    entry_hash  TEXT NOT NULL,
    op          TEXT NOT NULL,
    -- 'favorite' | 'sensitive' | 'expires' | 'tags' | 'delete' | 'restore'
    payload     TEXT NOT NULL,             -- JSON, op-specific new value
    lamport     INTEGER NOT NULL,
    device_id   TEXT NOT NULL,
    changed_at  DATETIME NOT NULL DEFAULT (datetime('now','localtime')),
    synced      INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_changelog_synced ON entry_metadata_changelog(synced, changed_at);
CREATE INDEX idx_changelog_hash   ON entry_metadata_changelog(entry_hash);
```

`entry_hash` is used (not `id`) because entry `id` is per-device AUTOINCREMENT and not stable across devices; `hash` is the cross-device identifier.

### 4.3 New table — `sync_conflicts` (queue + history, single table)

```sql
CREATE TABLE sync_conflicts (
    id                    INTEGER PRIMARY KEY AUTOINCREMENT,
    entry_hash            TEXT NOT NULL,
    conflict_type         TEXT NOT NULL,
    -- 'metadata_concurrent' | 'delete_vs_modify' | 'sensitive_downgrade'
    local_snapshot        TEXT NOT NULL,   -- JSON EntryMetadataSnapshot
    remote_snapshot       TEXT NOT NULL,   -- JSON EntryMetadataSnapshot
    remote_device         TEXT NOT NULL,
    detected_at           DATETIME NOT NULL DEFAULT (datetime('now','localtime')),
    status                TEXT NOT NULL,
    -- 'pending' | 'auto_resolved' | 'manual_resolved' | 'dismissed'
    resolution            TEXT,            -- JSON merged EntryMetadataSnapshot
    resolution_strategy   TEXT,
    -- 'smart-merge' | 'last-write-wins' | 'prefer-local' | 'prefer-remote' | 'manual'
    resolved_at           DATETIME,
    resolved_by           TEXT              -- 'auto' or resolving device_id
);
CREATE INDEX idx_conflicts_status ON sync_conflicts(status, detected_at DESC);
CREATE INDEX idx_conflicts_hash   ON sync_conflicts(entry_hash);
```

`pending` rows are the user-facing queue; all other statuses form the audit history.

### 4.4 Lamport counter

A single monotonically increasing `u64` persisted in app config (`config.rs`). It is **not** derived from `updated_at`. Rules:

- Before any local metadata mutation: `local_lamport += 1`.
- After receiving a remote change with lamport `r`: `local_lamport = max(local_lamport, r) + 1`.

Vector clock for an entry is `{device_id: lamport_at_time_of_write}`. Two VCs compare as `Less`, `Greater`, `Equal`, or `Concurrent`.

### 4.5 `EntryMetadataSnapshot` type (Rust + TS, serde `camelCase`)

Identical shape on both sides; used in conflict rows and manual-resolution UI.

```rust
struct EntryMetadataSnapshot {
    is_favorite:  bool,
    is_sensitive: bool,
    tags:         Vec<String>,      // tag names, not ids (ids are per-device)
    expires_at:   Option<String>,   // RFC3339 or null
    is_deleted:   bool,
    use_count:    i64,
    vc:           serde_json::Value, // {device_id: lamport}
    last_modified_by: String,
    last_modified_at: String,       // RFC3339
}
```

## 5. Conflict Detection

### 5.1 Vector-clock comparison

```rust
enum VcOrder { Less, Greater, Equal, Concurrent }

fn compare_vc(local: &VectorClock, remote: &VectorClock) -> VcOrder {
    // Less      => every local[d] <= remote[d], and some local[d] < remote[d]
    // Greater   => every local[d] >= remote[d], and some local[d] > remote[d]
    // Equal     => all equal
    // Concurrent=> neither dominates
}

fn merge_vc(a: &VectorClock, b: &VectorClock) -> VectorClock {
    // For each device_id in a ∪ b: max(a[d].unwrap_or(0), b[d].unwrap_or(0))
}
```

### 5.2 Trigger points

- **LAN sync — `src-tauri/src/sync/mod.rs` (message handler)** reacts to a new `SyncMessage::MetadataUpdate { entry_hash, vc, fields, sender_device_id, lamport }` and calls `ConflictResolver::apply_remote_change(...)`.
- **WebDAV sync — `src-tauri/src/sync/webdav/poller.rs`** after `load_index` compares each `IndexEntry.vc` + `metadata_hash` to local. If different, downloads per-entry metadata payload and calls the same resolver.
- **Local mutation — `src-tauri/src/commands.rs`** — all metadata-writing commands (`toggle_favorite`, `set_favorite_state_for_entries`, `set_tags_for_entries`, `delete_entry`, `delete_entries`, a new `set_sensitive_state`, plus any expiry setters) go through a single helper `apply_local_metadata_change(entry_hash, op, payload)` that bumps the lamport counter, updates `metadata_version[self]`, writes a `entry_metadata_changelog` row, and emits the change for sync.

### 5.3 Conflict classification

`local.vc` in the pseudocode below is the value of the `metadata_version` column on the local row; `remote_vc` is the corresponding value received from the peer.

`touched_fields: Vec<MetadataField>` is **always supplied by the caller** (LAN handler from `SyncMessage::MetadataUpdate.fields`, WebDAV poller from the per-entry metadata file which carries the same field list). The resolver does not infer it.

```
input: local_entry (current row), remote_snapshot, remote_vc, touched_fields
case compare_vc(local.vc, remote_vc):
  Less      => not a conflict, apply remote directly
  Greater   => not a conflict, ignore remote (already superseded)
  Equal     => not a conflict, no-op
  Concurrent =>
      if local.is_deleted && !remote_snapshot.is_deleted
            && !touched_fields.is_empty():
          => delete_vs_modify
      elif !local.is_deleted && remote_snapshot.is_deleted:
          => delete_vs_modify
      elif local.is_sensitive == true
            && remote_snapshot.is_sensitive == false:
          => sensitive_downgrade
      else:
          => metadata_concurrent
```

## 6. Resolution Strategy Engine

### 6.1 Five selectable strategies

| Strategy                    | Behavior                                                                                    | Surfaces UI? |
| --------------------------- | ------------------------------------------------------------------------------------------- | ------------ |
| `smart-merge` **(default)** | Per-field rules in §6.2; only `sensitive_downgrade` is deferred to the user                 | Rarely       |
| `last-write-wins`           | Pick snapshot with newer `last_modified_at`; ties broken by `device_id` lexicographic order | Never        |
| `prefer-local`              | Keep local snapshot verbatim                                                                | Never        |
| `prefer-remote`             | Adopt remote snapshot verbatim                                                              | Never        |
| `manual`                    | All concurrent conflicts go to the pending queue                                            | Always       |

Strategy is stored in app config as `conflict_resolution_strategy: String` (default `"smart-merge"`), plus a boolean `require_manual_for_sensitive_downgrade` (default `true`).

### 6.2 `smart-merge` per-field rules

| Field          | Merge rule                                                                                                                                                                                  |
| -------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `is_favorite`  | OR (any side `true` ⇒ `true`)                                                                                                                                                               |
| `is_sensitive` | OR in the safe direction: any side `true` ⇒ `true`. The only case that defers to manual is `local=true` + `remote=false` (a _downgrade_), gated by `require_manual_for_sensitive_downgrade` |
| `tags`         | Union of tag name sets                                                                                                                                                                      |
| `expires_at`   | `NULL` wins (never-expires); otherwise `max()` (later time)                                                                                                                                 |
| `is_deleted`   | Presence wins: if either side has `is_deleted=false` the entry is kept alive; only `true` on both sides results in a deleted entry                                                          |
| `use_count`    | `max()`                                                                                                                                                                                     |

### 6.3 Auto-resolution flow

1. Compute merged `EntryMetadataSnapshot`.
2. Update local `clipboard_entries` row to match the merged snapshot.
3. `metadata_version = merge_vc(local, remote)`; `last_modified_at = now()`; `last_modified_by = self_device_id`.
4. Insert `sync_conflicts` row with `status='auto_resolved'`, the resolution snapshot, and `resolution_strategy`.
5. Append a row to `entry_metadata_changelog` so the merged result is propagated outward on the next sync tick.

### 6.4 Manual-resolution flow (`sensitive_downgrade` or strategy = `manual`)

1. Insert `sync_conflicts` row with `status='pending'`, `local_snapshot`, `remote_snapshot`, `remote_device`, `conflict_type`.
2. **Do not** mutate the local entry yet — local state stays as-is.
3. Emit Tauri event `conflict-detected` with the new conflict id; frontend store increments the pending counter.
4. User chooses in UI; the frontend invokes `resolve_conflict_manually(id, choice)` where `choice` is `'local' | 'remote' | 'merge'`:
   - `'local'` → resolved snapshot = `local_snapshot`
   - `'remote'` → resolved snapshot = `remote_snapshot`
   - `'merge'` → resolved snapshot = smart-merge of the two (same rules as §6.2)
5. Backend then applies the chosen snapshot by running §6.3 steps 2, 3, and 5 (update entry row, merge VC, append changelog), **plus** updating the existing conflict row (not inserting a new one) with `status='manual_resolved'`, `resolution=<chosen snapshot>`, `resolution_strategy='manual'`, `resolved_at=now()`, `resolved_by=self_device_id`.

## 7. UI Design

### 7.1 Entry point — `SyncPanel.vue`

Add a third tab alongside LAN and WebDAV: **Conflicts**, with a badge showing pending count.

```
┌─────────────────────────────────────────┐
│ [LAN] [WebDAV] [Conflicts ⚠ 3]          │
├─────────────────────────────────────────┤
│ Pending (3)         History (127)       │
│ ─────────────────────                    │
│ 🔀 "API_KEY=sk-..."                      │
│   Local:  Sensitive ✓, Tags: [key]      │
│   Remote (iPhone): Sensitive ✗          │
│   [Keep Local] [Keep Remote] [Merge]    │
│                                          │
│ 🗑 "会议纪要 2024..."                      │
│   Local: Deleted                         │
│   Remote (Mac): Favorite ✓              │
│   [Confirm Delete] [Restore]            │
└─────────────────────────────────────────┘
```

### 7.2 New components

- **`ConflictPanel.vue`** — container for the Conflicts tab; holds the Pending/History sub-tab switcher and pagination for history.
- **`ConflictCard.vue`** — one row per conflict. Displays:
  - Content preview (first 60 chars of `content`, expandable).
  - Three side-by-side columns: Local / Remote / Suggested merge.
  - Conflict type badge, detection time, remote device name.
  - Buttons: `Keep Local`, `Keep Remote`, `Apply Merge`, `Dismiss` (for history items, only a "Revert to this choice" affordance if still possible, otherwise read-only).

### 7.3 Settings — `SettingsPanel.vue` new "Sync Conflicts" section

- Default strategy dropdown (5 options, default `smart-merge`).
- Toggle: "Require manual confirmation when a device tries to clear the Sensitive flag" (default on).
- Number input: "Keep conflict history for N days" (default `90`, after which rows with `status != 'pending'` are hard-deleted by the same sweeper that handles tombstones).
- Button: "Clear conflict history" (keeps pending rows).

### 7.4 New Pinia store — `src/stores/conflictStore.ts`

```ts
interface SyncConflict {
  id: number;
  entryHash: string;
  conflictType: 'metadata_concurrent' | 'delete_vs_modify' | 'sensitive_downgrade';
  localSnapshot: EntryMetadataSnapshot;
  remoteSnapshot: EntryMetadataSnapshot;
  remoteDevice: string;
  detectedAt: string;
  status: 'pending' | 'auto_resolved' | 'manual_resolved' | 'dismissed';
  resolution: EntryMetadataSnapshot | null;
  resolutionStrategy: string;
  resolvedAt: string | null;
}
```

Exposed methods: `loadPending()`, `loadHistory(offset, limit, filter?)`, `resolveManually(id, choice)`, `dismiss(id)`, `clearHistory(olderThanDays?)`, `getStats()`, `clearSensitiveState()`.

### 7.5 Event wiring

`App.vue` subscribes to the Tauri event `conflict-detected` on mount and calls `conflictStore.loadPending()` to refresh the badge. Unsubscribe in `onUnmounted`. When the app is locked (security model), sensitive state is cleared via `clearSensitiveState()` alongside other stores.

## 8. Backend Commands

New Tauri commands in `src-tauri/src/commands.rs`:

| Command                                                                                                      | Returns                                                            |
| ------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------ |
| `list_pending_conflicts() -> Vec<SyncConflict>`                                                              | All `status='pending'` rows, newest first                          |
| `list_conflict_history(offset: i64, limit: i64, filter: Option<ConflictFilter>) -> (Vec<SyncConflict>, i64)` | Pageable audit list with total count                               |
| `resolve_conflict_manually(id: i64, choice: String) -> Result<(), String>`                                   | Applies chosen snapshot, marks row `manual_resolved`               |
| `dismiss_conflict(id: i64) -> Result<(), String>`                                                            | Marks `dismissed` without applying either side (local stays as-is) |
| `clear_conflict_history(older_than_days: i64) -> u32`                                                        | Hard-deletes rows with `status != 'pending'` older than cutoff     |
| `get_conflict_stats() -> ConflictStats`                                                                      | `{ pending: i64, auto_resolved_7d: i64, manual_resolved_7d: i64 }` |

All six commands are guarded by the existing app-lock (`security.rs`) the same way existing sync commands are — refuse while locked.

## 9. Sync Protocol Changes

### 9.1 LAN — `src-tauri/src/sync/protocol.rs`

Add a new `SyncMessage` variant:

```rust
MetadataUpdate {
    entry_hash: String,
    vc: serde_json::Value,                 // {device_id: lamport}
    lamport: u64,                          // sender's lamport at time of write
    fields: Vec<MetadataField>,            // which ops happened
    snapshot: EntryMetadataSnapshot,       // post-change snapshot
    sender_device_id: String,
    timestamp: i64,
}
```

`MetadataField` is a lightweight enum: `Favorite`, `Sensitive`, `Tags`, `Expires`, `Delete`, `Restore`.

Older clients with `serde(other)` fallback (to be added to `SyncMessage` deserialization) silently ignore unknown variants.

### 9.2 WebDAV — `src-tauri/src/sync/webdav/index.rs`

Bump `SyncIndex.version` from `1` to `2`. `IndexEntry` gains three optional fields:

```rust
pub struct IndexEntry {
    // ...existing fields...
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vc: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata_hash: Option<String>,        // SHA-256 of canonicalized metadata
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_modified_at: Option<String>,
}
```

When reading a `version < 2` index, the manager treats all entries as `vc = {entry.source_device: 0}` and `metadata_hash = None` (forces one-time reconcile on next poll). When writing, it always emits `version = 2`.

Per-entry metadata updates are stored as a new encrypted file per entry id in `{remote_path}/metadata/{hash_prefix}.enc`, separate from the content file. This keeps content files immutable (cacheable) and lets metadata churn without rewriting the content payload. The poller downloads the metadata file only when the index shows `metadata_hash` has changed.

### 9.3 Rate-limit impact

Each per-entry metadata update costs one token from the existing `TokenBucketLimiter` (capacity 150, refill 30min by default). Batched changes for the same entry within a poll interval are coalesced into a single upload.

## 10. Migration Plan

In `src-tauri/src/storage/migrations.rs`, follow the existing idempotent `IF NOT EXISTS` + `PRAGMA table_info` pattern:

1. For each new column on `clipboard_entries`, check `PRAGMA table_info(clipboard_entries)` and add if absent.
2. `CREATE TABLE IF NOT EXISTS entry_metadata_changelog (...)` plus its two indices.
3. `CREATE TABLE IF NOT EXISTS sync_conflicts (...)` plus its two indices.
4. **Backfill** (one-time, guarded by a migration sentinel row in a `schema_meta` table):
   - `UPDATE clipboard_entries SET metadata_version = '{"' || ? || '":0}'` where `?` is the current device id.
   - `UPDATE clipboard_entries SET last_modified_at = updated_at, last_modified_by = COALESCE(source_device, ?)`.
5. On first launch after upgrade, mark all existing entries as synced from the local POV (`metadata_version[self]=0`), so the first poll/handshake with any peer will drive a single reconciliation pass rather than flagging everything as conflict.

Rollback: migrations are additive; a user downgrading keeps the new columns but the old code ignores them.

## 11. Edge Cases & Risks

| Case                                                      | Handling                                                                                                                                                                               |
| --------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Peer gets removed (unpaired)                              | Prune its `device_id` from all `metadata_version` JSONs after a 7-day grace period (handled by the same sweeper)                                                                       |
| Clock skew between devices                                | `last_modified_at` is only used for LWW tiebreaks, never as the primary signal. VC is authoritative                                                                                    |
| `entry_metadata_changelog` growth                         | Sweeper: delete rows where `synced=1 AND changed_at < now-7d`                                                                                                                          |
| Tombstone growth                                          | Hard-delete rows where `is_deleted=1 AND deleted_at < now-30d`                                                                                                                         |
| Conflict history growth                                   | Sweeper honors user-configured retention (default 90d)                                                                                                                                 |
| Two devices simultaneously flip `is_sensitive` true↔false | The true-side wins (safe direction) under smart-merge; the false-side device sees a `sensitive_downgrade` row on its next sync                                                         |
| Partial failure during resolution                         | §6.3 steps 2–5 run inside a single SQLite transaction; on failure, no state changes and the conflict row (if step 4 succeeded) stays `pending`                                         |
| WebDAV index v1 ↔ v2 mixed fleet                          | Covered by §9.2 defaults; never loses data, at worst triggers a one-time reconcile                                                                                                     |
| VC size blowup                                            | Hard cap: when a VC contains entries for more than 60 distinct `device_id`s, prune any entries whose `device_id` is not in the current `paired_devices` table and whose counter is `0` |

## 12. Testing Strategy

### 12.1 Rust unit tests

- `conflict/vector_clock.rs` — every `compare_vc` branch (`Less`/`Greater`/`Equal`/`Concurrent`), `merge_vc` idempotence/commutativity, serialization round-trip.
- `conflict/resolver.rs` — each smart-merge field rule, each of the five strategies, each of the three `conflict_type` classifications.
- `storage/database.rs` — CRUD for `entry_metadata_changelog` and `sync_conflicts`, including index-backed queries.
- `storage/migrations.rs` — extend existing style: add tests that seed a v1 schema, run the new migration, assert new columns/tables/backfill values.

### 12.2 Rust integration tests (in `src-tauri/src/integration_tests/`)

- Two in-memory DBs + mock sync transport. Scenario matrix:
  - Two devices concurrently favorite the same entry → auto-resolved on both.
  - Device A deletes, Device B retags → `delete_vs_modify` pending on both.
  - Device A turns `is_sensitive=true`, Device B turns it `false` → auto on A (stays true), pending on B.
  - v1 WebDAV index read by v2 client → no crash, one-time reconcile path covered.

### 12.3 Frontend unit tests (Vitest, extending `tests/unit/`)

- `conflictStore.test.ts` — `loadPending`, `resolveManually`, event handling, sensitive-state clearing.
- `ConflictCard.test.ts` — rendering for each `conflict_type`, button wiring, preview truncation.

### 12.4 Manual verification checklist

- Fresh install → two paired laptops → simultaneous favorite → both show `auto_resolved` history entry within one sync cycle, both have favorite set.
- Lock app mid-resolution → commands refuse, pending queue preserved on unlock.
- Flip strategy to `manual` → every concurrent change lands in the pending tab.

## 13. File Structure Delta

```
src-tauri/src/
├── conflict/                       # new module
│   ├── mod.rs                      # ConflictResolver — single entry point
│   ├── vector_clock.rs             # VectorClock type + compare/merge/increment
│   ├── resolver.rs                 # 5 strategies + smart-merge field rules
│   └── snapshot.rs                 # EntryMetadataSnapshot + conversions
├── storage/
│   ├── database.rs                 # extend: CRUD for conflicts + changelog
│   └── migrations.rs               # extend: new cols + new tables + backfill
├── sync/
│   ├── protocol.rs                 # extend: SyncMessage::MetadataUpdate
│   ├── mod.rs                      # LAN handler routes MetadataUpdate to resolver
│   └── webdav/
│       ├── index.rs                # IndexEntry gains vc/metadata_hash/last_modified_at
│       └── poller.rs               # diff metadata_hash, fetch metadata file, route to resolver
└── commands.rs                     # extend: 6 new conflict commands + apply_local_metadata_change helper

src/
├── stores/conflictStore.ts         # new
├── components/
│   ├── ConflictPanel.vue           # new
│   ├── ConflictCard.vue            # new
│   ├── SyncPanel.vue               # modify: add Conflicts tab + badge
│   └── SettingsPanel.vue           # modify: add "Sync Conflicts" section
├── types/index.ts                  # extend: SyncConflict, EntryMetadataSnapshot, ConflictFilter
└── i18n/                           # extend: EN + ZH strings for all new UI

tests/unit/
├── conflictStore.test.ts           # new
└── ConflictCard.test.ts            # new
```

## 14. Effort Estimate

- Rust core (conflict module + migration + sync integration): ~2.5 days
- Frontend (store + 2 components + panel/settings integration + i18n): ~1.5 days
- Tests (unit + integration): ~1 day
- Buffer (edge-case fixes, docs): ~0.5 day
- **Total: ~5–6 days**

## 15. Open Questions for Implementation Plan

None remaining for this spec. All decisions are locked:

- Scope B (full loop with vector clocks + manual UI + history log) — approved.
- Default strategy = `smart-merge` with `require_manual_for_sensitive_downgrade = true` — approved.

The implementation plan (next step, via `writing-plans` skill) will decompose the above §4–§13 into TDD-style bite-sized tasks with exact files, failing tests, and commits.
