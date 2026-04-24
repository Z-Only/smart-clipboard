import { beforeEach, describe, expect, it, vi } from 'vitest';
import { mount, flushPromises } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import i18n from '@/i18n';

const { invoke, updaterStoreMock } = vi.hoisted(() => ({
  invoke: vi.fn(),
  updaterStoreMock: {
    status: {
      phase: 'ready_to_install',
      currentVersion: '2.1.0',
      availableVersion: '2.2.0',
      availableNotes: 'notes',
      availableReleaseDate: '2026-04-23T10:30:00Z',
      pendingUpdate: {
        version: '2.2.0',
        releaseDate: null,
        currentVersion: '2.1.0',
        notes: 'notes',
        artifactPath: '/tmp/app',
        signaturePath: '/tmp/app.sig',
        canonicalAssetUrl: 'https://github.com/x',
        sourceAssetUrl: 'https://mirror/x',
        downloadedAt: '2026-04-23T10:35:00Z',
      },
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
    refresh: vi.fn().mockResolvedValue(undefined),
    updateSettings: vi.fn().mockResolvedValue(undefined),
    setPassword: vi.fn().mockResolvedValue(undefined),
    lock: vi.fn().mockResolvedValue(undefined),
  }),
}));
vi.mock('@/stores/updaterStore', () => ({
  useUpdaterStore: () => updaterStoreMock,
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

describe('SettingsPanel updater section', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    invoke.mockReset();
    updaterStoreMock.checkNow.mockClear();
    updaterStoreMock.discardPending.mockClear();
    updaterStoreMock.installPending.mockClear();
    invoke.mockImplementation((cmd: string) => {
      if (cmd === 'get_config') return Promise.resolve(baseConfig);
      if (cmd === 'get_autostart_enabled') return Promise.resolve(false);
      if (cmd === 'update_config') return Promise.resolve();
      return Promise.resolve();
    });
  });

  it('renders pending update actions', async () => {
    const wrapper = mount(SettingsPanel, {
      props: { isOpen: true },
      global: { plugins: [createPinia(), i18n] },
    });

    await flushPromises();

    expect(wrapper.text()).toContain('Install and restart');
    expect(wrapper.text()).toContain('Cancel and delete installer');
  });

  it('shows quit action while install handoff is in progress', async () => {
    updaterStoreMock.status = {
      phase: 'installing',
      currentVersion: '2.1.0',
      availableVersion: '2.2.0',
      availableNotes: 'Release notes',
      availableReleaseDate: '2026-04-23T10:30:00Z',
      pendingUpdate: null,
      downloadProgress: null,
      lastError: null,
      lastCheckSilent: false,
    };

    const wrapper = mount(SettingsPanel, {
      props: { isOpen: true },
      global: { plugins: [createPinia(), i18n] },
    });
    await flushPromises();

    expect(wrapper.text()).toContain('Quit app now');
  });

  it('shows installing guidance after installer handoff starts', async () => {
    updaterStoreMock.status = {
      phase: 'installing',
      currentVersion: '2.1.0',
      availableVersion: '2.2.0',
      availableNotes: 'Release notes',
      availableReleaseDate: '2026-04-23T10:30:00Z',
      pendingUpdate: null,
      downloadProgress: null,
      lastError: null,
      lastCheckSilent: false,
    };

    const wrapper = mount(SettingsPanel, {
      props: { isOpen: true },
      global: { plugins: [createPinia(), i18n] },
    });
    await flushPromises();

    expect(wrapper.text()).toContain('Installer handoff started');
    expect(wrapper.text()).toContain('You may need to quit the app to finish installation');
  });

  it('shows download progress while installer is downloading', async () => {
    updaterStoreMock.status = {
      phase: 'downloading',
      currentVersion: '2.1.0',
      availableVersion: '2.2.0',
      availableNotes: 'Release notes',
      availableReleaseDate: '2026-04-23T10:30:00Z',
      pendingUpdate: null,
      downloadProgress: 0.42,
      lastError: null,
      lastCheckSilent: false,
    };

    const wrapper = mount(SettingsPanel, {
      props: { isOpen: true },
      global: { plugins: [createPinia(), i18n] },
    });
    await flushPromises();

    expect(wrapper.text()).toContain('Downloading installer...');
    expect(wrapper.text()).toContain('42%');
  });

  it('shows download action when update is available without pending installer', async () => {
    updaterStoreMock.status = {
      phase: 'update_available',
      currentVersion: '2.1.0',
      availableVersion: '2.2.0',
      availableNotes: 'Release notes',
      availableReleaseDate: '2026-04-23T10:30:00Z',
      pendingUpdate: null,
      downloadProgress: null,
      lastError: null,
      lastCheckSilent: false,
    };

    const wrapper = mount(SettingsPanel, {
      props: { isOpen: true },
      global: { plugins: [createPinia(), i18n] },
    });
    await flushPromises();

    expect(wrapper.text()).toContain('Download installer');
  });

  it('renders available version when update is available without pending installer', async () => {
    updaterStoreMock.status = {
      phase: 'update_available',
      currentVersion: '2.1.0',
      availableVersion: '2.2.0',
      pendingUpdate: null,
      availableNotes: 'Release notes',
      availableReleaseDate: '2026-04-23T10:30:00Z',
      downloadProgress: null,
      lastError: null,
      lastCheckSilent: false,
    };

    const wrapper = mount(SettingsPanel, {
      props: { isOpen: true },
      global: { plugins: [createPinia(), i18n] },
    });
    await flushPromises();

    expect(wrapper.text()).toContain('Update available');
    expect(wrapper.text()).toContain('Current version 2.1.0');
  });

  it('blocks save when mirrors are invalid', async () => {
    const wrapper = mount(SettingsPanel, {
      props: { isOpen: true },
      global: { plugins: [createPinia(), i18n] },
    });
    await flushPromises();

    const textarea = wrapper.get('[data-test="updater-mirrors"]');
    await textarea.setValue('http://bad-mirror/{url}');
    await wrapper.get('[data-test="settings-save"]').trigger('click');

    expect(invoke).not.toHaveBeenCalledWith('update_config', expect.anything());
    expect(wrapper.text()).toContain('Mirror must start with https:// and include');
  });
});
