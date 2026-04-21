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
});
