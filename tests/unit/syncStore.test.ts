import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';

const invoke = vi.fn();
const errorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
vi.mock('@tauri-apps/api/core', () => ({ invoke }));

describe('useSyncStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    invoke.mockReset();
    errorSpy.mockClear();
  });

  it('normalizes refreshAll payloads', async () => {
    invoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'get_sync_status') {
        return {
          enabled: true,
          deviceName: 'MacBook',
          port: 9000,
          status: 'connected',
          pairedDevices: [{ id: 'dev-1', name: 'Phone', status: 'online' }],
          discoveredDevices: [{ id: 'dev-2', name: 'iPad', status: 'mystery' }],
        };
      }
      if (cmd === 'get_sync_config') {
        return { enabled: true, deviceName: 'MacBook', port: 9000 };
      }
      if (cmd === 'get_paired_devices') {
        return [{ id: 'dev-1', name: 'Phone', status: 'connected' }];
      }
      if (cmd === 'get_discovered_devices') {
        return [{ id: 'dev-2', name: 'iPad', status: 'unknown-status' }];
      }
      return null;
    });

    const { useSyncStore } = await import('@/stores/syncStore');
    const store = useSyncStore();

    await store.refreshAll();

    expect(store.enabled).toBe(true);
    expect(store.deviceName).toBe('MacBook');
    expect(store.port).toBe(9000);
    expect(store.status).toBe('connected');
    expect(store.pairedDevices[0].id).toBe('dev-1');
    expect(store.discoveredDevices[0].status).toBe('unknown');
  });

  it('tracks save errors and resets saving state', async () => {
    invoke.mockRejectedValue(new Error('save failed'));

    const { useSyncStore } = await import('@/stores/syncStore');
    const store = useSyncStore();

    await expect(
      store.saveConfig({ enabled: true, deviceName: 'MacBook', port: 9000 }),
    ).rejects.toThrow('save failed');

    expect(store.isSaving).toBe(false);
    expect(store.error).toBe('save failed');
  });

  it('captures refreshAll errors without throwing', async () => {
    invoke.mockRejectedValue(new Error('refresh failed'));

    const { useSyncStore } = await import('@/stores/syncStore');
    const store = useSyncStore();

    await store.refreshAll();

    expect(store.isLoading).toBe(false);
    expect(store.error).toBe('refresh failed');
    expect(errorSpy).toHaveBeenCalled();
  });

  it('clears sensitive state and transient errors', async () => {
    const { useSyncStore } = await import('@/stores/syncStore');
    const store = useSyncStore();

    store.enabled = true;
    store.deviceName = 'MacBook';
    store.port = 9000;
    store.status = 'connected';
    store.pairedDevices = [
      {
        id: 'dev-1',
        name: 'Phone',
        deviceName: 'Phone',
        address: null,
        ip: null,
        port: null,
        status: 'online',
        syncEnabled: true,
        enabled: true,
        lastSeenAt: null,
        pairedAt: null,
        fingerprint: null,
      },
    ];
    store.discoveredDevices = [
      {
        id: 'dev-2',
        name: 'iPad',
        deviceName: 'iPad',
        address: null,
        ip: null,
        port: null,
        status: 'unknown',
        syncEnabled: true,
        enabled: true,
        lastSeenAt: null,
        pairedAt: null,
        fingerprint: null,
      },
    ];
    store.isLoading = true;
    store.isSaving = true;
    store.error = 'boom';

    store.clearSensitiveState();

    expect(store.enabled).toBe(false);
    expect(store.deviceName).toBe('');
    expect(store.port).toBe(8484);
    expect(store.status).toBe('unknown');
    expect(store.pairedDevices).toEqual([]);
    expect(store.discoveredDevices).toEqual([]);
    expect(store.isLoading).toBe(false);
    expect(store.isSaving).toBe(false);
    expect(store.error).toBeNull();
  });
});
