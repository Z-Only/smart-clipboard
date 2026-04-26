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
/*  Stub heavy child components to keep tests focused on SyncPanel     */
/* ------------------------------------------------------------------ */

vi.mock('@/components/WebDavPanel.vue', () => ({
  default: { template: '<div data-testid="webdav-stub">WebDavPanel</div>', props: ['isActive'] },
}));
vi.mock('@/components/ConflictLogPanel.vue', () => ({
  default: { template: '<div data-testid="conflict-log-stub">ConflictLogPanel</div>' },
}));

/* ------------------------------------------------------------------ */
/*  Imports (must come after vi.mock)                                  */
/* ------------------------------------------------------------------ */

import SyncPanel from '@/components/SyncPanel.vue';
import { useSyncStore } from '@/stores/syncStore';
import { useConflictStore } from '@/stores/conflictStore';

/* ------------------------------------------------------------------ */
/*  i18n messages (only keys actually used in assertions)              */
/* ------------------------------------------------------------------ */

const i18n = createI18n({
  legacy: false,
  locale: 'en',
  messages: {
    en: {
      sync: {
        title: 'Sync',
        subtitle: 'Manage device synchronization',
        tab: { lan: 'LAN', webdav: 'WebDAV' },
        panel: {
          enabled: 'Enable Sync',
          deviceName: 'Device Name',
          port: 'Port',
          status: 'Status',
          pairedDevices: 'Paired Devices',
          discoveredDevices: 'Discovered Devices',
          activeDevices: 'Active Devices',
        },
        hints: { enabled: 'Enable to discover and sync with nearby devices' },
        placeholders: { deviceName: 'My Device' },
        actions: {
          refresh: 'Refresh',
          refreshing: 'Refreshing...',
          save: 'Save',
          saving: 'Saving...',
          close: 'Close',
          pair: 'Pair',
          unpair: 'Unpair',
        },
        statusValues: {
          online: 'Online',
          offline: 'Offline',
          connected: 'Connected',
          unknown: 'Unknown',
          connecting: 'Connecting',
          pairing: 'Pairing',
          disabled: 'Disabled',
          error: 'Error',
          discovering: 'Discovering',
        },
        empty: { paired: 'No paired devices yet', discovered: 'No devices discovered' },
        loading: 'Scanning...',
        device: { syncOn: 'Sync on', syncOff: 'Sync off', availableToPair: 'Available' },
      },
      conflict: {
        tab: 'Conflicts',
        pendingCount: '{count} pending conflict(s)',
        noPending: 'No pending conflicts',
        noPendingHint: 'All sync conflicts have been resolved',
        resolve: { title: 'Resolve' },
        strategy: {
          title: 'Resolution Strategy',
          hint: 'Choose how conflicts are resolved',
          'last-write-wins': 'Last Write',
          'last-write-wins-hint': 'Most recent wins',
          'local-first': 'Local First',
          'local-first-hint': 'Keep local',
          'remote-first': 'Remote First',
          'remote-first-hint': 'Keep remote',
          manual: 'Manual',
          'manual-hint': 'Decide each',
        },
        keepLog: 'Keep Log',
        keepLogHint: 'Store resolved conflict history',
        maxLogEntries: 'Max Log Entries',
      },
    },
  },
});

/* ------------------------------------------------------------------ */
/*  Test data                                                          */
/* ------------------------------------------------------------------ */

const pairedDevice = {
  id: 'dev-1',
  name: 'Office PC',
  deviceName: 'Office PC',
  address: '192.168.1.20',
  ip: '192.168.1.20',
  port: 8484,
  status: 'online',
  fingerprint: 'fp-123',
  syncEnabled: true,
  enabled: true,
  lastSeenAt: null,
  pairedAt: null,
};

const discoveredDevice = {
  id: 'dev-2',
  name: 'Laptop',
  deviceName: 'Laptop',
  address: '192.168.1.30',
  ip: '192.168.1.30',
  port: 8484,
  status: 'offline',
  fingerprint: 'fp-456',
  syncEnabled: false,
  enabled: false,
  lastSeenAt: null,
  pairedAt: null,
};

/** Standard invoke mock that satisfies syncStore.refreshAll() */
function setupDefaultInvoke() {
  invoke.mockImplementation(async (cmd: string) => {
    if (cmd === 'get_sync_status') {
      return {
        enabled: true,
        deviceName: 'MacBook',
        port: 9000,
        status: 'online',
        pairedDevices: [pairedDevice],
        discoveredDevices: [discoveredDevice],
      };
    }
    if (cmd === 'get_sync_config') {
      return { enabled: true, deviceName: 'MacBook', port: 9000 };
    }
    if (cmd === 'get_paired_devices') return [pairedDevice];
    if (cmd === 'get_discovered_devices') return [discoveredDevice];
    if (cmd === 'update_sync_config') return undefined;
    if (cmd === 'pair_device') return undefined;
    if (cmd === 'unpair_device') return undefined;
    if (cmd === 'toggle_device_sync') return undefined;
    return null;
  });
}

function mountPanel(isOpen = true) {
  return mount(SyncPanel, {
    props: { isOpen },
    global: { plugins: [createPinia(), i18n] },
  });
}

/* ------------------------------------------------------------------ */
/*  Test suite                                                         */
/* ------------------------------------------------------------------ */

describe('SyncPanel', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    invoke.mockReset();
    setupDefaultInvoke();
  });

  // ─── Basic rendering ───────────────────────────────────────────

  it('does not render when isOpen is false', () => {
    const wrapper = mountPanel(false);
    expect(wrapper.find('h2').exists()).toBe(false);
  });

  it('renders panel title when open', async () => {
    const wrapper = mountPanel();
    await flushPromises();
    expect(wrapper.text()).toContain('Sync');
    expect(wrapper.text()).toContain('Manage device synchronization');
  });

  // ─── Watch isOpen triggers refreshAll ──────────────────────────

  it('calls refreshAll when panel opens', async () => {
    const wrapper = mountPanel(false);
    await flushPromises();
    invoke.mockClear();
    await wrapper.setProps({ isOpen: true });
    await flushPromises();
    expect(invoke).toHaveBeenCalledWith('get_sync_status');
  });

  // ─── LAN tab (default) ────────────────────────────────────────

  it('shows LAN configuration by default', async () => {
    const wrapper = mountPanel();
    await flushPromises();
    expect(wrapper.text()).toContain('Enable Sync');
    expect(wrapper.text()).toContain('Device Name');
    expect(wrapper.text()).toContain('Port');
  });

  it('displays status text from store', async () => {
    const wrapper = mountPanel(false);
    await wrapper.setProps({ isOpen: true });
    await flushPromises();
    expect(wrapper.text()).toContain('Online');
  });

  it('displays paired and discovered devices', async () => {
    const wrapper = mountPanel(false);
    await wrapper.setProps({ isOpen: true });
    await flushPromises();
    expect(wrapper.text()).toContain('Office PC');
    expect(wrapper.text()).toContain('Laptop');
  });

  it('shows empty messages when no devices', async () => {
    invoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'get_sync_status') {
        return {
          enabled: false,
          deviceName: 'Test',
          port: 8484,
          status: 'offline',
          pairedDevices: [],
          discoveredDevices: [],
        };
      }
      if (cmd === 'get_sync_config') return { enabled: false, deviceName: 'Test', port: 8484 };
      if (cmd === 'get_paired_devices') return [];
      if (cmd === 'get_discovered_devices') return [];
      return null;
    });
    const wrapper = mountPanel(false);
    await wrapper.setProps({ isOpen: true });
    await flushPromises();
    expect(wrapper.text()).toContain('No paired devices yet');
    expect(wrapper.text()).toContain('No devices discovered');
  });

  it('displays error message from store', async () => {
    invoke.mockRejectedValue(new Error('Network error'));
    const wrapper = mountPanel(false);
    await wrapper.setProps({ isOpen: true });
    await flushPromises();
    expect(wrapper.text()).toContain('Network error');
  });

  it('shows saving state on save button', async () => {
    const wrapper = mountPanel(false);
    await wrapper.setProps({ isOpen: true });
    await flushPromises();

    const syncStore = useSyncStore();
    syncStore.isSaving = true;
    await flushPromises();
    expect(wrapper.text()).toContain('Saving...');
  });

  it('shows refreshing state on refresh button', async () => {
    const wrapper = mountPanel(false);
    await wrapper.setProps({ isOpen: true });
    await flushPromises();

    const syncStore = useSyncStore();
    syncStore.isLoading = true;
    await flushPromises();
    expect(wrapper.text()).toContain('Refreshing...');
  });

  // ─── Save config ──────────────────────────────────────────────

  it('calls saveConfig on save button click', async () => {
    const wrapper = mountPanel(false);
    await wrapper.setProps({ isOpen: true });
    await flushPromises();

    invoke.mockClear();
    setupDefaultInvoke();

    const saveBtn = wrapper.findAll('button').find((b) => b.text().includes('Save'));
    await saveBtn!.trigger('click');
    await flushPromises();
    expect(invoke).toHaveBeenCalledWith('update_sync_config', expect.anything());
  });

  // ─── Refresh ──────────────────────────────────────────────────

  it('calls refreshAll on refresh button click', async () => {
    const wrapper = mountPanel(false);
    await wrapper.setProps({ isOpen: true });
    await flushPromises();

    invoke.mockClear();
    setupDefaultInvoke();

    const refreshBtn = wrapper.findAll('button').find((b) => b.text().includes('Refresh'));
    await refreshBtn!.trigger('click');
    await flushPromises();
    expect(invoke).toHaveBeenCalledWith('get_sync_status');
  });

  // ─── Close ────────────────────────────────────────────────────

  it('emits close event when close button is clicked', async () => {
    const wrapper = mountPanel();
    await flushPromises();
    const closeBtn = wrapper.findAll('button').find((b) => b.text().includes('Close'));
    await closeBtn!.trigger('click');
    expect(wrapper.emitted('close')).toBeTruthy();
  });

  // ─── Tab switching ────────────────────────────────────────────

  it('switches to WebDAV tab', async () => {
    const wrapper = mountPanel();
    await flushPromises();
    const webdavTab = wrapper.findAll('button').find((b) => b.text().includes('WebDAV'));
    await webdavTab!.trigger('click');
    expect(wrapper.text()).toContain('WebDavPanel');
  });

  it('switches to Conflicts tab and shows no-pending message', async () => {
    const wrapper = mountPanel();
    await flushPromises();
    const conflictsTab = wrapper.findAll('button').find((b) => b.text().includes('Conflicts'));
    await conflictsTab!.trigger('click');
    await flushPromises();
    expect(wrapper.text()).toContain('No pending conflicts');
  });

  it('shows conflict badge and pending banner when conflicts exist', async () => {
    const wrapper = mountPanel();
    await flushPromises();

    const conflictStore = useConflictStore();
    // Add a fake pending conflict
    conflictStore.detectConflicts(
      [
        {
          id: 'entry-1',
          content: 'local',
          hash: 'hash-local',
          category: 'text',
          created_at: '2024-01-01T00:00:00Z',
          updated_at: '2024-01-01T01:00:00Z',
          is_favorite: false,
          use_count: 0,
          source: 'clipboard',
          tags: [],
          is_sensitive: false,
        },
      ],
      [
        {
          id: 'entry-1',
          content: 'remote',
          hash: 'hash-remote',
          category: 'text',
          created_at: '2024-01-01T00:00:00Z',
          updated_at: '2024-01-01T02:00:00Z',
          is_favorite: false,
          use_count: 0,
          source: 'clipboard',
          tags: [],
          is_sensitive: false,
        },
      ],
      'remote-dev',
      'Remote Device',
    );
    await flushPromises();

    // Badge should show "1"
    expect(wrapper.text()).toContain('1');

    // Switch to conflicts tab
    const conflictsTab = wrapper.findAll('button').find((b) => b.text().includes('Conflicts'));
    await conflictsTab!.trigger('click');
    await flushPromises();
    expect(wrapper.text()).toContain('pending conflict(s)');
  });

  it('displays strategy buttons on conflicts tab', async () => {
    const wrapper = mountPanel();
    await flushPromises();
    const conflictsTab = wrapper.findAll('button').find((b) => b.text().includes('Conflicts'));
    await conflictsTab!.trigger('click');
    await flushPromises();
    expect(wrapper.text()).toContain('Resolution Strategy');
    expect(wrapper.text()).toContain('Last Write');
    expect(wrapper.text()).toContain('Local First');
    expect(wrapper.text()).toContain('Remote First');
    expect(wrapper.text()).toContain('Manual');
  });

  it('renders ConflictLogPanel on conflicts tab', async () => {
    const wrapper = mountPanel();
    await flushPromises();
    const conflictsTab = wrapper.findAll('button').find((b) => b.text().includes('Conflicts'));
    await conflictsTab!.trigger('click');
    await flushPromises();
    expect(wrapper.text()).toContain('ConflictLogPanel');
  });
});
