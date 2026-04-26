import { describe, it, expect, vi, beforeEach } from 'vitest';
import { mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { createI18n } from 'vue-i18n';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn().mockResolvedValue(() => {}) }));

import ConflictResolveDialog from '@/components/ConflictResolveDialog.vue';

const i18n = createI18n({
  legacy: false,
  locale: 'en',
  messages: {
    en: {
      conflict: {
        resolve: {
          title: 'Resolve Conflict',
          description: 'Choose which version to keep',
          remaining: '{count} remaining',
          localVersion: 'Local',
          remoteVersion: 'Remote',
          modifiedAt: 'Modified: {time}',
          contentType: 'Type: {type}',
          fromDevice: 'From: {device}',
          keepLocal: 'Keep Local',
          keepRemote: 'Keep Remote',
          dismiss: 'Dismiss',
          nextConflict: 'Next',
        },
      },
    },
  },
});

describe('ConflictResolveDialog', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it('does not render when no conflict exists', () => {
    const wrapper = mount(ConflictResolveDialog, {
      props: { conflict: null },
      global: { plugins: [i18n] },
    });
    // The dialog uses v-if="conflict", conflict is null by default
    expect(wrapper.find('h3').exists()).toBe(false);
  });
});
