# Quick Paste Panel Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a lightweight keyboard-driven quick-paste overlay that lets users instantly paste recent clipboard entries via number keys 1-9, with seamless transition to full search.

**Architecture:** A new `QuickPasteOverlay.vue` component renders as a modal overlay inside the existing main window. A second global shortcut (`Cmd/Ctrl+Shift+C`, configurable) triggers the overlay via a Tauri event. A dedicated `get_recent_entries` backend command provides a fast, lightweight query for the N most recent entries. The overlay captures keyboard input for number-key selection, arrow navigation, and search transition.

**Tech Stack:** Vue 3 + TypeScript (frontend overlay & store), Rust (backend command & hotkey), Tauri 2 IPC events, vue-i18n (translations)

---

## File Structure

| Action | File                                   | Responsibility                                                  |
| ------ | -------------------------------------- | --------------------------------------------------------------- |
| Modify | `src-tauri/src/config.rs`              | Add `quick_paste_shortcut` and `quick_paste_entry_count` fields |
| Modify | `src-tauri/src/commands/clipboard.rs`  | Add `get_recent_entries` command                                |
| Modify | `src-tauri/src/commands/mod.rs`        | Re-export `get_recent_entries`                                  |
| Modify | `src-tauri/src/lib.rs`                 | Register `get_recent_entries` in invoke handler                 |
| Modify | `src-tauri/src/hotkey.rs`              | Register quick-paste shortcut, emit event                       |
| Modify | `src/stores/clipboardStore.ts`         | Add `recentEntries` state and `fetchRecentEntries()` action     |
| Create | `src/components/QuickPasteOverlay.vue` | Overlay UI with keyboard handling                               |
| Modify | `src/App.vue`                          | Mount overlay, listen for quick-paste event                     |
| Modify | `src/i18n/locales/en.ts`               | English quick-paste i18n keys                                   |
| Modify | `src/i18n/locales/zh-CN.ts`            | Chinese quick-paste i18n keys                                   |
| Create | `tests/unit/QuickPasteOverlay.test.ts` | Frontend overlay unit tests                                     |
| Modify | `tests/unit/clipboardStore.test.ts`    | Test `fetchRecentEntries` action                                |

---

### Task 1: Add quick-paste config fields to backend

**Files:**

- Modify: `src-tauri/src/config.rs`

- [ ] **Step 1: Add quick-paste fields to `AppConfig`**

In `src-tauri/src/config.rs`, add two new fields to the `AppConfig` struct (after the `plugin_enabled` field):

```rust
    #[serde(default)]
    pub plugin_enabled: HashMap<String, bool>,
    #[serde(default = "default_quick_paste_shortcut")]
    pub quick_paste_shortcut: String,
    #[serde(default = "default_quick_paste_entry_count")]
    pub quick_paste_entry_count: u8,
```

Add the default functions before the `AppConfig` struct:

```rust
fn default_quick_paste_shortcut() -> String {
    "CommandOrControl+Shift+C".to_string()
}

fn default_quick_paste_entry_count() -> u8 {
    9
}
```

Update the `Default` impl for `AppConfig` to include the new fields:

```rust
            plugin_enabled: HashMap::new(),
            quick_paste_shortcut: default_quick_paste_shortcut(),
            quick_paste_entry_count: default_quick_paste_entry_count(),
```

- [ ] **Step 2: Verify Rust compiles**

Run: `cargo check --manifest-path src-tauri/Cargo.toml 2>&1 | tail -5`
Expected: no errors

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/config.rs
git commit -m "feat(config): add quick-paste shortcut and entry count settings"
```

---

### Task 2: Add `get_recent_entries` backend command

**Files:**

- Modify: `src-tauri/src/commands/clipboard.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add the command to `commands/clipboard.rs`**

Append the following function at the end of `src-tauri/src/commands/clipboard.rs` (before the closing of the file):

```rust
#[tauri::command]
pub async fn get_recent_entries(
    lock: State<'_, Arc<AppLockManager>>,
    encryption: State<'_, Arc<EncryptionManager>>,
    db: State<'_, Arc<Database>>,
    limit: i64,
) -> Result<Vec<crate::storage::ClipboardEntry>, String> {
    require_unlocked(&lock)?;
    let capped_limit = limit.clamp(1, 9);
    let result = db
        .get_entries(capped_limit, 0, None, None)
        .map_err(|e| e.to_string())?;
    let mut entries = result.entries;
    decrypt_entries(&encryption, &mut entries);
    Ok(entries)
}
```

- [ ] **Step 2: Re-export in `commands/mod.rs`**

In `src-tauri/src/commands/mod.rs`, add `get_recent_entries` to the `pub use clipboard::{...}` block:

```rust
pub use clipboard::{
    copy_entries, delete_entries, delete_entry, get_entries, get_entry_count, get_recent_entries,
    get_statistics, paste_entry, search_entries, set_favorite_state_for_entries, toggle_favorite,
};
```

- [ ] **Step 3: Register in `lib.rs` invoke handler**

In `src-tauri/src/lib.rs`, add `commands::clipboard::get_recent_entries,` right after the `commands::clipboard::paste_entry,` line inside the `invoke_handler` macro:

```rust
            commands::clipboard::paste_entry,
            commands::clipboard::get_recent_entries,
```

- [ ] **Step 4: Verify Rust compiles**

Run: `cargo check --manifest-path src-tauri/Cargo.toml 2>&1 | tail -5`
Expected: no errors

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands/clipboard.rs src-tauri/src/commands/mod.rs src-tauri/src/lib.rs
git commit -m "feat(commands): add get_recent_entries command for quick paste"
```

---

### Task 3: Register quick-paste hotkey in backend

**Files:**

- Modify: `src-tauri/src/hotkey.rs`

- [ ] **Step 1: Add quick-paste hotkey setup**

In `src-tauri/src/hotkey.rs`, add a new public function after the existing `setup_hotkey` function (and before `toggle_window`):

```rust
pub fn setup_quick_paste_hotkey<R: Runtime>(
    app: &AppHandle<R>,
    shortcut_str: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let shortcut: Shortcut = shortcut_str.parse()?;

    app.global_shortcut()
        .on_shortcut(shortcut, move |app_handle, _shortcut, event| {
            if event.state == ShortcutState::Pressed {
                activate_quick_paste(app_handle);
            }
        })?;

    Ok(())
}

fn activate_quick_paste<R: Runtime>(app_handle: &AppHandle<R>) {
    let Some(window) = app_handle.get_webview_window("main") else {
        return;
    };

    // Check lock state first
    if let Some(lock_manager) = app_handle.try_state::<std::sync::Arc<AppLockManager>>() {
        if lock_manager.is_locked() {
            enforce_window_access(app_handle, &lock_manager, "quick_paste");
            return;
        }
    }

    // Show window if hidden, then emit quick-paste event
    if !window.is_visible().unwrap_or(false) {
        let _ = window.show();
        let _ = window.set_focus();
    }
    let _ = app_handle.emit("quick-paste-activated", ());
}
```

Add the missing import for `enforce_window_access` if not already present (it is already imported at the top of the file).

- [ ] **Step 2: Wire up in `app_setup.rs`**

In `src-tauri/src/app_setup.rs`, after the existing hotkey setup block, add the quick-paste hotkey setup. Find the block:

```rust
    if let Err(e) = hotkey::setup_hotkey(app.handle()) {
        log::error!("Failed to setup hotkey: {}", e);
    }
```

Add after it:

```rust
    let quick_paste_shortcut = config.quick_paste_shortcut.clone();
    if let Err(e) = hotkey::setup_quick_paste_hotkey(app.handle(), &quick_paste_shortcut) {
        log::error!("Failed to setup quick-paste hotkey: {}", e);
    }
```

- [ ] **Step 3: Verify Rust compiles**

Run: `cargo check --manifest-path src-tauri/Cargo.toml 2>&1 | tail -5`
Expected: no errors

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/hotkey.rs src-tauri/src/app_setup.rs
git commit -m "feat(hotkey): register quick-paste keyboard shortcut"
```

---

### Task 4: Add i18n keys for quick paste

**Files:**

- Modify: `src/i18n/locales/en.ts`
- Modify: `src/i18n/locales/zh-CN.ts`

- [ ] **Step 1: Add English i18n keys**

In `src/i18n/locales/en.ts`, add a new `quickPaste` section. Insert it before the closing `};` of the default export (before the `encryption` section):

```typescript
  quickPaste: {
    title: 'Quick Paste',
    dismiss: 'Esc to close',
    typeToSearch: 'Type to search...',
    empty: 'No recent entries',
    imageEntry: '[Image]',
  },
```

Also add quick-paste settings keys inside the existing `settings` object (after the `plugins` subsection):

```typescript
    quickPaste: {
      title: 'Quick Paste',
      hint: 'Configure the quick paste shortcut and behavior',
      shortcut: 'Shortcut',
      entryCount: 'Entries to show',
    },
```

- [ ] **Step 2: Add Chinese i18n keys**

In `src/i18n/locales/zh-CN.ts`, add a matching `quickPaste` section before the closing `};` (before the `encryption` section):

```typescript
  quickPaste: {
    title: '快捷粘贴',
    dismiss: 'Esc 关闭',
    typeToSearch: '输入搜索...',
    empty: '暂无最近记录',
    imageEntry: '[图片]',
  },
```

Also add quick-paste settings keys inside the existing `settings` object (after the `plugins` subsection):

```typescript
    quickPaste: {
      title: '快捷粘贴',
      hint: '配置快捷粘贴的快捷键和行为',
      shortcut: '快捷键',
      entryCount: '显示条目数',
    },
```

- [ ] **Step 3: Verify TypeScript compiles**

Run: `pnpm run typecheck 2>&1 | tail -5`
Expected: no errors

- [ ] **Step 4: Commit**

```bash
git add src/i18n/locales/en.ts src/i18n/locales/zh-CN.ts
git commit -m "feat(i18n): add quick paste English and Chinese translations"
```

---

### Task 5: Add `fetchRecentEntries` to clipboardStore

**Files:**

- Modify: `src/stores/clipboardStore.ts`
- Modify: `tests/unit/clipboardStore.test.ts`

- [ ] **Step 1: Write the failing test**

In `tests/unit/clipboardStore.test.ts`, add the following test at the end of the outer `describe('useClipboardStore', ...)` block (before its closing `});`):

```typescript
describe('fetchRecentEntries', () => {
  it('invokes get_recent_entries and stores result', async () => {
    const entry1 = makeEntry({ id: 1, content: 'first' });
    const entry2 = makeEntry({ id: 2, content: 'second' });
    invoke.mockResolvedValueOnce([entry1, entry2]);

    const { useClipboardStore } = await import('@/stores/clipboardStore');
    const store = useClipboardStore();

    await store.fetchRecentEntries(9);
    expect(invoke).toHaveBeenCalledWith('get_recent_entries', { limit: 9 });
    expect(store.recentEntries).toEqual([entry1, entry2]);
  });

  it('sets recentEntries to empty array on error', async () => {
    invoke.mockRejectedValueOnce(new Error('locked'));

    const { useClipboardStore } = await import('@/stores/clipboardStore');
    const store = useClipboardStore();

    await store.fetchRecentEntries(9);
    expect(store.recentEntries).toEqual([]);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `pnpm run test:web -- --reporter=verbose 2>&1 | grep -A 2 "fetchRecentEntries" | cat`
Expected: FAIL — `store.fetchRecentEntries is not a function` or `store.recentEntries` is undefined

- [ ] **Step 3: Implement `recentEntries` state and `fetchRecentEntries` action**

In `src/stores/clipboardStore.ts`:

1. Add a new ref after the existing `pendingLoadMore` declaration (around line 56):

```typescript
const recentEntries = ref<ClipboardEntry[]>([]);
```

2. Add a new async function after the `clearSensitiveViewState` function:

```typescript
async function fetchRecentEntries(count: number) {
  try {
    recentEntries.value = await invoke<ClipboardEntry[]>('get_recent_entries', { limit: count });
  } catch (e) {
    console.error('Failed to fetch recent entries:', e);
    recentEntries.value = [];
  }
}
```

3. Add both to the return statement — add `recentEntries` after `entries` and `fetchRecentEntries` after `fetchEntries`:

```typescript
    recentEntries,
```

```typescript
    fetchRecentEntries,
```

- [ ] **Step 4: Run test to verify it passes**

Run: `pnpm run test:web -- --reporter=verbose 2>&1 | grep -A 2 "fetchRecentEntries" | cat`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/stores/clipboardStore.ts tests/unit/clipboardStore.test.ts
git commit -m "feat(store): add recentEntries state and fetchRecentEntries action"
```

---

### Task 6: Create `QuickPasteOverlay.vue` component

**Files:**

- Create: `src/components/QuickPasteOverlay.vue`

- [ ] **Step 1: Create the overlay component**

Create `src/components/QuickPasteOverlay.vue`:

```vue
<template>
  <Teleport to="body">
    <div
      v-if="isActive"
      class="fixed inset-0 z-50 flex items-start justify-center pt-16"
      @click.self="$emit('dismiss')"
      @keydown="handleKeydown"
    >
      <div
        ref="panelRef"
        class="w-[380px] max-h-[480px] rounded-xl border border-border bg-background shadow-2xl flex flex-col overflow-hidden"
        tabindex="-1"
      >
        <!-- Header -->
        <div
          class="flex items-center justify-between px-4 py-2.5 border-b border-border bg-muted/30"
        >
          <span class="text-sm font-medium">{{ $t('quickPaste.title') }}</span>
          <span class="text-xs text-muted-foreground">{{ $t('quickPaste.dismiss') }}</span>
        </div>

        <!-- Entry list -->
        <div
          v-if="entries.length === 0"
          class="flex items-center justify-center py-10 text-sm text-muted-foreground"
        >
          {{ $t('quickPaste.empty') }}
        </div>
        <div v-else class="flex-1 overflow-y-auto">
          <button
            v-for="(entry, index) in entries"
            :key="entry.id"
            class="flex w-full items-center gap-3 px-4 py-2.5 text-left transition-colors hover:bg-accent/50"
            :class="{ 'bg-accent': index === activeIndex }"
            @click="$emit('paste', entry.id)"
            @mouseenter="activeIndex = index"
          >
            <span
              class="flex h-5 w-5 shrink-0 items-center justify-center rounded bg-primary/10 text-xs font-semibold text-primary"
            >
              {{ index + 1 }}
            </span>
            <span class="text-sm shrink-0">{{ categoryIcon(entry.category) }}</span>
            <span class="flex-1 truncate text-sm">{{ displayContent(entry) }}</span>
            <span class="shrink-0 text-[10px] text-muted-foreground">{{
              relativeTime(entry.created_at)
            }}</span>
          </button>
        </div>

        <!-- Footer -->
        <div class="border-t border-border px-4 py-2 text-xs text-muted-foreground">
          {{ $t('quickPaste.typeToSearch') }}
        </div>
      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { ref, watch, nextTick } from 'vue';
import { useI18n } from 'vue-i18n';
import type { ClipboardEntry } from '@/types';
import { CATEGORIES } from '@/types';

const props = defineProps<{
  entries: ClipboardEntry[];
  isActive: boolean;
}>();

const emit = defineEmits<{
  paste: [entryId: number];
  dismiss: [];
  search: [text: string];
}>();

const { t } = useI18n();

const activeIndex = ref(0);
const panelRef = ref<HTMLDivElement | null>(null);

watch(
  () => props.isActive,
  async (active) => {
    if (active) {
      activeIndex.value = 0;
      await nextTick();
      panelRef.value?.focus();
    }
  },
);

function categoryIcon(category: string): string {
  const found = CATEGORIES.find((c) => c.key === category);
  return found?.icon ?? '📋';
}

function displayContent(entry: ClipboardEntry): string {
  if (entry.content_type === 'image') return t('quickPaste.imageEntry');
  return entry.content.replace(/\n/g, ' ').slice(0, 80);
}

function relativeTime(dateStr: string): string {
  const now = Date.now();
  const then = new Date(dateStr).getTime();
  const diffSeconds = Math.floor((now - then) / 1000);
  if (diffSeconds < 60) return t('entry.justNow');
  const diffMinutes = Math.floor(diffSeconds / 60);
  if (diffMinutes < 60) return t('entry.minutesAgo', { n: diffMinutes });
  const diffHours = Math.floor(diffMinutes / 60);
  if (diffHours < 24) return t('entry.hoursAgo', { n: diffHours });
  const diffDays = Math.floor(diffHours / 24);
  return t('entry.daysAgo', { n: diffDays });
}

function handleKeydown(event: KeyboardEvent) {
  const key = event.key;

  // Number keys 1-9 → paste corresponding entry
  if (key >= '1' && key <= '9') {
    const index = parseInt(key) - 1;
    if (index < props.entries.length) {
      event.preventDefault();
      emit('paste', props.entries[index].id);
    }
    return;
  }

  switch (key) {
    case 'Escape':
      event.preventDefault();
      emit('dismiss');
      break;
    case 'Enter':
      event.preventDefault();
      if (props.entries.length > 0) {
        emit('paste', props.entries[activeIndex.value].id);
      }
      break;
    case 'ArrowDown':
      event.preventDefault();
      activeIndex.value = Math.min(activeIndex.value + 1, props.entries.length - 1);
      break;
    case 'ArrowUp':
      event.preventDefault();
      activeIndex.value = Math.max(activeIndex.value - 1, 0);
      break;
    default:
      // Any printable character → transition to search
      if (key.length === 1 && !event.ctrlKey && !event.metaKey && !event.altKey) {
        event.preventDefault();
        emit('search', key);
      }
      break;
  }
}
</script>
```

- [ ] **Step 2: Verify TypeScript compiles**

Run: `pnpm run typecheck 2>&1 | tail -5`
Expected: no errors

- [ ] **Step 3: Commit**

```bash
git add src/components/QuickPasteOverlay.vue
git commit -m "feat(ui): create QuickPasteOverlay component with keyboard navigation"
```

---

### Task 7: Integrate overlay into `App.vue`

**Files:**

- Modify: `src/App.vue`

- [ ] **Step 1: Add imports and quick-paste logic**

In `src/App.vue`, add the import for the new component after the existing `ConflictResolveDialog` import:

```typescript
import QuickPasteOverlay from '@/components/QuickPasteOverlay.vue';
```

Add quick-paste reactive state after the existing `showLockOverlay` computed (around line 178):

```typescript
const quickPasteActive = ref(false);
const { recentEntries } = storeToRefs(store);
```

Add quick-paste handler functions after the `watch` block for `security.status.locked`:

```typescript
async function activateQuickPaste() {
  if (security.status.locked) return;
  await store.fetchRecentEntries(9);
  quickPasteActive.value = true;
}

async function handleQuickPaste(entryId: number) {
  quickPasteActive.value = false;
  await store.pasteEntry(entryId);
  const window = (await import('@tauri-apps/api/window')).getCurrentWindow();
  await window.hide();
}

function handleQuickPasteDismiss() {
  quickPasteActive.value = false;
}

function handleQuickPasteSearch(text: string) {
  quickPasteActive.value = false;
  searchBarRef.value?.focus();
  store.setSearch(text);
}
```

In the `onMounted` callback, add a listener for the quick-paste event after the existing `listen('open-settings', ...)` block:

```typescript
await listen('quick-paste-activated', () => {
  activateQuickPaste();
});
```

Also ensure that the `quickPasteActive` is reset when the app locks. In the `watch` for `security.status.locked`, add inside the `if (locked) { ... }` block:

```typescript
quickPasteActive.value = false;
```

- [ ] **Step 2: Add the component to the template**

In the `<template>` section of `src/App.vue`, add the overlay right before the closing `</div>` of the root element (after the `ConflictResolveDialog`):

```vue
<!-- Quick paste overlay -->
<QuickPasteOverlay
  :entries="recentEntries"
  :is-active="quickPasteActive"
  @paste="handleQuickPaste"
  @dismiss="handleQuickPasteDismiss"
  @search="handleQuickPasteSearch"
/>
```

- [ ] **Step 3: Verify TypeScript compiles**

Run: `pnpm run typecheck 2>&1 | tail -5`
Expected: no errors

- [ ] **Step 4: Commit**

```bash
git add src/App.vue
git commit -m "feat(app): integrate QuickPasteOverlay with event listener and paste handler"
```

---

### Task 8: Write unit tests for QuickPasteOverlay

**Files:**

- Create: `tests/unit/QuickPasteOverlay.test.ts`

- [ ] **Step 1: Create the test file**

Create `tests/unit/QuickPasteOverlay.test.ts`:

```typescript
import { describe, expect, it, vi } from 'vitest';
import { mount } from '@vue/test-utils';
import { createI18n } from 'vue-i18n';
import QuickPasteOverlay from '@/components/QuickPasteOverlay.vue';
import type { ClipboardEntry } from '@/types';

function makeEntry(overrides: Partial<ClipboardEntry> = {}): ClipboardEntry {
  return {
    id: 1,
    content: 'test content',
    content_type: 'text',
    category: 'text',
    hash: 'abc123',
    source_app: null,
    is_favorite: false,
    is_sensitive: false,
    use_count: 1,
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
    expires_at: null,
    ...overrides,
  };
}

const i18n = createI18n({
  legacy: false,
  locale: 'en',
  messages: {
    en: {
      quickPaste: {
        title: 'Quick Paste',
        dismiss: 'Esc to close',
        typeToSearch: 'Type to search...',
        empty: 'No recent entries',
        imageEntry: '[Image]',
      },
      entry: {
        justNow: 'just now',
        minutesAgo: '{n}m ago',
        hoursAgo: '{n}h ago',
        daysAgo: '{n}d ago',
      },
    },
  },
});

function mountOverlay(entries: ClipboardEntry[] = [], isActive = true) {
  return mount(QuickPasteOverlay, {
    props: { entries, isActive },
    global: { plugins: [i18n] },
    attachTo: document.body,
  });
}

describe('QuickPasteOverlay', () => {
  it('renders nothing when isActive is false', () => {
    const wrapper = mountOverlay([], false);
    expect(wrapper.find('.fixed').exists()).toBe(false);
  });

  it('renders entry list with number badges when active', () => {
    const entries = [
      makeEntry({ id: 1, content: 'First entry' }),
      makeEntry({ id: 2, content: 'Second entry' }),
      makeEntry({ id: 3, content: 'Third entry' }),
    ];
    const wrapper = mountOverlay(entries);
    const buttons = wrapper.findAll('button');
    expect(buttons).toHaveLength(3);
    expect(buttons[0].text()).toContain('1');
    expect(buttons[0].text()).toContain('First entry');
    expect(buttons[1].text()).toContain('2');
    expect(buttons[2].text()).toContain('3');
  });

  it('shows empty message when no entries', () => {
    const wrapper = mountOverlay([]);
    expect(wrapper.text()).toContain('No recent entries');
  });

  it('emits paste when number key 1 is pressed', async () => {
    const entries = [makeEntry({ id: 42, content: 'hello' })];
    const wrapper = mountOverlay(entries);
    await wrapper.find('.fixed').trigger('keydown', { key: '1' });
    expect(wrapper.emitted('paste')).toEqual([[42]]);
  });

  it('emits paste on Enter for active entry', async () => {
    const entries = [makeEntry({ id: 10 }), makeEntry({ id: 20 })];
    const wrapper = mountOverlay(entries);
    await wrapper.find('.fixed').trigger('keydown', { key: 'Enter' });
    expect(wrapper.emitted('paste')).toEqual([[10]]);
  });

  it('emits dismiss on Escape', async () => {
    const wrapper = mountOverlay([makeEntry()]);
    await wrapper.find('.fixed').trigger('keydown', { key: 'Escape' });
    expect(wrapper.emitted('dismiss')).toHaveLength(1);
  });

  it('emits search when a letter is typed', async () => {
    const wrapper = mountOverlay([makeEntry()]);
    await wrapper.find('.fixed').trigger('keydown', { key: 'a' });
    expect(wrapper.emitted('search')).toEqual([['a']]);
  });

  it('displays [Image] for image entries', () => {
    const entries = [
      makeEntry({ id: 1, content: '/path/to/img.png', content_type: 'image', category: 'image' }),
    ];
    const wrapper = mountOverlay(entries);
    expect(wrapper.text()).toContain('[Image]');
  });

  it('navigates with arrow keys', async () => {
    const entries = [makeEntry({ id: 1 }), makeEntry({ id: 2 })];
    const wrapper = mountOverlay(entries);
    const overlay = wrapper.find('.fixed');

    await overlay.trigger('keydown', { key: 'ArrowDown' });
    await overlay.trigger('keydown', { key: 'Enter' });
    expect(wrapper.emitted('paste')).toEqual([[2]]);
  });
});
```

- [ ] **Step 2: Run tests to verify they pass**

Run: `pnpm run test:web -- --reporter=verbose 2>&1 | grep -E "(QuickPasteOverlay|PASS|FAIL)" | cat`
Expected: all tests PASS

- [ ] **Step 3: Commit**

```bash
git add tests/unit/QuickPasteOverlay.test.ts
git commit -m "test: add QuickPasteOverlay unit tests"
```

---

### Task 9: Run full quality gate

**Files:** (none — verification only)

- [ ] **Step 1: Run format check**

Run: `pnpm run format:check 2>&1 | tail -5`
Expected: no formatting issues (run `pnpm run format` first if needed)

- [ ] **Step 2: Run lint**

Run: `pnpm run lint:web 2>&1 | tail -5`
Expected: no lint errors

- [ ] **Step 3: Run typecheck**

Run: `pnpm run typecheck 2>&1 | tail -5`
Expected: no type errors

- [ ] **Step 4: Run all frontend tests**

Run: `pnpm run test:web 2>&1 | tail -10`
Expected: all tests pass, including the new QuickPasteOverlay and clipboardStore tests

- [ ] **Step 5: Run Rust checks**

Run: `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings 2>&1 | tail -5`
Expected: no warnings

- [ ] **Step 6: Final commit if any formatting changes**

```bash
git add -A
git commit -m "chore: format and lint fixes for quick paste feature"
```

(Skip if no changes were needed.)
