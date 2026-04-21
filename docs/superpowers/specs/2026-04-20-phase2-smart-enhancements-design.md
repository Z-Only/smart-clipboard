# Phase 2 -- Smart Enhancements Design Spec

## Context

Phase 1 MVP is complete: clipboard monitoring, SQLite+FTS5 storage, content classification, Vue 3 UI, global hotkey, system tray, settings, i18n, and theming. Phase 2 adds five smart enhancement features on top of this foundation.

Current version: **0.3.0**. Target version: **0.4.0**.

---

## Feature 9: Sensitive Info Detection + Auto-Expiry

### Purpose

Automatically detect passwords, API keys, tokens, and other secrets in clipboard content. Flag them and optionally auto-expire them to prevent accidental exposure.

### Backend Changes

**New file**: `src-tauri/src/analyzer/sensitive.rs`

Regex patterns to detect:

- **API Keys**: `(?i)(api[_-]?key|apikey)\s*[:=]\s*['"]?[A-Za-z0-9_\-]{16,}`
- **AWS Keys**: `AKIA[0-9A-Z]{16}`
- **Tokens**: `(?i)(token|bearer|auth)\s*[:=]\s*['"]?[A-Za-z0-9_\-\.]{20,}`
- **Private Keys**: `-----BEGIN\s+(RSA|EC|DSA|OPENSSH)?\s*PRIVATE KEY-----`
- **Generic Secrets**: `(?i)(password|passwd|secret|credential)\s*[:=]\s*['"]?[^\s'"]{4,}`
- **JWT**: `eyJ[A-Za-z0-9_-]{10,}\.eyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_\-]+`
- **Connection Strings**: `(?i)(mysql|postgres|mongodb|redis)://[^\s]+`

**Function**: `detect_sensitive(content: &str) -> bool`

### Integration

- Called in `lib.rs` after classification, before storage
- If sensitive, set `is_sensitive = 1` and `expires_at = now + sensitive_expiry_minutes`
- Cleanup runs on the existing periodic cleanup cycle

### Config Addition

- `sensitive_expiry_minutes: u64` (default: 5) in `AppConfig`
- `0` means no auto-expiry for sensitive content

### Frontend

- Sensitive entries display a shield/lock icon badge
- Sensitive entries show countdown to expiry
- Settings panel: sensitive expiry time input

---

## Feature 10: Content Transforms

### Purpose

One-click text transformations: case conversion, encoding/decoding, formatting.

### Transform Types

| Transform       | Function                                | Reversible |
| --------------- | --------------------------------------- | ---------- |
| UPPERCASE       | `to_uppercase()`                        | Yes        |
| lowercase       | `to_lowercase()`                        | Yes        |
| Title Case      | Capitalize first letter of each word    | Yes        |
| URL Encode      | `percent_encoding`                      | Yes        |
| URL Decode      | `percent_encoding`                      | Yes        |
| JSON Format     | Pretty-print JSON                       | Yes        |
| JSON Compact    | Minify JSON                             | Yes        |
| Base64 Encode   | Standard base64                         | Yes        |
| Base64 Decode   | Standard base64                         | Yes        |
| Trim Whitespace | Strip leading/trailing/extra whitespace | No         |
| HTML Escape     | `&` → `&amp;` etc.                      | Yes        |
| HTML Unescape   | `&amp;` → `&` etc.                      | Yes        |

### Backend

**New command**: `transform_content(content: String, transform_type: String) -> Result<String, String>`

Uses Rust standard library for case/trim; `serde_json` for JSON; `base64` crate for base64; manual impl for URL encode/HTML escape.

**New dependency**: `base64 = "0.22"` in Cargo.toml

### Frontend

- Right-click context menu on entry cards with "Transform" submenu
- Transform result copies to clipboard and shows a toast notification
- Available transforms are context-aware (JSON format only for JSON entries, etc.)

---

## Feature 11: Tag Management

### Purpose

User-defined tags for organizing clipboard entries beyond auto-classification.

### Database Schema

```sql
CREATE TABLE IF NOT EXISTS tags (
    id   INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE
);

CREATE TABLE IF NOT EXISTS entry_tags (
    entry_id INTEGER NOT NULL REFERENCES clipboard_entries(id) ON DELETE CASCADE,
    tag_id   INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (entry_id, tag_id)
);
```

Run as migration in `migrations.rs`.

### Backend Commands

- `create_tag(name: String) -> Tag`
- `delete_tag(id: i64)`
- `get_all_tags() -> Vec<Tag>`
- `add_tag_to_entry(entry_id: i64, tag_id: i64)`
- `remove_tag_from_entry(entry_id: i64, tag_id: i64)`
- `get_entry_tags(entry_id: i64) -> Vec<Tag>`
- `get_entries_by_tag(tag_id: i64) -> Vec<ClipboardEntry>`

### Frontend

- Tag chips displayed on entry cards (small colored badges)
- Click "+" on entry card to add/create tags via popover
- Sidebar: "Tags" section below categories listing user tags
- Click tag in sidebar to filter entries by that tag

---

## Feature 12: Image Clipboard Support

### Purpose

Capture and display images copied to clipboard.

### Backend Changes

**Clipboard Monitor** (`monitor.rs`):

- After text check, also call `clipboard.get_image()` from arboard
- If image found (and hash differs from last), save as PNG file
- Hash the raw image bytes for deduplication

**Storage**:

- Images saved as PNG files in `{app_data_dir}/images/{hash}.png`
- `content` field stores the file path
- `content_type` = `"image"`
- `category` = `"image"` (new category)

**New functions in database.rs**:

- Handle image entries in existing CRUD operations
- Delete image file when deleting image entry

**New dependency**: `image = "0.25"` for PNG encoding (arboard returns raw RGBA)

### Frontend

- New `"image"` category in sidebar with icon
- EntryCard: if content_type is image, render `<img>` thumbnail (64px height)
- Click image entry: copy image back to clipboard
- Image entries show dimensions and file size instead of text preview

### Category Addition

- Add `"image"` to `CategoryType` union type
- Add image category to `CATEGORIES` array with camera icon

---

## Feature 13: Usage Statistics Panel

### Purpose

Show clipboard usage patterns: category breakdown, daily activity, most-used entries.

### Backend Commands

- `get_statistics() -> Statistics`

**Statistics struct**:

```rust
pub struct Statistics {
    pub total_entries: i64,
    pub total_favorites: i64,
    pub entries_by_category: Vec<CategoryCount>,  // (category, count)
    pub entries_by_day: Vec<DayCount>,             // (date, count) last 30 days
    pub most_used: Vec<ClipboardEntry>,            // top 10 by use_count
    pub storage_size_bytes: u64,                   // DB file size
}
```

### SQL Queries

- `SELECT category, COUNT(*) FROM clipboard_entries GROUP BY category`
- `SELECT DATE(created_at), COUNT(*) FROM clipboard_entries GROUP BY DATE(created_at) ORDER BY DATE(created_at) DESC LIMIT 30`
- `SELECT * FROM clipboard_entries ORDER BY use_count DESC LIMIT 10`

### Frontend

- New "Statistics" panel accessible from settings gear menu or tray
- Category distribution: horizontal bar chart (CSS-only, no chart library)
- Daily activity: simple sparkline or bar chart (CSS-only)
- Most used entries list
- Total count, favorites count, storage size display

---

## Architecture Notes

### No New Crates Beyond

- `base64 = "0.22"` (for content transforms)
- `image = "0.25"` (for image clipboard encoding)

### Migration Strategy

- Single migration adds tags tables and image category support
- Backward compatible: existing entries unaffected

### File Organization

All new Rust code follows existing module patterns:

- New files: `analyzer/sensitive.rs`, new commands in `commands.rs`
- Extend existing: `monitor.rs`, `database.rs`, `models.rs`, `config.rs`

### Frontend Organization

- New components: `TransformMenu.vue`, `TagPicker.vue`, `StatisticsPanel.vue`
- Extend existing: `EntryCard.vue`, `CategoryFilter.vue`, `SettingsPanel.vue`
- Update i18n files for new strings
