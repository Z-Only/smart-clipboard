# 截图预览

以下是 Smart Clipboard 各核心功能的界面截图，帮助你快速了解应用的全貌。

<div class="screenshot-grid">

<figure>
  <img src="/images/screenshots/app-overview.png" alt="主界面总览" loading="lazy" />
  <figcaption>主界面 —— 剪贴板历史记录一目了然</figcaption>
</figure>

<figure>
  <img src="/images/screenshots/search-and-filters.png" alt="搜索与分类" loading="lazy" />
  <figcaption>搜索与分类过滤 —— 快速定位目标内容</figcaption>
</figure>

<figure>
  <img src="/images/screenshots/template-workflow.png" alt="模板面板" loading="lazy" />
  <figcaption>模板面板 —— 从空状态开始创建可复用模板</figcaption>
</figure>

<figure>
  <img src="/images/screenshots/statistics-panel.png" alt="统计面板" loading="lazy" />
  <figcaption>使用统计 —— 分类分布与活动趋势</figcaption>
</figure>

<figure>
  <img src="/images/screenshots/settings-panel.png" alt="设置面板" loading="lazy" />
  <figcaption>设置面板 —— 外观、安全、同步、自启动</figcaption>
</figure>

<figure>
  <img src="/images/screenshots/sync-panel.png" alt="同步面板" loading="lazy" />
  <figcaption>同步面板 —— 局域网与 WebDAV 云同步</figcaption>
</figure>

</div>

## 素材管理

项目截图素材存放在两个位置：

| 目录                              | 用途                            |
| --------------------------------- | ------------------------------- |
| `docs-assets/screenshots/raw/`    | 原始 PNG 截图，保留本地采集版本 |
| `site/public/images/screenshots/` | 官网/文档站直接引用的发布素材   |

### 更新截图

1. 本地运行应用并打开目标界面
2. 使用系统截图工具截取（macOS: `Cmd+Shift+4`，Windows: `Win+Shift+S`）
3. 直接保存到 `docs-assets/screenshots/raw/`
4. 复制一份到 `site/public/images/screenshots/`
5. 在 Markdown 中使用 `![描述](/images/screenshots/<文件名>.png)` 引用

### 命名规范

使用英文短横线命名：

- `app-overview.png`
- `search-and-filters.png`
- `settings-panel.png`
- `template-workflow.png`
- `statistics-panel.png`
- `sync-panel.png`
