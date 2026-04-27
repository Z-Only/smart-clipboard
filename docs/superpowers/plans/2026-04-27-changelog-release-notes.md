# Changelog-Driven Release Notes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace auto-generated GitHub Release notes with actual CHANGELOG content, add Chinese CHANGELOG, and display locale-aware update notes in the app.

**Architecture:** CI extracts version-specific content from bilingual CHANGELOG files into the GitHub Release body using HTML comment language markers. The frontend filters `availableNotes` by the user's locale before display.

**Tech Stack:** GitHub Actions (shell/awk), TypeScript, Vue 3, Vitest

---

### Task 1: Create `extractLocalizedNotes` utility with tests

**Files:**

- Create: `src/lib/changelog.ts`
- Create: `tests/unit/changelog.test.ts`

- [ ] **Step 1: Write the failing tests**

Create `tests/unit/changelog.test.ts`:

```typescript
import { describe, expect, it } from 'vitest';
import { extractLocalizedNotes } from '@/lib/changelog';

describe('extractLocalizedNotes', () => {
  const dualLangNotes = [
    '<!-- lang:en -->',
    '### Added',
    '',
    '- **Quick Paste Panel**: Global shortcut invokes overlay',
    '',
    '### Changed',
    '',
    '- **Version bump to 2.8.0**',
    '',
    '<!-- lang:zh-CN -->',
    '### 新增',
    '',
    '- **快速粘贴面板**：全局快捷键唤出覆盖面板',
    '',
    '### 变更',
    '',
    '- **版本升级至 2.8.0**',
  ].join('\n');

  it('returns English block when locale is en', () => {
    const result = extractLocalizedNotes(dualLangNotes, 'en');
    expect(result).toContain('Quick Paste Panel');
    expect(result).not.toContain('快速粘贴面板');
  });

  it('returns Chinese block when locale is zh-CN', () => {
    const result = extractLocalizedNotes(dualLangNotes, 'zh-CN');
    expect(result).toContain('快速粘贴面板');
    expect(result).not.toContain('Quick Paste Panel');
  });

  it('falls back to English when requested locale is missing', () => {
    const enOnly = '<!-- lang:en -->\n### Added\n\n- Feature X';
    const result = extractLocalizedNotes(enOnly, 'zh-CN');
    expect(result).toContain('Feature X');
  });

  it('returns original string when no lang markers are present (backward compat)', () => {
    const legacy = 'Full Changelog: v2.7.0...v2.8.0';
    expect(extractLocalizedNotes(legacy, 'en')).toBe(legacy);
    expect(extractLocalizedNotes(legacy, 'zh-CN')).toBe(legacy);
  });

  it('returns empty string for null input', () => {
    expect(extractLocalizedNotes(null, 'en')).toBe('');
  });

  it('returns empty string for empty string input', () => {
    expect(extractLocalizedNotes('', 'en')).toBe('');
  });

  it('trims whitespace from extracted blocks', () => {
    const result = extractLocalizedNotes(dualLangNotes, 'en');
    expect(result).not.toMatch(/^\s/);
    expect(result).not.toMatch(/\s$/);
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `pnpm vitest run tests/unit/changelog.test.ts`
Expected: FAIL — module `@/lib/changelog` does not exist

- [ ] **Step 3: Implement `extractLocalizedNotes`**

Create `src/lib/changelog.ts`:

```typescript
const LANG_MARKER_REGEX = /<!--\s*lang:(\S+)\s*-->/g;

export function extractLocalizedNotes(notes: string | null, locale: string): string {
  if (!notes) return '';

  const markers = [...notes.matchAll(LANG_MARKER_REGEX)];
  if (markers.length === 0) return notes;

  const blocks = new Map<string, string>();

  for (let i = 0; i < markers.length; i++) {
    const lang = markers[i][1];
    const startIndex = markers[i].index! + markers[i][0].length;
    const endIndex = i + 1 < markers.length ? markers[i + 1].index! : notes.length;
    blocks.set(lang, notes.slice(startIndex, endIndex).trim());
  }

  return blocks.get(locale) ?? blocks.get('en') ?? '';
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `pnpm vitest run tests/unit/changelog.test.ts`
Expected: All 7 tests PASS

- [ ] **Step 5: Commit**

```bash
git add src/lib/changelog.ts tests/unit/changelog.test.ts
git commit -m "feat: add extractLocalizedNotes utility for bilingual release notes"
```

---

### Task 2: Integrate locale filter into `SettingsUpdaterSection.vue`

**Files:**

- Modify: `src/components/SettingsUpdaterSection.vue`

- [ ] **Step 1: Read current file to confirm line numbers**

Read `src/components/SettingsUpdaterSection.vue` fully to identify the exact lines where `availableNotes` and `pendingUpdate.notes` are rendered.

The relevant lines are approximately:

- Line 141: `<div class="text-xs text-muted-foreground">{{ updater.status.availableNotes }}</div>`
- Line 152: `{{ updater.status.pendingUpdate.notes }}`

- [ ] **Step 2: Add import and computed properties**

In the `<script setup>` section (after existing imports around line 171), add:

```typescript
import { useI18n } from 'vue-i18n';
import { extractLocalizedNotes } from '@/lib/changelog';

const { locale } = useI18n();

const localizedAvailableNotes = computed(() =>
  extractLocalizedNotes(updater.status.availableNotes, locale.value),
);

const localizedPendingNotes = computed(() =>
  extractLocalizedNotes(updater.status.pendingUpdate?.notes ?? null, locale.value),
);
```

- [ ] **Step 3: Replace template bindings**

Replace `{{ updater.status.availableNotes }}` with `{{ localizedAvailableNotes }}`.

Replace `{{ updater.status.pendingUpdate.notes }}` with `{{ localizedPendingNotes }}`.

- [ ] **Step 4: Run existing updater tests to verify no regressions**

Run: `pnpm vitest run tests/unit/SettingsPanel.updater.test.ts`
Expected: All existing tests PASS

- [ ] **Step 5: Commit**

```bash
git add src/components/SettingsUpdaterSection.vue
git commit -m "feat: display locale-aware release notes in updater UI"
```

---

### Task 3: Create `CHANGELOG.zh-CN.md`

**Files:**

- Create: `CHANGELOG.zh-CN.md`

- [ ] **Step 1: Create the Chinese changelog file**

Create `CHANGELOG.zh-CN.md` with the header and v2.8.0 section translated from `CHANGELOG.md`. Use the same Keep a Changelog format.

```markdown
# 更新日志

本文件记录项目的所有重要变更。

格式基于 [Keep a Changelog](https://keepachangelog.com/)，版本号遵循 [语义化版本](https://semver.org/)。

## [2.8.0] - 2026-04-27

### 新增

- **快速粘贴面板**：全局快捷键（`Cmd/Ctrl+Shift+1`）唤出轻量覆盖面板，展示最近的剪贴板条目，支持数字键（1-9）即时粘贴、方向键导航、Esc 关闭和输入即搜索
- **快速粘贴后端命令**：新增 `get_recent_entries` Tauri 命令，在安全守卫下获取最近 N 条剪贴板记录
- **快速粘贴热键注册**：新增 `setup_quick_paste_hotkey` 函数（`hotkey.rs`），注册可配置的快速粘贴快捷键并向前端发送激活事件
- **快速粘贴配置**：在 `AppConfig` 中新增 `quick_paste_shortcut` 和 `quick_paste_entry_count` 字段，通过 serde 默认值保持向后兼容
- **QuickPasteOverlay 组件**：新增 Vue 组件，使用 Teleport 渲染，支持键盘导航、分类图标、相对时间戳和搜索切换
- **快速粘贴国际化**：快速粘贴覆盖面板标签和设置键的中英文翻译
- **快速粘贴单元测试**：QuickPasteOverlay 组件测试（键盘导航、数字键粘贴、关闭、搜索切换）和 store 测试（`fetchRecentEntries`）

### 变更

- **版本升级至 2.8.0**：将快速粘贴面板作为新的次版本发布
- **App.vue 集成**：新增快速粘贴事件监听、激活处理、粘贴后隐藏流程和搜索切换
- **clipboardStore**：新增 `recentEntries` 响应式状态和 `fetchRecentEntries` action
- **项目文档**：更新 README、中文 README、CHANGELOG、VitePress 文档和版本元数据
```

- [ ] **Step 2: Commit**

```bash
git add CHANGELOG.zh-CN.md
git commit -m "docs: add Chinese changelog (CHANGELOG.zh-CN.md) with v2.8.0 content"
```

---

### Task 4: Modify `release.yml` to extract CHANGELOG into Release body

**Files:**

- Modify: `.github/workflows/release.yml`

- [ ] **Step 1: Read the current release.yml**

Read `.github/workflows/release.yml` to confirm the `create-release` job structure. The relevant section is:

```yaml
create-release:
  runs-on: ubuntu-latest
  needs: quality
  permissions:
    contents: write
  outputs:
    release_id: ${{ steps.create_release.outputs.id }}
  steps:
    - uses: actions/checkout@v4

    - name: Create Release
      id: create_release
      uses: softprops/action-gh-release@v2
      with:
        draft: true
        generate_release_notes: true
```

- [ ] **Step 2: Add changelog extraction step and modify the release step**

Replace the `create-release` job steps with:

```yaml
steps:
  - uses: actions/checkout@v4

  - name: Extract changelog for current tag
    id: changelog
    run: |
      VERSION="${GITHUB_REF_NAME#v}"
      BODY_FILE=$(mktemp)
      USE_GENERATED="false"

      extract_version_section() {
        local file="$1"
        awk -v ver="$VERSION" '
          /^## \[/ {
            if (found) exit
            if (index($0, "[" ver "]")) { found=1; next }
          }
          found { print }
        ' "$file"
      }

      EN_NOTES=$(extract_version_section "CHANGELOG.md")

      if [ -n "$EN_NOTES" ]; then
        echo "<!-- lang:en -->" > "$BODY_FILE"
        echo "$EN_NOTES" >> "$BODY_FILE"

        if [ -f "CHANGELOG.zh-CN.md" ]; then
          ZH_NOTES=$(extract_version_section "CHANGELOG.zh-CN.md")
          if [ -n "$ZH_NOTES" ]; then
            echo "" >> "$BODY_FILE"
            echo "<!-- lang:zh-CN -->" >> "$BODY_FILE"
            echo "$ZH_NOTES" >> "$BODY_FILE"
          fi
        fi
      else
        USE_GENERATED="true"
      fi

      echo "body_file=$BODY_FILE" >> "$GITHUB_OUTPUT"
      echo "use_generated=$USE_GENERATED" >> "$GITHUB_OUTPUT"

  - name: Create Release
    id: create_release
    uses: softprops/action-gh-release@v2
    with:
      draft: true
      body_path: ${{ steps.changelog.outputs.use_generated == 'false' && steps.changelog.outputs.body_file || '' }}
      generate_release_notes: ${{ steps.changelog.outputs.use_generated == 'true' }}
```

- [ ] **Step 3: Verify YAML syntax**

Run: `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/release.yml'))" 2>&1 || echo "YAML syntax error"`
Expected: No output (valid YAML)

- [ ] **Step 4: Commit**

```bash
git add .github/workflows/release.yml
git commit -m "ci: extract CHANGELOG content into GitHub Release body with bilingual support"
```

---

### Task 5: Update project documentation

**Files:**

- Modify: `CHANGELOG.md` (add entry about this feature)
- Modify: `CHANGELOG.zh-CN.md` (add matching Chinese entry)

- [ ] **Step 1: Add changelog entries for this feature**

Add to the `### Added` section of `## [2.8.0]` in `CHANGELOG.md`:

```markdown
- **Bilingual release notes**: GitHub Releases now display actual CHANGELOG content instead of generic diff links, with Chinese translation support and locale-aware display in the updater UI
```

Add matching entry to `CHANGELOG.zh-CN.md` in the `### 新增` section:

```markdown
- **双语更新日志**：GitHub Releases 现在展示实际的 CHANGELOG 内容而非通用的 diff 链接，支持中文翻译并在更新界面根据语言设置展示对应内容
```

- [ ] **Step 2: Commit**

```bash
git add CHANGELOG.md CHANGELOG.zh-CN.md
git commit -m "docs: add bilingual release notes feature to changelog"
```
