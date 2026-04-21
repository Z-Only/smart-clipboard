import { defineConfig } from 'vitepress';

const repo = 'https://github.com/Z-Only/smart-clipboard';
const isGithubActions = process.env.GITHUB_ACTIONS === 'true';
const repoName = process.env.GITHUB_REPOSITORY?.split('/')[1] ?? 'smart-clipboard';
const base = isGithubActions ? `/${repoName}/` : '/';
const siteUrl = 'https://z-only.github.io';
const ogImage = `${siteUrl}/${repoName}/images/branding/logo-mark.svg`;

export default defineConfig({
  lang: 'zh-CN',
  title: 'Smart Clipboard',
  description: '跨平台、轻量、注重安全与同步体验的智能剪贴板管理器',
  base,
  titleTemplate: ':title · Smart Clipboard',
  cleanUrls: true,
  lastUpdated: true,
  ignoreDeadLinks: true,

  head: [
    ['meta', { name: 'theme-color', content: '#111827' }],
    ['meta', { property: 'og:type', content: 'website' }],
    ['meta', { property: 'og:title', content: 'Smart Clipboard' }],
    [
      'meta',
      {
        property: 'og:description',
        content: '跨平台、轻量、注重安全与同步体验的智能剪贴板管理器。',
      },
    ],
    ['meta', { property: 'og:image', content: ogImage }],
    ['link', { rel: 'icon', type: 'image/svg+xml', href: '/images/branding/logo-mark.svg' }],
  ],

  themeConfig: {
    logo: '/images/branding/logo-mark.svg',
    siteTitle: 'Smart Clipboard',

    nav: [
      { text: '首页', link: '/' },
      { text: '快速开始', link: '/guide/getting-started' },
      { text: '功能特性', link: '/guide/features' },
      { text: '截图预览', link: '/guide/screenshots' },
      {
        text: '更多',
        items: [
          { text: '部署说明', link: '/reference/deployment' },
          { text: 'GitHub', link: repo },
        ],
      },
    ],

    socialLinks: [{ icon: 'github', link: repo }],

    search: {
      provider: 'local',
      options: {
        locales: {
          root: {
            translations: {
              button: { buttonText: '搜索', buttonAriaLabel: '搜索' },
              modal: {
                noResultsText: '无法找到相关结果',
                resetButtonTitle: '清除查询条件',
                footer: { selectText: '选择', navigateText: '切换', closeText: '关闭' },
              },
            },
          },
        },
      },
    },

    footer: {
      message: 'Based on VitePress · Hosted on GitHub Pages',
      copyright: 'Copyright © 2026 Smart Clipboard contributors',
    },

    editLink: {
      pattern: `${repo}/edit/main/site/:path`,
      text: '在 GitHub 上编辑此页',
    },

    outline: {
      level: [2, 3],
      label: '页面导航',
    },

    lastUpdatedText: '最近更新',
    returnToTopLabel: '返回顶部',

    docFooter: {
      prev: '上一页',
      next: '下一页',
    },

    sidebar: {
      '/guide/': [
        {
          text: '指南',
          items: [
            { text: '快速开始', link: '/guide/getting-started' },
            { text: '功能总览', link: '/guide/features' },
            { text: '截图预览', link: '/guide/screenshots' },
          ],
        },
      ],
      '/reference/': [
        {
          text: '参考',
          items: [{ text: '部署说明', link: '/reference/deployment' }],
        },
      ],
    },

    carbonAds: undefined,
  },

  sitemap: {
    hostname: `${siteUrl}/${repoName}/`,
  },
});
