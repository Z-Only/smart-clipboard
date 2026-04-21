import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type {
  SyncConfig,
  SyncDevice,
  SyncStatus,
  SyncStatusResponse,
  UpdateSyncConfigPayload,
} from '@/types';

function normalizeSyncStatus(status: unknown): SyncStatus {
  switch (status) {
    case 'idle':
    case 'discovering':
    case 'connecting':
    case 'connected':
    case 'online':
    case 'offline':
    case 'disabled':
    case 'pairing':
    case 'error':
      return status;
    default:
      return 'unknown';
  }
}

function normalizeDevice(
  device: Partial<SyncDevice> | null | undefined,
  fallbackId = 'unknown',
): SyncDevice {
  return {
    id: String(device?.id ?? fallbackId),
    name: String(device?.name ?? device?.id ?? fallbackId),
    deviceName: String(device?.deviceName ?? device?.name ?? device?.id ?? fallbackId),
    address: device?.address ?? null,
    ip: device?.ip ?? null,
    port: typeof device?.port === 'number' ? device.port : null,
    status: normalizeSyncStatus(device?.status),
    syncEnabled: Boolean(device?.syncEnabled ?? device?.enabled ?? true),
    enabled: Boolean(device?.enabled ?? device?.syncEnabled ?? true),
    lastSeenAt: device?.lastSeenAt ?? null,
    pairedAt: device?.pairedAt ?? null,
    fingerprint: device?.fingerprint ?? null,
  };
}

function normalizeDevices(devices: unknown): SyncDevice[] {
  if (!Array.isArray(devices)) return [];
  return devices.map((device, index) =>
    normalizeDevice(device as Partial<SyncDevice>, `device-${index + 1}`),
  );
}

function normalizeStatus(payload: unknown): SyncStatusResponse {
  const data = (payload ?? {}) as Partial<SyncStatusResponse>;
  return {
    enabled: Boolean(data.enabled),
    deviceName: String(data.deviceName ?? ''),
    port: typeof data.port === 'number' ? data.port : 8484,
    status: normalizeSyncStatus(data.status),
    pairedDevices: normalizeDevices(data.pairedDevices),
    discoveredDevices: normalizeDevices(data.discoveredDevices),
  };
}

export const useSyncStore = defineStore('sync', () => {
  const enabled = ref(false);
  const deviceName = ref('');
  const port = ref(8484);
  const status = ref<SyncStatus>('unknown');
  const pairedDevices = ref<SyncDevice[]>([]);
  const discoveredDevices = ref<SyncDevice[]>([]);
  const isLoading = ref(false);
  const isSaving = ref(false);
  const error = ref<string | null>(null);

  const activePairedCount = computed(
    () =>
      pairedDevices.value.filter((device) => ['online', 'connected'].includes(device.status))
        .length,
  );

  async function refreshStatus() {
    const result = normalizeStatus(await invoke('get_sync_status'));
    enabled.value = result.enabled;
    deviceName.value = result.deviceName;
    port.value = result.port;
    status.value = result.status;
    pairedDevices.value = result.pairedDevices;
    discoveredDevices.value = result.discoveredDevices;
  }

  async function loadConfig() {
    const config = (await invoke('get_sync_config')) as Partial<SyncConfig> | null;
    if (!config) return;
    enabled.value = Boolean(config.enabled ?? enabled.value);
    deviceName.value = String(config.deviceName ?? deviceName.value);
    port.value = typeof config.port === 'number' ? config.port : port.value;
  }

  async function loadPairedDevices() {
    pairedDevices.value = normalizeDevices(await invoke('get_paired_devices'));
  }

  async function loadDiscoveredDevices() {
    discoveredDevices.value = normalizeDevices(await invoke('get_discovered_devices'));
  }

  async function refreshAll() {
    isLoading.value = true;
    error.value = null;
    try {
      await Promise.all([
        refreshStatus(),
        loadConfig(),
        loadPairedDevices(),
        loadDiscoveredDevices(),
      ]);
    } catch (e) {
      console.error('Failed to refresh sync data:', e);
      error.value = e instanceof Error ? e.message : 'Failed to refresh sync data';
    } finally {
      isLoading.value = false;
    }
  }

  async function saveConfig(payload: UpdateSyncConfigPayload) {
    isSaving.value = true;
    error.value = null;
    try {
      await invoke('update_sync_config', { newConfig: payload });
      await refreshAll();
    } catch (e) {
      console.error('Failed to update sync config:', e);
      error.value = e instanceof Error ? e.message : 'Failed to update sync config';
      throw e;
    } finally {
      isSaving.value = false;
    }
  }

  async function pairDevice(deviceId: string) {
    try {
      await invoke('pair_device', { deviceId });
      await refreshAll();
    } catch (e) {
      console.error('Failed to pair device:', e);
      error.value = e instanceof Error ? e.message : 'Failed to pair device';
    }
  }

  async function unpairDevice(deviceId: string) {
    try {
      await invoke('unpair_device', { deviceId });
      await refreshAll();
    } catch (e) {
      console.error('Failed to unpair device:', e);
      error.value = e instanceof Error ? e.message : 'Failed to unpair device';
    }
  }

  async function toggleDeviceSync(deviceId: string, enabled: boolean) {
    try {
      await invoke('toggle_device_sync', { deviceId, enabled });
      await refreshAll();
    } catch (e) {
      console.error('Failed to toggle device sync:', e);
      error.value = e instanceof Error ? e.message : 'Failed to toggle device sync';
    }
  }

  function clearSensitiveState() {
    enabled.value = false;
    deviceName.value = '';
    port.value = 8484;
    status.value = 'unknown';
    pairedDevices.value = [];
    discoveredDevices.value = [];
    isLoading.value = false;
    isSaving.value = false;
    error.value = null;
  }

  function clearError() {
    error.value = null;
  }

  return {
    enabled,
    deviceName,
    port,
    status,
    pairedDevices,
    discoveredDevices,
    isLoading,
    isSaving,
    error,
    activePairedCount,
    refreshAll,
    saveConfig,
    pairDevice,
    unpairDevice,
    toggleDeviceSync,
    clearError,
    clearSensitiveState,
  };
});
