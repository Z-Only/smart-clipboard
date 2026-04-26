import { describe, it, expect, vi, beforeEach } from 'vitest';
import { mount, flushPromises } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { createI18n } from 'vue-i18n';

const { invoke, listen } = vi.hoisted(() => ({
  invoke: vi.fn(),
  listen: vi.fn().mockResolvedValue(() => {}),
}));
vi.mock('@tauri-apps/api/core', () => ({ invoke }));
vi.mock('@tauri-apps/api/event', () => ({ listen }));

import LockScreen from '@/components/LockScreen.vue';

const i18n = createI18n({
  legacy: false,
  locale: 'en',
  messages: {
    en: {
      lock: {
        title: 'Locked',
        passwordPlaceholder: 'Enter password',
        unlockWithPassword: 'Unlock',
        unlockWithBiometric: 'Biometric',
        failedAttempts: '{count} failed attempts',
        reasonStartup: 'App just started',
        reasonAutoLock: 'Auto-locked',
        reasonManual: 'Manually locked',
        reasonProtected: 'Protected action',
        reasonDefault: 'App is locked',
      },
    },
  },
});

describe('LockScreen', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    invoke.mockReset();
  });

  it('renders lock screen with title and password input', () => {
    const wrapper = mount(LockScreen, { global: { plugins: [i18n] } });
    expect(wrapper.find('h2').text()).toBe('Locked');
    expect(wrapper.find('input[type="password"]').exists()).toBe(true);
  });

  it('renders unlock button', () => {
    const wrapper = mount(LockScreen, { global: { plugins: [i18n] } });
    const buttons = wrapper.findAll('button');
    expect(buttons.some((b) => b.text().includes('Unlock'))).toBe(true);
  });

  it('calls security.unlock when password submit button is clicked', async () => {
    invoke.mockResolvedValue({
      enabled: true,
      configured: true,
      locked: false,
      biometric_available: false,
      biometric_enabled: false,
      auto_lock_seconds: 0,
      unlock_reason: null,
      failed_attempts: 0,
    });

    const wrapper = mount(LockScreen, { global: { plugins: [i18n] } });
    await wrapper.find('input[type="password"]').setValue('secret');
    const unlockBtn = wrapper.findAll('button').find((b) => b.text().includes('Unlock'));
    await unlockBtn!.trigger('click');
    await flushPromises();

    expect(invoke).toHaveBeenCalledWith('unlock_app', {
      payload: { password: 'secret', prefer_biometric: false },
    });
  });

  it('does not show biometric button when not available', () => {
    const wrapper = mount(LockScreen, { global: { plugins: [i18n] } });
    const buttons = wrapper.findAll('button');
    expect(buttons.some((b) => b.text().includes('Biometric'))).toBe(false);
  });
});
