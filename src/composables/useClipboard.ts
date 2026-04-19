import { onMounted, onUnmounted } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useClipboardStore } from "@/stores/clipboardStore";
import type { ClipboardEntry } from "@/types";

export function useClipboard() {
  const store = useClipboardStore();
  let unlisten: UnlistenFn | null = null;

  onMounted(async () => {
    unlisten = await listen<ClipboardEntry>("clipboard-changed", (event) => {
      store.onClipboardChanged(event.payload);
    });
  });

  onUnmounted(() => {
    unlisten?.();
  });
}
