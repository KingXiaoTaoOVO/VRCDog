import { flushPromises, mount } from '@vue/test-utils';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
  getAudioDevices: vi.fn(async () => [
    { id: 'mic:1', index: 1, name: 'Test microphone', source: 'mic', is_default: true, sample_rate: 48000, channels: 1 },
    { id: 'speaker:2', index: 2, name: 'Test speakers [Loopback]', source: 'speaker', is_default: true, sample_rate: 48000, channels: 2 },
  ]),
  startAudioCapture: vi.fn(async () => undefined),
  stopAudioCapture: vi.fn(async () => undefined),
}));

vi.hoisted(() => {
  const values = new Map<string, string>();
  Object.defineProperty(globalThis, 'localStorage', {
    configurable: true,
    value: {
      getItem: (key: string) => values.get(key) ?? null,
      setItem: (key: string, value: string) => values.set(key, String(value)),
      removeItem: (key: string) => values.delete(key),
      clear: () => values.clear(),
      key: (index: number) => Array.from(values.keys())[index] ?? null,
      get length() { return values.size; },
    },
  });
});

vi.mock('@tauri-apps/api/core', () => ({ isTauri: () => true }));
vi.mock('@tauri-apps/api/event', () => ({
  emit: vi.fn(async () => undefined),
  listen: vi.fn(async () => vi.fn()),
}));
vi.mock('@tauri-apps/api/webviewWindow', () => ({
  WebviewWindow: class {
    static getByLabel = vi.fn(async () => null);
    once = vi.fn();
    onCloseRequested = vi.fn();
  },
}));
vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (key: string) => key,
    locale: { value: 'zh-CN' },
  }),
}));
vi.mock('../api', () => ({
  SysApi: {
    getAudioDevices: mocks.getAudioDevices,
    getAudioCaptureStatus: vi.fn(async () => []),
    startAudioCapture: mocks.startAudioCapture,
    stopAudioCapture: mocks.stopAudioCapture,
    setAudioCapturePaused: vi.fn(async () => undefined),
    synthesizeGptSovits: vi.fn(),
    sendOscChatbox: vi.fn(),
  },
  VrctApi: {
    getHistory: vi.fn(async () => []),
    processMessage: vi.fn(),
    clearHistory: vi.fn(),
  },
}));

import TranslatorView from './TranslatorView.vue';

describe('TranslatorView audio capture', () => {
  beforeEach(() => {
    localStorage.clear();
    mocks.getAudioDevices.mockClear();
    mocks.startAudioCapture.mockClear();
    mocks.stopAudioCapture.mockClear();
  });

  it('loads real device choices and starts the selected microphone worker', async () => {
    const wrapper = mount(TranslatorView);
    await flushPromises();

    expect(mocks.getAudioDevices).toHaveBeenCalledOnce();
    expect(wrapper.text()).toContain('Test microphone');
    expect(wrapper.text()).toContain('Test speakers [Loopback]');

    const startButton = wrapper.get('[data-testid="start-microphone"]');
    await startButton.trigger('click');
    await flushPromises();

    expect(mocks.startAudioCapture).toHaveBeenCalledWith(expect.objectContaining({
      source: 'mic',
      sourceLang: 'zh-CN',
      engine: 'cloud',
      deviceIndex: 1,
    }));

    wrapper.unmount();
  });
});
