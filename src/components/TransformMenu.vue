<template>
  <div class="relative" ref="menuRef">
    <button
      class="p-1 rounded hover:bg-background/80 text-muted-foreground"
      @click.stop="toggleMenu"
      :title="t('transforms.title')"
    >
      <svg class="h-3.5 w-3.5" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"
        stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <path d="M7 16V4m0 0L3 8m4-4l4 4" />
        <path d="M17 8v12m0 0l4-4m-4 4l-4-4" />
      </svg>
    </button>
    <div
      v-if="isOpen"
      class="absolute right-0 top-full mt-1 z-50 min-w-[180px] rounded-md border bg-popover p-1 text-popover-foreground shadow-md"
    >
      <button
        v-for="item in availableTransforms"
        :key="item.type"
        class="flex w-full items-center rounded-sm px-2 py-1.5 text-xs hover:bg-accent hover:text-accent-foreground cursor-pointer"
        @click.stop="handleTransform(item.type)"
      >
        {{ t(item.labelKey) }}
      </button>
    </div>
    <div
      v-if="toastMessage"
      class="fixed bottom-4 left-1/2 -translate-x-1/2 z-[100] bg-foreground text-background text-xs px-3 py-1.5 rounded-md shadow-lg"
    >
      {{ toastMessage }}
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";

const { t } = useI18n();

const props = defineProps<{
  content: string;
  category: string;
}>();

const isOpen = ref(false);
const menuRef = ref<HTMLElement | null>(null);
const toastMessage = ref("");

interface TransformItem {
  type: string;
  labelKey: string;
  jsonOnly?: boolean;
}

const allTransforms: TransformItem[] = [
  { type: "uppercase", labelKey: "transforms.uppercase" },
  { type: "lowercase", labelKey: "transforms.lowercase" },
  { type: "title_case", labelKey: "transforms.titleCase" },
  { type: "url_encode", labelKey: "transforms.urlEncode" },
  { type: "url_decode", labelKey: "transforms.urlDecode" },
  { type: "json_format", labelKey: "transforms.jsonFormat", jsonOnly: true },
  { type: "json_compact", labelKey: "transforms.jsonCompact", jsonOnly: true },
  { type: "base64_encode", labelKey: "transforms.base64Encode" },
  { type: "base64_decode", labelKey: "transforms.base64Decode" },
  { type: "trim", labelKey: "transforms.trim" },
  { type: "html_escape", labelKey: "transforms.htmlEscape" },
  { type: "html_unescape", labelKey: "transforms.htmlUnescape" },
];

const availableTransforms = computed(() => {
  return allTransforms.filter((item) => {
    if (item.jsonOnly && props.category !== "json") return false;
    return true;
  });
});

function toggleMenu() {
  isOpen.value = !isOpen.value;
}

function closeMenu() {
  isOpen.value = false;
}

async function handleTransform(transformType: string) {
  try {
    const result = await invoke<string>("transform_content", {
      content: props.content,
      transformType: transformType,
    });
    await navigator.clipboard.writeText(result);
    showToast(t("transforms.copied"));
  } catch (err) {
    showToast(String(err));
  }
  closeMenu();
}

let toastTimer: ReturnType<typeof setTimeout> | null = null;

function showToast(message: string) {
  toastMessage.value = message;
  if (toastTimer) clearTimeout(toastTimer);
  toastTimer = setTimeout(() => {
    toastMessage.value = "";
  }, 2000);
}

function handleClickOutside(event: MouseEvent) {
  if (menuRef.value && !menuRef.value.contains(event.target as Node)) {
    closeMenu();
  }
}

onMounted(() => {
  document.addEventListener("click", handleClickOutside);
});

onUnmounted(() => {
  document.removeEventListener("click", handleClickOutside);
  if (toastTimer) clearTimeout(toastTimer);
});
</script>
