import { describe, it, expect, vi, beforeEach } from 'vitest';
import { mount, flushPromises } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { createI18n } from 'vue-i18n';

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke }));
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn().mockResolvedValue(() => {}) }));

import TagPicker from '@/components/TagPicker.vue';

const i18n = createI18n({
  legacy: false,
  locale: 'en',
  messages: {
    en: {
      tags: {
        manageTags: 'Tags',
        addTag: 'Add tag',
        createTag: 'Create',
        newTagPlaceholder: 'New tag name',
      },
    },
  },
});

describe('TagPicker', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    invoke.mockReset();
    invoke.mockResolvedValue([]);
  });

  it('renders tag picker button', () => {
    const wrapper = mount(TagPicker, {
      props: { entryId: 1 },
      global: { plugins: [i18n] },
    });
    expect(wrapper.find('button').exists()).toBe(true);
  });

  it('opens picker and loads tags on button click', async () => {
    invoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'get_all_tags') return [{ id: 1, name: 'work' }];
      if (cmd === 'get_entry_tags') return [];
      return [];
    });

    const wrapper = mount(TagPicker, {
      props: { entryId: 1 },
      global: { plugins: [i18n] },
    });

    await wrapper.find('button').trigger('click');
    await flushPromises();

    expect(invoke).toHaveBeenCalledWith('get_all_tags');
    expect(invoke).toHaveBeenCalledWith('get_entry_tags', { entryId: 1 });
  });
});
