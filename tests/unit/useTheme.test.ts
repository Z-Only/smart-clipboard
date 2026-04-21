import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { defineComponent, nextTick } from 'vue';
import { mount } from '@vue/test-utils';

function createMatchMedia(matches = false) {
  const listeners = new Set<(event: MediaQueryListEvent) => void>();
  return {
    matches,
    media: '(prefers-color-scheme: dark)',
    onchange: null,
    addEventListener: vi.fn((_: string, cb: (event: MediaQueryListEvent) => void) =>
      listeners.add(cb),
    ),
    removeEventListener: vi.fn((_: string, cb: (event: MediaQueryListEvent) => void) =>
      listeners.delete(cb),
    ),
    dispatch(nextMatches: boolean) {
      this.matches = nextMatches;
      const event = { matches: nextMatches, media: this.media } as MediaQueryListEvent;
      listeners.forEach((cb) => cb(event));
    },
  };
}

describe('useTheme', () => {
  beforeEach(() => {
    localStorage.clear();
    document.documentElement.className = '';
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('applies theme color and appearance changes', async () => {
    const matchMedia = createMatchMedia(false);
    vi.stubGlobal(
      'matchMedia',
      vi.fn(() => matchMedia),
    );

    const { useTheme } = await import('@/composables/useTheme');

    const TestComponent = defineComponent({
      template: '<div />',
      setup() {
        return useTheme();
      },
    });

    const wrapper = mount(TestComponent);
    await nextTick();

    expect(document.documentElement.classList.contains('theme-zinc')).toBe(true);
    expect(document.documentElement.classList.contains('dark')).toBe(false);

    wrapper.vm.setAppearance('dark');
    wrapper.vm.setThemeColor('violet');
    await nextTick();

    expect(localStorage.getItem('smart-clipboard-appearance')).toBe('dark');
    expect(localStorage.getItem('smart-clipboard-theme')).toBe('violet');
    expect(document.documentElement.classList.contains('dark')).toBe(true);
    expect(document.documentElement.classList.contains('theme-violet')).toBe(true);

    wrapper.unmount();
    expect(matchMedia.removeEventListener).toHaveBeenCalled();
  });
});
