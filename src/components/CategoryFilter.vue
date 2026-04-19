<template>
  <div class="flex flex-col gap-1 py-2">
    <button
      v-for="cat in CATEGORIES"
      :key="cat.key"
      class="flex items-center gap-2 px-3 py-1.5 rounded-md text-sm transition-colors text-left"
      :class="[
        selectedCategory === cat.key
          ? 'bg-primary text-primary-foreground'
          : 'text-muted-foreground hover:bg-accent hover:text-accent-foreground',
      ]"
      @click="store.setCategory(cat.key)"
    >
      <span class="w-5 text-center text-xs">{{ cat.icon }}</span>
      <span class="truncate">{{ cat.label }}</span>
    </button>
  </div>
</template>

<script setup lang="ts">
import { storeToRefs } from "pinia";
import { useClipboardStore } from "@/stores/clipboardStore";
import { CATEGORIES } from "@/types";

const store = useClipboardStore();
const { selectedCategory } = storeToRefs(store);
</script>
