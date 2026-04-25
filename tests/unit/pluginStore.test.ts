import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import type { PluginListItem, PluginTransformAction } from '@/types';

const invoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({ invoke }));

describe('usePluginStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    invoke.mockReset();
  });

  it('loads plugin list from backend with ui-facing plugin fields', async () => {
    const plugins: PluginListItem[] = [
      {
        id: 'builtin.uppercase',
        name: 'Uppercase',
        version: '1.0.0',
        description: 'Convert text to uppercase',
        kind: 'builtin',
        handler: 'transform',
        capabilities: ['transform'],
        enabled: true,
        valid: true,
        error: null,
      },
      {
        id: 'broken.plugin',
        name: 'Broken Plugin',
        version: '0.1.0',
        description: null,
        kind: 'external',
        handler: null,
        capabilities: [],
        enabled: false,
        valid: false,
        error: 'Failed to load manifest',
      },
    ];
    invoke.mockResolvedValue(plugins);

    const { usePluginStore } = await import('@/stores/pluginStore');
    const store = usePluginStore();

    await store.loadPlugins();

    expect(invoke).toHaveBeenCalledWith('list_plugins');
    expect(store.plugins).toEqual(plugins);
  });

  it('updates local state after enabling or disabling a plugin while preserving ui metadata', async () => {
    invoke.mockResolvedValue(undefined);

    const { usePluginStore } = await import('@/stores/pluginStore');
    const store = usePluginStore();
    store.plugins = [
      {
        id: 'builtin.uppercase',
        name: 'Uppercase',
        version: '1.0.0',
        description: 'Convert text to uppercase',
        kind: 'builtin',
        handler: 'transform',
        capabilities: ['transform'],
        enabled: false,
        valid: true,
        error: null,
      },
    ];

    await store.setPluginEnabled('builtin.uppercase', true);

    expect(invoke).toHaveBeenCalledWith('set_plugin_enabled', {
      pluginId: 'builtin.uppercase',
      enabled: true,
    });
    expect(store.plugins).toEqual([
      {
        id: 'builtin.uppercase',
        name: 'Uppercase',
        version: '1.0.0',
        description: 'Convert text to uppercase',
        kind: 'builtin',
        handler: 'transform',
        capabilities: ['transform'],
        enabled: true,
        valid: true,
        error: null,
      },
    ]);
  });

  it('loads transform actions using the camelCase frontend contract', async () => {
    const transforms: PluginTransformAction[] = [
      {
        pluginId: 'builtin.uppercase',
        pluginName: 'Uppercase',
        transformId: 'uppercase',
        label: 'Uppercase',
      },
      {
        pluginId: 'builtin.slugify',
        pluginName: 'Slugify',
        transformId: 'slugify',
        label: 'Slugify',
      },
    ];
    invoke.mockResolvedValue(transforms);

    const { usePluginStore } = await import('@/stores/pluginStore');
    const store = usePluginStore();

    await store.loadTransforms('Hello World');

    expect(invoke).toHaveBeenCalledWith('list_plugin_transforms', { content: 'Hello World' });
    expect(store.transforms).toEqual(transforms);
  });
});
