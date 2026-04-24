# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/), and this project adheres to [Semantic Versioning](https://semver.org/).

## [2.4.0] - 2026-04-25

### Added

- **Database-at-rest encryption**: Optional AES-256-GCM application-layer encryption for clipboard entry content, with encryption keys stored securely in the OS keyring (macOS Keychain / Windows Credential Manager / Linux Secret Service)
- **Encryption engine module**: New `src-tauri/src/encryption.rs` with key management, transparent encrypt/decrypt, and batch data migration support
- **Encryption Tauri commands**: `get_encryption_status`, `enable_encryption`, `disable_encryption` with automatic migration of all existing entries
- **Encryption settings UI**: Toggle switch in Settings panel with real-time status display (encrypted/unencrypted entry counts, migration progress)
- **Encryption i18n**: English and Chinese translations for all encryption-related labels and messages

### Changed

- **Version bump to 2.4.0**: Database-at-rest encryption as a minor release
- **commands.rs**: All read-path commands (`get_entries`, `search_entries`, `paste_entry`, `copy_entries`, `get_entries_by_tag`, `get_statistics`) now transparently decrypt encrypted entries
- **Clipboard write path**: New entries are automatically encrypted when encryption is enabled

## [2.3.0] - 2026-04-24

### Added

- **Native Touch ID unlock (macOS)**: Real biometric authentication via the LocalAuthentication framework (`LAContext`), replacing the previous `osascript`-based convenience path
- **Native Windows Hello unlock (Windows)**: Biometric authentication via `UserConsentVerifier` (fingerprint, face, PIN)
- **Platform-specific biometric module**: New `src-tauri/src/biometric.rs` encapsulating all platform FFI with `#[cfg(target_os)]` conditional compilation and test injection helpers
- **Biometric unit tests**: 5 new tests covering availability injection, successful unlock, user cancel, error handling, and settings auto-downgrade when biometric is unavailable

### Changed

- **Version bump to 2.3.0**: Native biometric integration as a minor release
- **security.rs refactored**: Inline biometric functions removed; now delegates to the dedicated `biometric` module via re-exports for backward compatibility

### Dependencies

- Added `objc2-local-authentication`, `objc2-foundation`, `objc2`, `block2` (macOS)
- Added `windows` crate with `Security_Credentials_UI` and `Foundation` features (Windows)

## [2.2.0] - 2026-04-24

### Added

- **Managed updater backend**: Full Rust updater module (`src-tauri/src/updater/`) with manifest fetching, mirror resolution, artifact download with progress, minisign signature verification, pending update persistence, and install handoff
- **Updater config model**: `UpdaterConfig` in `AppConfig` with auto-check, interval, auto-download, Wi-Fi-only, and custom mirror endpoint support
- **Updater Tauri commands**: `get_updater_status`, `check_for_updates_now`, `download_available_update`, `install_pending_update`, `discard_pending_update`, and `quit_app`
- **Frontend updater store**: Pinia store with reactive status, event binding, and actions for check, download, install, and discard flows
- **Settings updater UI**: Version row with manual check, auto-check toggles, check frequency selector, auto-download and Wi-Fi-only options, mirror endpoint textarea with inline validation, phase-specific status text, download progress, pending update install/discard actions, and quit button during install handoff
- **Release scaffolding**: Enabled `createUpdaterArtifacts` in Tauri config, added signing key validation and updater artifact upload to the release workflow
- **Updater i18n**: English and Chinese translations for all updater labels, hints, status text, and validation messages
- **Updater test coverage**: 34 Rust unit tests covering mirrors, pending persistence, policy helpers, manager workflows, signature verification, and download progress; 11 Vitest tests covering the frontend store and Settings panel updater section

### Changed

- **Version bump to 2.2.0**: Added managed updater as a minor release

## [2.1.0] - 2026-04-21

### Fixed

- **Docs site link integrity**: Fixed the homepage CTA so it resolves correctly on GitHub Pages with the repository base path instead of jumping to an invalid root-level URL
- **Base-aware favicon path**: Updated the VitePress head icon configuration so deployed pages load the favicon from the correct GitHub Pages subpath

### Changed

- **Version bump to 2.1.0**: Promoted the docs-site hotfix and deployment-path cleanup as a minor release

## [2.0.0] - 2026-04-21

### Added

- **Phase 4 access security**: Added app lock with password enable/disable flow, startup lock gate, manual lock, tray/hotkey wakeup interception, auto-lock, and a lock screen managed primarily by Rust-side security logic
- **Secure local password handling**: Added Argon2 password hashing with OS credential store persistence via keyring; passwords are never stored in plaintext in config or database
- **Biometric/system-auth convenience unlock path**: Added a convenience unlock path on supported platforms with mandatory password fallback on failure
- **Rust-side command guards**: Added access-control checks for clipboard, sync, WebDAV, and template commands so locked apps refuse sensitive command access
- **Frontend sensitive-state clearing**: Added lock-triggered clearing of clipboard, sync, template, WebDAV, and statistics state with automatic reload after unlock
- **Security regression tests**: Added password/hash tests, command-guard tests, and template-guard tests to validate lock/unlock behavior contracts

### Changed

- **Version bump to 2.0.0**: Promoted the project to a major release now that clipboard management, sync, templates, and Phase 4 access security are all implemented together
- Updated README documentation in English and Chinese to reflect completed Phase 4 capabilities, current platform behavior, and future roadmap items

### Notes

- Native Touch ID / LocalAuthentication integration is still planned as a future improvement; current macOS behavior uses a convenience system-auth path rather than a full native biometric bridge
- Full invoke-level black-box runtime tests are still planned; current tests cover the guard contracts and core security behavior in Rust

## [1.0.0] - 2026-04-21

### Added

- **WebDAV cloud sync**: End-to-end encrypted clipboard sync via WebDAV (compatible with Jianguoyun/Nextcloud/Synology), with Argon2id password-derived key, AES-256-GCM file encryption, ETag-based conflict resolution, token-bucket rate limiting, configurable poll interval, and device registry
- **LAN sync (production-ready)**: Complete mDNS discovery + encrypted WebSocket transport with X25519 key exchange, real-time clipboard broadcast, dedup-by-hash, loop prevention, heartbeat monitoring, and automatic reconnection with exponential backoff
- **Platform-aware source tracking**: Captures the frontmost application name for each clipboard entry (macOS via osascript, Linux via xdotool, Windows via PowerShell) and enforces excluded-apps filtering
- **mDNS log noise suppression**: Custom log filter to silence harmless "Network is down" errors from mdns_sd on macOS awdl0 interface

### Changed

- Bumped version to **1.0.0** — all three development phases (MVP, Smart Enhancements, Sync & Templates) are now complete

### Notes

- Phase 4 roadmap items (not yet implemented at the time): SQLCipher database encryption, biometric/password app lock, pinyin fuzzy search, virtual scrolling for large lists, and batch operations (multi-select merge paste / bulk delete)

## [0.6.0] - 2026-04-20

- **Phase 3 UI polish & validation**: Added pairing confirmation dialog, SyncPanel loading/refreshing states, enhanced error banner with dismiss, status dot indicator, and i18n additions for sync UI.
- Added Phase 3 end-to-end encryption and secure pairing for LAN sync, including X25519 key exchange, HKDF-SHA256 shared secret derivation, AES-256-GCM encrypted messaging, key fingerprint verification, and persistent key storage.
- Updated SyncPanel device status rendering to show more realistic transport states such as connecting, connected, disabled, and reconnecting-related activity.
- **Added Phase 3 clipboard sync business flow** — new clipboard entries are now automatically synced to paired devices over encrypted WebSocket connections.

### Added

#### Clipboard Sync Flow (new)

- Added `ClipboardSync` protocol message and `SyncEntryPayload` for real clipboard entry synchronization between devices.
- Added `source_device` column to `clipboard_entries` table to track sync origin and prevent loop syncing.
- Added `sync_log` database methods (`insert_sync_log`, `has_sync_log`, `has_received_entry`) for deduplication and audit.
- Added `broadcast_entry()` in SyncManager — new local clipboard entries are broadcast to all connected paired devices via a `tokio::broadcast` channel.
- Added `handle_incoming_sync()` in SyncManager — incoming entries are deduplicated (by hash + sync_log), stored with `source_device` metadata, and trigger frontend refresh via `clipboard-changed` event.
- Added sync filtering based on configuration: `enabled`, `auto_sync`, `sync_images`, `sync_sensitive`, and 1MB payload size limit.
- Added loop sync prevention: entries with `source_device` set are never re-broadcast.
- Added outgoing broadcast support in both WebSocket server and client connections via `tokio::select!`.
- Hooked clipboard monitor into sync pipeline — after local DB insert, entries are automatically broadcast to paired devices.

#### Previous Phase 3 additions

- Added X25519 key pair generation with persistent storage in app config for device identity.
- Added HKDF-SHA256 shared secret derivation from Diffie-Hellman key exchange during device pairing.
- Added AES-256-GCM encryption and decryption for all sync protocol messages between paired devices.
- Added human-readable key fingerprint generation (SHA-256, 8-byte hex format) for pairing verification UI.
- Added `KeyVerification` protocol message type for mutual pairing confirmation flow.
- Added `fingerprint` column to `paired_devices` database table with automatic migration.
- Added automatic fingerprint computation and persistence during the pairing secret establishment flow.
- Added 7 unit tests for crypto module covering key derivation, encryption roundtrip, fingerprint format, and error cases.
- Added a Phase 3 LAN Sync MVP management experience with a dedicated Sync panel in the desktop UI.
- Added sync configuration, discovered-device list, paired-device list, and per-device enable/disable controls.
- Added backend sync scaffolding with persisted sync config, paired devices storage, and Tauri sync commands.
- Added real mDNS / DNS-SD LAN discovery with `_smartclip._tcp.local.` service advertisement and browsing.
- Added discovered-device deduplication, last-seen refresh, and online/offline status updates in the Phase 3 sync backend.

### Changed

- Updated the app version to 0.6.0 for the Phase 3 partial delivery.
- Phase 3 now ships a complete clipboard sync flow: LAN discovery → pairing → encrypted WebSocket transport → real-time clipboard entry synchronization with dedup and loop prevention.

## [0.5.0] - 2026-04-20

### Added

- **Clipboard templates**: Reusable text snippets with parameterized `{{placeholder}}` syntax — create, edit, delete, and organize templates by category
- **Template fill dialog**: When using a template with placeholders, a dialog prompts for each value with a live preview of the rendered result
- **Template engine**: Rust-based placeholder extraction and rendering with support for duplicate placeholders and special characters
- **Template management UI**: Dedicated panel accessible from the top bar with category filtering, search, and inline actions (use, edit, delete)
- **Template i18n**: Full English and Chinese support for all template-related UI

## [0.4.0] - 2026-04-20

### Added

- **Sensitive info detection**: Automatically detects passwords, API keys, tokens, private keys, JWTs, and connection strings in clipboard content using 7 regex patterns
- **Auto-expiry for sensitive content**: Sensitive entries are flagged and optionally auto-deleted after a configurable time (default: 5 minutes)
- **Content transforms**: 12 one-click text transformations — UPPERCASE, lowercase, Title Case, URL Encode/Decode, JSON Format/Compact, Base64 Encode/Decode, Trim Whitespace, HTML Escape/Unescape
- **Tag management**: User-defined tags for organizing clipboard entries with create/delete/assign operations, tag chips on entry cards, and sidebar tag filtering
- **Image clipboard support**: Captures images from clipboard, stores as PNG files, displays thumbnails in the entry list, and supports copying images back to clipboard
- **Usage statistics panel**: Dashboard showing total entries, favorites count, storage size, category distribution (horizontal bar chart), daily activity (30-day vertical bar chart), and top 10 most-used entries
- **New dependencies**: `base64` (transforms), `image` (PNG encoding for image clipboard)

## [0.3.0] - 2026-04-20

### Added

- **Appearance mode switching**: System / Light / Dark mode with OS preference detection via `prefers-color-scheme` media query
- **Multi-theme color selection**: 6 color themes — Zinc (default), Blue, Green, Rose, Orange, Violet
- **Theme composable**: `useTheme` composable with reactive state, localStorage persistence, and real-time OS theme listener
- **Theme CSS variables**: OKLCh color overrides for primary/ring/sidebar colors in both light and dark variants
- **Settings UI**: Appearance toggle buttons and color picker dots integrated into SettingsPanel
- **i18n support**: Theme-related labels in both English and Chinese

## [0.2.0] - 2026-04-20

### Added

- **Multi-language support (i18n)**: Full internationalization with vue-i18n, supporting English and Chinese
- **Language switcher**: Users can switch between English and Chinese in the Settings panel
- **Auto language detection**: Defaults to system language (Chinese for zh-\* locales, English otherwise)
- **Bilingual README**: Separate README.md (English) and README.zh-CN.md (Chinese) with language switch links

## [0.1.0] - 2026-04-20

### Added

- **Clipboard monitoring**: Background polling (500ms) with SHA-256 change detection via arboard
- **Smart classification**: Regex rule chain auto-categorizes content into URL, Email, Color, FilePath, JSON, XML, Code, Phone, Address, and Text
- **SQLite storage**: Persistent storage with FTS5 full-text search, deduplication by content hash, and auto-cleanup
- **Vue 3 frontend**: History list with date grouping, real-time updates, search bar with debounced input, and category sidebar filter
- **Global hotkey**: `Cmd/Ctrl + Shift + V` toggles the clipboard panel with auto-focus on search
- **System tray**: Tray icon with context menu (Show/Settings/Quit), left-click toggle, close-to-tray behavior
- **Settings panel**: Configurable max entries, retention days, monitor interval, excluded apps, and auto-start toggle
- **Auto-start**: Optional launch on system login via tauri-plugin-autostart
- **Favorites**: Star entries to keep them permanently (exempt from auto-cleanup)
- **Keyboard navigation**: Arrow keys to navigate entries, Enter to paste, Escape to hide window
