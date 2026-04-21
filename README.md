[English](README.md) | [中文](README.zh-CN.md)

# Smart Clipboard Manager

A cross-platform, lightweight smart clipboard manager built with **Tauri 2** + **Vue 3** + **Rust**. It runs in the background, automatically captures and classifies clipboard content, supports secure local protection, and provides fast search, retrieval, and sync workflows.

## Release Status — Phase 4 Access Security Complete

This repository now includes the full **Phase 4 Access Security** package on top of clipboard history, smart enhancements, templates, LAN sync, and WebDAV cloud sync.

### Phase 4 highlights

- **App lock**: Enable or disable a local app lock from Settings
- **Password setup**: Create or update the unlock password
- **Secure password storage**: Passwords are never stored in plaintext; Rust hashes them with Argon2 and stores only the hash in the OS credential store
- **Startup unlock**: If app lock is enabled, the app starts in a locked state
- **Manual lock**: Lock the app immediately from Settings
- **Tray / hotkey interception**: Tray and global shortcut wakeups are blocked by Rust-side access checks when locked
- **Auto-lock**: Re-lock after a configurable idle timeout
- **Biometric convenience unlock**: When available, users can try a faster biometric/system-auth unlock path, with password fallback on failure
- **Rust-side command guards**: Sensitive Tauri commands now refuse access while the app is locked
- **Frontend sensitive-state clearing**: Clipboard, sync, template, WebDAV, and statistics state are cleared on lock and reloaded after unlock

## Features

- **Clipboard History** -- Automatically captures copied content with deduplication
- **Smart Classification** -- Auto-categorizes content into URL, Email, Code, JSON, FilePath, Color, Phone, Address, Image, and plain Text
- **Full-Text Search** -- Fast search powered by SQLite FTS5
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
- **App Access Security** -- Password lock, auto-lock, guarded wakeups, and secure unlock flow
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

- **macOS**: Password lock, auto-lock, tray/hotkey interception, and a system-auth convenience unlock path are available
- **Windows / Linux**: Password lock, auto-lock, and tray/hotkey interception are available; biometric unlock currently falls back to password-only behavior

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

_Coming soon_

## Tech Stack

| Layer          | Technology                                     |
| -------------- | ---------------------------------------------- |
| Frontend       | Vue 3 + TypeScript + Tailwind CSS + shadcn-vue |
| Backend        | Rust                                           |
| Framework      | Tauri 2                                        |
| Database       | SQLite with FTS5 (via rusqlite)                |
| Clipboard      | arboard                                        |
| Local security | argon2 + keyring                               |
| LAN discovery  | mdns-sd                                        |
| i18n           | vue-i18n                                       |

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

- [ ] **Native biometric integration**: Replace the current macOS convenience path with a fully native Touch ID / LocalAuthentication bridge and expand platform coverage where possible
- [ ] **Deeper runtime integration tests**: Add full invoke-level black-box tests around locked/unlocked command behavior
- [ ] **Database-at-rest encryption**: Optional encrypted local storage for clipboard data
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
