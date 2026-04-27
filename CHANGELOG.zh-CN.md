# 更新日志

本文件记录项目的所有重要变更。

格式基于 [Keep a Changelog](https://keepachangelog.com/)，版本号遵循 [语义化版本](https://semver.org/)。

## [2.8.0] - 2026-04-27

### 新增

- **快速粘贴面板**：全局快捷键（`Cmd/Ctrl+Shift+1`）唤出轻量覆盖面板，展示最近的剪贴板条目，支持数字键（1-9）即时粘贴、方向键导航、Esc 关闭和输入即搜索
- **快速粘贴后端命令**：新增 `get_recent_entries` Tauri 命令，在安全守卫下获取最近 N 条剪贴板记录
- **快速粘贴热键注册**：新增 `setup_quick_paste_hotkey` 函数（`hotkey.rs`），注册可配置的快速粘贴快捷键并向前端发送激活事件
- **快速粘贴配置**：在 `AppConfig` 中新增 `quick_paste_shortcut` 和 `quick_paste_entry_count` 字段，通过 serde 默认值保持向后兼容
- **QuickPasteOverlay 组件**：新增 Vue 组件，使用 Teleport 渲染，支持键盘导航、分类图标、相对时间戳和搜索切换
- **快速粘贴国际化**：快速粘贴覆盖面板标签和设置键的中英文翻译
- **快速粘贴单元测试**：QuickPasteOverlay 组件测试（键盘导航、数字键粘贴、关闭、搜索切换）和 store 测试（`fetchRecentEntries`）

### 变更

- **版本升级至 2.8.0**：将快速粘贴面板作为新的次版本发布
- **App.vue 集成**：新增快速粘贴事件监听、激活处理、粘贴后隐藏流程和搜索切换
- **clipboardStore**：新增 `recentEntries` 响应式状态和 `fetchRecentEntries` action
- **项目文档**：更新 README、中文 README、CHANGELOG、VitePress 文档和版本元数据
