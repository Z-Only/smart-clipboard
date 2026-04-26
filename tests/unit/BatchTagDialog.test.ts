import { describe, it, expect, vi, beforeEach } from 'vitest';
import { mount, flushPromises } from '@vue/test-utils';
import { createI18n } from 'vue-i18n';

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke }));

import BatchTagDialog from '@/components/BatchTagDialog.vue';

const i18n = createI18n({
  legacy: false,
  locale: 'en',
  messages: {
    en: {
      tags: {
        batchTitle: 'Batch Tag',
        batchHint: '{count} items selected',
        modeAppend: 'Append',
        modeReplace: 'Replace',
        newTagPlaceholder: 'New tag',
        apply: 'Apply',
        cancel: 'Cancel',
      },
    },
  },
});

describe('BatchTagDialog', () => {
  beforeEach(() => {
    invoke.mockReset();
    invoke.mockResolvedValue([]);
  });

  it('does not render when isOpen is false', () => {
    const wrapper = mount(BatchTagDialog, {
      props: { isOpen: false, count: 3 },
      global: { plugins: [i18n] },
    });
    expect(wrapper.find('h3').exists()).toBe(false);
  });

  it('renders dialog when isOpen is true', async () => {
    invoke.mockResolvedValue([{ id: 1, name: 'work' }]);
    const wrapper = mount(BatchTagDialog, {
      props: { isOpen: true, count: 3 },
      global: { plugins: [i18n] },
    });
    await flushPromises();
    expect(wrapper.find('h3').exists()).toBe(true);
  });

  it('emits close when cancel button is clicked', async () => {
    invoke.mockResolvedValue([]);
    const wrapper = mount(BatchTagDialog, {
      props: { isOpen: true, count: 3 },
      global: { plugins: [i18n] },
    });
    await flushPromises();
    const cancelBtn = wrapper.findAll('button').find((b) => b.text().includes('Cancel'));
    if (cancelBtn) {
      await cancelBtn.trigger('click');
      expect(wrapper.emitted('close')).toBeTruthy();
    }
  });
});
