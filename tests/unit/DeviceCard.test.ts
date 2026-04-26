import { describe, it, expect, vi } from 'vitest';
import { mount } from '@vue/test-utils';
import { createI18n } from 'vue-i18n';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));

import DeviceCard from '@/components/DeviceCard.vue';

const i18n = createI18n({
  legacy: false,
  locale: 'en',
  messages: {
    en: {
      sync: {
        panel: { port: 'Port', deviceName: 'Device' },
        actions: { pair: 'Pair', unpair: 'Unpair' },
        statusValues: {
          online: 'Online',
          offline: 'Offline',
          connected: 'Connected',
          unknown: 'Unknown',
          connecting: 'Connecting',
          pairing: 'Pairing',
          disabled: 'Disabled',
          error: 'Error',
        },
        device: { syncOn: 'Sync on', syncOff: 'Sync off', availableToPair: 'Available' },
      },
    },
  },
});

const sampleDevice = {
  id: 'dev-1',
  name: 'Test Device',
  deviceName: 'My Laptop',
  address: '192.168.1.10',
  ip: '192.168.1.10',
  port: 8080,
  status: 'online' as const,
  fingerprint: 'abc123',
  syncEnabled: true,
};

describe('DeviceCard', () => {
  it('renders device name and address in paired mode', () => {
    const wrapper = mount(DeviceCard, {
      props: { device: sampleDevice, mode: 'paired' },
      global: { plugins: [i18n] },
    });
    expect(wrapper.text()).toContain('My Laptop');
    expect(wrapper.text()).toContain('192.168.1.10');
  });

  it('shows pair button in discovered mode', () => {
    const wrapper = mount(DeviceCard, {
      props: { device: { ...sampleDevice, status: 'offline' as const }, mode: 'discovered' },
      global: { plugins: [i18n] },
    });
    expect(wrapper.text()).toContain('Pair');
  });

  it('shows unpair button in paired mode', () => {
    const wrapper = mount(DeviceCard, {
      props: { device: sampleDevice, mode: 'paired' },
      global: { plugins: [i18n] },
    });
    expect(wrapper.text()).toContain('Unpair');
  });

  it('emits pair event when pair button is clicked', async () => {
    const wrapper = mount(DeviceCard, {
      props: { device: sampleDevice, mode: 'discovered' },
      global: { plugins: [i18n] },
    });
    const pairBtn = wrapper.findAll('button').find((b) => b.text().includes('Pair'));
    await pairBtn!.trigger('click');
    expect(wrapper.emitted('pair')).toBeTruthy();
  });

  it('emits unpair event when unpair button is clicked', async () => {
    const wrapper = mount(DeviceCard, {
      props: { device: sampleDevice, mode: 'paired' },
      global: { plugins: [i18n] },
    });
    const unpairBtn = wrapper.findAll('button').find((b) => b.text().includes('Unpair'));
    await unpairBtn!.trigger('click');
    expect(wrapper.emitted('unpair')).toBeTruthy();
  });
});
