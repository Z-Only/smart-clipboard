# 测试覆盖全面提升（Store + 组件） 设计文档

- **作者**：Aone Copilot × 蝉雨
- **日期**：2026-04-26
- **版本**：v1.0
- **状态**：Approved

## 1. 背景

Smart Clipboard v2.7.0 的前端测试覆盖率已从初始 ~6% 提升至 ~40%（语句），但仍存在两个明显短板：

- **Store 层残留低覆盖**：`templateStore.ts`（语句 46.77%，分支 16.66%）和 `securityStore.ts`（语句 52%，分支 100%）仍有大量未覆盖的方法
- **组件层几乎空白**：22 个业务组件中 16 个覆盖率为 0%，任何重构都缺乏测试保护

### 1.1 当前覆盖率明细

| 模块             | 语句    | 分支       | 函数       | 行         |
| ---------------- | ------- | ---------- | ---------- | ---------- |
| templateStore.ts | 46.77%  | 16.66%     | 46.66%     | 49.09%     |
| securityStore.ts | 52%     | 100%       | 54.54%     | 52%        |
| 16 个零覆盖组件  | 0%      | 0%         | 0%         | 0%         |
| **整体**         | **40%** | **29.28%** | **34.87%** | **40.59%** |

## 2. 目标

### 2.1 Store 测试补充

- `templateStore.ts` 覆盖率 ≥70%：补充 fetchTemplates、fetchCategories、updateTemplate、deleteTemplate、setCategory、filteredTemplates 等未测场景
- `securityStore.ts` 覆盖率 ≥70%：补充 updateSettings、enableEncryption、disableEncryption、refreshEncryption、refresh 错误处理等

### 2.2 组件基础测试

为 14 个零覆盖组件编写基础测试，每个组件至少覆盖：

- 初始渲染（mount 后关键元素存在）
- 核心 props/emits 行为
- 主要用户交互（点击/输入事件）

目标组件按大小排序：SearchBar(73行)、CategoryFilter(80行)、LockScreen(90行)、PairConfirmDialog(94行)、TemplateFillDialog(102行)、DeviceCard(109行)、BatchTagDialog(141行)、ConflictLogPanel(145行)、TagPicker(168行)、TemplateEditor(168行)、EntryCard(225行)、TemplateList(233行)、StatisticsPanel(244行)、ConflictResolveDialog(187行)

### 2.3 覆盖率阈值提升

从 25/15/25/25 提升至 35/20/30/35

## 3. 测试策略

### 3.1 Store 测试模式

遵循项目已有模式：

- `vi.mock('@tauri-apps/api/core', () => ({ invoke }))` mock Tauri IPC
- `setActivePinia(createPinia())` 初始化 store
- 动态 `import()` 确保 store 在 mock 之后加载
- `vi.spyOn(console, 'error')` 捕获错误日志

### 3.2 组件测试模式

遵循 App.test.ts 已有模式：

- `@vue/test-utils` 的 `mount` / `shallowMount`
- mock 所有子组件依赖为 stub
- mock Tauri IPC 和 event listener
- mock store 注入（createPinia + setActivePinia）
- 使用 `flushPromises()` / `nextTick()` 等待异步更新

### 3.3 组件测试范围

每个组件的基础测试聚焦于：

1. **渲染测试**：组件挂载后关键 DOM 元素存在
2. **Props 测试**：传入不同 props 后渲染正确
3. **Emits 测试**：用户操作触发正确的事件
4. **Store 交互**：验证组件读取/写入 store 的关键路径

## 4. 非目标

- 不修改任何业务逻辑代码
- 不添加 E2E 测试
- 不测试 ClipboardList（377行，已有虚拟滚动复杂逻辑，需独立计划）
- 不测试 SyncPanel（565行）和 WebDavPanel（456行）（过大，需独立计划）

## 5. 验收标准

- `pnpm run test:web` 全部通过
- `pnpm run typecheck` 无类型错误
- `templateStore.ts` 覆盖率 ≥70%
- `securityStore.ts` 覆盖率 ≥70%
- 14 个零覆盖组件均有 >0% 覆盖率
- 整体覆盖率阈值提升至 35/20/30/35 且 CI 通过
