import { describe, it, expect, vi, beforeEach } from 'vitest';
import { mount, flushPromises } from '@vue/test-utils';
import { createI18n } from 'vue-i18n';

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke }));

import StatisticsPanel from '@/components/StatisticsPanel.vue';

const i18n = createI18n({
  legacy: false,
  locale: 'en',
  messages: {
    en: {
      statistics: {
        title: 'Statistics',
        totalEntries: 'Total',
        totalFavorites: 'Favorites',
        storageSize: 'Storage',
        categoryBreakdown: 'Category Breakdown',
        dailyActivity: 'Daily Activity',
        mostUsed: 'Most Used',
        noData: 'No data',
      },
      list: {
        loading: 'Loading...',
      },
    },
  },
});

describe('StatisticsPanel', () => {
  beforeEach(() => {
    invoke.mockReset();
  });

  it('does not render when isOpen is false', () => {
    const wrapper = mount(StatisticsPanel, {
      props: { isOpen: false },
      global: { plugins: [i18n] },
    });
    expect(wrapper.find('h2').exists()).toBe(false);
  });

  it('renders statistics panel when isOpen is true', async () => {
    invoke.mockResolvedValue({
      total_entries: 100,
      total_favorites: 10,
      storage_size_bytes: 1024,
      entries_by_category: [{ category: 'text', count: 50 }],
      entries_by_day: [],
      most_used: [],
    });

    const wrapper = mount(StatisticsPanel, {
      props: { isOpen: true },
      global: { plugins: [i18n] },
    });

    await flushPromises();

    expect(wrapper.find('h2').exists()).toBe(true);
    expect(wrapper.text()).toContain('Statistics');
  });
});
