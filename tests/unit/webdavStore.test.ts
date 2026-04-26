import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import type { WebDavConfig, WebDavSyncStatus } from '@/types';

const invoke = vi.fn();
const errorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
vi.mock('@tauri-apps/api/core', () => ({ invoke }));

const defaultConfig: WebDavConfig = {
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

const sampleStatus: WebDavSyncStatus = {
  status: 'connected',
  lastSyncAt: '2026-04-26 12:00:00',
  cloudEntryCount: 42,
  registeredDevices: [
    {
      deviceId: 'dev-1',
      deviceName: 'MacBook',
      publicKey: 'pk-1',
      registeredAt: '2026-04-20 10:00:00',
      lastSyncAt: '2026-04-26 12:00:00',
    },
  ],
  rateLimitAvailable: 100,
  rateLimitCapacity: 150,
  error: null,
};

describe('useWebDavStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    invoke.mockReset();
    errorSpy.mockClear();
  });

  describe('配置管理', () => {
    it('loadConfig 成功加载配置', async () => {
      const savedConfig: WebDavConfig = {
        ...defaultConfig,
        enabled: true,
        serverUrl: 'https://dav.example.com',
        username: 'user',
      };
      invoke.mockResolvedValue(savedConfig);

      const { useWebDavStore } = await import('@/stores/webdavStore');
      const store = useWebDavStore();

      await store.loadConfig();

      expect(invoke).toHaveBeenCalledWith('webdav_get_config');
      expect(store.config.enabled).toBe(true);
      expect(store.config.serverUrl).toBe('https://dav.example.com');
    });

    it('loadConfig 失败时设置 error', async () => {
      invoke.mockRejectedValue(new Error('load failed'));

      const { useWebDavStore } = await import('@/stores/webdavStore');
      const store = useWebDavStore();

      await store.loadConfig();

      expect(store.error).toBe('load failed');
      expect(errorSpy).toHaveBeenCalled();
    });

    it('saveConfig 成功保存并更新本地状态', async () => {
      invoke.mockResolvedValue(undefined);

      const { useWebDavStore } = await import('@/stores/webdavStore');
      const store = useWebDavStore();

      const newConfig: WebDavConfig = {
        ...defaultConfig,
        enabled: true,
        serverUrl: 'https://new.dav.example.com',
      };

      await store.saveConfig(newConfig);

      expect(invoke).toHaveBeenCalledWith('webdav_update_config', { newConfig });
      expect(store.config.serverUrl).toBe('https://new.dav.example.com');
    });

    it('saveConfig 失败时设置 error 并抛出异常', async () => {
      invoke.mockRejectedValue(new Error('save failed'));

      const { useWebDavStore } = await import('@/stores/webdavStore');
      const store = useWebDavStore();

      await expect(store.saveConfig(defaultConfig)).rejects.toThrow('save failed');
      expect(store.error).toBe('save failed');
    });
  });

  describe('连接管理', () => {
    it('connect 成功连接并刷新状态', async () => {
      invoke.mockImplementation(async (cmd: string) => {
        if (cmd === 'webdav_connect') return undefined;
        if (cmd === 'webdav_get_status') return sampleStatus;
        return undefined;
      });

      const { useWebDavStore } = await import('@/stores/webdavStore');
      const store = useWebDavStore();

      await store.connect('https://dav.example.com', 'user', 'pass', 'syncPass');

      expect(invoke).toHaveBeenCalledWith('webdav_connect', {
        serverUrl: 'https://dav.example.com',
        username: 'user',
        password: 'pass',
        syncPassword: 'syncPass',
      });
      expect(store.status).toBe('connected');
      expect(store.isConnecting).toBe(false);
    });

    it('connect 失败时设置 error、恢复 isConnecting 并抛出异常', async () => {
      invoke.mockRejectedValue(new Error('connection refused'));

      const { useWebDavStore } = await import('@/stores/webdavStore');
      const store = useWebDavStore();

      await expect(
        store.connect('https://bad.dav.example.com', 'user', 'pass', 'syncPass'),
      ).rejects.toThrow('connection refused');

      expect(store.error).toBe('connection refused');
      expect(store.isConnecting).toBe(false);
    });

    it('disconnect 成功断开并重置状态', async () => {
      invoke.mockResolvedValue(undefined);

      const { useWebDavStore } = await import('@/stores/webdavStore');
      const store = useWebDavStore();

      // 先模拟连接状态
      store.status = 'connected';
      store.cloudEntryCount = 10;

      await store.disconnect();

      expect(invoke).toHaveBeenCalledWith('webdav_disconnect');
      expect(store.status).toBe('disconnected');
      expect(store.lastSyncAt).toBeNull();
      expect(store.cloudEntryCount).toBe(0);
      expect(store.registeredDevices).toEqual([]);
      expect(store.error).toBeNull();
    });

    it('refreshStatus 成功刷新所有状态字段', async () => {
      invoke.mockResolvedValue(sampleStatus);

      const { useWebDavStore } = await import('@/stores/webdavStore');
      const store = useWebDavStore();

      await store.refreshStatus();

      expect(store.status).toBe('connected');
      expect(store.lastSyncAt).toBe('2026-04-26 12:00:00');
      expect(store.cloudEntryCount).toBe(42);
      expect(store.registeredDevices).toHaveLength(1);
      expect(store.rateLimitAvailable).toBe(100);
      expect(store.rateLimitCapacity).toBe(150);
    });

    it('refreshStatus 有 error 字段时设置 error', async () => {
      invoke.mockResolvedValue({ ...sampleStatus, error: 'rate limited' });

      const { useWebDavStore } = await import('@/stores/webdavStore');
      const store = useWebDavStore();

      await store.refreshStatus();

      expect(store.error).toBe('rate limited');
    });

    it('refreshAll 并行加载配置和状态并管理 isLoading', async () => {
      invoke.mockImplementation(async (cmd: string) => {
        if (cmd === 'webdav_get_config') return { ...defaultConfig, enabled: true };
        if (cmd === 'webdav_get_status') return sampleStatus;
        return undefined;
      });

      const { useWebDavStore } = await import('@/stores/webdavStore');
      const store = useWebDavStore();

      await store.refreshAll();

      expect(store.config.enabled).toBe(true);
      expect(store.status).toBe('connected');
      expect(store.isLoading).toBe(false);
    });
  });

  describe('同步与设备管理', () => {
    it('triggerSync 成功触发同步并返回同步数量', async () => {
      invoke.mockImplementation(async (cmd: string) => {
        if (cmd === 'webdav_trigger_sync') return 5;
        if (cmd === 'webdav_get_status') return sampleStatus;
        return undefined;
      });

      const { useWebDavStore } = await import('@/stores/webdavStore');
      const store = useWebDavStore();

      const count = await store.triggerSync();

      expect(count).toBe(5);
      expect(invoke).toHaveBeenCalledWith('webdav_trigger_sync');
    });

    it('triggerSync 失败时设置 error 并抛出异常', async () => {
      invoke.mockRejectedValue(new Error('sync failed'));

      const { useWebDavStore } = await import('@/stores/webdavStore');
      const store = useWebDavStore();

      await expect(store.triggerSync()).rejects.toThrow('sync failed');
      expect(store.error).toBe('sync failed');
    });

    it('removeDevice 成功移除设备并刷新状态', async () => {
      invoke.mockImplementation(async (cmd: string) => {
        if (cmd === 'webdav_remove_device') return undefined;
        if (cmd === 'webdav_get_status') return { ...sampleStatus, registeredDevices: [] };
        return undefined;
      });

      const { useWebDavStore } = await import('@/stores/webdavStore');
      const store = useWebDavStore();

      await store.removeDevice('dev-1');

      expect(invoke).toHaveBeenCalledWith('webdav_remove_device', { deviceId: 'dev-1' });
      expect(store.registeredDevices).toHaveLength(0);
    });
  });

  describe('错误处理与状态清理', () => {
    it('clearError 清除错误状态', async () => {
      const { useWebDavStore } = await import('@/stores/webdavStore');
      const store = useWebDavStore();

      store.error = 'some error';
      store.clearError();
      expect(store.error).toBeNull();
    });

    it('clearSensitiveState 重置所有状态为默认值', async () => {
      const { useWebDavStore } = await import('@/stores/webdavStore');
      const store = useWebDavStore();

      // 先设置一些非默认值
      store.config = {
        ...defaultConfig,
        enabled: true,
        serverUrl: 'https://dav.example.com',
      };
      store.status = 'connected';
      store.lastSyncAt = '2026-04-26 12:00:00';
      store.cloudEntryCount = 42;
      store.error = 'some error';
      store.isLoading = true;
      store.isConnecting = true;

      store.clearSensitiveState();

      expect(store.config).toEqual(defaultConfig);
      expect(store.status).toBe('disconnected');
      expect(store.lastSyncAt).toBeNull();
      expect(store.cloudEntryCount).toBe(0);
      expect(store.registeredDevices).toEqual([]);
      expect(store.rateLimitAvailable).toBe(0);
      expect(store.rateLimitCapacity).toBe(0);
      expect(store.error).toBeNull();
      expect(store.isLoading).toBe(false);
      expect(store.isConnecting).toBe(false);
    });
  });
});
