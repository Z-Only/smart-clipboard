import type { Ref } from 'vue';
import { watchDebounced } from '@vueuse/core';
import { useClipboardStore } from '@/stores/clipboardStore';

export function useSearch(keyword: Ref<string>) {
  const store = useClipboardStore();

  watchDebounced(
    keyword,
    (val) => {
      store.setSearch(val);
    },
    { debounce: 300 },
  );
}
