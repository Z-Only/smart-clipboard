import { beforeEach, describe, expect, it, vi } from 'vitest';
import { mount, flushPromises } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import i18n from '@/i18n';
import type { PluginListItem } from '@/types';

const { invoke, pluginStoreMock, updaterStoreMock } = vi.hoisted(() => ({
  invoke: vi.fn(),
  pluginStoreMock: {
    plugins: [] as PluginListItem[],
    loadPlugins: vi.fn().mockResolvedValue(undefined),
    setPluginEnabled: vi.fn().mockResolvedValue(undefined),
  },
  updaterStoreMock: {
    status: {
      phase: 'idle',
      currentVersion: '2.1.0',
      availableVersion: null,
      availableNotes: null,
      availableReleaseDate: null,
      pendingUpdate: null,
      downloadProgress: null,
      lastError: null,
      lastCheckSilent: false,
    },
    isChecking: false,
    loadStatus: vi.fn().mockResolvedValue(undefined),
    checkNow: vi.fn().mockResolvedValue(undefined),
    installPending: vi.fn().mockResolvedValue(undefined),
    downloadAvailable: vi.fn().mockResolvedValue(undefined),
    discardPending: vi.fn().mockResolvedValue(undefined),
    bindEvents: vi.fn().mockResolvedValue(undefined),
  },
}));

vi.mock('@tauri-apps/api/core', () => ({ invoke }));
vi.mock('@/i18n', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/i18n')>();
  return { ...actual, setLocale: vi.fn() };
});
vi.mock('@/composables/useTheme', () => ({
  useTheme: () => ({
    appearance: 'system',
    themeColor: 'zinc',
    setAppearance: vi.fn(),
    setThemeColor: vi.fn(),
  }),
}));
vi.mock('@/stores/securityStore', () => ({
  useSecurityStore: () => ({
    status: { biometric_available: true },
    encryption: {
      enabled: false,
      key_exists: false,
      encrypted_count: 0,
      plaintext_count: 0,
      migrating: false,
    },
    loading: false,
    refresh: vi.fn().mockResolvedValue(undefined),
    updateSettings: vi.fn().mockResolvedValue(undefined),
    setPassword: vi.fn().mockResolvedValue(undefined),
    lock: vi.fn().mockResolvedValue(undefined),
    refreshEncryption: vi.fn().mockResolvedValue(undefined),
    enableEncryption: vi.fn().mockResolvedValue(undefined),
    disableEncryption: vi.fn().mockResolvedValue(undefined),
  }),
}));
vi.mock('@/stores/updaterStore', () => ({
  useUpdaterStore: () => updaterStoreMock,
}));
vi.mock('@/stores/pluginStore', () => ({
  usePluginStore: () => pluginStoreMock,
}));

import SettingsPanel from '@/components/SettingsPanel.vue';

const baseConfig = {
  max_entries: 5000,
  retention_days: 30,
  excluded_apps: [],
  monitor_interval_ms: 500,
  autostart_enabled: false,
  sensitive_expiry_minutes: 5,
  app_lock: { enabled: false, auto_lock_seconds: 0, biometric_enabled: false },
  updater: {
    auto_check_enabled: true,
    check_interval_hours: 24,
    auto_download_enabled: false,
    wifi_only: true,
    mirrors: [],
    last_check_at: null,
  },
};

describe('SettingsPanel plugins section', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    invoke.mockReset();
    pluginStoreMock.loadPlugins.mockClear();
    pluginStoreMock.setPluginEnabled.mockClear();
    pluginStoreMock.plugins = [
      {
        id: 'uppercase-tools',
        name: 'Uppercase Tools',
        version: '1.0.0',
        description: 'Transforms clipboard text',
        kind: 'transform',
        handler: 'main.js',
        capabilities: ['transform'],
        enabled: true,
        valid: true,
        error: null,
      },
      {
        id: 'broken-plugin',
        name: 'Broken Plugin',
        version: null,
        description: null,
        kind: null,
        handler: null,
        capabilities: [],
        enabled: false,
        valid: false,
        error: 'Manifest is invalid',
      },
    ];
    invoke.mockImplementation((cmd: string) => {
      if (cmd === 'get_config') return Promise.resolve(baseConfig);
      if (cmd === 'get_autostart_enabled') return Promise.resolve(false);
      if (cmd === 'update_config') return Promise.resolve();
      return Promise.resolve();
    });
  });

  it('loads and renders valid and invalid plugins with metadata', async () => {
    const wrapper = mount(SettingsPanel, {
      props: { isOpen: true },
      global: { plugins: [createPinia(), i18n] },
    });

    await flushPromises();

    expect(pluginStoreMock.loadPlugins).toHaveBeenCalledTimes(1);
    expect(wrapper.text()).toContain('Plugins');
    expect(wrapper.text()).toContain('Uppercase Tools');
    expect(wrapper.text()).toContain('Transforms clipboard text');
    expect(wrapper.text()).toContain('transform');
    expect(wrapper.text()).toContain('1.0.0');
    expect(wrapper.text()).toContain('Broken Plugin');
    expect(wrapper.text()).toContain('Invalid plugin');
    expect(wrapper.text()).toContain('Manifest is invalid');
  });

  it('shows enable toggle only for valid plugins and toggles next state', async () => {
    const wrapper = mount(SettingsPanel, {
      props: { isOpen: true },
      global: { plugins: [createPinia(), i18n] },
    });

    await flushPromises();

    expect(wrapper.find('[data-test="plugin-toggle-uppercase-tools"]').exists()).toBe(true);
    expect(wrapper.find('[data-test="plugin-toggle-broken-plugin"]').exists()).toBe(false);

    await wrapper.get('[data-test="plugin-toggle-uppercase-tools"]').trigger('click');

    expect(pluginStoreMock.setPluginEnabled).toHaveBeenCalledWith('uppercase-tools', false);
  });
});
