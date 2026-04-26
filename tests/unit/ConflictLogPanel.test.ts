import { describe, it, expect, vi, beforeEach } from 'vitest';
import { mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { createI18n } from 'vue-i18n';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn().mockResolvedValue(() => {}) }));

import ConflictLogPanel from '@/components/ConflictLogPanel.vue';

const i18n = createI18n({
  legacy: false,
  locale: 'en',
  messages: {
    en: {
      conflict: {
        log: {
          title: 'Conflict Log',
          clearAll: 'Clear All',
          clearConfirm: 'Are you sure?',
          empty: 'No conflicts',
          emptyHint: 'Resolved conflicts will appear here',
          entry: {
            localContent: 'Local',
            remoteContent: 'Remote',
            device: 'Device',
            strategy: 'Strategy',
          },
          outcome: {
            'kept-local': 'Kept Local',
            'kept-remote': 'Kept Remote',
            'auto-resolved': 'Auto-resolved',
            dismissed: 'Dismissed',
          },
        },
        strategy: {
          'local-wins': 'Local Wins',
          'remote-wins': 'Remote Wins',
          'newest-wins': 'Newest Wins',
          manual: 'Manual',
        },
      },
    },
  },
});

describe('ConflictLogPanel', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it('shows empty state when no log entries', () => {
    const wrapper = mount(ConflictLogPanel, {
      global: { plugins: [i18n] },
    });
    expect(wrapper.text()).toContain('No conflicts');
  });

  it('renders conflict log title', () => {
    const wrapper = mount(ConflictLogPanel, {
      global: { plugins: [i18n] },
    });
    expect(wrapper.find('h3').text()).toBe('Conflict Log');
  });

  it('does not show clear button when log is empty', () => {
    const wrapper = mount(ConflictLogPanel, {
      global: { plugins: [i18n] },
    });
    const clearBtn = wrapper.findAll('button').find((b) => b.text().includes('Clear All'));
    expect(clearBtn).toBeUndefined();
  });
});
