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

describe('useSecurityStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    invoke.mockReset();
    listen.mockReset();
    listen.mockResolvedValue(() => {});
  });

  it('initializes once and refreshes status', async () => {
    invoke.mockResolvedValue(defaultStatus);

    const { useSecurityStore } = await import('@/stores/securityStore');
    const store = useSecurityStore();

    await store.init();
    await store.init();

    expect(listen).toHaveBeenCalledTimes(1);
    expect(invoke).toHaveBeenCalledWith('get_app_lock_status');
    expect(store.status).toEqual(defaultStatus);
  });

  it('stores error when unlock fails', async () => {
    const error = new Error('bad password');
    invoke.mockRejectedValue(error);

    const { useSecurityStore } = await import('@/stores/securityStore');
    const store = useSecurityStore();

    await expect(store.unlock('wrong')).rejects.toThrow('bad password');
    expect(store.loading).toBe(false);
    expect(store.error).toContain('bad password');
  });

  it('stores error when setPassword fails', async () => {
    invoke.mockRejectedValue(new Error('update failed'));

    const { useSecurityStore } = await import('@/stores/securityStore');
    const store = useSecurityStore();

    await expect(store.setPassword(null, 'new-secret')).rejects.toThrow('update failed');
    expect(store.loading).toBe(false);
    expect(store.error).toContain('update failed');
  });

  it('locks immediately when lock succeeds', async () => {
    invoke.mockResolvedValue({ ...defaultStatus, locked: true, unlock_reason: 'manual' });

    const { useSecurityStore } = await import('@/stores/securityStore');
    const store = useSecurityStore();

    await store.lock();
    expect(store.status.locked).toBe(true);
    expect(store.status.unlock_reason).toBe('manual');
  });
});
