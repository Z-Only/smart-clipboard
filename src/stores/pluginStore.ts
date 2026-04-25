import { defineStore } from 'pinia';
import { invoke } from '@tauri-apps/api/core';
import type { PluginListItem, PluginTransformAction } from '@/types';

interface PluginStoreState {
  plugins: PluginListItem[];
  transforms: PluginTransformAction[];
}

export const usePluginStore = defineStore('plugin', {
  state: (): PluginStoreState => ({
    plugins: [],
    transforms: [],
  }),
  actions: {
    async loadPlugins() {
      this.plugins = await invoke<PluginListItem[]>('list_plugins');
    },
    async setPluginEnabled(pluginId: string, enabled: boolean) {
      await invoke('set_plugin_enabled', { pluginId, enabled });
      this.plugins = this.plugins.map((plugin) =>
        plugin.id === pluginId ? { ...plugin, enabled } : plugin,
      );
    },
    async loadTransforms(content: string) {
      this.transforms = await invoke<PluginTransformAction[]>('list_plugin_transforms', {
        content,
      });
    },
  },
});
