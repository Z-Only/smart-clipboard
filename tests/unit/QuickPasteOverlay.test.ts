import { describe, expect, it } from 'vitest';
import { mount } from '@vue/test-utils';
import { createI18n } from 'vue-i18n';
import QuickPasteOverlay from '@/components/QuickPasteOverlay.vue';
import type { ClipboardEntry } from '@/types';

function makeEntry(overrides: Partial<ClipboardEntry> = {}): ClipboardEntry {
  return {
    id: 1,
    content: 'test content',
    content_type: 'text',
    category: 'text',
    hash: 'abc123',
    source_app: null,
    is_favorite: false,
    is_sensitive: false,
    use_count: 1,
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
    expires_at: null,
    ...overrides,
  };
}

const i18n = createI18n({
  legacy: false,
  locale: 'en',
  messages: {
    en: {
      quickPaste: {
        title: 'Quick Paste',
        dismiss: 'Esc to close',
        typeToSearch: 'Type to search...',
        empty: 'No recent entries',
        imageEntry: '[Image]',
      },
      entry: {
        justNow: 'just now',
        minutesAgo: '{n}m ago',
        hoursAgo: '{n}h ago',
        daysAgo: '{n}d ago',
      },
    },
  },
});

function mountOverlay(entries: ClipboardEntry[] = [], isActive = true) {
  return mount(QuickPasteOverlay, {
    props: { entries, isActive },
    global: { plugins: [i18n] },
    attachTo: document.body,
  });
}

describe('QuickPasteOverlay', () => {
  it('renders nothing when isActive is false', () => {
    const wrapper = mountOverlay([], false);
    expect(wrapper.find('.fixed').exists()).toBe(false);
  });

  it('renders entry list with number badges when active', () => {
    const entries = [
      makeEntry({ id: 1, content: 'First entry' }),
      makeEntry({ id: 2, content: 'Second entry' }),
      makeEntry({ id: 3, content: 'Third entry' }),
    ];
    const wrapper = mountOverlay(entries);
    const buttons = wrapper.findAll('button');
    expect(buttons).toHaveLength(3);
    expect(buttons[0].text()).toContain('1');
    expect(buttons[0].text()).toContain('First entry');
    expect(buttons[1].text()).toContain('2');
    expect(buttons[2].text()).toContain('3');
  });

  it('shows empty message when no entries', () => {
    const wrapper = mountOverlay([]);
    expect(wrapper.text()).toContain('No recent entries');
  });

  it('emits paste when number key 1 is pressed', async () => {
    const entries = [makeEntry({ id: 42, content: 'hello' })];
    const wrapper = mountOverlay(entries);
    await wrapper.find('.fixed').trigger('keydown', { key: '1' });
    expect(wrapper.emitted('paste')).toEqual([[42]]);
  });

  it('emits paste on Enter for active entry', async () => {
    const entries = [makeEntry({ id: 10 }), makeEntry({ id: 20 })];
    const wrapper = mountOverlay(entries);
    await wrapper.find('.fixed').trigger('keydown', { key: 'Enter' });
    expect(wrapper.emitted('paste')).toEqual([[10]]);
  });

  it('emits dismiss on Escape', async () => {
    const wrapper = mountOverlay([makeEntry()]);
    await wrapper.find('.fixed').trigger('keydown', { key: 'Escape' });
    expect(wrapper.emitted('dismiss')).toHaveLength(1);
  });

  it('emits search when a letter is typed', async () => {
    const wrapper = mountOverlay([makeEntry()]);
    await wrapper.find('.fixed').trigger('keydown', { key: 'a' });
    expect(wrapper.emitted('search')).toEqual([['a']]);
  });

  it('displays [Image] for image entries', () => {
    const entries = [
      makeEntry({
        id: 1,
        content: '/path/to/img.png',
        content_type: 'image',
        category: 'image',
      }),
    ];
    const wrapper = mountOverlay(entries);
    expect(wrapper.text()).toContain('[Image]');
  });

  it('navigates with arrow keys', async () => {
    const entries = [makeEntry({ id: 1 }), makeEntry({ id: 2 })];
    const wrapper = mountOverlay(entries);
    const overlay = wrapper.find('.fixed');

    await overlay.trigger('keydown', { key: 'ArrowDown' });
    await overlay.trigger('keydown', { key: 'Enter' });
    expect(wrapper.emitted('paste')).toEqual([[2]]);
  });
});
