---
layout: home

hero:
  name: Smart Clipboard
  text: 智能剪贴板管理器
  tagline: 轻量、安全、跨平台 —— 自动记录每一次复制，让粘贴更聪明。
  image:
    src: /images/branding/logo-mark.svg
    alt: Smart Clipboard
  actions:
    - theme: brand
      text: 快速开始 →
      link: /guide/getting-started
    - theme: alt
      text: 浏览功能
      link: /guide/features
    - theme: alt
      text: 查看 GitHub
      link: https://github.com/Z-Only/smart-clipboard

features:
  - icon: 📋
    title: 剪贴板历史
    details: 自动捕获每一次复制，去重保留、全文搜索、分类过滤，再也不怕覆盖丢失。
  - icon: 🧠
    title: 智能分类
    details: URL / 邮箱 / 代码 / JSON / 文件路径 / 颜色 / 电话自动识别，一秒定位目标内容。
  - icon: 🔐
    title: 安全优先
    details: Argon2 密码哈希 + OS 凭据存储、原生 Touch ID / Windows Hello 生物识别、应用锁、自动锁定、敏感数据检测与过期清除。
  - icon: 🔄
    title: 多设备同步
    details: 局域网端到端加密同步 + WebDAV 云同步，跨设备工作流无缝衔接。
  - icon: 📝
    title: 模板引擎
    details: 占位符模板一次性填写，多段文本复用，告别重复输入。
  - icon: ⚡
    title: 轻量极速
    details: Tauri 2 + Rust 后端 + 原生 WebView，包体小、启动快、资源占用极低。
---

<div class="screenshot-grid" style="margin-top:3rem">
  <figure>
    <img src="/images/screenshots/app-overview.png" alt="主界面总览" loading="lazy" />
    <figcaption>主界面 —— 历史记录一览无余</figcaption>
  </figure>
  <figure>
    <img src="/images/screenshots/search-and-filters.png" alt="搜索与分类" loading="lazy" />
    <figcaption>搜索与分类过滤</figcaption>
  </figure>
</div>

## 为什么选择 Smart Clipboard？

| 特性       | Smart Clipboard | 系统剪贴板 | 传统剪贴板工具 |
| ---------- | :-------------: | :--------: | :------------: |
| 历史记录   |     ✅ 无限     | ❌ 仅 1 条 |    ⚠️ 有限     |
| 智能分类   |     ✅ 自动     |     ❌     |       ❌       |
| 全文搜索   |     ✅ FTS5     |     ❌     |       ⚠️       |
| 安全锁定   |    ✅ Argon2    |     ❌     |       ❌       |
| 跨设备同步 | ✅ LAN + WebDAV |     ❌     |       ⚠️       |
| 模板引擎   |       ✅        |     ❌     |       ❌       |
| 开源免费   |     ✅ MIT      |     ✅     |       ⚠️       |

<p style="text-align:center;margin-top:2rem">
  <a href="./guide/getting-started" class="VPButton brand">立即开始使用 →</a>
</p>
