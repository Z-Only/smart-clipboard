[English](README.md) | [中文](README.zh-CN.md)

# Smart Clipboard Manager

A cross-platform, lightweight smart clipboard manager built with **Tauri 2** + **Vue 3** + **Rust**. It runs in the background, automatically captures and classifies clipboard content, supports secure local protection, and provides fast search, retrieval, and sync workflows.

- Website / Docs: https://z-only.github.io/smart-clipboard/
- Repository: https://github.com/Z-Only/smart-clipboard

## Release Status — v2.6.0 Pinyin Fuzzy Search

This repository now includes **Pinyin Fuzzy Search** on top of advanced sync conflict handling, clipboard history, smart enhancements, templates, LAN sync, WebDAV cloud sync, Phase 4 access security, native biometric integration, the managed updater, and database-at-rest encryption.

### v2.6.0 highlights

- **Pinyin fuzzy search**: Search Chinese clipboard entries by original Hanzi, full pinyin, or initials
- **Short-input fallback**: Very short inputs such as `zn` or `z` still return matches via LIKE fallback
- **Case-insensitive initials**: Uppercase queries such as `ZNJTB` match lowercase pinyin indexes transparently
- **Improved search guidance**: Search placeholder text now explicitly explains support for text / pinyin / initials

## Features

- **Clipboard History** -- Automatically captures copied content with deduplication
- **Smart Classification** -- Auto-categorizes content into URL, Email, Code, JSON, FilePath, Color, Phone, Address, Image, and plain Text
- **Full-Text Search** -- Fast search powered by SQLite FTS5, with Chinese pinyin full-text and initials matching
- **Category Filter** -- Browse clipboard history by content type
- **Global Hotkey** -- `Cmd/Ctrl + Shift + V` to toggle the clipboard panel
- **System Tray** -- Runs in the background with tray controls
- **Favorites** -- Pin frequently used entries to prevent auto-cleanup
- **Configurable** -- Max entries, retention period, excluded apps, monitor interval, and sensitive-content expiry
- **Auto-Start** -- Optionally launch on system login
- **Appearance Modes** -- System / Light / Dark mode
- **Theme Colors** -- 6 built-in color themes
- **Multi-Language** -- English and Chinese UI support
- **Sensitive Detection** -- Detects passwords, API keys, tokens, JWTs, and connection strings with optional auto-expiry
- **Content Transforms** -- One-click case, encoding, and formatting transforms
- **Tag Management** -- Custom tags for organizing entries
- **Image Clipboard** -- Captures and displays clipboard images with PNG storage
- **Usage Statistics** -- Dashboard with category breakdown, daily activity, and most-used entries
- **Clipboard Templates** -- Reusable text snippets with `{{placeholder}}` syntax and fill-in dialog
- **LAN Sync** -- Encrypted peer-to-peer sync over mDNS + WebSocket
- **WebDAV Cloud Sync** -- End-to-end encrypted cloud sync with device registry, polling, and rate limiting
- **Sync Conflict Handling** -- Smart conflict detection with configurable resolution strategies and manual diff UI
- **Pinyin Fuzzy Search** -- Search Chinese entries by Hanzi, full pinyin, and initials with short-query fallback
- **App Access Security** -- Password lock, auto-lock, guarded wakeups, and secure unlock flow
- **Managed Updater** -- Background update checks, mirror endpoints, artifact download with progress, signature verification, and install handoff
- **Database Encryption** -- Optional AES-256-GCM encryption for clipboard data at rest, with keys in the OS keyring
- **Lightweight** -- Small binary with low CPU/memory usage thanks to Rust + native WebView

## Security Model (Phase 4)

### What is protected

When app lock is enabled:

- The main window starts locked
- Tray and hotkey wakeups are intercepted before showing protected content
- Sensitive Tauri commands refuse access while locked
- Frontend cached sensitive state is cleared when the app locks
- Biometric/system-auth unlock failures fall back to password unlock

### Password storage

- The password is **not** stored in plaintext
- Rust hashes the password with **Argon2**
- Only the password hash is stored, and it is saved in the **OS credential store** via keyring integration
- App config stores only app-lock settings such as enabled state, timeout, and biometric preference

### Current platform behavior

- **macOS**: Password lock, auto-lock, tray/hotkey interception, and native Touch ID unlock via LocalAuthentication framework
- **Windows**: Password lock, auto-lock, tray/hotkey interception, and native Windows Hello unlock (fingerprint, face, PIN)
- **Linux**: Password lock, auto-lock, and tray/hotkey interception are available; biometric unlock falls back to password-only behavior

## Sync Overview

### LAN Sync

- Real-time clipboard sync between paired devices
- X25519 key exchange + AES-256-GCM encrypted transport
- WebSocket heartbeats and reconnect backoff
- Loop prevention and deduplication

### WebDAV Cloud Sync

- End-to-end encrypted clipboard sync across networks
- Password-derived key via Argon2id
- AES-256-GCM file encryption
- ETag-based conflict handling
- Device registry and configurable polling

## Screenshots

See the live screenshots and docs site: https://z-only.github.io/smart-clipboard/guide/screenshots

## Tech Stack

| Layer          | Technology                                                                         |
| -------------- | ---------------------------------------------------------------------------------- |
| Frontend       | Vue 3 + TypeScript + Tailwind CSS + shadcn-vue                                     |
| Backend        | Rust                                                                               |
| Framework      | Tauri 2                                                                            |
| Database       | SQLite with FTS5 (via rusqlite)                                                    |
| Clipboard      | arboard                                                                            |
| Local security | argon2 + aes-gcm + keyring + LocalAuthentication (macOS) + Windows Hello (Windows) |
| LAN discovery  | mdns-sd                                                                            |
| i18n           | vue-i18n                                                                           |

## Getting Started

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (1.77+)
- [Node.js](https://nodejs.org/) (18+)
- [pnpm](https://pnpm.io/)
- Platform-specific dependencies for [Tauri](https://v2.tauri.app/start/prerequisites/)

### Development

```bash
# Clone the repository
git clone https://github.com/Z-Only/smart-clipboard.git
cd smart-clipboard

# Install dependencies
pnpm install

# Run in development mode
pnpm tauri dev
```

### Quality checks

```bash
# Format all supported files
pnpm run format

# Verify formatting only
pnpm run format:check

# Run web + Rust lint
pnpm run lint

# Run TypeScript checks
pnpm run typecheck

# Run frontend unit tests
pnpm run test:web

# Run frontend unit tests with coverage
pnpm run test:web:coverage

# Run Rust tests
pnpm run test:rust

# Run the full local quality gate
pnpm run check
```

### Git hooks and commit rules

This repository includes local Husky hooks:

- `pre-commit`: formats staged files
- `commit-msg`: validates commit messages with commitlint
- `pre-push`: runs the main quality checks

Commit messages should follow a conventional style such as:

- `feat: add xxx`
- `fix: resolve xxx`
- `test: add xxx`
- `ci: improve xxx`
- `chore: update xxx`

See [CONTRIBUTING.md](CONTRIBUTING.md) for the contributor workflow.

### Build

```bash
# Build for production
pnpm tauri build
```

The built binary will be in `src-tauri/target/release/bundle/`.

## Usage

1. Launch the app -- it starts minimized to the system tray
2. Copy text or images in any application
3. Press `Cmd+Shift+V` (macOS) or `Ctrl+Shift+V` (Windows/Linux) to open the clipboard panel
4. Search, filter, tag, or click an entry to paste it
5. Use templates for reusable snippets
6. Open the sync panel to manage LAN or WebDAV sync
7. Open Settings to configure app lock, auto-lock, and other preferences
8. Unlock the app with password, or use biometric/system-auth convenience unlock when available

## Project Structure

```text
smart-clipboard/
├── src/                          # Vue 3 frontend
│   ├── components/               # UI components
│   ├── composables/              # Vue composables
│   ├── i18n/                     # Internationalization
│   ├── stores/                   # Pinia state management
│   └── types/                    # TypeScript types
├── src-tauri/                    # Rust backend
│   └── src/
│       ├── analyzer/             # Content classifier and sensitive detection
│       ├── clipboard/            # Clipboard monitor
│       ├── storage/              # SQLite + FTS5 database layer
│       ├── sync/                 # LAN sync + WebDAV cloud sync
│       ├── templates/            # Clipboard template engine and commands
│       ├── security.rs           # App lock, unlock, and access-control runtime
│       ├── encryption.rs         # AES-256-GCM database encryption engine
│       ├── commands.rs           # Main Tauri IPC commands
│       ├── config.rs             # Settings management
│       ├── hotkey.rs             # Global shortcut handling
│       ├── tray.rs               # System tray integration
│       └── lib.rs                # App entry point
└── docs/                         # Design documents
```

## Roadmap

### Completed

- [x] **Phase 1 -- MVP**: Clipboard monitoring, storage, classification, search UI, hotkey, tray, settings
- [x] **Phase 2 -- Smart Enhancements**: Sensitive detection, content transforms, tags, images, usage stats
- [x] **Templates**: Parameterized reusable clipboard snippets
- [x] **Phase 3 -- Sync**: LAN sync, pairing, encrypted WebSocket transport, WebDAV cloud sync
- [x] **Phase 4 -- Access Security**: App lock, secure password storage, startup unlock gate, tray/hotkey interception, auto-lock, and guarded commands

### Planned / Future Improvements

- [x] **Native biometric integration**: Native Touch ID (macOS) and Windows Hello (Windows) via platform FFI
- [x] **Deeper runtime integration tests**: Add full invoke-level black-box tests around locked/unlocked command behavior
- [x] **Database-at-rest encryption**: Optional AES-256-GCM encrypted local storage for clipboard data with OS keyring key management
- [ ] **Advanced sync conflict handling**: Smarter merge / conflict-resolution behavior
- [ ] **Plugin / extension system**: User-extensible automations and transformations
- [ ] **More platform-specific hardening**: Better idle detection and richer OS-native unlock UX

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'feat: add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

This project is licensed under the MIT License -- see the [LICENSE](LICENSE) file for details.

## Updater configuration

The managed updater expects release artifacts generated by Tauri updater builds.

Required release secrets:

- `TAURI_SIGNING_PRIVATE_KEY`
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`

Runtime updater public key sources:

1. `SMART_CLIPBOARD_UPDATER_PUBLIC_KEY` environment variable
2. `src-tauri/tauri.conf.json` -> `plugins.updater.pubkey`

Current status:

- `sha256:` signatures are supported as a development fallback verifier
- `minisign:` signatures have runtime public-key plumbing, but the final verifier implementation is still pending
