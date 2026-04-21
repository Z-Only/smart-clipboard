# 快速开始

## 关于 Smart Clipboard

Smart Clipboard 是一款基于 **Tauri 2 + Vue 3 + Rust** 构建的跨平台智能剪贴板管理器。它常驻后台，自动捕获并分类剪贴板内容，支持安全保护与多设备同步。

![主界面总览](/images/screenshots/app-overview.png)

## 前置条件

| 依赖                                            | 版本要求 | 安装方式                                                          |
| ----------------------------------------------- | -------- | ----------------------------------------------------------------- |
| [Rust](https://www.rust-lang.org/tools/install) | 1.77+    | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| [Node.js](https://nodejs.org/)                  | 18+      | [nvm](https://github.com/nvm-sh/nvm) 或官网安装                   |
| [pnpm](https://pnpm.io/)                        | 10+      | `npm i -g pnpm`                                                   |
| 平台依赖                                        | —        | 参考 [Tauri 2 预备](https://v2.tauri.app/start/prerequisites/)    |

## 安装与运行

```bash
# 克隆仓库
git clone https://github.com/Z-Only/smart-clipboard.git
cd smart-clipboard

# 安装依赖
pnpm install

# 开发模式运行
pnpm tauri dev
```

应用启动后会在系统托盘驻留。

## 首次使用

1. **启动应用** —— 应用默认以系统托盘图标运行
2. **触发剪贴板面板** —— 按 `Cmd+Shift+V`（macOS）或 `Ctrl+Shift+V`（Windows / Linux）
3. **开始复制** —— 在任意应用中复制文本或图片，Smart Clipboard 会自动记录
4. **搜索与过滤** —— 在面板顶部输入关键词，或按类别筛选
5. **粘贴** —— 点击条目即可粘贴到活跃窗口

## 设置亮点

![设置面板](/images/screenshots/settings-panel.png)

在设置面板中可以配置：

- **外观模式**：跟随系统 / 浅色 / 深色
- **主题色**：6 种内置配色
- **剪贴板**：最大条数、保留天数、监听间隔、排除应用
- **安全**：应用锁开关、自动锁定超时、生物识别解锁
- **同步**：局域网配对与 WebDAV 云同步
- **自启动**：登录时自动启动

## 构建生产版本

```bash
pnpm tauri build
```

构建产物位于 `src-tauri/target/release/bundle/`，支持 `.dmg`（macOS）、`.msi` / `.exe`（Windows）、`.deb` / `.AppImage`（Linux）。

## 下一步

- [功能总览](/guide/features) —— 了解所有核心能力
- [截图预览](/guide/screenshots) —— 浏览各功能界面
- [部署说明](/reference/deployment) —— 了解官网部署方式
