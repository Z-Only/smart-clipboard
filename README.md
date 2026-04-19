[English](README.md) | [中文](README.zh-CN.md)

# Smart Clipboard Manager

A cross-platform, lightweight smart clipboard manager built with **Tauri 2** + **Vue 3** + **Rust**. It runs in the background, automatically captures and classifies clipboard content, and provides instant search and retrieval.

## Features

- **Clipboard History** -- Automatically captures all copied text with deduplication
- **Smart Classification** -- Auto-categorizes content into URL, Email, Code, JSON, FilePath, Color, Phone, Address, and plain Text
- **Full-Text Search** -- Millisecond-level search powered by SQLite FTS5
- **Category Filter** -- Browse clipboard history by content type
- **Global Hotkey** -- `Cmd/Ctrl + Shift + V` to toggle the clipboard panel
- **System Tray** -- Runs quietly in the background with a tray icon
- **Favorites** -- Pin frequently used entries to prevent auto-cleanup
- **Configurable** -- Max entries, retention period, excluded apps, monitor interval
- **Auto-Start** -- Optionally launch on system login
- **Appearance Modes** -- System / Light / Dark mode with automatic OS preference detection
- **Theme Colors** -- 6 built-in color themes: Zinc, Blue, Green, Rose, Orange, Violet
- **Multi-Language** -- English and Chinese UI support
- **Lightweight** -- ~5MB binary, minimal CPU/memory usage thanks to Rust + native WebView

## Screenshots

*Coming soon*

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Frontend | Vue 3 + TypeScript + Tailwind CSS + shadcn-vue |
| Backend | Rust |
| Framework | Tauri 2 |
| Database | SQLite with FTS5 (via rusqlite) |
| Clipboard | arboard |
| i18n | vue-i18n |

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
2. Copy any text in any application
3. Press `Cmd+Shift+V` (macOS) or `Ctrl+Shift+V` (Windows/Linux) to open the clipboard panel
4. Search, filter by category, or click an entry to paste it
5. Star entries to keep them permanently
6. Right-click the tray icon for quick access and settings
7. Switch language in Settings (English / Chinese)
8. Choose appearance mode (System / Light / Dark) and theme color in Settings

## Project Structure

```
smart-clipboard/
├── src/                          # Vue 3 frontend
│   ├── components/               # UI components
│   ├── composables/              # Vue composables
│   ├── i18n/                     # Internationalization
│   │   ├── locales/              # Language files (en, zh-CN)
│   │   └── index.ts              # i18n configuration
│   ├── stores/                   # Pinia state management
│   └── types/                    # TypeScript types
├── src-tauri/                    # Rust backend
│   └── src/
│       ├── analyzer/             # Content classifier (regex rules)
│       ├── clipboard/            # Clipboard monitor (arboard polling)
│       ├── storage/              # SQLite + FTS5 database layer
│       ├── commands.rs           # Tauri IPC commands
│       ├── config.rs             # Settings management
│       ├── hotkey.rs             # Global shortcut
│       ├── tray.rs               # System tray
│       └── lib.rs                # App entry point
└── docs/                         # Design documents
```

## Roadmap

- [x] **Phase 1 -- MVP**: Clipboard monitoring, storage, classification, search UI, hotkey, tray, settings
- [x] **i18n**: Multi-language support (English, Chinese)
- [x] **Theming**: Appearance mode switching (System/Light/Dark) and 6 color themes
- [ ] **Phase 2 -- Smart Enhancements**: Sensitive content detection, content transforms, image support, usage stats
- [ ] **Phase 3 -- Sync & Advanced**: LAN sync, E2E encrypted cloud sync, clipboard templates, plugin system

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'feat: add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

This project is licensed under the MIT License -- see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- [Tauri](https://tauri.app/) -- Cross-platform app framework
- [Vue.js](https://vuejs.org/) -- Frontend framework
- [vue-i18n](https://vue-i18n.intlify.dev/) -- Internationalization for Vue.js
- [shadcn-vue](https://www.shadcn-vue.com/) -- UI components
- [arboard](https://github.com/1Password/arboard) -- Cross-platform clipboard library
- [rusqlite](https://github.com/rusqlite/rusqlite) -- SQLite bindings for Rust
