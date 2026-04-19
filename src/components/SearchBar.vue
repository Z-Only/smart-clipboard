<template>
  <div class="relative">
    <svg
      class="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground"
      xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"
      stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"
    >
      <circle cx="11" cy="11" r="8" />
      <path d="m21 21-4.3-4.3" />
    </svg>
    <Input
      ref="inputRef"
      v-model="searchText"
      placeholder="Search clipboard..."
      class="pl-9 pr-9 h-9"
      @keydown.escape="handleEscape"
    />
    <button
      v-if="searchText"
      class="absolute right-3 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
      @click="clearSearch"
    >
      <svg class="h-4 w-4" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"
        stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <path d="M18 6 6 18" /><path d="m6 6 12 12" />
      </svg>
    </button>
  </div>
</template>

<script setup lang="ts">
import { ref } from "vue";
import { Input } from "@/components/ui/input";
import { useSearch } from "@/composables/useSearch";

const searchText = ref("");
const inputRef = ref<InstanceType<typeof Input> | null>(null);

useSearch(searchText);

function clearSearch() {
  searchText.value = "";
}

function handleEscape() {
  if (searchText.value) {
    clearSearch();
  } else {
    (inputRef.value?.$el as HTMLInputElement)?.blur();
  }
}

function focus() {
  const el = inputRef.value?.$el as HTMLInputElement;
  el?.focus();
}

defineExpose({ focus });
</script>
