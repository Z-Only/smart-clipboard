# Changelog

## [0.6.0] - 2026-04-20
- Added Phase 3 end-to-end encryption and secure pairing for LAN sync, including X25519 key exchange, HKDF-SHA256 shared secret derivation, AES-256-GCM encrypted messaging, key fingerprint verification, and persistent key storage.
- Added Phase 3 WebSocket transport skeleton with backend server/client handshake, protocol message framing, heartbeat ping/pong, reconnect backoff, and paired-device runtime connection states.
- Updated SyncPanel device status rendering to show more realistic transport states such as connecting, connected, disabled, and reconnecting-related activity.

### Added
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
- Replaced demo discovered devices with real mDNS discovery results while keeping the existing Sync panel UI contract stable.
- Updated Phase 3 scope: this release now ships real LAN discovery, WebSocket transport, and end-to-end encryption. Actual clipboard payload sync across devices remains for follow-up work.

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/), and this project adheres to [Semantic Versioning](https://semver.org/).

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
- **Auto language detection**: Defaults to system language (Chinese for zh-* locales, English otherwise)
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