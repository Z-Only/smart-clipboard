# 前端列表性能优化（虚拟滚动加固 + 渲染性能修复）Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 修复 Smart Clipboard 前端列表的 3 个高优先级性能问题：EntryCard 标签 IPC 风暴、ClipboardList ResizeObserver 内存泄漏、clipboardStore 虚拟滚动高度全量重渲染。

**Architecture:** 在现有自建虚拟滚动基础上做增量修复，不引入外部库。核心改动集中在 store 响应式策略（shallowRef+Map）、ResizeObserver 生命周期管理、标签批量预加载三个方面。

**Tech Stack:** Vue 3, Pinia, TypeScript, Tauri IPC

---

## File Map

- **Modify:** `src/stores/clipboardStore.ts`
  - 责任：measuredItemHeights 改为 Map+shallowRef+triggerRef；entryTagsMap 同理；新增 batchLoadEntryTags 方法
- **Modify:** `src/components/ClipboardList.vue`
  - 责任：ResizeObserver 生命周期管理；批量标签预加载 watch；清理不再可见的 observer
- **Modify:** `src/components/EntryCard.vue`
  - 责任：移除独立 loadEntryTags IPC 调用，改为从 store 缓存读取的计算属性
- **Verify only:** `src/types/index.ts`
  - 责任：确认类型定义无需变更

## Task 1: clipboardStore.ts 响应式优化

**Files:** Modify `src/stores/clipboardStore.ts`

- [ ] **Step 1:** 将 `measuredItemHeights` 从 `ref<Record<string, number>>` 改为 `shallowRef(new Map<string, number>())`
- [ ] **Step 2:** 更新 `getVirtualItemHeight` 使用 `Map.get()`
- [ ] **Step 3:** 更新 `setVirtualItemHeight` 使用 `Map.set()` + `triggerRef()`，移除对象展开
- [ ] **Step 4:** 更新 `clearVirtualItemHeights` 使用 `Map.clear()` + `triggerRef()`
- [ ] **Step 5:** 将 `entryTagsMap` 从 `ref<Record<number, Tag[]>>` 改为 `shallowRef(new Map<number, Tag[]>())`
- [ ] **Step 6:** 更新 `setEntryTags` / `clearEntryTagsCache` 使用 Map API + `triggerRef()`
- [ ] **Step 7:** 新增 `batchLoadEntryTags(entryIds: number[])` 方法，过滤已缓存 ID 后并发加载

## Task 2: ClipboardList.vue ResizeObserver 生命周期修复

**Files:** Modify `src/components/ClipboardList.vue`

- [ ] **Step 1:** 添加 watch 监听 visibleItems 的 key 集合变化
- [ ] **Step 2:** 对离开可视区的 key，disconnect 对应 ResizeObserver 并从 Map 中删除
- [ ] **Step 3:** 确保 onUnmounted 清理逻辑作为兜底保留

## Task 3: EntryCard.vue 标签加载改造

**Files:** Modify `src/components/EntryCard.vue`

- [ ] **Step 1:** 移除 `loadEntryTags()` 函数和 `watch(entry.id)` 调用
- [ ] **Step 2:** 将 `entryTags` 改为 computed，从 `store.entryTagsMap` 读取
- [ ] **Step 3:** 保留 `onTagsChanged` 回调，更新时写入 store

## Task 4: ClipboardList.vue 批量标签预加载

**Files:** Modify `src/components/ClipboardList.vue`

- [ ] **Step 1:** 添加 watch 监听 visibleItems 中 entry 类型的条目 ID 列表
- [ ] **Step 2:** 调用 `store.batchLoadEntryTags()` 批量预取未缓存标签

## Task 5: 验证与回归检查

- [ ] **Step 1:** 运行 `pnpm lint:web` 确认无 lint 错误
- [ ] **Step 2:** 运行 `pnpm typecheck` 确认无类型错误
- [ ] **Step 3:** 运行 `pnpm test:web` 确认无测试回归
