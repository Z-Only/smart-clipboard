<template>
  <div class="space-y-3">
    <div class="space-y-1">
      <label class="text-sm font-medium">{{ $t('settings.plugins.title') }}</label>
      <p class="text-xs text-muted-foreground">{{ $t('settings.plugins.hint') }}</p>
    </div>

    <div v-if="plugins.length === 0" class="text-xs text-muted-foreground">
      {{ $t('settings.plugins.empty') }}
    </div>

    <div
      v-for="plugin in plugins"
      :key="plugin.id"
      class="rounded-md border border-input p-3 space-y-2"
    >
      <div class="flex items-start justify-between gap-3">
        <div class="min-w-0 space-y-1">
          <div class="text-sm font-medium break-words">{{ plugin.name }}</div>
          <div v-if="plugin.description" class="text-xs text-muted-foreground break-words">
            {{ plugin.description }}
          </div>
          <div class="flex flex-wrap gap-2 text-xs text-muted-foreground">
            <span>{{ $t('settings.plugins.id', { id: plugin.id }) }}</span>
            <span v-if="plugin.kind">{{ $t('settings.plugins.kind', { kind: plugin.kind }) }}</span>
            <span v-if="plugin.version">{{
              $t('settings.plugins.version', { version: plugin.version })
            }}</span>
          </div>
        </div>

        <button
          v-if="plugin.valid"
          :data-test="`plugin-toggle-${plugin.id}`"
          class="relative inline-flex h-5 w-9 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
          :class="plugin.enabled ? 'bg-primary' : 'bg-input'"
          @click="pluginStore.setPluginEnabled(plugin.id, !plugin.enabled)"
        >
          <span
            class="pointer-events-none block h-4 w-4 rounded-full bg-background shadow-lg ring-0 transition-transform"
            :class="plugin.enabled ? 'translate-x-4' : 'translate-x-0'"
          />
        </button>
      </div>

      <div
        v-if="!plugin.valid"
        class="rounded-md border border-destructive/30 bg-destructive/5 px-2 py-1 text-xs text-destructive"
      >
        <div class="font-medium">{{ $t('settings.plugins.invalid') }}</div>
        <div v-if="plugin.error">{{ plugin.error }}</div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import { usePluginStore } from '@/stores/pluginStore';

const pluginStore = usePluginStore();
const plugins = computed(() => pluginStore.plugins ?? []);
</script>
