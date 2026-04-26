# 核心模块测试覆盖补充 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 为 clipboardStore、conflictStore、webdavStore 三个核心 store 补充单元测试，将覆盖率从 16-26% 提升至 ≥60%，并提升 CI 覆盖率阈值。

**Architecture:** 遵循项目已有测试模式（vi.mock Tauri IPC、createPinia、动态 import），按功能域分组编写测试。不修改业务代码，仅新增测试文件和调整覆盖率阈值。

**Tech Stack:** Vitest, Pinia, Vue 3, TypeScript

---

## File Map

- **Create:** `tests/unit/clipboardStore.test.ts`
  - 责任：clipboardStore 全面测试（虚拟滚动、标签缓存、数据获取、多选、收藏删除、分组、状态清理）
- **Create:** `tests/unit/conflictStore.test.ts`
  - 责任：conflictStore 全面测试（冲突检测、自动解决、手动解决、日志管理、配置）
- **Create:** `tests/unit/webdavStore.test.ts`
  - 责任：webdavStore 全面测试（配置加载保存、连接管理、同步触发、错误处理、状态清理）
- **Modify:** `vite.config.ts`
  - 责任：提升覆盖率阈值

## Task 1: clipboardStore.ts 测试 — 虚拟滚动与标签缓存

**Files:** Create `tests/unit/clipboardStore.test.ts`

- [ ] **Step 1:** 创建测试文件，编写虚拟滚动高度管理测试（setVirtualItemHeight / getVirtualItemHeight / clearVirtualItemHeights）
- [ ] **Step 2:** 编写标签缓存测试（setEntryTags / getEntryTags / clearEntryTagsCache / batchLoadEntryTags）
- [ ] **Step 3:** 运行 `pnpm run test:web -- tests/unit/clipboardStore.test.ts` 确认通过

## Task 2: clipboardStore.ts 测试 — 数据获取与分类

**Files:** Modify `tests/unit/clipboardStore.test.ts`

- [ ] **Step 1:** 编写 fetchEntries 测试（基本获取、搜索、分类过滤、标签过滤、收藏过滤）
- [ ] **Step 2:** 编写 loadMore 测试（分页加载、防重复加载）
- [ ] **Step 3:** 编写 groupedEntryItems 分组逻辑测试
- [ ] **Step 4:** 运行测试确认通过

## Task 3: clipboardStore.ts 测试 — 多选操作与收藏删除

**Files:** Modify `tests/unit/clipboardStore.test.ts`

- [ ] **Step 1:** 编写多选操作测试（enterMultiSelectMode / exitMultiSelectMode / toggleEntrySelection / selectAllLoadedEntries / invertLoadedSelection / selectRangeTo）
- [ ] **Step 2:** 编写收藏与删除测试（toggleFavorite / deleteEntry / deleteSelectedEntries / favoriteSelectedEntries）
- [ ] **Step 3:** 编写 onClipboardChanged / pasteEntry / clearSensitiveViewState 测试
- [ ] **Step 4:** 运行测试确认通过

## Task 4: conflictStore.ts 测试

**Files:** Create `tests/unit/conflictStore.test.ts`

- [ ] **Step 1:** 编写冲突检测测试（detectConflict / detectConflicts）
- [ ] **Step 2:** 编写自动解决策略测试（autoResolve 四种策略 / autoResolveAll）
- [ ] **Step 3:** 编写手动解决与日志测试（resolveManually / dismissConflict / clearLog / removeLogEntry）
- [ ] **Step 4:** 编写配置与对话框管理测试（updateConfig / updateStrategy / openConflictDialog / openNextConflict）
- [ ] **Step 5:** 运行测试确认通过

## Task 5: webdavStore.ts 测试

**Files:** Create `tests/unit/webdavStore.test.ts`

- [ ] **Step 1:** 编写配置管理测试（loadConfig / saveConfig）
- [ ] **Step 2:** 编写连接管理测试（connect / disconnect / refreshStatus / refreshAll）
- [ ] **Step 3:** 编写同步与设备管理测试（triggerSync / removeDevice）
- [ ] **Step 4:** 编写错误处理与状态清理测试（clearSensitiveState / clearError / 各方法错误场景）
- [ ] **Step 5:** 运行测试确认通过

## Task 6: 提升覆盖率阈值并全量验证

**Files:** Modify `vite.config.ts`

- [ ] **Step 1:** 将覆盖率阈值提升为 statements: 25, branches: 15, functions: 25, lines: 25
- [ ] **Step 2:** 运行 `pnpm run test:web:coverage` 确认覆盖率满足阈值
- [ ] **Step 3:** 运行 `pnpm run typecheck` 确认无类型错误
- [ ] **Step 4:** 运行 `pnpm run lint:web` 确认无 lint 错误
