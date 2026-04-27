# Changelog-Driven Release Notes & Bilingual Update Display

## Summary

Replace GitHub's auto-generated release notes ("Full Changelog: v2.7.0...v2.8.0") with actual CHANGELOG content extracted at release time. Add a Chinese CHANGELOG file and make the in-app update notification display the correct language based on the user's locale.

## Background

- The current `release.yml` uses `generate_release_notes: true` in the `softprops/action-gh-release` action, producing a generic diff link instead of meaningful release notes.
- The Tauri updater already reads the GitHub Release body into `availableNotes` and `pendingUpdate.notes`, and `SettingsUpdaterSection.vue` already renders these fields.
- The project maintains `CHANGELOG.md` (English, Keep a Changelog format) but has no Chinese equivalent.
- The app supports `en` and `zh-CN` locales via `vue-i18n`.

## Requirements

1. **GitHub Release body from CHANGELOG**: When a tag is pushed, the release workflow extracts the matching version section from `CHANGELOG.md` (and `CHANGELOG.zh-CN.md` if present) and uses it as the Release body.
2. **In-app locale-aware display**: The updater UI shows release notes in the user's current language, falling back to English when no Chinese content is available.
3. **Chinese CHANGELOG**: A new `CHANGELOG.zh-CN.md` file mirrors the structure of `CHANGELOG.md`.

## Architecture

### Release Body Format

The Release body contains both languages separated by HTML comment markers:

```markdown
<!-- lang:en -->

### Added

- **Quick Paste Panel**: Global shortcut invokes a lightweight overlay...

### Changed

- **Version bump to 2.8.0**: Promoted the quick paste panel...

<!-- lang:zh-CN -->

### 新增

- **快速粘贴面板**：全局快捷键唤出轻量覆盖面板...

### 变更

- **版本升级至 2.8.0**：将快速粘贴面板作为新的次版本发布...
```

If `CHANGELOG.zh-CN.md` does not exist or has no matching version section, only the English block is included (no `<!-- lang:zh-CN -->` marker at all).

### CI Extraction Script

Located inline in `.github/workflows/release.yml`, inside the `create-release` job:

1. Parse `$GITHUB_REF_NAME` to get the version number (strip leading `v`).
2. Use `awk` to extract lines between `## [<version>]` and the next `## [` (or EOF) from `CHANGELOG.md`.
3. If `CHANGELOG.zh-CN.md` exists, repeat for the Chinese file.
4. Wrap each block with `<!-- lang:xx -->` markers and write to a temp file.
5. Pass the temp file via `body_path` to `softprops/action-gh-release`.
6. **Fallback**: If extraction produces empty content, set `generate_release_notes: true` instead.

### Frontend Locale Filter

A new utility function in `src/lib/changelog.ts`:

```typescript
export function extractLocalizedNotes(notes: string | null, locale: string): string;
```

**Behavior:**

- If `notes` is null/empty, return empty string.
- Split on `<!-- lang:xx -->` markers.
- If the requested locale block exists, return it (trimmed).
- Otherwise fall back to the `en` block.
- If no markers are found at all, return the original string unchanged (backward compatibility with old releases that used `generate_release_notes`).

**Integration:**

- `SettingsUpdaterSection.vue` wraps `availableNotes` and `pendingUpdate.notes` through a computed that calls `extractLocalizedNotes` with the current `i18n.global.locale`.
- No changes to Rust backend, `UpdaterStatus` type, or `updaterStore`.

### `CHANGELOG.zh-CN.md`

- Mirrors `CHANGELOG.md` structure (Keep a Changelog format, Semantic Versioning).
- Initially contains only the `[2.8.0]` section (current version).
- Older versions are not translated; they can be added incrementally.

## Files Affected

| File                                        | Action | Purpose                                                                  |
| ------------------------------------------- | ------ | ------------------------------------------------------------------------ |
| `.github/workflows/release.yml`             | Modify | Add extraction script, replace `generate_release_notes` with `body_path` |
| `CHANGELOG.zh-CN.md`                        | Create | Chinese changelog                                                        |
| `src/lib/changelog.ts`                      | Create | `extractLocalizedNotes` utility                                          |
| `src/components/SettingsUpdaterSection.vue` | Modify | Use locale filter for notes display                                      |
| `tests/changelog.test.ts`                   | Create | Unit tests for `extractLocalizedNotes`                                   |

## Not In Scope

- No new Rust commands or backend changes.
- No `UpdaterStatus` type changes.
- No GitHub API calls from the app.
- No Markdown rendering in the updater UI (stays plain text).
- No translation of historical CHANGELOG entries (v2.7.0 and earlier).
- No changes to the Tauri updater plugin configuration.

## Testing Strategy

- **`extractLocalizedNotes`**: Unit tests covering dual-language input, English-only input, no markers (backward compat), null/empty input, locale fallback.
- **GitHub Action**: Validated by tag-push CI run. No unit tests for shell scripts.

## Error Handling

- CI extraction yields empty content → falls back to `generate_release_notes: true`.
- `CHANGELOG.zh-CN.md` missing → Release body contains English only (no error).
- `availableNotes` from an old release without lang markers → `extractLocalizedNotes` returns the raw string (backward compatible).
