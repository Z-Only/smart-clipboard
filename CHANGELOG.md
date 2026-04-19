# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/), and this project adheres to [Semantic Versioning](https://semver.org/).

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
