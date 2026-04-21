# Phase 4 Implementation Package Breakdown

## Purpose

This document reorganizes the remaining Phase 4 roadmap items into four larger implementation packages that are better suited for iterative delivery with superpowers. The primary goal is to reduce context switching, keep each development session focused, and improve the odds that each session ends with a verifiable, usable increment.

Compared with implementing Phase 4 strictly by feature number, this package-based plan groups work by technical coupling and user-facing outcomes.

---

## Recommended Package Order

1. **Phase 4A — Search Enhancements**
2. **Phase 4B — List Interaction Enhancements**
3. **Phase 4C — Access Security**
4. **Phase 4D — Data Security**

This order is recommended because it starts with lower-risk, highly visible improvements, then moves into more invasive UI work, followed by app-level security controls, and finally the highest-risk storage-layer change.

---

## Phase 4A — Search Enhancements

### Included roadmap items

- **Feature 16: Pinyin Fuzzy Search**

### Goal

Improve retrieval quality for Chinese-language clipboard entries by allowing users to search Chinese content using:

- original Chinese text
- full pinyin
- pinyin initials

### Why this is a standalone package

This work is the most self-contained item in Phase 4:

- it does not require list rendering changes
- it does not depend on app lock or database encryption
- it can be implemented largely in the storage and search layer
- it provides immediate user-facing value with relatively low implementation risk

### Recommended implementation scope

Use a **precomputed pinyin field** approach instead of a custom SQLite tokenizer.

Recommended scope for this package:

1. Add searchable fields such as:
   - `pinyin_full`
   - `pinyin_initials`
2. Generate pinyin data when clipboard entries are inserted or updated
3. Extend search to match against:
   - original content
   - full pinyin
   - pinyin initials
4. Preserve current behavior for:
   - category filtering
   - pagination
   - English and non-Chinese search

### Explicitly out of scope

- custom SQLite tokenizer work
- advanced multilingual tokenization
- search suggestions
- search highlighting
- pinyin indexing for unrelated entities unless needed later

### Expected files/modules

Likely areas of change:

- `/Users/chanyu/AIProjects/smart-clipboard/src-tauri/src/storage/database.rs`
- `/Users/chanyu/AIProjects/smart-clipboard/src-tauri/src/storage/migrations.rs`
- `/Users/chanyu/AIProjects/smart-clipboard/src-tauri/src/storage/models.rs`
- `/Users/chanyu/AIProjects/smart-clipboard/src-tauri/src/commands.rs`
- `/Users/chanyu/AIProjects/smart-clipboard/src/stores/clipboardStore.ts`
- `/Users/chanyu/AIProjects/smart-clipboard/src/composables/useSearch.ts`
- `/Users/chanyu/AIProjects/smart-clipboard/src-tauri/Cargo.toml` (if a Rust pinyin dependency is introduced)

### Acceptance criteria

- searching by original Chinese text still works
- searching by full pinyin matches relevant Chinese entries
- searching by pinyin initials matches relevant Chinese entries
- English search behavior does not regress
- category filtering and pagination continue to work correctly

### Suggested superpowers prompt

> Please implement the Phase 4A Search Enhancements package, which currently includes pinyin fuzzy search only. Use a precomputed pinyin-field approach rather than a custom SQLite tokenizer. Add `pinyin_full` and `pinyin_initials` fields for clipboard entries, generate them at write time, and make search match original text, full pinyin, and initials while preserving existing category filtering and pagination behavior. Prioritize a stable, simple, migration-friendly implementation. When finished, report changed files, design decisions, test steps, and known limitations.

---

## Phase 4B — List Interaction Enhancements

### Included roadmap items

- **Feature 17: Virtual Scrolling**
- **Feature 18: Batch Operations**

### Goal

Upgrade the main clipboard list so that it is both more powerful to operate and more scalable with large data volumes.

This package focuses on two user-facing outcomes:

- users can select and act on multiple entries efficiently
- the list remains smooth and responsive with large numbers of entries

### Why these items are grouped together

Both roadmap items target the same high-traffic UI area and share list-state concerns:

- list rendering
- item selection state
- scroll behavior
- keyboard navigation compatibility

They are strongly coupled in the frontend implementation, especially around `/Users/chanyu/AIProjects/smart-clipboard/src/components/ClipboardList.vue`.

### Recommended internal delivery order

Although this is one package, it should still be delivered in this order:

1. **Batch operations foundation**
2. **Virtual scrolling integration**

This reduces risk because selection state can be designed first, then adapted into a virtualized list instead of being invented during virtualization work.

### Recommended implementation scope

#### Part 1: Batch operations foundation

Recommended scope:

- add a multi-select mode toggle
- allow selecting multiple entries
- show selection count
- support batch delete
- support merge-and-copy using newline concatenation
- preserve current single-click paste behavior outside multi-select mode

#### Part 2: Virtual scrolling

Recommended scope:

- replace current effective full rendering with true virtualized rendering
- keep pagination/load-more behavior working
- maintain compatibility with single-select and multi-select state
- preserve keyboard navigation as much as possible
- keep date grouping where practical, but do not block on pixel-perfect sticky headers in the first pass

### Explicitly out of scope for the first pass

- Shift+Click range selection
- Cmd/Ctrl advanced selection details
- batch tagging
- automatic batch paste into external apps
- perfect sticky group headers from day one
- aggressive dynamic-height optimization for all content types

### Expected files/modules

Likely areas of change:

- `/Users/chanyu/AIProjects/smart-clipboard/src/components/ClipboardList.vue`
- `/Users/chanyu/AIProjects/smart-clipboard/src/components/EntryCard.vue`
- `/Users/chanyu/AIProjects/smart-clipboard/src/App.vue`
- `/Users/chanyu/AIProjects/smart-clipboard/src/stores/clipboardStore.ts`
- `/Users/chanyu/AIProjects/smart-clipboard/src-tauri/src/commands.rs`
- `/Users/chanyu/AIProjects/smart-clipboard/src-tauri/src/storage/database.rs`
- `/Users/chanyu/AIProjects/smart-clipboard/package.json` (if a virtualization library is added)

### Suggested design notes

- centralize multi-select state in the clipboard store
- consider flattening grouped list data into renderable item records such as:
  - header item
  - entry item
- adapt keyboard navigation after virtualization rather than preserving implementation details from the current list component

### Acceptance criteria

- users can enter and exit multi-select mode
- users can select multiple entries and see the selected count
- batch delete works reliably
- merge-and-copy works reliably
- large lists scroll noticeably more smoothly than before
- DOM node count remains controlled during scrolling
- existing load-more behavior still functions
- selection state does not break under virtualization

### Suggested superpowers prompt

> Please implement the Phase 4B List Interaction Enhancements package, which includes batch operations and virtual scrolling. Deliver it in two steps within the same package: first build a stable multi-select foundation with selection count, batch delete, and merge-and-copy; then refactor the list into a virtualized rendering approach. Keep selection state centralized in the store and reshape list data into a form suitable for virtualization if needed. Prioritize performance and reliability over perfect visual parity in the first pass. When finished, report changed files, implementation order, state design, compatibility handling, and remaining enhancement opportunities.

---

## Phase 4C — Access Security

### Included roadmap items

- **Feature 15: App Lock (Password / Biometric)**

### Goal

Prevent unauthorized access to clipboard history through app-level access control.

This package should protect the app at the interaction layer so that opening the window, showing it from the tray, or invoking it via hotkey does not expose data without authentication when locking is enabled.

### Why this is a standalone package

Although it belongs to the broader security theme, it is fundamentally different from database encryption:

- app lock protects access to the running app
- SQLCipher protects data at rest in storage

This package touches application state, launch behavior, tray behavior, hotkey flow, and settings UI. It should be implemented independently from storage encryption.

### Recommended implementation scope

This package should be developed in two layers within the same package.

#### Layer 1: Password lock baseline

Recommended scope:

- enable or disable app lock in settings
- set a password
- require unlock on app startup when enabled
- support manual lock
- require unlock when the app is shown from tray or hotkey if currently locked

#### Layer 2: Enhanced lock behavior

Recommended scope:

- configurable idle timeout
- automatic re-lock after inactivity
- biometric unlock when platform support is available
- fallback to password when biometric unlock is unavailable or fails

### Recommended design principles

- keep the security-sensitive logic primarily in Rust
- avoid storing plaintext passwords
- store password hashes only
- keep unlocked state primarily in memory
- let the frontend focus on lock-screen presentation and user interaction

### Explicitly important behavior

- tray reveal must not bypass lock state
- hotkey reveal must not bypass lock state
- biometric flows must fail safely back to password

### Expected files/modules

Likely areas of change:

- `/Users/chanyu/AIProjects/smart-clipboard/src/components/SettingsPanel.vue`
- `/Users/chanyu/AIProjects/smart-clipboard/src/App.vue`
- `/Users/chanyu/AIProjects/smart-clipboard/src/components/LockScreen.vue` (likely new)
- `/Users/chanyu/AIProjects/smart-clipboard/src/stores/lockStore.ts` (likely new)
- `/Users/chanyu/AIProjects/smart-clipboard/src-tauri/src/commands.rs`
- `/Users/chanyu/AIProjects/smart-clipboard/src-tauri/src/config.rs`
- `/Users/chanyu/AIProjects/smart-clipboard/src-tauri/src/lib.rs`
- `/Users/chanyu/AIProjects/smart-clipboard/src-tauri/src/main.rs`
- `/Users/chanyu/AIProjects/smart-clipboard/src-tauri/src/hotkey.rs`
- `/Users/chanyu/AIProjects/smart-clipboard/src-tauri/src/tray.rs`

### Acceptance criteria

- when app lock is enabled, restarting the app requires unlock
- manual lock requires successful unlock before content is shown again
- tray and hotkey reveal paths do not bypass lock
- password is not stored in plaintext
- idle auto-lock works when configured
- biometric unlock works where available and falls back safely where unavailable

### Suggested superpowers prompt

> Please implement the Phase 4C Access Security package for app-level access control. This package includes app lock with password and biometric support. Build it in two layers: first a stable password-lock baseline with startup unlock, manual lock, and tray/hotkey interception; then add idle auto-lock and biometric unlock with safe password fallback. Keep security-sensitive logic in Rust as much as possible, avoid plaintext password storage, and prioritize correct state transitions over UI polish. When finished, report changed files, lock-state flow, password storage strategy, platform-specific considerations, and remaining risks.

---

## Phase 4D — Data Security

### Included roadmap items

- **Feature 14: SQLCipher Database Encryption**

### Goal

Protect clipboard data at rest by migrating from plaintext SQLite storage to SQLCipher-encrypted storage.

### Why this is a standalone package

This is the most invasive and highest-risk remaining Phase 4 change because it affects:

- storage initialization
- database open behavior
- migration behavior
- compatibility with all existing stored features
- dependency and build stability

It should be implemented only after other behavior is relatively stable.

### Recommended implementation scope

Recommended scope for this package:

1. switch database dependencies and connection logic to support SQLCipher
2. define and implement a key-management strategy
3. support encrypted initialization for new databases
4. support migration from existing plaintext databases
5. expose clear encryption status in settings

### Key-management options

One of the following strategies should be selected for the first implementation:

#### Option A: User-provided database password

- user explicitly sets the database password
- app derives or uses an encryption key from that password
- user experience is more explicit but involves more prompts and recovery complexity

#### Option B: OS keychain-managed random key

- app generates a random key when encryption is enabled
- key is stored in the OS keychain
- user experience is smoother but keychain integration becomes important

Either strategy can be valid. The first version should optimize for implementation safety and user clarity.

### Explicitly out of scope for the first pass

- changing the encryption password
- disabling encryption after enablement
- advanced recovery workflows
- supporting multiple key-management modes simultaneously unless clearly justified

### Expected files/modules

Likely areas of change:

- `/Users/chanyu/AIProjects/smart-clipboard/src-tauri/Cargo.toml`
- `/Users/chanyu/AIProjects/smart-clipboard/src-tauri/src/storage/database.rs`
- `/Users/chanyu/AIProjects/smart-clipboard/src-tauri/src/storage/migrations.rs`
- `/Users/chanyu/AIProjects/smart-clipboard/src-tauri/src/config.rs`
- `/Users/chanyu/AIProjects/smart-clipboard/src/components/SettingsPanel.vue`
- application startup/initialization code as needed
- keychain integration code as needed

### Acceptance criteria

- new databases can be initialized and opened using encryption
- existing plaintext databases can be migrated to encrypted storage
- migrated data remains readable and correct after migration
- invalid or missing keys do not silently open the database
- key existing features such as search, tags, templates, and sync still operate without obvious regression

### Suggested superpowers prompt

> Please implement the Phase 4D Data Security package, whose goal is to migrate the current plaintext SQLite database to SQLCipher-encrypted storage. This package should include dependency updates, encrypted database open logic, a clear key-management strategy, encrypted initialization for new databases, migration for existing plaintext databases, and clear encryption status exposure in settings. Prioritize compatibility with existing data and minimization of regression risk. The first pass does not need password changes, disabling encryption, or advanced recovery flows. When finished, report dependency changes, key strategy, migration flow, failure modes, and verification steps.

---

## Cross-package execution guidance

### General guidance for each superpowers session

For each implementation package, use the following execution rules:

- stay strictly within the current package scope
- prefer the smallest usable closed loop over broad expansion
- avoid unrelated refactors
- reuse existing project structure where possible
- keep new state centralized rather than scattered across components

### Suggested completion checklist for every package

When a package session is complete, the implementation report should include:

1. changed files
2. implementation summary
3. testing steps
4. known limitations
5. recommended handoff for the next package

### Suggested handoff section template

At the end of each package implementation, include a short **Next Package Handoff** section with:

- completed capabilities
- unresolved boundary issues
- recommended next files to modify
- minimum target for the next package
- technical debt or follow-up risks to remember

---

## Summary

The remaining Phase 4 roadmap should be implemented as four larger packages:

1. **Search Enhancements** — pinyin fuzzy search
2. **List Interaction Enhancements** — batch operations and virtual scrolling
3. **Access Security** — password lock, auto-lock, and biometric unlock
4. **Data Security** — SQLCipher database encryption and migration

This package plan is intended to make Phase 4 delivery more predictable, easier to verify incrementally, and better aligned with the way superpowers sessions are typically executed.
