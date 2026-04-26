# Quick Paste Panel — Design Spec

## Context

Smart Clipboard Manager v2.7.0 has matured into a full-featured clipboard tool with history, classification, templates, sync, security, plugins, and more. However, the current interaction model requires opening the full main window every time a user wants to retrieve a past clipboard entry. This creates friction for the most common use case: quickly pasting one of the last few copied items.

Every major clipboard manager (Paste, Alfred Clipboard, CopyQ, Maccy) provides a lightweight quick-paste mechanism — typically a compact overlay showing recent entries with keyboard shortcuts for instant selection. Adding this capability will close a significant UX gap.

## Goals

1. Provide a fast, keyboard-driven way to paste recent clipboard entries without navigating the full UI
2. Support number-key selection (1-9) for instant paste
3. Allow seamless transition to full search when the user starts typing
4. Respect the existing security model (lock screen, access control)
5. Maintain the lightweight, low-resource character of the app

## Non-Goals

- A separate standalone window (complexity vs. benefit tradeoff unfavorable in Tauri 2)
- Rich entry editing in quick-paste mode
- Multi-select or batch operations in quick-paste mode

## Architecture

### Approach: In-Window Quick Paste Mode

Rather than creating a second Tauri window (which introduces cross-platform focus management issues and duplicates security plumbing), the quick-paste panel is implemented as a **mode** within the existing main window.

A new dedicated keyboard shortcut (`Cmd/Ctrl + Shift + C`, configurable) triggers quick-paste mode. If the window is hidden, it is shown first. The mode overlays a compact list of the 9 most recent entries on top of the normal UI.

### Interaction Flow

```
User presses quick-paste hotkey
    → Backend: toggle_quick_paste() in hotkey.rs
    → If window hidden → show window
    → If locked → show lock screen (existing flow)
    → Emit "quick-paste-activated" event to frontend

Frontend receives event
    → clipboardStore: fetch latest 9 entries (dedicated lightweight query)
    → Show QuickPasteOverlay component
    → Focus is captured by the overlay
    → Entries displayed as compact rows with number badges 1-9

User interaction:
    → Press 1-9     → paste corresponding entry, hide window, exit mode
    → Press Enter   → paste first entry, hide window, exit mode
    → Press Escape  → exit quick-paste mode (return to normal UI if visible)
    → Press ↑/↓     → navigate entries, Enter to paste selected
    → Start typing  → exit quick-paste mode, populate SearchBar, enter normal search
    → Click entry   → paste it, hide window, exit mode
    → Click outside → exit quick-paste mode
```

### Component Design

#### 1. `QuickPasteOverlay.vue` (New)

A modal-like overlay that renders on top of the main UI when quick-paste mode is active.

**Responsibilities:**

- Display up to 9 recent entries in a compact format
- Show number badges (1-9) next to each entry
- Handle keyboard navigation (1-9, ↑/↓, Enter, Escape, typing)
- Emit paste/dismiss events
- Show entry category icon, truncated content preview, and relative timestamp

**Props:**

- `entries: ClipboardEntry[]` — up to 9 most recent entries
- `isActive: boolean` — controls visibility

**Emits:**

- `paste(entryId: number)` — user selected an entry to paste
- `dismiss()` — user wants to exit quick-paste mode
- `search(text: string)` — user started typing, transition to search

**Visual layout:**

```
┌─────────────────────────────────┐
│  Quick Paste          Esc close │
├─────────────────────────────────┤
│  1  📋  Meeting notes from...   │  ← highlighted (active)
│  2  🔗  https://github.com/...  │
│  3  💻  const result = awa...   │
│  4  📧  john@example.com        │
│  5  📝  Hello, thanks for...    │
│  6  {}  {"name": "test",...     │
│  7  📋  Project deadline i...   │
│  8  🔗  https://docs.rust...    │
│  9  📷  [Image]                 │
├─────────────────────────────────┤
│  Type to search...              │
└─────────────────────────────────┘
```

#### 2. Backend Changes

**`hotkey.rs`** — Register a second global shortcut for quick-paste. The shortcut string is read from `AppConfig`. The handler emits a `"quick-paste-activated"` event instead of toggling the window normally.

**`config.rs`** — Add `quick_paste_shortcut: String` field to `AppConfig` with default `"CommandOrControl+Shift+C"`. Add `quick_paste_entry_count: u8` with default `9`.

**`commands/clipboard.rs`** — Add a `get_recent_entries` command that returns the N most recent non-expired entries (lightweight, no pagination, no FTS).

#### 3. Frontend Integration

**`App.vue`** — Add `QuickPasteOverlay` component. Listen for `"quick-paste-activated"` event, set `quickPasteActive = true`, fetch recent entries via the new command.

**`clipboardStore.ts`** — Add `recentEntries` state and `fetchRecentEntries(count)` action that invokes `get_recent_entries`.

**`useClipboard.ts`** — No changes needed; the existing clipboard-changed listener keeps the store fresh.

### Security Integration

Quick-paste mode respects the existing security model:

- If app is locked, `enforce_window_access()` intercepts the hotkey and shows the lock screen (same as the existing `Cmd+Shift+V` flow)
- The `get_recent_entries` command will use `require_unlocked()` guard
- Sensitive entries shown in quick-paste will respect the existing sensitive-content display rules

### Configuration

New fields in `AppConfig`:

| Field                     | Type     | Default                      | Description                       |
| ------------------------- | -------- | ---------------------------- | --------------------------------- |
| `quick_paste_shortcut`    | `String` | `"CommandOrControl+Shift+C"` | Keyboard shortcut for quick-paste |
| `quick_paste_entry_count` | `u8`     | `9`                          | Number of entries shown (1-9)     |

These will be exposed in the Settings panel under a new "Quick Paste" subsection.

### Internationalization

New i18n keys needed:

**English:**

- `quickPaste.title` → "Quick Paste"
- `quickPaste.dismiss` → "Press Esc to close"
- `quickPaste.typeToSearch` → "Type to search..."
- `quickPaste.empty` → "No recent entries"
- `quickPaste.imageEntry` → "[Image]"
- `settings.quickPaste` → "Quick Paste"
- `settings.quickPasteShortcut` → "Shortcut"
- `settings.quickPasteEntryCount` → "Entries to show"

**Chinese (zh-CN):** Matching translations.

### Error Handling

- If the shortcut conflicts with an existing system shortcut, log a warning and skip registration (same pattern as the existing main hotkey)
- If fetching recent entries fails, show the overlay with an error state and allow dismissal
- If the user pastes an entry that has been deleted since the overlay opened, show a brief toast and refresh

### Testing Strategy

**Frontend unit tests:**

- `QuickPasteOverlay.test.ts`: Renders entries with number badges, handles keyboard events (1-9, Escape, Enter, arrow keys, typing), emits correct events
- `clipboardStore.test.ts`: Test `fetchRecentEntries` action

**Backend tests:**

- `get_recent_entries` command returns correct number of entries
- Quick-paste hotkey registration and event emission
- Security guard: command rejected when locked

**Integration:**

- End-to-end: hotkey → overlay → paste → window hide cycle

## File Map

| Action | File                                   | Responsibility                               |
| ------ | -------------------------------------- | -------------------------------------------- |
| Create | `src/components/QuickPasteOverlay.vue` | Overlay UI with keyboard handling            |
| Modify | `src/App.vue`                          | Mount overlay, listen for quick-paste event  |
| Modify | `src/stores/clipboardStore.ts`         | Add `recentEntries` + `fetchRecentEntries()` |
| Modify | `src-tauri/src/hotkey.rs`              | Register second shortcut, emit event         |
| Modify | `src-tauri/src/config.rs`              | Add quick-paste config fields                |
| Modify | `src-tauri/src/commands/clipboard.rs`  | Add `get_recent_entries` command             |
| Modify | `src-tauri/src/lib.rs`                 | Register new command                         |
| Modify | `src/i18n/locales/en.ts`               | English quick-paste keys                     |
| Modify | `src/i18n/locales/zh-CN.ts`            | Chinese quick-paste keys                     |
| Modify | `src/components/SettingsPanel.vue`     | Quick-paste settings subsection              |
| Create | `tests/unit/QuickPasteOverlay.test.ts` | Frontend overlay tests                       |
| Modify | `tests/unit/clipboardStore.test.ts`    | Test fetchRecentEntries                      |

## Version

This feature targets **v2.8.0** as a minor release.
