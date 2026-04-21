import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { effectScope, ref } from 'vue';

const setSearch = vi.fn();

vi.mock('@/stores/clipboardStore', () => ({
  useClipboardStore: () => ({
    setSearch,
  }),
}));

describe('useSearch', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    setSearch.mockClear();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('updates store search keyword after debounce', async () => {
    const { useSearch } = await import('@/composables/useSearch');
    const keyword = ref('');
    const scope = effectScope();

    scope.run(() => {
      useSearch(keyword);
    });

    keyword.value = 'hello';
    await vi.advanceTimersByTimeAsync(300);

    expect(setSearch).toHaveBeenCalledWith('hello');
    scope.stop();
  });
});
