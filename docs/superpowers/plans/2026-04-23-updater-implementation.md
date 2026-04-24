# Managed Updater Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add managed updater preferences, runtime status, pending update persistence, and Settings UI for manual check / discard / install flows based on the approved updater design.

**Architecture:** Keep updater preferences inside `AppConfig`, add a focused backend `src-tauri/src/updater/` module for validation, status, pending-state persistence, and a manager facade, then expose dedicated commands consumed by a frontend updater store and the Settings panel. Implement the feature in vertical slices: config + pure backend logic first, then command wiring, then frontend UI/store integration, then release config scaffolding.

**Tech Stack:** Rust (Tauri 2, serde, semver), Vue 3 + TypeScript + Pinia + Vitest

---

## File Structure

### New Files

- `src-tauri/src/updater/mod.rs` — updater manager facade and exported types
- `src-tauri/src/updater/types.rs` — updater config-independent runtime/pending types
- `src-tauri/src/updater/mirrors.rs` — mirror validation and URL resolution
- `src-tauri/src/updater/pending.rs` — pending update persistence and cleanup helpers
- `src-tauri/src/updater/policy.rs` — interval and network policy helpers
- `src/stores/updaterStore.ts` — frontend updater state/actions
- `tests/unit/updaterStore.test.ts` — store tests
- `tests/unit/SettingsPanel.updater.test.ts` — settings updater UI tests

### Modified Files

- `src-tauri/Cargo.toml`
- `src-tauri/src/config.rs`
- `src-tauri/src/commands.rs`
- `src-tauri/src/lib.rs`
- `src-tauri/tauri.conf.json`
- `src/components/SettingsPanel.vue`
- `src/types/index.ts`
- `src/i18n/locales/en.ts`
- `src/i18n/locales/zh-CN.ts`
- `.github/workflows/release.yml`

### Task 1: Add updater config model and pure backend updater helpers

**Files:**

- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/src/config.rs`
- Modify: `src-tauri/src/lib.rs`
- Create: `src-tauri/src/updater/mod.rs`
- Create: `src-tauri/src/updater/types.rs`
- Create: `src-tauri/src/updater/mirrors.rs`
- Create: `src-tauri/src/updater/pending.rs`
- Create: `src-tauri/src/updater/policy.rs`
- Test: `src-tauri/src/updater/mirrors.rs`
- Test: `src-tauri/src/updater/pending.rs`
- Test: `src-tauri/src/updater/policy.rs`

- [ ] **Step 1: Write failing Rust tests for mirror validation and pending persistence**

Create the new updater helper files with tests first, covering:

- valid mirror requires `https://` and `{url}`
- blank mirror lines are ignored by normalization
- resolved candidate URLs preserve mirror order and append canonical URL last
- interval-due helper returns false when `last_check_at` is recent and true when absent/expired
- pending state write/read/clear round-trips JSON correctly

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml updater:: -- --nocapture
```

Expected: FAIL because updater module/types do not exist yet.

- [ ] **Step 2: Add Rust dependency for semantic version comparison**

Add to `src-tauri/Cargo.toml`:

```toml
semver = "1.0"
```

- [ ] **Step 3: Add `UpdaterConfig` to `AppConfig`**

In `src-tauri/src/config.rs`, add:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdaterConfig {
    pub auto_check_enabled: bool,
    pub check_interval_hours: u64,
    pub auto_download_enabled: bool,
    pub wifi_only: bool,
    pub mirrors: Vec<String>,
    pub last_check_at: Option<String>,
}

impl Default for UpdaterConfig {
    fn default() -> Self {
        Self {
            auto_check_enabled: true,
            check_interval_hours: 24,
            auto_download_enabled: false,
            wifi_only: true,
            mirrors: vec![],
            last_check_at: None,
        }
    }
}
```

And add to `AppConfig`:

```rust
#[serde(default)]
pub updater: UpdaterConfig,
```

And default initializer:

```rust
updater: UpdaterConfig::default(),
```

- [ ] **Step 4: Implement updater types and helpers**

Create `src-tauri/src/updater/types.rs` with:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UpdaterPhase {
    Idle,
    Checking,
    UpdateAvailable,
    Downloading,
    ReadyToInstall,
    UpToDate,
    Installing,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PendingUpdateRecord {
    pub version: String,
    pub release_date: Option<String>,
    pub current_version: String,
    pub notes: Option<String>,
    pub artifact_path: String,
    pub signature_path: String,
    pub canonical_asset_url: String,
    pub source_asset_url: String,
    pub downloaded_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UpdaterStatus {
    pub phase: UpdaterPhase,
    pub current_version: String,
    pub available_version: Option<String>,
    pub pending_update: Option<PendingUpdateRecord>,
    pub download_progress: Option<f64>,
    pub last_error: Option<String>,
    pub last_check_silent: bool,
}

impl UpdaterStatus {
    pub fn idle(current_version: String) -> Self {
        Self {
            phase: UpdaterPhase::Idle,
            current_version,
            available_version: None,
            pending_update: None,
            download_progress: None,
            last_error: None,
            last_check_silent: false,
        }
    }
}
```

Create `src-tauri/src/updater/mirrors.rs` with functions:

- `normalize_mirrors(input: &[String]) -> Vec<String>`
- `validate_mirror_template(template: &str) -> Result<(), String>`
- `resolve_candidate_urls(canonical_url: &str, mirrors: &[String]) -> Vec<String>`

Create `src-tauri/src/updater/policy.rs` with:

- `is_interval_due(last_check_at: Option<&str>, interval_hours: u64, now: chrono::DateTime<chrono::Utc>) -> bool`
- `auto_download_allowed(auto_download_enabled: bool, wifi_only: bool, wifi_known_and_connected: bool) -> bool`

Create `src-tauri/src/updater/pending.rs` with:

- `pending_state_path(app_data_dir: &std::path::Path) -> std::path::PathBuf`
- `write_pending_update(app_data_dir: &std::path::Path, record: &PendingUpdateRecord) -> Result<(), String>`
- `read_pending_update(app_data_dir: &std::path::Path) -> Result<Option<PendingUpdateRecord>, String>`
- `clear_pending_update(app_data_dir: &std::path::Path) -> Result<(), String>`

Create `src-tauri/src/updater/mod.rs` to export those modules and define a minimal `UpdaterManager` with in-memory status:

```rust
use std::sync::Mutex;

use tauri::AppHandle;

pub mod mirrors;
pub mod pending;
pub mod policy;
pub mod types;

pub use types::{PendingUpdateRecord, UpdaterPhase, UpdaterStatus};

pub struct UpdaterManager {
    status: Mutex<UpdaterStatus>,
}

impl UpdaterManager {
    pub fn new(current_version: String) -> Self {
        Self {
            status: Mutex::new(UpdaterStatus::idle(current_version)),
        }
    }

    pub fn get_status(&self) -> UpdaterStatus {
        self.status.lock().unwrap().clone()
    }

    pub fn set_status(&self, status: UpdaterStatus) {
        *self.status.lock().unwrap() = status;
    }

    pub fn emit_status<R: tauri::Runtime>(&self, app: &AppHandle<R>) {
        let _ = app.emit("updater-status-changed", self.get_status());
    }
}
```

Export module from `src-tauri/src/lib.rs`:

```rust
pub mod updater;
```

- [ ] **Step 5: Run Rust tests to verify helpers pass**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml updater:: -- --nocapture
```

Expected: PASS

- [ ] **Step 6: Commit backend helper foundation**

```bash
git add src-tauri/Cargo.toml src-tauri/src/config.rs src-tauri/src/lib.rs src-tauri/src/updater
git commit -m "feat(updater): add config model and backend helper foundation"
```

### Task 2: Add updater commands and bootstrap status restoration

**Files:**

- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/config.rs`
- Test: `src-tauri/src/updater/mod.rs`

- [ ] **Step 1: Write failing command/manager tests**

Add tests covering:

- `discard_pending_update` clears persisted pending state
- startup restoration loads pending state into runtime status as `ReadyToInstall`
- manual `check_for_updates_now` without implementation returns at least a deterministic state transition placeholder

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml updater_manager -- --nocapture
```

Expected: FAIL until commands/manager methods exist.

- [ ] **Step 2: Extend `UpdaterManager` with restore/discard/manual-check placeholder methods**

Implement methods on `UpdaterManager`:

- `restore_from_disk(app_data_dir: &Path, current_version: &str) -> Result<(), String>`
- `discard_pending(app_data_dir: &Path) -> Result<UpdaterStatus, String>`
- `check_now(config: &crate::config::UpdaterConfig, silent: bool) -> Result<UpdaterStatus, String>`

For this slice, `check_now` should:

- set phase to `Checking`
- if a pending update exists, return `ReadyToInstall`
- otherwise set `UpToDate` with no download
- clear `last_error`
- record `last_check_silent`

- [ ] **Step 3: Add Tauri commands**

In `src-tauri/src/commands.rs`, add:

- `get_updater_status`
- `check_for_updates_now`
- `install_pending_update`
- `discard_pending_update`

Behavior for this slice:

- `get_updater_status` returns manager status
- `check_for_updates_now` calls manager `check_now`
- `install_pending_update` returns an error if no pending update exists, otherwise sets `Installing` then returns current status placeholder
- `discard_pending_update` clears pending state and returns refreshed status

- [ ] **Step 4: Wire updater manager in app setup and invoke handler**

In `src-tauri/src/lib.rs`:

- create `UpdaterManager` with package version (`env!("CARGO_PKG_VERSION")`)
- restore pending state during setup
- manage it in app state
- register updater commands in `invoke_handler`
- emit initial updater status after setup

- [ ] **Step 5: Run backend tests**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml updater -- --nocapture
```

Expected: PASS

- [ ] **Step 6: Commit command/bootstrap wiring**

```bash
git add src-tauri/src/commands.rs src-tauri/src/lib.rs src-tauri/src/updater src-tauri/src/config.rs
git commit -m "feat(updater): add command wiring and pending-status restoration"
```

### Task 3: Add frontend updater types/store and Settings UI controls

**Files:**

- Modify: `src/types/index.ts`
- Create: `src/stores/updaterStore.ts`
- Modify: `src/components/SettingsPanel.vue`
- Modify: `src/i18n/locales/en.ts`
- Modify: `src/i18n/locales/zh-CN.ts`
- Test: `tests/unit/updaterStore.test.ts`
- Test: `tests/unit/SettingsPanel.updater.test.ts`

- [ ] **Step 1: Write failing Vitest coverage for updater store and Settings panel**

Create tests covering:

- updater dependent toggles disable correctly
- invalid mirrors block save with inline feedback
- clicking version row calls manual check action
- ready-to-install state renders install/discard actions
- discard action calls backend and refreshes state

Run:

```bash
pnpm vitest run tests/unit/updaterStore.test.ts tests/unit/SettingsPanel.updater.test.ts
```

Expected: FAIL because store/UI do not exist yet.

- [ ] **Step 2: Add updater types to frontend**

In `src/types/index.ts`, add:

- `UpdaterConfig`
- `PendingUpdateRecord`
- `UpdaterPhase`
- `UpdaterStatus`
  And extend existing `AppConfig`-equivalent usage to include `updater`.

- [ ] **Step 3: Implement updater store**

Create `src/stores/updaterStore.ts` with state/actions:

- `status`
- `isChecking`
- `loadStatus()`
- `checkNow()`
- `installPending()`
- `discardPending()`
- `bindEvents()`
  Use Tauri invoke/event APIs with `updater-status-changed`.

- [ ] **Step 4: Update SettingsPanel UI**

In `src/components/SettingsPanel.vue`:

- extend local `AppConfig` type with `updater`
- add updater section controls from design
- add mirror textarea and inline validation
- add clickable version row that triggers `checkNow`
- render phase-specific state text
- render pending update confirmation block with install/discard buttons
- keep save routed through `update_config`

- [ ] **Step 5: Add i18n strings**

Add updater labels/messages in English and Chinese for controls, hints, status text, buttons, validation errors.

- [ ] **Step 6: Run web tests**

Run:

```bash
pnpm vitest run tests/unit/updaterStore.test.ts tests/unit/SettingsPanel.updater.test.ts
```

Expected: PASS

- [ ] **Step 7: Commit frontend updater UI slice**

```bash
git add src/types/index.ts src/stores/updaterStore.ts src/components/SettingsPanel.vue src/i18n/locales/en.ts src/i18n/locales/zh-CN.ts tests/unit/updaterStore.test.ts tests/unit/SettingsPanel.updater.test.ts
git commit -m "feat(updater): add settings UI and frontend runtime store"
```

### Task 4: Add release config scaffolding and full verification

**Files:**

- Modify: `src-tauri/tauri.conf.json`
- Modify: `.github/workflows/release.yml`
- Modify: `README.md`
- Modify: `README.zh-CN.md`

- [ ] **Step 1: Update Tauri updater bundle config**

In `src-tauri/tauri.conf.json`, enable updater artifact generation and document env-based key expectations in the relevant config block.

- [ ] **Step 2: Update release workflow skeleton**

In `.github/workflows/release.yml`, ensure the workflow uploads `latest.json`, platform updater artifacts, and signatures.

- [ ] **Step 3: Run project verification**

Run:

```bash
pnpm test:web
cargo test --manifest-path src-tauri/Cargo.toml
pnpm typecheck
```

Expected: PASS

- [ ] **Step 4: Commit release scaffolding**

```bash
git add src-tauri/tauri.conf.json .github/workflows/release.yml README.md README.zh-CN.md
git commit -m "chore(updater): add release scaffolding for updater artifacts"
```
