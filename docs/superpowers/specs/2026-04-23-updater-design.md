# Managed Updater Design Spec

## Overview

Add a managed application-update flow to Smart Clipboard so users can:

- configure whether the app checks for updates automatically
- configure how often automatic checks run
- configure whether the latest installer should be downloaded automatically
- restrict automatic downloads to Wi-Fi only
- configure multiple GitHub mirror endpoints for regions where GitHub access is unreliable
- click the version row in Settings to run a manual update check
- review a downloaded update and explicitly choose whether to install and restart

The design keeps GitHub Releases as the canonical distribution source while adding a mirror-aware, silent-failure-tolerant experience inside the app.

## Current State

The repository currently has:

- a `SettingsPanel.vue` modal that loads and saves a single `AppConfig`
- no updater-specific configuration, commands, store, or UI
- a GitHub release workflow that builds Tauri artifacts and publishes release assets
- no application-level mechanism for update checks, package download, or in-app installation

The release workflow already gives us the right distribution shape to build on. Tauri's updater documentation supports generating updater artifacts and a static `latest.json` file that can be hosted on GitHub Releases, which makes GitHub a suitable canonical source for update discovery and package download without introducing a separate update server.

## Goals

- add updater preferences to Settings without disrupting existing config flows
- check for updates silently in the background when enabled
- allow manual checks from the Settings version row even when automatic checks are disabled
- allow automatic package download after a successful check
- support a Wi-Fi-only guard for automatic downloads
- support multiple user-configurable GitHub mirror endpoints
- download the installer to local storage first, then wait for explicit user confirmation before installation
- persist a downloaded pending update across app restarts until the user installs it or explicitly discards it
- discard the local installer only when the user explicitly chooses to cancel the pending update
- tolerate background check failures silently so clipboard features continue working normally

## Non-Goals

- redesigning the entire Settings layout beyond the updater section needed for this feature
- adding toast-heavy notification flows for background failures
- introducing a separate hosted update service outside GitHub Releases in this iteration
- adding delta patches, channel switching, or prerelease opt-in in this iteration
- changing release branching or tag strategy beyond the updater artifact requirements

## User Decisions Locked In

The following product decisions were confirmed during brainstorming and are part of this design:

- automatic download stores the installer locally first
- installation only starts after the user explicitly confirms
- if the user explicitly cancels the pending update, the cached installer is deleted
- if the user closes Settings without confirming installation, the cached installer is retained
- if Wi-Fi state cannot be determined reliably, the app treats the network as not eligible for Wi-Fi-only auto-download and only performs the check

## Recommended Approach

Use a hybrid updater design:

- GitHub Releases remains the source of truth for release assets
- the GitHub release workflow generates Tauri updater artifacts plus a static `latest.json`
- the app performs its own mirror-aware manifest fetch and asset download flow
- the app verifies downloaded updater artifacts against the configured updater public key before marking them ready
- the app performs installation from the verified local pending artifact only after explicit confirmation

This is preferred over relying entirely on Tauri's one-shot updater flow because the built-in endpoint fallback behavior is not sufficient for the required mirror strategy, and the product requirement explicitly separates download from installation.

## Source of Truth and Distribution Model

### Canonical release metadata

The canonical update manifest URL is the GitHub Releases asset:

`https://github.com/Z-Only/smart-clipboard/releases/latest/download/latest.json`

This file is generated as part of the release build and published alongside the platform-specific updater artifacts and signatures.

### Mirror model

Users can configure zero or more mirror templates. Each mirror entry must use `https://` and contain the `{url}` placeholder. At runtime, the app substitutes `{url}` with the full canonical GitHub URL and tries mirrors in the configured order before trying the canonical GitHub URL directly.

Examples:

- `https://mirror.example/{url}`
- `https://ghproxy.example/{url}`

Blank lines are ignored. Invalid mirror entries are rejected during save.

### Why this model

This keeps GitHub Releases as the real source of packages while still allowing proxy-style mirrors for the manifest and the platform asset itself. It avoids a second release publishing system and keeps the release workflow aligned with Tauri's updater artifact format.

## Architecture

```text
SettingsPanel / App.vue
        │
        ▼
   updaterStore.ts
        │
        ├─ get status
        ├─ check now
        ├─ install pending
        └─ discard pending
        │
        ▼
      commands.rs
        │
        ▼
   updater manager
        │
        ├─ manifest fetcher
        ├─ mirror resolver
        ├─ network policy
        ├─ downloader
        ├─ pending state store
        └─ install adapter
```

## Backend Design

Add a new `src-tauri/src/updater/` module with the following responsibilities.

### 1. Manifest fetcher

Responsibilities:

- resolve ordered candidate manifest URLs from configured mirrors plus canonical GitHub
- fetch the first successfully reachable `latest.json`
- parse release metadata for the current target
- compare the manifest version against the currently running version

Rules:

- only a strictly newer version counts as available
- prerelease and draft channels are ignored by only consuming the published `latest.json`
- manual checks bypass the interval gate
- automatic checks honor the interval gate

### 2. Mirror resolver

Responsibilities:

- validate mirror template syntax on save
- build candidate URLs for manifest fetch and asset download
- preserve user ordering

Rules:

- configured mirrors are always attempted before canonical GitHub
- canonical GitHub is always appended implicitly as the final fallback
- malformed mirror templates are not saved
- mirror failures never block the fallback to the next candidate

### 3. Network policy

Responsibilities:

- determine whether automatic download is allowed under current preferences
- expose a single decision point used by the manager before starting any background download

Rules:

- if `auto_download_enabled` is `false`, never auto-download
- if `wifi_only` is `false`, any network is eligible
- if `wifi_only` is `true` and Wi-Fi can be positively identified, auto-download is allowed
- if `wifi_only` is `true` and Wi-Fi cannot be identified or the detection is uncertain, auto-download is denied

The implementation can use platform-specific detection, but the behavior above is the product contract.

### 4. Downloader

Responsibilities:

- download the platform-specific updater artifact to a deterministic cache directory
- emit progress updates
- verify the downloaded artifact signature before marking it ready
- remove partial files on failure

Storage layout:

- pending files live under `app-data/updates/pending/<version>/`
- the downloaded artifact and its signature are stored together in that directory

Rules:

- only one active download runs at a time
- a newly downloaded version replaces any older pending update
- download failures remove partial files and do not leave stale pending state behind

### 5. Pending state store

Responsibilities:

- persist the ready-to-install update record across restarts
- restore pending state when the app starts

Persisted file:

- `app-data/updates/pending.json`

Suggested shape:

```json
{
  "version": "2.2.0",
  "release_date": "2026-04-23T10:30:00Z",
  "current_version": "2.1.0",
  "notes": "...",
  "artifact_path": "/abs/path/to/file",
  "signature_path": "/abs/path/to/file.sig",
  "canonical_asset_url": "https://github.com/...",
  "source_asset_url": "https://mirror.example/https://github.com/...",
  "downloaded_at": "2026-04-23T10:35:00Z"
}
```

Rules:

- pending state exists only after the artifact has been fully downloaded and verified
- pending state is cleared only after successful installation or explicit discard
- pending state survives app restart

### 6. Install adapter

Responsibilities:

- take a verified local pending updater artifact
- perform the platform-specific replace/install step
- restart the app after a successful handoff

Rules:

- installation never starts automatically after download
- installation only starts from explicit user action
- installation is driven from the local verified artifact, not a fresh network fetch
- if installation startup fails, the app reports the error and clears invalid half-written state

Implementation note:

This adapter should mirror the final platform-specific install behavior expected by Tauri updater artifacts, but it remains an app-owned component because the product requirement splits download from installation. A short-lived helper process may be used where necessary so the running executable can exit before replacement.

## Configuration Model

Extend `AppConfig` with a new `updater` section.

```rust
pub struct UpdaterConfig {
    pub auto_check_enabled: bool,
    pub check_interval_hours: u64,
    pub auto_download_enabled: bool,
    pub wifi_only: bool,
    pub mirrors: Vec<String>,
    pub last_check_at: Option<String>,
}
```

Default values:

- `auto_check_enabled = true`
- `check_interval_hours = 24`
- `auto_download_enabled = false`
- `wifi_only = true`
- `mirrors = []`
- `last_check_at = None`

Reasons for these defaults:

- users get passive update awareness by default
- automatic package download remains opt-in
- Wi-Fi-only is the safer default once automatic download is enabled
- empty mirrors keeps GitHub as the default path

`UpdaterConfig` belongs inside `AppConfig`, but pending installer state does not. Preferences are durable settings; pending update records are operational state and should remain in the dedicated updater storage area.

Frontend TypeScript config types must mirror this structure.

## Runtime Status Model

Expose updater runtime state separately from preferences.

Suggested shape:

```rust
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
```

The backend should expose a serializable status payload that includes:

- current phase
- current installed version
- available version, if any
- pending update metadata, if any
- download progress, if any
- last human-readable error for manual actions
- whether the last check was silent or user-triggered

The frontend store should subscribe to a single updater status event and refresh itself from command results when needed.

## Command Design

Add dedicated updater commands instead of overloading the existing generic config commands.

Recommended commands:

- `get_updater_status`
- `check_for_updates_now`
- `install_pending_update`
- `discard_pending_update`

`get_config` and `update_config` continue to own preference persistence. The updater commands own operational behavior and runtime state.

Command behavior:

### `check_for_updates_now`

- bypasses the automatic interval gate
- fetches manifest using mirror-aware ordering
- returns `UpToDate`, `UpdateAvailable`, or `ReadyToInstall`
- if a verified pending update already exists for a newer version, returns `ReadyToInstall` immediately and does not start a second download
- if auto-download is enabled and allowed by network policy, starts download immediately
- if auto-download is disabled or disallowed, stops after status update
- surfaces errors to the caller because the user explicitly asked for the check

### background automatic check

- triggered by manager bootstrap when `auto_check_enabled` is true and the interval is due
- skipped when a verified pending update already exists
- behaves like `check_for_updates_now` except failures are logged silently and only update internal state
- never blocks clipboard usage

### `install_pending_update`

- validates that pending state exists and files are present
- performs the install handoff
- restarts the app on success

### `discard_pending_update`

- deletes the pending artifact directory
- deletes `pending.json`
- emits status update back to `UpdateAvailable` if the manager still knows about a newer version, otherwise `Idle`

## Application Lifecycle

### Startup

At backend setup:

- load updater preferences from `AppConfig`
- restore `pending.json` if present
- publish initial updater status
- start a detached background task that evaluates whether an automatic check is due

The startup task must remain low priority and non-blocking.

If a verified pending update is restored, startup does not start another automatic download for that version.

### Settings open

When the Settings panel opens:

- load the latest `AppConfig`
- load updater runtime status
- show any existing pending update immediately

### Manual check

The version row at the bottom of Settings is clickable.

Behavior:

- click starts `check_for_updates_now`
- while checking, the row shows a loading state
- if no update is available, the UI shows an explicit "already up to date" result
- if an update is available, the UI shows version and release summary
- if a pending update already exists, the UI shows the ready-to-install actions instead of starting another download

## Settings UI Design

Add an `Application Updates` section to `SettingsPanel.vue`.

Controls:

- `Automatically check for updates`
- `Check frequency`
- `Automatically download latest installer`
- `Only auto-download on Wi-Fi`
- `Mirror endpoints`

Version footer row:

- label: `Current version {version}`
- hint: `Click to check for updates`

Validation and enablement rules:

- `Check frequency` is disabled when automatic checks are off
- `Only auto-download on Wi-Fi` is disabled when automatic download is off
- mirror textarea accepts one entry per line
- invalid mirrors block save and show inline feedback
- a valid mirror must start with `https://` and include `{url}`

Check frequency options are fixed:

- `Every 6 hours`
- `Every 12 hours`
- `Daily`
- `Weekly`

The UI stores those values as `6`, `12`, `24`, or `168`.

## User-Facing Update States

The Settings updater section should render one clear state at a time.

Examples:

- `Checking for updates...`
- `You are up to date`
- `Version 2.2.0 is available`
- `Downloading installer... 42%`
- `Installer ready to install`
- `Update check failed`

Manual checks may show inline error text. Automatic background failures do not show blocking error UI.

## Confirmed Installation UX

When a verified pending installer exists, the Settings updater section shows a confirmation block with:

- release version
- optional release notes excerpt
- download timestamp
- `Install and restart`
- `Cancel and delete installer`

Rules:

- closing Settings does not discard the pending update
- app restart does not discard the pending update
- only the explicit cancel action discards the pending update

This matches the agreed product rule labeled "A" during brainstorming.

## Error Handling

### Silent background failures

Automatic checks and automatic downloads may fail silently in the sense that they must not interrupt clipboard usage with modal errors or forced retries.

Allowed behavior:

- log the error
- update internal runtime status
- keep the app fully usable

### Manual action failures

Failures caused by explicit user actions should be visible inline in Settings:

- manual check failure
- download failure initiated by manual check
- install failure
- discard failure

### Cleanup rules

- partial download files are always deleted on download failure
- corrupted or unverifiable pending artifacts are deleted immediately and pending state is cleared
- explicit discard removes the entire pending version directory and metadata file

## Signature Verification

Downloaded updater artifacts must be verified with the updater public key before they are treated as installable.

Required behavior:

- manifest and artifact download alone is not enough to mark an update ready
- verification failure is treated as a hard error for the current update
- failed verification removes the downloaded files and clears pending state

The public key is the same one used by Tauri updater artifact signing and is stored in Tauri configuration for release compatibility.

## Release Pipeline Changes

Update the release pipeline so GitHub Releases carries everything the app needs:

- enable `bundle.createUpdaterArtifacts = true`
- configure updater signing with the Tauri public/private key pair
- ensure `latest.json` is uploaded as a release asset
- ensure the platform-specific updater artifact and signature are uploaded for each target

Expected outcome:

- GitHub Releases remains the canonical source
- mirrors proxy GitHub instead of replacing it
- the app can fetch `latest.json` and platform assets without a custom update server

## Testing Strategy

### Frontend tests

Add Vitest coverage for:

- rendering updater preferences in `SettingsPanel.vue`
- disabling dependent controls correctly
- saving valid updater preferences through config update
- rejecting invalid mirror entries
- clicking the version row to trigger a manual check
- rendering pending update actions when runtime status reports a ready installer
- discarding a pending update from the UI

### Backend unit tests

Add Rust coverage for:

- mirror URL resolution order
- mirror validation
- interval-due calculation
- Wi-Fi-only policy fallback when network type is unknown
- pending state read/write/clear behavior
- cleanup on failed download or failed verification
- version comparison against current app version

### Backend integration tests

Add command-level coverage for:

- manual check command updating runtime status
- discard command deleting persisted pending state
- startup restoration of a pending update
- automatic check scheduling not blocking initialization

## File and Module Impact

Expected primary touch points:

- `src/components/SettingsPanel.vue`
- `src/App.vue`
- `src/types/index.ts`
- `src-tauri/src/config.rs`
- `src-tauri/src/commands.rs`
- `src-tauri/src/lib.rs`
- `src-tauri/src/updater/` new module tree
- `src/i18n/locales/en.ts`
- `src/i18n/locales/zh-CN.ts`
- `tests/unit/` updater-related UI tests
- `src-tauri/tauri.conf.json`
- `.github/workflows/release.yml`

## Final Scope Boundary

This package delivers:

- updater preferences in Settings
- mirror-aware GitHub-based update discovery
- silent background checks with configurable frequency
- optional auto-download with Wi-Fi-only enforcement
- local pending installer persistence
- explicit install-or-discard confirmation flow
- release pipeline support for updater artifacts

It does not deliver:

- prerelease channels
- differential updates
- passive toast notifications outside the Settings surface
- a separate hosted update service
