# 核心模块测试覆盖补充 设计文档

- **作者**：Aone Copilot × 蝉雨
- **日期**：2026-04-26
- **版本**：v1.0
- **状态**：Approved

## 1. 背景

Smart Clipboard 项目已发展至 v2.7.0，功能涵盖剪贴板历史管理、智能分类、多设备同步、安全锁定、插件系统等完整能力。然而，前端测试覆盖率存在严重不足：

- **整体覆盖率**：语句 35.94%，分支 17.6%，函数 28.48%
- **核心 store 覆盖率**：`clipboardStore.ts`（16.24%）、`conflictStore.ts`（17.94%）、`webdavStore.ts`（25.88%）
- **覆盖率阈值**：当前设置为语句 6%、分支 5%、函数 7%，形同虚设
- **组件测试**：19 个组件中 16 个完全无测试

最新的性能优化（shallowRef + Map + triggerRef）在 `clipboardStore.ts` 中引入了新的响应式模式，但缺乏对应的测试保护，任何后续改动都有回归风险。

## 2. 目标

- **为 `clipboardStore.ts` 补充测试**：覆盖虚拟滚动高度管理、标签缓存、数据获取、多选操作、收藏/删除等核心场景，目标覆盖率 ≥60%
- **为 `conflictStore.ts` 补充测试**：覆盖冲突检测、自动解决策略、手动解决、日志管理等，目标覆盖率 ≥60%
- **为 `webdavStore.ts` 补充测试**：覆盖连接管理、同步触发、错误处理、状态清理等，目标覆盖率 ≥60%
- **提升覆盖率阈值**：从 5-7% 提升至 ≥25%，建立有效的质量门禁

**非目标**：

- 不修改任何业务逻辑代码
- 不添加组件级测试（本次聚焦 store 层）
- 不添加 E2E 测试

## 3. 测试策略

### 3.1 Mock 模式

遵循项目现有测试模式：

- 使用 `vi.mock('@tauri-apps/api/core', () => ({ invoke }))` mock Tauri IPC
- 使用 `vi.spyOn(console, 'error').mockImplementation(() => {})` 捕获错误日志
- 每个测试前 `setActivePinia(createPinia())` 初始化 store
- 使用动态 `import()` 确保 store 在 mock 之后加载

### 3.2 测试分组

每个 store 测试按功能域分组：

- **clipboardStore**：虚拟滚动 / 标签缓存 / 数据获取 / 多选操作 / 收藏删除 / 分组逻辑 / 状态清理
- **conflictStore**：冲突检测 / 自动解决 / 手动解决 / 日志管理 / 配置管理
- **webdavStore**：加载配置 / 保存配置 / 连接管理 / 同步触发 / 错误处理 / 状态清理

## 4. 风险与回退

- `clipboardStore` 依赖 `i18n`，测试中需 mock `@/i18n` 模块
- `conflictStore` 依赖 `localStorage`，jsdom 环境已自带支持
- 如果测试运行时间过长，可考虑减少冗余场景

## 5. 验收标准

- `pnpm run test:web` 全部通过
- `pnpm run typecheck` 无类型错误
- 三个 store 的覆盖率均 ≥60%
- 覆盖率阈值提升且 CI 通过
