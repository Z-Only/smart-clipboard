# 更新日志

本文件记录项目的所有重要变更。

格式基于 [Keep a Changelog](https://keepachangelog.com/)，版本号遵循 [语义化版本](https://semver.org/)。

## [2.9.0] - 2026-04-27

### 新增

- **智能分组（智能聚类）**：基于聚合聚类算法自动将条目分组，并自动生成分组标签，可通过新增的"智能分组"侧边面板访问
- **标签建议**：基于内容相似度为条目推荐标签，以内联标签芯片形式显示在条目卡片上，支持一键接受或忽略
- **相关条目推荐**：在条目详情下方显示可折叠的"相关"区域，展示相似度最高的 5 条相关条目及相似度百分比
- **相似度引擎**：纯 Rust 实现的 N-gram 分词器、TF-IDF 索引、Jaccard 与余弦相似度计算，通过 `SimilarityScorer` trait 为未来向量/嵌入后端预留 Phase B 扩展点
- **搜索结果相关性重排序**：使用 TF-IDF 余弦相似度对 FTS5 候选结果进行重排序，在模糊查询时显著改善排序效果
- **智能搜索配置**：新增 4 个配置项 — `smart_search_enabled`、`cluster_similarity_threshold`、`tag_suggestion_min_confidence`、`max_related_entries`
- **智能搜索 IPC 命令**：新增 7 个 Tauri 命令，覆盖聚类、标签建议和相关条目功能，均带安全守卫
- **智能搜索国际化**：智能分组、标签建议和相关条目 UI 的中英文翻译

### 变更

- **版本升级至 2.9.0**：将智能搜索与知识组织作为新的次版本发布
- **App.vue 集成**：在工具栏新增"智能分组"按钮和 SmartGroupsPanel 侧边面板
- **项目文档**：更新 README、中文 README、CHANGELOG 和版本元数据

## [2.8.0] - 2026-04-27

### 新增

- **快速粘贴面板**：全局快捷键（`Cmd/Ctrl+Shift+1`）唤出轻量覆盖面板，展示最近的剪贴板条目，支持数字键（1-9）即时粘贴、方向键导航、Esc 关闭和输入即搜索
- **快速粘贴后端命令**：新增 `get_recent_entries` Tauri 命令，在安全守卫下获取最近 N 条剪贴板记录
- **快速粘贴热键注册**：新增 `setup_quick_paste_hotkey` 函数（`hotkey.rs`），注册可配置的快速粘贴快捷键并向前端发送激活事件
- **快速粘贴配置**：在 `AppConfig` 中新增 `quick_paste_shortcut` 和 `quick_paste_entry_count` 字段，通过 serde 默认值保持向后兼容
- **QuickPasteOverlay 组件**：新增 Vue 组件，使用 Teleport 渲染，支持键盘导航、分类图标、相对时间戳和搜索切换
- **快速粘贴国际化**：快速粘贴覆盖面板标签和设置键的中英文翻译
- **快速粘贴单元测试**：QuickPasteOverlay 组件测试（键盘导航、数字键粘贴、关闭、搜索切换）和 store 测试（`fetchRecentEntries`）
- **双语更新日志**：GitHub Releases 现在展示实际的 CHANGELOG 内容而非通用的 diff 链接，支持中文翻译并在更新界面根据语言设置展示对应内容

### 变更

- **版本升级至 2.8.0**：将快速粘贴面板作为新的次版本发布
- **App.vue 集成**：新增快速粘贴事件监听、激活处理、粘贴后隐藏流程和搜索切换
- **clipboardStore**：新增 `recentEntries` 响应式状态和 `fetchRecentEntries` action
- **项目文档**：更新 README、中文 README、CHANGELOG、VitePress 文档和版本元数据
