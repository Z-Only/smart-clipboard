# Phase 4B — 批量操作 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 完成 Phase 4B 批量操作的 superpowers 工作流闭环：产出设计文档、实现计划，补齐 clipboardStore 批量操作单元测试，验证全量测试和 lint/typecheck 通过，产出完成报告。

**Architecture:** Phase 4B 的核心功能（多选模式、批量删除、合并复制、批量收藏、批量打标签、虚拟滚动）已在前序开发中实现。本次会话聚焦于测试覆盖补齐和工作流文档闭环。

**Tech Stack:** Vue 3, Pinia, TypeScript, Vitest, Tauri IPC

---

## File Map

- **Create:** `docs/superpowers/specs/2026-04-26-phase4b-batch-operations-design.md` — 设计文档
- **Create:** `docs/superpowers/plans/2026-04-26-phase4b-batch-operations.md` — 实现计划（本文件）
- **Modify:** `tests/unit/clipboardStore.test.ts` — 补齐批量操作单元测试
- **Verify only:** `src/stores/clipboardStore.ts` — 确认被测方法签名
- **Verify only:** `src/components/ClipboardList.vue` — 确认 UI 交互逻辑
- **Verify only:** `src/components/EntryCard.vue` — 确认 checkbox 交互
- **Verify only:** `src/components/BatchTagDialog.vue` — 确认批量标签弹窗

## Task 1: 产出设计文档和实现计划

- [x] **Step 1:** 创建 `docs/superpowers/specs/2026-04-26-phase4b-batch-operations-design.md`
- [x] **Step 2:** 创建 `docs/superpowers/plans/2026-04-26-phase4b-batch-operations.md`

## Task 2: 补齐 clipboardStore 多选模式生命周期测试

**Files:** Modify `tests/unit/clipboardStore.test.ts`

- [ ] **Step 1:** 测试 `enterMultiSelectMode()` 设置 isMultiSelectMode 为 true
- [ ] **Step 2:** 测试 `enterMultiSelectMode(id)` 将初始条目加入选中集合并设置锚点
- [ ] **Step 3:** 测试 `exitMultiSelectMode()` 清除所有选中、重置模式标志和锚点
- [ ] **Step 4:** 测试 `clearSelection()` 只清除选中 ID 不改变模式标志

## Task 3: 补齐 clipboardStore 选择操作测试

**Files:** Modify `tests/unit/clipboardStore.test.ts`

- [ ] **Step 1:** 测试 `toggleEntrySelection(id)` 切换选中状态
- [ ] **Step 2:** 测试 `toggleEntrySelection(id, true/false)` 强制选中/取消
- [ ] **Step 3:** 测试选中最后一个条目后取消 → 自动退出多选模式
- [ ] **Step 4:** 测试 `selectAllLoadedEntries()` 选中所有已加载条目
- [ ] **Step 5:** 测试 `invertLoadedSelection()` 反转选择
- [ ] **Step 6:** 测试 `selectRangeTo(id)` 基于锚点范围选择

## Task 4: 补齐 clipboardStore 批量动作测试

**Files:** Modify `tests/unit/clipboardStore.test.ts`

- [ ] **Step 1:** 测试 `deleteSelectedEntries()` 调用后端并更新本地状态、退出多选模式
- [ ] **Step 2:** 测试 `deleteSelectedEntries()` 失败时打印错误不崩溃
- [ ] **Step 3:** 测试 `copySelectedEntries()` 调用后端命令
- [ ] **Step 4:** 测试 `copySelectedEntries()` 无选中时不调用
- [ ] **Step 5:** 测试 `favoriteSelectedEntries(true/false)` 批量设置/取消收藏
- [ ] **Step 6:** 测试 `favoriteSelectedEntries(false)` 在收藏分类下过滤条目
- [ ] **Step 7:** 测试 `applyTagsToSelectedEntries()` 调用后端并刷新标签缓存
- [ ] **Step 8:** 测试 `handleEntryPrimaryAction()` 在多选模式下切换选中而非粘贴

## Task 5: 补齐 reconcileSelection 测试

**Files:** Modify `tests/unit/clipboardStore.test.ts`

- [ ] **Step 1:** 测试 `reconcileSelection` 移除不存在的选中 ID
- [ ] **Step 2:** 测试 `reconcileSelection` 选中数归零时自动退出多选模式
- [ ] **Step 3:** 测试 `reconcileSelection` 更新 activeEntryId

## Task 6: 验证与回归检查

- [ ] **Step 1:** 运行 `pnpm test:web` 确认所有测试通过
- [ ] **Step 2:** 运行 `pnpm lint:web` 确认无 lint 错误
- [ ] **Step 3:** 运行 `pnpm typecheck` 确认无类型错误
