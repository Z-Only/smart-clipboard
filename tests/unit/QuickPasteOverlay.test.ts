import { afterEach, describe, expect, it } from 'vitest';
import { mount, type VueWrapper } from '@vue/test-utils';
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

/**
 * Because QuickPasteOverlay uses <Teleport to="body">, the teleported
 * content lives outside the wrapper's DOM tree. We query the body directly
 * for assertions and event dispatching.
 */
function queryTeleported(selector: string): HTMLElement | null {
  return document.body.querySelector(selector);
}

function queryAllTeleported(selector: string): NodeListOf<HTMLElement> {
  return document.body.querySelectorAll(selector);
}

const mountedWrappers: VueWrapper[] = [];

function mountOverlay(entries: ClipboardEntry[] = [], isActive = true) {
  const wrapper = mount(QuickPasteOverlay, {
    props: { entries, isActive },
    global: { plugins: [i18n] },
    attachTo: document.body,
  });
  mountedWrappers.push(wrapper);
  return wrapper;
}

describe('QuickPasteOverlay', () => {
  afterEach(() => {
    while (mountedWrappers.length > 0) {
      mountedWrappers.pop()?.unmount();
    }
  });

  it('renders nothing when isActive is false', () => {
    mountOverlay([], false);
    expect(queryTeleported('.fixed')).toBeNull();
  });

  it('renders entry list with number badges when active', () => {
    const entries = [
      makeEntry({ id: 1, content: 'First entry' }),
      makeEntry({ id: 2, content: 'Second entry' }),
      makeEntry({ id: 3, content: 'Third entry' }),
    ];
    mountOverlay(entries);
    const buttons = queryAllTeleported('button');
    expect(buttons).toHaveLength(3);
    expect(buttons[0].textContent).toContain('1');
    expect(buttons[0].textContent).toContain('First entry');
    expect(buttons[1].textContent).toContain('2');
    expect(buttons[2].textContent).toContain('3');
  });

  it('shows empty message when no entries', () => {
    mountOverlay([]);
    const overlay = queryTeleported('.fixed');
    expect(overlay).not.toBeNull();
    expect(overlay!.textContent).toContain('No recent entries');
  });

  it('emits paste when number key 1 is pressed', async () => {
    const entries = [makeEntry({ id: 42, content: 'hello' })];
    const wrapper = mountOverlay(entries);
    const overlay = queryTeleported('.fixed')!;
    overlay.dispatchEvent(new KeyboardEvent('keydown', { key: '1', bubbles: true }));
    await wrapper.vm.$nextTick();
    expect(wrapper.emitted('paste')).toEqual([[42]]);
  });

  it('emits paste on Enter for active entry', async () => {
    const entries = [makeEntry({ id: 10 }), makeEntry({ id: 20 })];
    const wrapper = mountOverlay(entries);
    const overlay = queryTeleported('.fixed')!;
    overlay.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
    await wrapper.vm.$nextTick();
    expect(wrapper.emitted('paste')).toEqual([[10]]);
  });

  it('emits dismiss on Escape', async () => {
    const wrapper = mountOverlay([makeEntry()]);
    const overlay = queryTeleported('.fixed')!;
    overlay.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    await wrapper.vm.$nextTick();
    expect(wrapper.emitted('dismiss')).toHaveLength(1);
  });

  it('emits search when a letter is typed', async () => {
    const wrapper = mountOverlay([makeEntry()]);
    const overlay = queryTeleported('.fixed')!;
    overlay.dispatchEvent(new KeyboardEvent('keydown', { key: 'a', bubbles: true }));
    await wrapper.vm.$nextTick();
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
    mountOverlay(entries);
    const overlay = queryTeleported('.fixed');
    expect(overlay).not.toBeNull();
    expect(overlay!.textContent).toContain('[Image]');
  });

  it('navigates with arrow keys', async () => {
    const entries = [makeEntry({ id: 1 }), makeEntry({ id: 2 })];
    const wrapper = mountOverlay(entries);
    const overlay = queryTeleported('.fixed')!;

    overlay.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowDown', bubbles: true }));
    await wrapper.vm.$nextTick();
    overlay.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
    await wrapper.vm.$nextTick();
    expect(wrapper.emitted('paste')).toEqual([[2]]);
  });
});
