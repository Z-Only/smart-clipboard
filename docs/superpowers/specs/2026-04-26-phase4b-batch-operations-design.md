# Phase 4B — 批量操作 设计文档

- **作者**：Aone Copilot × 蝉雨
- **日期**：2026-04-26
- **版本**：v1.0
- **状态**：Approved

## 1. 背景

Phase 4B "List Interaction Enhancements" 是 Phase 4 路线图中的第二个实施包，目标是让用户能够高效地选择和操作多条剪贴板条目，同时保持列表在大数据量下的流畅性。

根据 `2026-04-21-phase4-package-breakdown.md` 的规划，Phase 4B 包含两个路线图项：

- **Feature 17: Virtual Scrolling** — 已在前序虚拟滚动性能优化中完成
- **Feature 18: Batch Operations** — 多选模式、批量删除、合并复制、批量收藏、批量打标签

### 1.1 当前实现状态

经过代码审查，批量操作的核心功能已在前序开发中逐步实现：

| 能力                         | 状态      | 所在模块                                                                    |
| ---------------------------- | --------- | --------------------------------------------------------------------------- |
| 多选模式切换                 | ✅ 已实现 | `clipboardStore.ts` + `ClipboardList.vue`                                   |
| 条目选择/取消（含 checkbox） | ✅ 已实现 | `EntryCard.vue` + `clipboardStore.ts`                                       |
| 选中计数显示                 | ✅ 已实现 | `ClipboardList.vue` 工具栏                                                  |
| 批量删除                     | ✅ 已实现 | `clipboardStore.deleteSelectedEntries` + `delete_entries`                   |
| 合并复制                     | ✅ 已实现 | `clipboardStore.copySelectedEntries` + `copy_entries`                       |
| 批量收藏/取消收藏            | ✅ 已实现 | `clipboardStore.favoriteSelectedEntries` + `set_favorite_state_for_entries` |
| 批量打标签（追加/替换）      | ✅ 已实现 | `BatchTagDialog.vue` + `set_tags_for_entries`                               |
| 全选/反选/清除选择           | ✅ 已实现 | `clipboardStore` 方法集                                                     |
| Shift+Click 范围选择         | ✅ 已实现 | `selectRangeTo` + `EntryCard` shiftKey 事件                                 |
| Ctrl/Cmd+A 全选快捷键        | ✅ 已实现 | `ClipboardList.vue` keydown handler                                         |
| Escape 退出多选              | ✅ 已实现 | `ClipboardList.vue` keydown handler                                         |
| Arrow+Shift 范围选择         | ✅ 已实现 | `ClipboardList.vue` keydown handler                                         |
| 虚拟滚动集成                 | ✅ 已实现 | 自建虚拟滚动 + shallowRef 优化                                              |
| 后端 Rust 命令               | ✅ 已实现 | `commands/clipboard.rs` + `commands/tags.rs`                                |
| i18n 中英文翻译              | ✅ 已实现 | `en.ts` + `zh-CN.ts`                                                        |
| **单元测试覆盖**             | ❌ 缺失   | `clipboardStore.test.ts` 批量操作测试未覆盖                                 |

### 1.2 需要补齐的工作

本次 superpowers 会话的工作重点是：

1. 产出设计文档和实现计划（本文档 + plan 文件）
2. 补齐 clipboardStore 中批量操作方法的单元测试
3. 验证全量测试通过和 lint/typecheck 无错误
4. 产出完成报告和 Next Package Handoff

## 2. 功能设计

### 2.1 多选状态模型

```
┌─────────────────────────────────────────────┐
│              clipboardStore                  │
│                                             │
│  isMultiSelectMode: boolean                 │
│  selectedEntryIds: number[]                 │
│  selectionAnchorId: number | null           │
│                                             │
│  computed:                                  │
│    selectedEntryIdSet: Set<number>           │
│    selectedCount: number                    │
│    selectedEntries: ClipboardEntry[]         │
│    canBatchCopy: boolean                    │
│                                             │
│  methods:                                   │
│    enterMultiSelectMode(initialId?)          │
│    exitMultiSelectMode()                    │
│    toggleEntrySelection(id, force?)          │
│    selectRangeTo(id)                        │
│    selectAllLoadedEntries()                 │
│    invertLoadedSelection()                  │
│    clearSelection()                         │
│    handleEntryPrimaryAction(id, options?)    │
│    reconcileSelection()                     │
│                                             │
│  batch actions:                             │
│    deleteSelectedEntries()                  │
│    copySelectedEntries()                    │
│    favoriteSelectedEntries(favorite)         │
│    applyTagsToSelectedEntries(tagIds, mode)  │
└─────────────────────────────────────────────┘
```

### 2.2 状态转换规则

- **进入多选模式**: 点击"多选"按钮 → `enterMultiSelectMode(activeEntryId)`
- **退出多选模式**: 点击"完成"按钮 或 Escape → `exitMultiSelectMode()` → 清除所有选中
- **自动退出**: 选中数量归零时自动退出多选模式 (`toggleEntrySelection` 内含判断)
- **选区锚点**: 首次选中的条目成为锚点，Shift+Click 基于锚点计算范围
- **数据变更后调和**: `reconcileSelection()` 在 entries 变更后移除不存在的选中 ID

### 2.3 键盘交互

| 按键         | 多选模式行为                            | 普通模式行为       |
| ------------ | --------------------------------------- | ------------------ |
| ArrowDown/Up | 移动活跃条目 + Shift 时扩展选区         | 移动活跃条目       |
| Enter        | 切换当前条目选中状态 + Shift 时范围选择 | 粘贴当前条目       |
| Ctrl/Cmd+A   | 全选所有已加载条目                      | 全选所有已加载条目 |
| Escape       | 退出多选模式                            | 无                 |

### 2.4 UI 布局

```
┌─────────────────────────────────────────────┐
│  [x selected]  [Tag] [★] [☆] [Copy] [Del]  │ ← 多选工具栏
│  [SelectAll] [Invert] [Clear] [Done]        │
├─────────────────────────────────────────────┤
│  [Virtualized list enabled]    [Multi-sel]  │ ← 状态栏
├─────────────────────────────────────────────┤
│  ─── Today ───                              │ ← sticky 日期分组
│  [☑] 📋 text  "Hello world"        2m ago  │ ← 虚拟化条目 + checkbox
│  [☐] 🔗 url   "https://..."        5m ago  │
│  ─── Yesterday ───                          │
│  [☑] 💻 code  "const x = 1"        1d ago  │
│  ...                                        │
└─────────────────────────────────────────────┘
```

## 3. 后端命令

所有批量操作所需的 Rust 后端命令已实现：

| 命令                             | 文件                    | 功能                                           |
| -------------------------------- | ----------------------- | ---------------------------------------------- |
| `delete_entries`                 | `commands/clipboard.rs` | 批量删除条目（含图片文件清理）                 |
| `copy_entries`                   | `commands/clipboard.rs` | 合并复制（过滤图片，换行拼接，写入系统剪贴板） |
| `set_favorite_state_for_entries` | `commands/clipboard.rs` | 批量设置收藏状态                               |
| `set_tags_for_entries`           | `commands/tags.rs`      | 批量设置标签（支持 append/replace 模式）       |

## 4. 不改动清单

- **不改** 后端 Rust 命令签名或行为
- **不改** 前端组件逻辑和 UI
- **不改** i18n 翻译内容
- **不改** 虚拟滚动核心算法
- **不改** BatchTagDialog 组件
- **不改** EntryCard checkbox 交互

## 5. 测试策略

### 5.1 需要补齐的 clipboardStore 测试用例

#### 多选模式生命周期

- `enterMultiSelectMode` 设置 isMultiSelectMode 为 true
- `enterMultiSelectMode(id)` 将初始条目加入选中集合
- `exitMultiSelectMode` 清除所有选中并重置模式标志
- `exitMultiSelectMode` 重置 selectionAnchorId

#### 选择操作

- `toggleEntrySelection(id)` 切换选中状态
- `toggleEntrySelection(id, true)` 强制选中
- `toggleEntrySelection(id, false)` 强制取消选中
- 选中最后一个条目后取消选中 → 自动退出多选模式
- `selectAllLoadedEntries` 选中所有已加载条目
- `invertLoadedSelection` 反转选择
- `clearSelection` 清除所有选中
- `selectRangeTo(id)` 基于锚点范围选择

#### 批量动作

- `deleteSelectedEntries` 调用后端并更新本地状态
- `deleteSelectedEntries` 完成后退出多选模式
- `deleteSelectedEntries` 失败时打印错误但不崩溃
- `copySelectedEntries` 调用后端 `copy_entries` 命令
- `copySelectedEntries` 无选中时不调用
- `favoriteSelectedEntries(true)` 批量设置收藏
- `favoriteSelectedEntries(false)` 批量取消收藏
- `favoriteSelectedEntries(false)` 在收藏分类下过滤已取消收藏的条目
- `applyTagsToSelectedEntries` 调用后端并刷新标签缓存
- `handleEntryPrimaryAction` 在多选模式下切换选中而非粘贴
- `handleEntryPrimaryAction` 在多选模式下 + range 选项调用 selectRangeTo

#### 选区调和

- `reconcileSelection` 移除不存在的选中 ID
- `reconcileSelection` 选中数归零时自动退出多选模式
- `reconcileSelection` 更新 activeEntryId 如果原值不存在

### 5.2 已有测试

- `BatchTagDialog.test.ts` — BatchTagDialog 组件测试
- `clipboardStore.test.ts` — 虚拟滚动高度管理、标签缓存、基本 CRUD

## 6. 验收标准

- clipboardStore 批量操作方法的单元测试全部通过
- `pnpm test:web` 全量测试通过
- `pnpm lint:web` 无 lint 错误
- `pnpm typecheck` 无类型错误
- 设计文档和实现计划已产出
- 完成报告包含 Next Package Handoff

## 7. Next Package Handoff 目标

Phase 4B 完成后，Phase 4C (Access Security) 可以开始。Phase 4B 的批量操作和虚拟滚动为 Phase 4C 提供了稳定的列表交互基础，Phase 4C 需要在此基础上添加锁屏拦截逻辑。
