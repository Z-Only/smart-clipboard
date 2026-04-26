import { describe, it, expect, vi } from 'vitest';
import { mount } from '@vue/test-utils';
import { createI18n } from 'vue-i18n';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));

import TemplateFillDialog from '@/components/TemplateFillDialog.vue';

const i18n = createI18n({
  legacy: false,
  locale: 'en',
  messages: {
    en: {
      templates: {
        fillPlaceholders: 'Fill Placeholders',
        preview: 'Preview',
        cancel: 'Cancel',
        copyToClipboard: 'Copy',
      },
    },
  },
});

const sampleTemplate = {
  id: 1,
  name: 'Greeting',
  content: 'Hello {{name}}, welcome to {{place}}',
  category: 'general',
  is_favorite: false,
  use_count: 0,
  created_at: '2026-04-21',
  updated_at: '2026-04-21',
};

describe('TemplateFillDialog', () => {
  it('renders title and placeholder inputs', () => {
    const wrapper = mount(TemplateFillDialog, {
      props: { template: sampleTemplate, placeholders: ['name', 'place'] },
      global: { plugins: [i18n] },
    });
    expect(wrapper.find('h3').text()).toBe('Fill Placeholders');
    const inputs = wrapper.findAll('input[type="text"]');
    expect(inputs).toHaveLength(2);
  });

  it('shows preview with unfilled placeholders', () => {
    const wrapper = mount(TemplateFillDialog, {
      props: { template: sampleTemplate, placeholders: ['name', 'place'] },
      global: { plugins: [i18n] },
    });
    expect(wrapper.text()).toContain('Hello {{name}}, welcome to {{place}}');
  });

  it('updates preview when values are entered', async () => {
    const wrapper = mount(TemplateFillDialog, {
      props: { template: sampleTemplate, placeholders: ['name', 'place'] },
      global: { plugins: [i18n] },
    });
    const inputs = wrapper.findAll('input[type="text"]');
    await inputs[0].setValue('Alice');
    await inputs[1].setValue('Wonderland');
    expect(wrapper.text()).toContain('Hello Alice, welcome to Wonderland');
  });

  it('emits submit with values when copy button is clicked', async () => {
    const wrapper = mount(TemplateFillDialog, {
      props: { template: sampleTemplate, placeholders: ['name', 'place'] },
      global: { plugins: [i18n] },
    });
    const inputs = wrapper.findAll('input[type="text"]');
    await inputs[0].setValue('Alice');
    await inputs[1].setValue('Wonderland');
    const copyBtn = wrapper.findAll('button').find((b) => b.text() === 'Copy');
    await copyBtn!.trigger('click');
    expect(wrapper.emitted('submit')).toBeTruthy();
    expect(wrapper.emitted('submit')![0]).toEqual([{ name: 'Alice', place: 'Wonderland' }]);
  });

  it('emits close when cancel button is clicked', async () => {
    const wrapper = mount(TemplateFillDialog, {
      props: { template: sampleTemplate, placeholders: ['name'] },
      global: { plugins: [i18n] },
    });
    const cancelBtn = wrapper.findAll('button').find((b) => b.text() === 'Cancel');
    await cancelBtn!.trigger('click');
    expect(wrapper.emitted('close')).toBeTruthy();
  });
});
