import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';

const invoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({ invoke }));

describe('usePluginStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    invoke.mockReset();
  });

  it('loads plugin list from backend', async () => {
    const plugins = [
      {
        id: 'builtin.uppercase',
        name: 'Uppercase',
        description: 'Convert text to uppercase',
        version: '1.0.0',
        enabled: true,
      },
      {
        id: 'builtin.lowercase',
        name: 'Lowercase',
        description: 'Convert text to lowercase',
        version: '1.0.0',
        enabled: false,
      },
    ];
    invoke.mockResolvedValue(plugins);

    const { usePluginStore } = await import('@/stores/pluginStore');
    const store = usePluginStore();

    await store.loadPlugins();

    expect(invoke).toHaveBeenCalledWith('list_plugins');
    expect(store.plugins).toEqual(plugins);
  });

  it('updates local state after enabling or disabling a plugin', async () => {
    invoke.mockResolvedValue(undefined);

    const { usePluginStore } = await import('@/stores/pluginStore');
    const store = usePluginStore();
    store.plugins = [
      {
        id: 'builtin.uppercase',
        name: 'Uppercase',
        description: 'Convert text to uppercase',
        version: '1.0.0',
        enabled: false,
      },
    ];

    await store.setPluginEnabled('builtin.uppercase', true);

    expect(invoke).toHaveBeenCalledWith('set_plugin_enabled', {
      pluginId: 'builtin.uppercase',
      enabled: true,
    });
    expect(store.plugins[0].enabled).toBe(true);
  });

  it('loads transform actions for given content', async () => {
    const transforms = [
      {
        plugin_id: 'builtin.uppercase',
        action_id: 'uppercase',
        label: 'Uppercase',
        description: 'Convert selection to uppercase',
      },
      {
        plugin_id: 'builtin.slugify',
        action_id: 'slugify',
        label: 'Slugify',
        description: 'Convert selection to slug',
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
