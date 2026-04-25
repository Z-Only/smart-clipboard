import { beforeEach, describe, expect, it, vi } from 'vitest';
import { mount, flushPromises } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import i18n from '@/i18n';
import type { PluginTransformAction } from '@/types';

const { invoke, pluginStoreMock, writeTextMock } = vi.hoisted(() => ({
  invoke: vi.fn(),
  pluginStoreMock: {
    transforms: [] as PluginTransformAction[],
    loadTransforms: vi.fn().mockResolvedValue(undefined),
  },
  writeTextMock: vi.fn().mockResolvedValue(undefined),
}));

vi.mock('@tauri-apps/api/core', () => ({ invoke }));
vi.mock('@/stores/pluginStore', () => ({
  usePluginStore: () => pluginStoreMock,
}));

import TransformMenu from '@/components/TransformMenu.vue';

describe('TransformMenu plugin transforms', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    invoke.mockReset();
    pluginStoreMock.loadTransforms.mockClear();
    pluginStoreMock.transforms = [
      {
        pluginId: 'builtin.uppercase',
        pluginName: 'Uppercase Tools',
        transformId: 'smartCase',
        label: 'Smart Case',
      },
      {
        pluginId: 'builtin.slugify',
        pluginName: 'Slugify',
        transformId: 'slugifyText',
        label: 'Slugify Text',
      },
    ];
    Object.defineProperty(globalThis, 'navigator', {
      value: {
        clipboard: {
          writeText: writeTextMock,
        },
      },
      configurable: true,
    });
  });

  it('loads and renders plugin transforms in a dedicated grouped section when the menu opens', async () => {
    const wrapper = mount(TransformMenu, {
      props: {
        content: 'Hello World',
        category: 'text',
      },
      global: { plugins: [createPinia(), i18n] },
    });

    await wrapper.get('button[title="Transform"]').trigger('click');
    await flushPromises();

    expect(pluginStoreMock.loadTransforms).toHaveBeenCalledWith('Hello World');
    expect(wrapper.text()).toContain('Plugins');
    expect(wrapper.text()).toContain('Uppercase Tools');
    expect(wrapper.text()).toContain('Smart Case');
    expect(wrapper.text()).toContain('Slugify');
    expect(
      wrapper.get('[data-test="plugin-transform-builtin.uppercase-smartCase"]').text(),
    ).toContain('Smart Case');
    expect(
      wrapper.get('[data-test="plugin-transform-builtin.slugify-slugifyText"]').text(),
    ).toContain('Slugify Text');
  });

  it('invokes apply_plugin_transform with camelCase plugin ids and the current content', async () => {
    invoke.mockResolvedValue('plugin output');

    const wrapper = mount(TransformMenu, {
      props: {
        content: 'Hello World',
        category: 'text',
      },
      global: { plugins: [createPinia(), i18n] },
    });

    await wrapper.get('button[title="Transform"]').trigger('click');
    await flushPromises();
    await wrapper
      .get('[data-test="plugin-transform-builtin.uppercase-smartCase"]')
      .trigger('click');
    await flushPromises();

    expect(invoke).toHaveBeenCalledWith('apply_plugin_transform', {
      pluginId: 'builtin.uppercase',
      transformId: 'smartCase',
      content: 'Hello World',
    });
    expect(writeTextMock).toHaveBeenCalledWith('plugin output');
  });
});
