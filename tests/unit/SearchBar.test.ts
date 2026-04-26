import { describe, it, expect, vi, beforeEach } from 'vitest';
import { mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { createI18n } from 'vue-i18n';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
vi.mock('@/composables/useSearch', () => ({ useSearch: vi.fn() }));
vi.mock('@/components/ui/input', async () => {
  const { defineComponent, h } = await import('vue');
  return {
    Input: defineComponent({
      name: 'InputStub',
      props: ['modelValue', 'placeholder'],
      emits: ['update:modelValue'],
      setup(props, { emit, expose }) {
        const el = null;
        expose({ $el: el });
        return () =>
          h('input', {
            value: props.modelValue,
            placeholder: props.placeholder,
            onInput: (e: Event) => emit('update:modelValue', (e.target as HTMLInputElement).value),
          });
      },
    }),
  };
});

import SearchBar from '@/components/SearchBar.vue';

const i18n = createI18n({
  legacy: false,
  locale: 'en',
  messages: { en: { search: { placeholder: 'Search...' } } },
});

describe('SearchBar', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it('renders search input', () => {
    const wrapper = mount(SearchBar, { global: { plugins: [i18n] } });
    expect(wrapper.find('input').exists()).toBe(true);
  });

  it('shows clear button when search text is not empty', async () => {
    const wrapper = mount(SearchBar, { global: { plugins: [i18n] } });
    expect(wrapper.find('button').exists()).toBe(false);
    await wrapper.find('input').setValue('test');
    expect(wrapper.find('button').exists()).toBe(true);
  });

  it('clears search text when clear button is clicked', async () => {
    const wrapper = mount(SearchBar, { global: { plugins: [i18n] } });
    await wrapper.find('input').setValue('test');
    await wrapper.find('button').trigger('click');
    expect((wrapper.find('input').element as HTMLInputElement).value).toBe('');
  });
});
