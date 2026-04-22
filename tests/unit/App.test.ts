import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { nextTick } from 'vue';
import { flushPromises, mount, type VueWrapper } from '@vue/test-utils';

const { focusSpy, eventHandlers, listen, invoke } = vi.hoisted(() => ({
  focusSpy: vi.fn(),
  eventHandlers: new Map<string, (event?: unknown) => unknown>(),
  listen: vi.fn(),
  invoke: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({ invoke }));
vi.mock('@tauri-apps/api/event', () => ({ listen }));
vi.mock('@/composables/useClipboard', () => ({ useClipboard: vi.fn() }));
vi.mock('@/composables/useTheme', () => ({ useTheme: vi.fn() }));

vi.mock('@/components/ui/separator', async () => {
  const { defineComponent, h } = await import('vue');
  return {
    Separator: defineComponent({
      name: 'SeparatorStub',
      setup() {
        return () => h('div', { 'data-test': 'separator' });
      },
    }),
  };
});

vi.mock('@/components/SearchBar.vue', async () => {
  const { defineComponent, h } = await import('vue');
  return {
    default: defineComponent({
      name: 'SearchBarStub',
      setup(_, { expose }) {
        expose({ focus: focusSpy });
        return () => h('div', { 'data-test': 'search-bar' });
      },
    }),
  };
});

vi.mock('@/components/CategoryFilter.vue', async () => {
  const { defineComponent, h } = await import('vue');
  return {
    default: defineComponent({
      name: 'CategoryFilterStub',
      setup() {
        return () => h('div', { 'data-test': 'category-filter' });
      },
    }),
  };
});

vi.mock('@/components/ClipboardList.vue', async () => {
  const { defineComponent, h } = await import('vue');
  return {
    default: defineComponent({
      name: 'ClipboardListStub',
      setup() {
        return () => h('div', { 'data-test': 'clipboard-list' });
      },
    }),
  };
});

vi.mock('@/components/LockScreen.vue', async () => {
  const { defineComponent, h } = await import('vue');
  return {
    default: defineComponent({
      name: 'LockScreenStub',
      setup() {
        return () => h('div', { 'data-test': 'lock-screen' });
      },
    }),
  };
});

vi.mock('@/components/SettingsPanel.vue', async () => {
  const { defineComponent, h } = await import('vue');
  return {
    default: defineComponent({
      name: 'SettingsPanelStub',
      props: { isOpen: { type: Boolean, default: false } },
      setup(props) {
        return () => h('div', { 'data-test': 'settings-panel', 'data-open': String(props.isOpen) });
      },
    }),
  };
});

vi.mock('@/components/StatisticsPanel.vue', async () => {
  const { defineComponent, h } = await import('vue');
  return {
    default: defineComponent({
      name: 'StatisticsPanelStub',
      props: { isOpen: { type: Boolean, default: false } },
      setup(props) {
        return () =>
          h('div', { 'data-test': 'statistics-panel', 'data-open': String(props.isOpen) });
      },
    }),
  };
});

vi.mock('@/components/TemplateList.vue', async () => {
  const { defineComponent, h } = await import('vue');
  return {
    default: defineComponent({
      name: 'TemplateListStub',
      props: { isOpen: { type: Boolean, default: false } },
      setup(props) {
        return () => h('div', { 'data-test': 'template-panel', 'data-open': String(props.isOpen) });
      },
    }),
  };
});

vi.mock('@/components/SyncPanel.vue', async () => {
  const { defineComponent, h } = await import('vue');
  return {
    default: defineComponent({
      name: 'SyncPanelStub',
      props: { isOpen: { type: Boolean, default: false } },
      setup(props) {
        return () => h('div', { 'data-test': 'sync-panel', 'data-open': String(props.isOpen) });
      },
    }),
  };
});

import App from '@/App.vue';
import { useClipboardStore } from '@/stores/clipboardStore';
import { useSecurityStore } from '@/stores/securityStore';
import { useSyncStore } from '@/stores/syncStore';
import { useTemplateStore } from '@/stores/templateStore';
import { useWebDavStore } from '@/stores/webdavStore';

function makeStatus(locked: boolean) {
  return {
    enabled: true,
    configured: true,
    locked,
    biometric_available: true,
    biometric_enabled: true,
    auto_lock_seconds: 30,
    unlock_reason: locked ? 'manual' : 'password',
    failed_attempts: 0,
  };
}

async function settle() {
  await flushPromises();
  await nextTick();
}

function panelIsOpen(wrapper: VueWrapper, testId: string) {
  return wrapper.get(`[data-test="${testId}"]`).attributes('data-open') === 'true';
}

async function emitAppEvent(name: string, payload?: unknown) {
  const handler = eventHandlers.get(name);
  if (!handler) throw new Error(`Missing event handler for ${name}`);
  await handler(payload);
  await settle();
}

async function createHarness(initiallyLocked: boolean) {
  const pinia = createPinia();
  setActivePinia(pinia);

  eventHandlers.clear();
  focusSpy.mockReset();
  listen.mockReset();
  invoke.mockReset();
  listen.mockImplementation(async (event: string, handler: (event?: unknown) => unknown) => {
    eventHandlers.set(event, handler);
    return () => {};
  });

  const security = useSecurityStore();
  security.$patch({ status: makeStatus(initiallyLocked), initialized: false, error: null });
  const clipboard = useClipboardStore();
  const sync = useSyncStore();
  const templates = useTemplateStore();
  const webdav = useWebDavStore();

  const initSpy = vi.spyOn(security, 'init').mockResolvedValue();
  const fetchEntriesSpy = vi.spyOn(clipboard, 'fetchEntries').mockResolvedValue();
  const fetchAllTagsSpy = vi.spyOn(clipboard, 'fetchAllTags').mockResolvedValue();
  const clearClipboardSpy = vi.spyOn(clipboard, 'clearSensitiveViewState');
  const refreshSyncSpy = vi.spyOn(sync, 'refreshAll').mockResolvedValue();
  const clearSyncSpy = vi.spyOn(sync, 'clearSensitiveState');
  const fetchTemplatesSpy = vi.spyOn(templates, 'fetchTemplates').mockResolvedValue();
  const fetchCategoriesSpy = vi.spyOn(templates, 'fetchCategories').mockResolvedValue();
  const clearTemplatesSpy = vi.spyOn(templates, 'clearSensitiveState');
  const refreshWebDavSpy = vi.spyOn(webdav, 'refreshAll').mockResolvedValue();
  const clearWebDavSpy = vi.spyOn(webdav, 'clearSensitiveState');

  const wrapper = mount(App, {
    global: {
      plugins: [pinia],
      mocks: {
        $t: (key: string) => key,
      },
    },
  });

  await settle();
  mountedWrappers.push(wrapper);

  return {
    wrapper,
    security,
    spies: {
      initSpy,
      fetchEntriesSpy,
      fetchAllTagsSpy,
      clearClipboardSpy,
      refreshSyncSpy,
      clearSyncSpy,
      fetchTemplatesSpy,
      fetchCategoriesSpy,
      clearTemplatesSpy,
      refreshWebDavSpy,
      clearWebDavSpy,
    },
  };
}

const mountedWrappers: VueWrapper[] = [];

describe('App', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    while (mountedWrappers.length > 0) {
      mountedWrappers.pop()?.unmount();
    }
  });

  it('clears sensitive state and closes panels when the app locks', async () => {
    const { wrapper, security, spies } = await createHarness(false);

    spies.fetchEntriesSpy.mockClear();
    await wrapper.get('[title="app.settings"]').trigger('click');
    await wrapper.get('[title="app.statistics"]').trigger('click');
    await wrapper.get('[title="templates.title"]').trigger('click');
    await wrapper.get('[title="sync.title"]').trigger('click');

    expect(panelIsOpen(wrapper, 'settings-panel')).toBe(true);
    expect(panelIsOpen(wrapper, 'statistics-panel')).toBe(true);
    expect(panelIsOpen(wrapper, 'template-panel')).toBe(true);
    expect(panelIsOpen(wrapper, 'sync-panel')).toBe(true);

    security.status.locked = true;
    await settle();

    expect(spies.clearClipboardSpy).toHaveBeenCalledTimes(1);
    expect(spies.clearSyncSpy).toHaveBeenCalledTimes(1);
    expect(spies.clearTemplatesSpy).toHaveBeenCalledTimes(1);
    expect(spies.clearWebDavSpy).toHaveBeenCalledTimes(1);
    expect(panelIsOpen(wrapper, 'settings-panel')).toBe(false);
    expect(panelIsOpen(wrapper, 'statistics-panel')).toBe(false);
    expect(panelIsOpen(wrapper, 'template-panel')).toBe(false);
    expect(panelIsOpen(wrapper, 'sync-panel')).toBe(false);
    expect(wrapper.find('[data-test="lock-screen"]').exists()).toBe(true);
  });

  it('reloads sensitive state and focuses search after unlocking', async () => {
    const { security, spies } = await createHarness(true);

    spies.fetchEntriesSpy.mockClear();
    spies.fetchAllTagsSpy.mockClear();
    spies.refreshSyncSpy.mockClear();
    spies.fetchTemplatesSpy.mockClear();
    spies.fetchCategoriesSpy.mockClear();
    spies.refreshWebDavSpy.mockClear();
    focusSpy.mockClear();

    security.status.locked = false;
    await settle();

    expect(spies.fetchEntriesSpy).toHaveBeenCalledWith(true);
    expect(spies.fetchAllTagsSpy).toHaveBeenCalledTimes(1);
    expect(spies.refreshSyncSpy).toHaveBeenCalledTimes(1);
    expect(spies.fetchTemplatesSpy).toHaveBeenCalledTimes(1);
    expect(spies.fetchCategoriesSpy).toHaveBeenCalledTimes(1);
    expect(spies.refreshWebDavSpy).toHaveBeenCalledTimes(1);
    expect(focusSpy).toHaveBeenCalledTimes(1);
  });

  it('refreshes and focuses on window-shown only when unlocked', async () => {
    const { security, spies } = await createHarness(false);

    spies.fetchEntriesSpy.mockClear();
    focusSpy.mockClear();

    await emitAppEvent('window-shown', {});

    expect(spies.fetchEntriesSpy).toHaveBeenCalledTimes(1);
    expect(spies.fetchEntriesSpy).toHaveBeenCalledWith();
    expect(focusSpy).toHaveBeenCalledTimes(1);

    security.status.locked = true;
    await settle();

    spies.fetchEntriesSpy.mockClear();
    focusSpy.mockClear();

    await emitAppEvent('window-shown', {});

    expect(spies.fetchEntriesSpy).not.toHaveBeenCalled();
    expect(focusSpy).not.toHaveBeenCalled();
  });

  it('opens settings when the tray settings event fires', async () => {
    const { wrapper } = await createHarness(false);

    expect(panelIsOpen(wrapper, 'settings-panel')).toBe(false);

    await emitAppEvent('open-settings', {});

    expect(panelIsOpen(wrapper, 'settings-panel')).toBe(true);
  });
});
