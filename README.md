[English](README.md) | [中文](README.zh-CN.md)

# Smart Clipboard Manager

A cross-platform, lightweight smart clipboard manager built with **Tauri 2** + **Vue 3** + **Rust**. It runs in the background, automatically captures and classifies clipboard content, supports secure local protection, and provides fast search, retrieval, and sync workflows.

- Website / Docs: https://z-only.github.io/smart-clipboard/
- Repository: https://github.com/Z-Only/smart-clipboard

## Release Status — v2.9.0 Smart Search & Knowledge Organization

The current main branch already delivers a full desktop workflow spanning clipboard history, smart enhancements, templates, secure local protection, LAN/WebDAV sync, managed updates, and plugin-based extensibility. Version 2.9.0 further strengthens **Smart Search & Knowledge Organization**.

### v2.9.0 highlights

- **Smart Groups**: Automatically clusters similar entries with N-gram / TF-IDF similarity and generates readable group labels
- **Tag Suggestions**: Recommends reusable tags from similar entries directly on each card
- **Related Entries**: Surfaces the top 5 most similar entries with similarity percentages for quick recall
- **Search Re-ranking**: Reorders FTS5 matches with TF-IDF cosine similarity to improve ambiguous queries
- **Similarity Scoring Extension Point**: The `SimilarityScorer` trait keeps the ranking pipeline open to future vector or embedding-based backends

## Features

### Core workflow

- **Clipboard History** -- Automatically captures copied content with deduplication
- **Full-Text Search** -- Fast search powered by SQLite FTS5, with Chinese pinyin full-text and initials matching
- **Category Filter** -- Browse clipboard history by content type
- **Global Hotkey** -- `Cmd/Ctrl + Shift + V` to toggle the clipboard panel
- **Quick Paste** -- `Cmd/Ctrl + Shift + 1` to open a lightweight overlay for instant number-key paste
- **Batch Operations** -- Multi-select mode with bulk delete, favorite/unfavorite, tagging, merge-copy, and select-all/invert/clear actions
- **Virtual Scroll** -- Virtualized list rendering for smooth scrolling with large clipboard histories

### Smart enhancements & knowledge organization

- **Smart Classification** -- Auto-categorizes content into URL, Email, Code, JSON, FilePath, Color, Phone, Address, Image, and plain Text
- **Sensitive Detection** -- Detects passwords, API keys, tokens, JWTs, and connection strings with optional auto-expiry
- **Content Transforms** -- One-click case, encoding, and formatting transforms
- **Tag Management** -- Custom tags for organizing entries
- **Image Clipboard** -- Captures and displays clipboard images with PNG storage
- **Usage Statistics** -- Dashboard with category breakdown, daily activity, and most-used entries
- **Clipboard Templates** -- Reusable text snippets with `{{placeholder}}` syntax and fill-in dialog
- **Smart Search & Knowledge Organization** -- Combines Smart Groups, tag suggestions, related entries, and search re-ranking to make history easier to organize and reuse

### Security, sync, and extensibility

- **App Access Security** -- Password lock, manual lock (settings & tray), auto-lock timeout, guarded tray/hotkey wakeups, and command-level access control
- **Database Encryption** -- Optional AES-256-GCM encryption for clipboard data at rest, with keys in the OS keyring
- **LAN Sync** -- Encrypted peer-to-peer sync over mDNS + WebSocket
- **WebDAV Cloud Sync** -- End-to-end encrypted cloud sync with device registry, polling, and rate limiting
- **Sync Conflict Handling** -- Smart conflict detection with configurable resolution strategies and manual diff UI
- **Managed Updater** -- Background update checks, mirror endpoints, artifact download with progress, signature verification, and install handoff
- **Plugin extension system** -- Local plugin discovery, validation, enable/disable management, trusted transform hooks, and the `SimilarityScorer` similarity-scoring extension point for future ranking backends
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

### Implemented

- [x] **Core clipboard workflow**: Background monitoring, deduplicated history, classification, search UI, hotkey, tray, and settings
- [x] **Smart enhancements**: Sensitive detection, content transforms, tags, images, usage statistics, and reusable templates
- [x] **Multi-device sync**: LAN sync, device pairing, encrypted WebSocket transport, WebDAV cloud sync, and conflict handling
- [x] **Access security**: App lock, secure password storage, startup unlock gate, tray/hotkey interception, auto-lock, guarded commands, native biometrics, and database-at-rest encryption
- [x] **Smart search & knowledge organization**: Smart Groups, tag suggestions, related entries, search re-ranking, batch operations, virtual scroll, and quick paste overlay
- [x] **Plugin extension system**: Local plugin discovery, validation, enable/disable management, trusted transform hooks, and the `SimilarityScorer` similarity-scoring extension point for future ranking backends

### Next

- [ ] **More platform-specific hardening**: Better idle detection, richer native unlock UX, and stronger consistency across protected entry points
- [ ] **Broader plugin capabilities**: Expand plugins from transform actions toward richer content processing and similarity backends
- [ ] **Further search/organization improvements**: Keep improving grouping quality, recommendation explainability, and result relevance while staying lightweight and local-first

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
