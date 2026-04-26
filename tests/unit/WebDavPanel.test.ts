import { describe, it, expect, vi, beforeEach } from 'vitest';
import { mount, flushPromises } from '@vue/test-utils';
import { createI18n } from 'vue-i18n';
import { createPinia, setActivePinia } from 'pinia';

/* ------------------------------------------------------------------ */
/*  Mock invoke — used by real Pinia stores under the hood             */
/* ------------------------------------------------------------------ */

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke }));
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn().mockResolvedValue(() => {}) }));

/* ------------------------------------------------------------------ */
/*  Imports (must come after vi.mock)                                  */
/* ------------------------------------------------------------------ */

import WebDavPanel from '@/components/WebDavPanel.vue';
import { useWebDavStore } from '@/stores/webdavStore';

/* ------------------------------------------------------------------ */
/*  i18n messages                                                      */
/* ------------------------------------------------------------------ */

const i18n = createI18n({
  legacy: false,
  locale: 'en',
  messages: {
    en: {
      webdav: {
        title: 'WebDAV Sync',
        description: 'Sync your clipboard via a WebDAV server',
        serverUrl: 'Server URL',
        serverUrlPlaceholder: 'https://dav.example.com/remote.php/dav',
        username: 'Username',
        password: 'Password',
        syncPassword: 'Sync Password',
        syncPasswordHint: 'Used to encrypt clipboard data on the server',
        connecting: 'Connecting...',
        connect: 'Connect',
        status: 'Status',
        cloudEntries: 'Cloud Entries',
        lastSync: 'Last Sync',
        rateLimit: 'Rate Limit',
        connected: 'Connected',
        disconnected: 'Disconnected',
        error: 'Error',
        triggerSync: 'Sync Now',
        disconnect: 'Disconnect',
        registeredDevices: 'Registered Devices',
        noDevices: 'No registered devices',
        removeDevice: 'Remove',
        removeDeviceConfirm: 'Remove device {name}?',
        settings: 'Settings',
        syncImages: 'Sync Images',
        syncSensitive: 'Sync Sensitive',
        pollInterval: 'Poll Interval',
        pollIntervalUnit: 'seconds',
        maxCloudEntries: 'Max Cloud Entries',
        advanced: 'Advanced',
        remotePath: 'Remote Path',
        rateLimitCapacity: 'Rate Limit Capacity',
        rateLimitRefillMinutes: 'Refill Minutes',
      },
      settings: {
        save: 'Save',
      },
    },
  },
});

/* ------------------------------------------------------------------ */
/*  Test data                                                          */
/* ------------------------------------------------------------------ */

const defaultConfig = {
  enabled: false,
  serverUrl: '',
  username: '',
  password: '',
  syncPassword: '',
  pollIntervalSecs: 30,
  syncImages: false,
  syncSensitive: false,
  rateLimitCapacity: 150,
  rateLimitRefillMinutes: 30,
  remotePath: '/SmartClipboard',
  maxCloudEntries: 2000,
};

const connectedConfig = {
  ...defaultConfig,
  enabled: true,
  serverUrl: 'https://dav.example.com',
  username: 'user',
  password: 'pass',
  syncPassword: 'secret',
};

const connectedStatus = {
  status: 'connected',
  lastSyncAt: '2024-06-15T10:30:00Z',
  cloudEntryCount: 42,
  registeredDevices: [
    {
      deviceId: 'dev-1',
      deviceName: 'MacBook',
      lastSyncAt: '2024-06-15T10:30:00Z',
      registeredAt: '2024-06-01T00:00:00Z',
    },
  ],
  rateLimitAvailable: 120,
  rateLimitCapacity: 150,
  error: null,
};

const disconnectedStatus = {
  status: 'disconnected',
  lastSyncAt: null,
  cloudEntryCount: 0,
  registeredDevices: [],
  rateLimitAvailable: 0,
  rateLimitCapacity: 0,
  error: null,
};

/* ------------------------------------------------------------------ */
/*  Helpers                                                            */
/* ------------------------------------------------------------------ */

function setupDisconnectedInvoke() {
  invoke.mockImplementation(async (cmd: string) => {
    if (cmd === 'webdav_get_config') return defaultConfig;
    if (cmd === 'webdav_get_status') return disconnectedStatus;
    if (cmd === 'webdav_update_config') return undefined;
    if (cmd === 'webdav_connect') return undefined;
    return null;
  });
}

function setupConnectedInvoke() {
  invoke.mockImplementation(async (cmd: string) => {
    if (cmd === 'webdav_get_config') return connectedConfig;
    if (cmd === 'webdav_get_status') return connectedStatus;
    if (cmd === 'webdav_update_config') return undefined;
    if (cmd === 'webdav_disconnect') return undefined;
    if (cmd === 'webdav_trigger_sync') return 5;
    if (cmd === 'webdav_remove_device') return undefined;
    return null;
  });
}

function mountPanel(isActive = false) {
  return mount(WebDavPanel, {
    props: { isActive },
    global: { plugins: [createPinia(), i18n] },
  });
}

/** Mount the panel then activate it (triggers the watch → refreshAll) */
async function mountAndActivate() {
  const wrapper = mountPanel(false);
  await flushPromises();
  await wrapper.setProps({ isActive: true });
  await flushPromises();
  return wrapper;
}

/* ------------------------------------------------------------------ */
/*  Test suite                                                         */
/* ------------------------------------------------------------------ */

describe('WebDavPanel', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    invoke.mockReset();
  });

  // ─── Disconnected state: connection form ──────────────────────

  describe('disconnected state', () => {
    beforeEach(() => {
      setupDisconnectedInvoke();
    });

    it('renders connection form when disconnected', async () => {
      const wrapper = await mountAndActivate();
      expect(wrapper.text()).toContain('WebDAV Sync');
      expect(wrapper.text()).toContain('Server URL');
      expect(wrapper.text()).toContain('Username');
      expect(wrapper.text()).toContain('Password');
      expect(wrapper.text()).toContain('Sync Password');
    });

    it('shows connect button', async () => {
      const wrapper = await mountAndActivate();
      const connectBtn = wrapper.findAll('button').find((b) => b.text().includes('Connect'));
      expect(connectBtn).toBeTruthy();
    });

    it('disables connect button when form is empty', async () => {
      const wrapper = await mountAndActivate();
      const connectBtn = wrapper.findAll('button').find((b) => b.text().includes('Connect'));
      expect(connectBtn!.attributes('disabled')).toBeDefined();
    });

    it('enables connect button when all fields are filled', async () => {
      const wrapper = await mountAndActivate();

      const inputs = wrapper.findAll('input');
      // serverUrl, username, password, syncPassword
      await inputs[0].setValue('https://dav.example.com');
      await inputs[1].setValue('user');
      await inputs[2].setValue('pass');
      await inputs[3].setValue('secret');
      await flushPromises();

      const connectBtn = wrapper.findAll('button').find((b) => b.text().includes('Connect'));
      expect(connectBtn!.attributes('disabled')).toBeUndefined();
    });

    it('calls webdav_connect when connect button clicked', async () => {
      const wrapper = await mountAndActivate();

      const inputs = wrapper.findAll('input');
      await inputs[0].setValue('https://dav.example.com');
      await inputs[1].setValue('user');
      await inputs[2].setValue('pass');
      await inputs[3].setValue('secret');
      await flushPromises();

      invoke.mockClear();
      setupDisconnectedInvoke();

      const connectBtn = wrapper.findAll('button').find((b) => b.text().includes('Connect'));
      await connectBtn!.trigger('click');
      await flushPromises();

      expect(invoke).toHaveBeenCalledWith('webdav_connect', {
        serverUrl: 'https://dav.example.com',
        username: 'user',
        password: 'pass',
        syncPassword: 'secret',
      });
    });

    it('shows sync password hint text', async () => {
      const wrapper = await mountAndActivate();
      expect(wrapper.text()).toContain('Used to encrypt clipboard data on the server');
    });
  });

  // ─── Connected state ──────────────────────────────────────────

  describe('connected state', () => {
    beforeEach(() => {
      setupConnectedInvoke();
    });

    it('shows status cards when connected', async () => {
      const wrapper = await mountAndActivate();
      expect(wrapper.text()).toContain('Connected');
      expect(wrapper.text()).toContain('42');
      expect(wrapper.text()).toContain('120');
      expect(wrapper.text()).toContain('150');
    });

    it('shows trigger sync and disconnect buttons', async () => {
      const wrapper = await mountAndActivate();
      expect(wrapper.text()).toContain('Sync Now');
      expect(wrapper.text()).toContain('Disconnect');
    });

    it('calls webdav_trigger_sync on sync button click', async () => {
      const wrapper = await mountAndActivate();
      invoke.mockClear();
      setupConnectedInvoke();

      const syncBtn = wrapper.findAll('button').find((b) => b.text().includes('Sync Now'));
      await syncBtn!.trigger('click');
      await flushPromises();

      expect(invoke).toHaveBeenCalledWith('webdav_trigger_sync');
    });

    it('calls webdav_disconnect on disconnect button click', async () => {
      const wrapper = await mountAndActivate();
      invoke.mockClear();
      setupConnectedInvoke();

      const disconnectBtn = wrapper.findAll('button').find((b) => b.text().includes('Disconnect'));
      await disconnectBtn!.trigger('click');
      await flushPromises();

      expect(invoke).toHaveBeenCalledWith('webdav_disconnect');
    });

    // ─── Registered devices ───────────────────────────────────

    it('shows registered devices list', async () => {
      const wrapper = await mountAndActivate();
      expect(wrapper.text()).toContain('Registered Devices');
      expect(wrapper.text()).toContain('MacBook');
    });

    it('shows no devices message when list is empty', async () => {
      invoke.mockImplementation(async (cmd: string) => {
        if (cmd === 'webdav_get_config') return connectedConfig;
        if (cmd === 'webdav_get_status') {
          return { ...connectedStatus, registeredDevices: [] };
        }
        return null;
      });

      const wrapper = await mountAndActivate();
      expect(wrapper.text()).toContain('No registered devices');
    });

    it('shows remove button for each device', async () => {
      const wrapper = await mountAndActivate();
      const removeBtn = wrapper.findAll('button').find((b) => b.text().includes('Remove'));
      expect(removeBtn).toBeTruthy();
    });

    // ─── Settings section ─────────────────────────────────────

    it('renders settings section', async () => {
      const wrapper = await mountAndActivate();
      expect(wrapper.text()).toContain('Settings');
    });

    it('renders settings fields when expanded', async () => {
      const wrapper = await mountAndActivate();
      // Open the <details> settings section
      const detailsSummary = wrapper.findAll('summary').find((s) => s.text().includes('Settings'));
      if (detailsSummary) {
        const details = detailsSummary.element.parentElement as HTMLDetailsElement;
        details.open = true;
        await flushPromises();
      }
      expect(wrapper.text()).toContain('Sync Images');
      expect(wrapper.text()).toContain('Sync Sensitive');
      expect(wrapper.text()).toContain('Poll Interval');
      expect(wrapper.text()).toContain('Max Cloud Entries');
    });
  });

  // ─── Error handling ───────────────────────────────────────────

  describe('error handling', () => {
    it('displays error message from store', async () => {
      invoke.mockImplementation(async (cmd: string) => {
        if (cmd === 'webdav_get_config') return connectedConfig;
        if (cmd === 'webdav_get_status') {
          return { ...connectedStatus, error: 'Connection timed out' };
        }
        return null;
      });

      const wrapper = await mountAndActivate();
      expect(wrapper.text()).toContain('Connection timed out');
    });

    it('shows connecting state on connect button', async () => {
      setupDisconnectedInvoke();
      const wrapper = await mountAndActivate();

      const store = useWebDavStore();
      store.isConnecting = true;
      await flushPromises();

      expect(wrapper.text()).toContain('Connecting...');
    });
  });

  // ─── Watch isActive triggers refresh ──────────────────────────

  describe('activation lifecycle', () => {
    it('calls refreshAll when panel becomes active', async () => {
      setupDisconnectedInvoke();
      const wrapper = mountPanel(false);
      await flushPromises();

      invoke.mockClear();
      setupDisconnectedInvoke();

      await wrapper.setProps({ isActive: true });
      await flushPromises();

      expect(invoke).toHaveBeenCalledWith('webdav_get_config');
      expect(invoke).toHaveBeenCalledWith('webdav_get_status');
    });

    it('does not call refreshAll when panel becomes inactive', async () => {
      setupDisconnectedInvoke();
      const wrapper = mountPanel(true);
      await flushPromises();

      invoke.mockClear();
      await wrapper.setProps({ isActive: false });
      await flushPromises();

      // Only the initial activation should have called invoke, not the deactivation
      expect(invoke).not.toHaveBeenCalledWith('webdav_get_config');
    });
  });
});
