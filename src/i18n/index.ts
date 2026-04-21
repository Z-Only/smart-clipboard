import { createI18n } from 'vue-i18n';
import en from './locales/en';
import zhCN from './locales/zh-CN';

const LOCALE_KEY = 'smart-clipboard-locale';

function getDefaultLocale(): string {
  const saved = localStorage.getItem(LOCALE_KEY);
  if (saved) return saved;

  const nav = navigator.language;
  if (nav.startsWith('zh')) return 'zh-CN';
  return 'en';
}

export function setLocale(locale: string) {
  localStorage.setItem(LOCALE_KEY, locale);
  i18n.global.locale.value = locale as 'en' | 'zh-CN';
}

export function getLocale(): string {
  return i18n.global.locale.value;
}

const i18n = createI18n({
  legacy: false,
  locale: getDefaultLocale(),
  fallbackLocale: 'en',
  messages: {
    en,
    'zh-CN': zhCN,
  },
});

export default i18n;
