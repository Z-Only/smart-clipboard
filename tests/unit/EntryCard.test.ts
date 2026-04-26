import { describe, it, expect, vi, beforeEach } from 'vitest';
import { mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { createI18n } from 'vue-i18n';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn().mockResolvedValue(() => {}) }));
vi.mock('@/composables/useSearch', () => ({ useSearch: vi.fn() }));

// Stub child components that EntryCard may use
vi.mock('@/components/TagPicker.vue', async () => {
  const { defineComponent, h } = await import('vue');
  return {
    default: defineComponent({
      name: 'TagPickerStub',
      setup() {
        return () => h('div', { 'data-test': 'tag-picker' });
      },
    }),
  };
});

vi.mock('@/components/TransformMenu.vue', async () => {
  const { defineComponent, h } = await import('vue');
  return {
    default: defineComponent({
      name: 'TransformMenuStub',
      setup() {
        return () => h('div', { 'data-test': 'transform-menu' });
      },
    }),
  };
});

import EntryCard from '@/components/EntryCard.vue';

const i18n = createI18n({
  legacy: false,
  locale: 'en',
  messages: {
    en: {
      entry: {
        justNow: 'Just now',
        minutesAgo: '{n} min ago',
        hoursAgo: '{n}h ago',
        daysAgo: '{n}d ago',
        sensitive: 'Sensitive',
        image: 'Image',
      },
    },
  },
});

const sampleEntry = {
  id: 1,
  content: 'Hello World',
  content_type: 'text',
  category: 'text',
  hash: 'abc123',
  source_app: null,
  is_favorite: false,
  is_sensitive: false,
  use_count: 3,
  created_at: new Date().toISOString(),
  updated_at: new Date().toISOString(),
  expires_at: null,
};

describe('EntryCard', () => {
  beforeEach(async () => {
    setActivePinia(createPinia());
    const { invoke } = await import('@tauri-apps/api/core');
    vi.mocked(invoke).mockReset();
    vi.mocked(invoke).mockResolvedValue([]);
  });

  it('renders entry content', () => {
    const wrapper = mount(EntryCard, {
      props: { entry: sampleEntry },
      global: { plugins: [i18n] },
    });
    expect(wrapper.text()).toContain('Hello World');
  });

  it('emits click when card is clicked', async () => {
    const wrapper = mount(EntryCard, {
      props: { entry: sampleEntry },
      global: { plugins: [i18n] },
    });
    await wrapper.trigger('click');
    expect(wrapper.emitted('click')).toBeTruthy();
  });

  it('shows favorite indicator when entry is favorited', () => {
    const wrapper = mount(EntryCard, {
      props: { entry: { ...sampleEntry, is_favorite: true } },
      global: { plugins: [i18n] },
    });
    expect(wrapper.find('.text-yellow-500').exists()).toBe(true);
  });

  it('shows checkbox in multi-select mode', () => {
    const wrapper = mount(EntryCard, {
      props: { entry: sampleEntry, showCheckbox: true, isChecked: false },
      global: { plugins: [i18n] },
    });
    expect(wrapper.find('button[aria-label="list.toggleSelection"]').exists()).toBe(true);
  });
});
