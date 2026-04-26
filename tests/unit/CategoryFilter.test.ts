import { describe, it, expect, vi, beforeEach } from 'vitest';
import { mount, flushPromises } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { createI18n } from 'vue-i18n';

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke }));
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn().mockResolvedValue(() => {}) }));
vi.mock('@/composables/useSearch', () => ({ useSearch: vi.fn() }));

import CategoryFilter from '@/components/CategoryFilter.vue';
import { CATEGORIES } from '@/types';

const i18n = createI18n({
  legacy: false,
  locale: 'en',
  messages: {
    en: {
      categories: {
        all: 'All',
        favorites: 'Favorites',
        tags: 'Tags',
        templates: 'Templates',
        image: 'Image',
        url: 'URL',
        email: 'Email',
        code: 'Code',
        json: 'JSON',
        filepath: 'File Path',
        color: 'Color',
        phone: 'Phone',
        address: 'Address',
        text: 'Text',
      },
      tags: { filterByTag: 'Filter by Tag', deleteTag: 'Delete' },
    },
  },
});

describe('CategoryFilter', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    invoke.mockReset();
    invoke.mockResolvedValue([]);
  });

  it('renders all category buttons', async () => {
    const wrapper = mount(CategoryFilter, { global: { plugins: [i18n] } });
    await flushPromises();
    const buttons = wrapper.findAll('button');
    expect(buttons.length).toBeGreaterThanOrEqual(CATEGORIES.length);
  });

  it('highlights the selected category', async () => {
    const wrapper = mount(CategoryFilter, { global: { plugins: [i18n] } });
    await flushPromises();
    const firstButton = wrapper.findAll('button')[0];
    expect(firstButton.classes()).toContain('bg-primary');
  });
});
