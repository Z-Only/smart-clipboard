# Phase 4 — Roadmap

These features were mentioned in the original design spec but deferred from Phase 1–3. They are candidates for future development.

## Feature 14: SQLCipher Database Encryption

**Priority**: High  
**Design Spec Reference**: Section 一.6 — "数据加密：本地数据库使用 SQLCipher 加密存储"

Replace the current plaintext SQLite database with an encrypted SQLCipher database. This ensures that clipboard data (which may contain sensitive information) is protected at rest.

### Implementation Notes

- Switch `rusqlite` dependency from `bundled` feature to `bundled-sqlcipher` or `bundled-sqlcipher-vendored-openssl`
- Derive encryption key from a user-configured password or a device-specific key stored in the OS keychain
- Add a "Database Encryption" section to Settings panel with options: enable/disable, change password
- Handle migration from unencrypted to encrypted database
- Performance impact should be negligible for the expected data volume

---

## Feature 15: App Lock (Password / Biometric)

**Priority**: Medium  
**Design Spec Reference**: Section 一.6 — "锁定功能：应用可设置密码/生物识别锁定"

Add an app-level lock that requires authentication before accessing the clipboard history.

### Implementation Notes

- Support two unlock methods: password and biometric (Touch ID / Windows Hello / Linux PAM)
- When enabled, the app shows a lock screen on launch and when revealed from tray/hotkey
- Use Tauri's biometric plugin or platform-specific APIs
- Auto-lock after configurable idle timeout
- Lock state persisted across app restarts

---

## Feature 16: Pinyin Fuzzy Search

**Priority**: Medium  
**Design Spec Reference**: Section 一.3 — "模糊匹配：支持拼音首字母、模糊关键词搜索"

Enhance the full-text search to support Chinese pinyin input, allowing users to find Chinese content by typing its pinyin initials.

### Implementation Notes

- Option A: Extend SQLite FTS5 with a custom pinyin tokenizer
- Option B: Pre-compute pinyin representation on insert and store in a separate column, then search both content and pinyin
- Option C: Use a Rust pinyin crate (e.g., `pinyin`) to convert content at search time
- Support both full pinyin (e.g., "zhejiang" → "浙江") and initial pinyin (e.g., "zj" → "浙江")

---

## Feature 17: Virtual Scrolling

**Priority**: Low  
**Design Spec Reference**: Section 二 — Project structure comment "ClipboardList.vue # 历史列表（虚拟滚动）"

Replace the current IntersectionObserver-based infinite scroll with true virtual scrolling to handle very large entry counts efficiently.

### Implementation Notes

- Integrate a virtual scroll library (e.g., `vue-virtual-scroller` or `@tanstack/vue-virtual`)
- Render only the visible portion of the entry list, keeping DOM node count constant
- Maintain smooth scrolling with date group headers as sticky labels
- Target: smooth performance with 10,000+ entries

---

## Feature 18: Batch Operations

**Priority**: Low  
**Design Spec Reference**: Section 一.4 — "批量操作：多选后合并粘贴、批量删除"

Allow users to select multiple entries and perform operations on them together.

### Implementation Notes

- Add a multi-select mode toggle in the toolbar
- Show checkboxes on entry cards when in multi-select mode
- Support Shift+Click and Ctrl/Cmd+Click for range and toggle selection
- Actions: merge selected entries (concatenate with newline), copy all, delete all, add tag to all
- Show selection count in the status bar
