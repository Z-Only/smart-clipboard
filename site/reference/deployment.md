# 部署说明

Smart Clipboard 的官网/文档站通过 **GitHub Actions** 自动构建并部署到 **GitHub Pages**。

## 架构概览

```mermaid
flowchart LR
  A[推送到 main 分支] --> B[GitHub Actions 触发]
  B --> C[安装依赖 + 构建 VitePress]
  C --> D[上传 Pages 产物]
  D --> E[部署到 GitHub Pages]
```

## 自动部署流程

项目已配置 `.github/workflows/docs-pages.yml`，当推送到 `main` 分支时自动执行：

1. **Checkout** —— 拉取代码
2. **Setup Node.js 20** —— 配合 pnpm 缓存
3. **Setup pnpm** —— 版本 10.33.0
4. **Configure Pages** —— 初始化 GitHub Pages 环境
5. **Install** —— `pnpm install --frozen-lockfile`
6. **Build** —— `pnpm run docs:build` → 输出到 `site/.vitepress/dist`
7. **Upload artifact** —— 上传构建产物
8. **Deploy** —— 部署到 `https://<owner>.github.io/<repo>/`

当前仓库 `Z-Only/smart-clipboard` 的实际 Pages 地址预期为 `https://z-only.github.io/smart-clipboard/`。

## 手动触发

在 GitHub 仓库的 Actions 页面，选择 **Docs Pages** 工作流，点击 **Run workflow** 即可手动触发部署。

## 本地预览

```bash
# 开发模式
pnpm run docs:dev

# 构建并预览
pnpm run docs:build
pnpm run docs:preview
```

## GitHub Pages 配置

确保仓库设置中：

1. **Pages** → **Source** 选择 **GitHub Actions**
2. **Environment** → `github-pages` 已创建（工作流会自动处理）

## 自定义域名（可选）

如果你有自己的域名，可以：

1. 在 `site/public/` 下创建 `CNAME` 文件，内容为你的域名
2. 在 DNS 提供商处添加 CNAME 记录指向 `<owner>.github.io`
3. 更新 `site/.vitepress/config.ts` 中的 `sitemap.hostname`

```ts
sitemap: {
  hostname: 'https://your-domain.com/',
}
```

## VitePress 站点配置

关键配置位于 `site/.vitepress/config.ts`：

| 配置项                        | 说明                                               |
| ----------------------------- | -------------------------------------------------- |
| `base`                        | GitHub Actions 时自动设为 `/<仓库名>/`，本地为 `/` |
| `cleanUrls`                   | 启用干净 URL（无 `.html` 后缀）                    |
| `lastUpdated`                 | 页面显示最后更新时间                               |
| `sitemap`                     | 自动生成 sitemap.xml                               |
| `themeConfig.nav`             | 顶部导航栏                                         |
| `themeConfig.sidebar`         | 侧边栏菜单                                         |
| `themeConfig.search`          | 本地搜索                                           |
| `themeConfig.lastUpdatedText` | 页面最近更新时间文案                               |

## 故障排查

| 问题       | 解决方案                                                    |
| ---------- | ----------------------------------------------------------- |
| 页面 404   | 检查 `base` 配置是否匹配仓库名                              |
| 图片不显示 | 确认路径以 `/images/` 开头，文件在 `site/public/images/` 下 |
| 构建失败   | 检查 Node.js 版本 ≥ 18，pnpm 版本匹配                       |
| 样式异常   | 清除 VitePress 缓存：`rm -rf site/.vitepress/cache`         |
