# SettingsPanel 组件拆分 设计文档

- **作者**：Aone Copilot × 蝉雨
- **日期**：2026-04-26
- **版本**：v1.0
- **状态**：Approved

## 1. 背景

`SettingsPanel.vue` 当前有 771 行，承载 6 个独立功能区域（通用设置、插件管理、更新器、安全锁、数据库加密、操作按钮），是项目中最大的组件。每个功能区域之间用 `<Separator />` 分隔，逻辑和模板互相独立，适合拆分。

主要问题：

- 文件过大（建议组件 < 300 行），难以维护和审查
- 功能域混合：一个组件同时处理插件、更新器、安全、加密等不相关的逻辑
- 测试困难：已有测试 mock 了 6 个模块来测 1 个组件

## 2. 拆分方案

将 SettingsPanel 拆分为 **1 个容器组件 + 4 个功能子组件**：

| 组件                                  | 职责                                             | 估计行数 |
| ------------------------------------- | ------------------------------------------------ | -------- |
| `SettingsPanel.vue`（改造）           | 对话框容器 + 通用设置 + 操作按钮 + form 状态管理 | ~300     |
| `SettingsPluginSection.vue`（新建）   | 插件列表渲染、启用/禁用                          | ~80      |
| `SettingsUpdaterSection.vue`（新建）  | 更新器配置与操作                                 | ~200     |
| `SettingsSecuritySection.vue`（新建） | 应用锁 + 密码管理 + 数据库加密                   | ~150     |

### 2.1 数据流设计

- `SettingsPanel.vue` 保持 `form` reactive 对象的唯一所有权
- 子组件通过 `v-model` 双向绑定各自负责的 form 子字段
- store 引用（security、updater、pluginStore）由各子组件自行引入，不经过父组件传递
- `save()` / `resetDefaults()` 仍在父组件中，子组件不直接触发保存

### 2.2 接口约定

```typescript
// SettingsPluginSection.vue
// 无 props，自行使用 pluginStore

// SettingsUpdaterSection.vue
defineProps<{ modelValue: UpdaterConfig }>();
defineEmits<{ 'update:modelValue': [value: UpdaterConfig] }>();

// SettingsSecuritySection.vue
defineProps<{ modelValue: AppLockConfig }>();
defineEmits<{ 'update:modelValue': [value: AppLockConfig] }>();
```

## 3. 非目标

- 不改变功能行为和 UI 外观
- 不修改 store 层代码
- 不引入新的状态管理模式
- 不修改 i18n key

## 4. 对已有测试的影响

- `SettingsPanel.plugins.test.ts`：测试的是 SettingsPanel mount 后的插件区域渲染。拆分后插件模板移到子组件，但因为 SettingsPanel 仍然 import 并渲染子组件，mount SettingsPanel 仍能看到插件内容。**现有测试应无需修改即可通过**。
- `SettingsPanel.updater.test.ts`：同理，updater 区域移到子组件后仍由父组件渲染。**现有测试应无需修改即可通过**。

## 5. 验收标准

- `SettingsPanel.vue` 行数 ≤ 350 行
- 所有现有测试通过：`pnpm run test:web`
- typecheck 通过：`pnpm run typecheck`
- lint 通过：`pnpm run lint:web`
- UI 功能行为与拆分前完全一致
