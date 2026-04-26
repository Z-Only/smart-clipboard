# SettingsPanel 组件拆分 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 将 771 行的 SettingsPanel.vue 拆分为 1 个容器组件 + 3 个功能子组件，降低单文件复杂度，同时保持功能行为和已有测试完全不变。

**Architecture:** 按功能域拆分：插件管理、更新器、安全+加密分别抽取为独立子组件。父组件保持 form 状态所有权，子组件通过 v-model 或 store 直接访问实现数据流。已有测试 mount SettingsPanel 后仍能看到子组件渲染内容，无需修改测试。

**Tech Stack:** Vue 3, TypeScript, Pinia

---

## File Map

- **Create:** `src/components/SettingsPluginSection.vue` — 插件列表与启用/禁用
- **Create:** `src/components/SettingsUpdaterSection.vue` — 更新器配置与操作
- **Create:** `src/components/SettingsSecuritySection.vue` — 应用锁 + 密码 + 数据库加密
- **Modify:** `src/components/SettingsPanel.vue` — 移除已抽取的模板和逻辑，引入子组件

## Task 1: 抽取 SettingsPluginSection.vue

**Files:** Create `src/components/SettingsPluginSection.vue`

- [ ] **Step 1:** 创建组件，包含插件列表模板和 pluginStore 引用
- [ ] **Step 2:** 运行 `pnpm run typecheck` 确认无类型错误

## Task 2: 抽取 SettingsUpdaterSection.vue

**Files:** Create `src/components/SettingsUpdaterSection.vue`

- [ ] **Step 1:** 创建组件，接收 `modelValue: UpdaterConfig` prop，包含更新器模板和逻辑
- [ ] **Step 2:** 运行 `pnpm run typecheck` 确认无类型错误

## Task 3: 抽取 SettingsSecuritySection.vue

**Files:** Create `src/components/SettingsSecuritySection.vue`

- [ ] **Step 1:** 创建组件，接收 `modelValue: AppLockConfig` prop，包含安全锁+密码+加密模板和逻辑
- [ ] **Step 2:** 运行 `pnpm run typecheck` 确认无类型错误

## Task 4: 改造 SettingsPanel.vue 引入子组件

**Files:** Modify `src/components/SettingsPanel.vue`

- [ ] **Step 1:** 移除已抽取到子组件的模板区域，替换为子组件标签
- [ ] **Step 2:** 移除已不需要的 import 和函数（updater/security/plugin 相关）
- [ ] **Step 3:** 运行 `pnpm run typecheck` 确认无类型错误

## Task 5: 全量验证

- [ ] **Step 1:** 运行 `pnpm run test:web` 确认所有测试通过
- [ ] **Step 2:** 运行 `pnpm run lint:web` 确认无 lint 错误
- [ ] **Step 3:** 确认 SettingsPanel.vue 行数 ≤ 350
