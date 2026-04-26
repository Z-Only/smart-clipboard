import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';

const invoke = vi.fn();
const listen = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({ invoke }));
vi.mock('@tauri-apps/api/event', () => ({ listen }));

const defaultStatus = {
  enabled: true,
  configured: true,
  locked: true,
  biometric_available: true,
  biometric_enabled: true,
  auto_lock_seconds: 30,
  unlock_reason: null,
  failed_attempts: 0,
};

const defaultEncryption = {
  enabled: false,
  key_exists: false,
  encrypted_count: 0,
  plaintext_count: 0,
  migrating: false,
};

describe('useSecurityStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    invoke.mockReset();
    listen.mockReset();
    listen.mockResolvedValue(() => {});
  });

  // --- init ---

  describe('init', () => {
    it('initializes once and refreshes status', async () => {
      invoke.mockImplementation(async (cmd: string) => {
        if (cmd === 'get_app_lock_status') return defaultStatus;
        if (cmd === 'get_encryption_status') return defaultEncryption;
        return {};
      });

      const { useSecurityStore } = await import('@/stores/securityStore');
      const store = useSecurityStore();

      await store.init();
      await store.init();

      expect(listen).toHaveBeenCalledTimes(1);
      expect(invoke).toHaveBeenCalledWith('get_app_lock_status');
      expect(store.status).toEqual(defaultStatus);
    });
  });

  // --- refresh ---

  describe('refresh', () => {
    it('refreshes both status and encryption', async () => {
      const encryptionEnabled = { ...defaultEncryption, enabled: true, key_exists: true };
      invoke.mockImplementation(async (cmd: string) => {
        if (cmd === 'get_app_lock_status') return defaultStatus;
        if (cmd === 'get_encryption_status') return encryptionEnabled;
        return {};
      });

      const { useSecurityStore } = await import('@/stores/securityStore');
      const store = useSecurityStore();

      await store.refresh();

      expect(store.status).toEqual(defaultStatus);
      expect(store.encryption).toEqual(encryptionEnabled);
    });

    it('silently ignores encryption fetch failure', async () => {
      invoke.mockImplementation(async (cmd: string) => {
        if (cmd === 'get_app_lock_status') return defaultStatus;
        if (cmd === 'get_encryption_status') throw new Error('locked');
        return {};
      });

      const { useSecurityStore } = await import('@/stores/securityStore');
      const store = useSecurityStore();

      await store.refresh();

      expect(store.status).toEqual(defaultStatus);
      // encryption stays at default since fetch failed silently
      expect(store.encryption.enabled).toBe(false);
    });
  });

  // --- setPassword ---

  describe('setPassword', () => {
    it('updates status on success', async () => {
      const updatedStatus = { ...defaultStatus, configured: true, locked: false };
      invoke.mockResolvedValue(updatedStatus);

      const { useSecurityStore } = await import('@/stores/securityStore');
      const store = useSecurityStore();

      await store.setPassword(null, 'new-secret');

      expect(invoke).toHaveBeenCalledWith('set_app_lock_password', {
        payload: { current_password: null, new_password: 'new-secret' },
      });
      expect(store.status).toEqual(updatedStatus);
      expect(store.loading).toBe(false);
      expect(store.error).toBeNull();
    });

    it('stores error when setPassword fails', async () => {
      invoke.mockRejectedValue(new Error('update failed'));

      const { useSecurityStore } = await import('@/stores/securityStore');
      const store = useSecurityStore();

      await expect(store.setPassword(null, 'new-secret')).rejects.toThrow('update failed');
      expect(store.loading).toBe(false);
      expect(store.error).toContain('update failed');
    });
  });

  // --- updateSettings ---

  describe('updateSettings', () => {
    it('updates status on success', async () => {
      const updatedStatus = { ...defaultStatus, auto_lock_seconds: 60, biometric_enabled: false };
      invoke.mockResolvedValue(updatedStatus);

      const { useSecurityStore } = await import('@/stores/securityStore');
      const store = useSecurityStore();

      await store.updateSettings({
        enabled: true,
        auto_lock_seconds: 60,
        biometric_enabled: false,
      });

      expect(invoke).toHaveBeenCalledWith('update_app_lock_settings', {
        payload: { enabled: true, auto_lock_seconds: 60, biometric_enabled: false },
      });
      expect(store.status).toEqual(updatedStatus);
      expect(store.loading).toBe(false);
      expect(store.error).toBeNull();
    });

    it('stores error and rethrows on failure', async () => {
      invoke.mockRejectedValue(new Error('settings failed'));

      const { useSecurityStore } = await import('@/stores/securityStore');
      const store = useSecurityStore();

      await expect(
        store.updateSettings({ enabled: true, auto_lock_seconds: 30, biometric_enabled: true }),
      ).rejects.toThrow('settings failed');
      expect(store.loading).toBe(false);
      expect(store.error).toContain('settings failed');
    });
  });

  // --- lock ---

  describe('lock', () => {
    it('locks immediately when lock succeeds', async () => {
      invoke.mockResolvedValue({ ...defaultStatus, locked: true, unlock_reason: 'manual' });

      const { useSecurityStore } = await import('@/stores/securityStore');
      const store = useSecurityStore();

      await store.lock();
      expect(store.status.locked).toBe(true);
      expect(store.status.unlock_reason).toBe('manual');
    });
  });

  // --- unlock ---

  describe('unlock', () => {
    it('unlocks and updates status on success', async () => {
      const unlockedStatus = { ...defaultStatus, locked: false, unlock_reason: 'password' };
      invoke.mockResolvedValue(unlockedStatus);

      const { useSecurityStore } = await import('@/stores/securityStore');
      const store = useSecurityStore();

      await store.unlock('correct-password');

      expect(invoke).toHaveBeenCalledWith('unlock_app', {
        payload: { password: 'correct-password', prefer_biometric: false },
      });
      expect(store.status.locked).toBe(false);
      expect(store.loading).toBe(false);
      expect(store.error).toBeNull();
    });

    it('passes preferBiometric flag', async () => {
      const unlockedStatus = { ...defaultStatus, locked: false };
      invoke.mockResolvedValue(unlockedStatus);

      const { useSecurityStore } = await import('@/stores/securityStore');
      const store = useSecurityStore();

      await store.unlock(null, true);

      expect(invoke).toHaveBeenCalledWith('unlock_app', {
        payload: { password: null, prefer_biometric: true },
      });
    });

    it('stores error when unlock fails', async () => {
      invoke.mockRejectedValue(new Error('bad password'));

      const { useSecurityStore } = await import('@/stores/securityStore');
      const store = useSecurityStore();

      await expect(store.unlock('wrong')).rejects.toThrow('bad password');
      expect(store.loading).toBe(false);
      expect(store.error).toContain('bad password');
    });
  });

  // --- enableEncryption ---

  describe('enableEncryption', () => {
    it('enables encryption and updates state', async () => {
      const encryptedStatus = { ...defaultEncryption, enabled: true, key_exists: true };
      invoke.mockResolvedValue(encryptedStatus);

      const { useSecurityStore } = await import('@/stores/securityStore');
      const store = useSecurityStore();

      await store.enableEncryption();

      expect(invoke).toHaveBeenCalledWith('enable_encryption');
      expect(store.encryption).toEqual(encryptedStatus);
      expect(store.loading).toBe(false);
      expect(store.error).toBeNull();
    });

    it('stores error and rethrows on failure', async () => {
      invoke.mockRejectedValue(new Error('encryption failed'));

      const { useSecurityStore } = await import('@/stores/securityStore');
      const store = useSecurityStore();

      await expect(store.enableEncryption()).rejects.toThrow('encryption failed');
      expect(store.loading).toBe(false);
      expect(store.error).toContain('encryption failed');
    });
  });

  // --- disableEncryption ---

  describe('disableEncryption', () => {
    it('disables encryption and updates state', async () => {
      const disabledStatus = { ...defaultEncryption, enabled: false };
      invoke.mockResolvedValue(disabledStatus);

      const { useSecurityStore } = await import('@/stores/securityStore');
      const store = useSecurityStore();

      await store.disableEncryption();

      expect(invoke).toHaveBeenCalledWith('disable_encryption');
      expect(store.encryption).toEqual(disabledStatus);
      expect(store.loading).toBe(false);
      expect(store.error).toBeNull();
    });

    it('stores error and rethrows on failure', async () => {
      invoke.mockRejectedValue(new Error('disable failed'));

      const { useSecurityStore } = await import('@/stores/securityStore');
      const store = useSecurityStore();

      await expect(store.disableEncryption()).rejects.toThrow('disable failed');
      expect(store.loading).toBe(false);
      expect(store.error).toContain('disable failed');
    });
  });

  // --- refreshEncryption ---

  describe('refreshEncryption', () => {
    it('refreshes encryption status', async () => {
      const encryptedStatus = {
        ...defaultEncryption,
        enabled: true,
        encrypted_count: 42,
      };
      invoke.mockResolvedValue(encryptedStatus);

      const { useSecurityStore } = await import('@/stores/securityStore');
      const store = useSecurityStore();

      await store.refreshEncryption();

      expect(invoke).toHaveBeenCalledWith('get_encryption_status');
      expect(store.encryption).toEqual(encryptedStatus);
    });

    it('silently ignores error when locked', async () => {
      invoke.mockRejectedValue(new Error('locked'));

      const { useSecurityStore } = await import('@/stores/securityStore');
      const store = useSecurityStore();

      await store.refreshEncryption();

      // Should not throw and encryption stays at default
      expect(store.encryption.enabled).toBe(false);
    });
  });
});
