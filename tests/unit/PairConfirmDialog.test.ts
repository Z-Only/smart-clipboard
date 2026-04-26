import { describe, it, expect, vi } from 'vitest';
import { mount } from '@vue/test-utils';
import { createI18n } from 'vue-i18n';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));

import PairConfirmDialog from '@/components/PairConfirmDialog.vue';

const i18n = createI18n({
  legacy: false,
  locale: 'en',
  messages: {
    en: {
      sync: {
        panel: { deviceName: 'Device' },
        pairDialog: {
          title: 'Pair Device',
          message: 'Confirm pairing',
          fingerprint: 'Fingerprint',
          warning: 'Warning text',
          cancel: 'Cancel',
          confirm: 'Confirm',
        },
      },
    },
  },
});

const sampleDevice = {
  id: 'dev-1',
  name: 'Test',
  deviceName: 'My Laptop',
  address: '192.168.1.10',
  ip: '192.168.1.10',
  port: 8080,
  status: 'online' as const,
  fingerprint: 'abc123',
  syncEnabled: true,
};

describe('PairConfirmDialog', () => {
  it('does not render when isOpen is false', () => {
    const wrapper = mount(PairConfirmDialog, {
      props: { isOpen: false, device: sampleDevice },
      global: { plugins: [i18n] },
    });
    expect(wrapper.find('h3').exists()).toBe(false);
  });

  it('renders dialog when isOpen is true', () => {
    const wrapper = mount(PairConfirmDialog, {
      props: { isOpen: true, device: sampleDevice },
      global: { plugins: [i18n] },
    });
    expect(wrapper.find('h3').text()).toBe('Pair Device');
    expect(wrapper.text()).toContain('My Laptop');
  });

  it('emits confirm when confirm button is clicked', async () => {
    const wrapper = mount(PairConfirmDialog, {
      props: { isOpen: true, device: sampleDevice },
      global: { plugins: [i18n] },
    });
    const confirmBtn = wrapper.findAll('button').find((b) => b.text() === 'Confirm');
    await confirmBtn!.trigger('click');
    expect(wrapper.emitted('confirm')).toBeTruthy();
  });

  it('emits cancel when cancel button is clicked', async () => {
    const wrapper = mount(PairConfirmDialog, {
      props: { isOpen: true, device: sampleDevice },
      global: { plugins: [i18n] },
    });
    const cancelBtn = wrapper.findAll('button').find((b) => b.text() === 'Cancel');
    await cancelBtn!.trigger('click');
    expect(wrapper.emitted('cancel')).toBeTruthy();
  });
});
