import { flushPromises, mount } from '@vue/test-utils';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
  clearEffects: vi.fn(async () => undefined),
  emit: vi.fn(async () => undefined),
  listen: vi.fn(async () => vi.fn()),
  onMoved: vi.fn(async () => vi.fn()),
  setAlwaysOnTop: vi.fn(async () => undefined),
  setEffects: vi.fn(async () => undefined),
  setResizable: vi.fn(async () => undefined),
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
      get length() {
        return values.size;
      },
    },
  });
});

vi.mock('@tauri-apps/api/core', () => ({ isTauri: () => true }));
vi.mock('@tauri-apps/api/event', () => ({ emit: mocks.emit, listen: mocks.listen }));
vi.mock('@tauri-apps/api/window', () => ({
  Effect: { Acrylic: 'acrylic' },
  getCurrentWindow: () => ({
    clearEffects: mocks.clearEffects,
    destroy: vi.fn(async () => undefined),
    onMoved: mocks.onMoved,
    setAlwaysOnTop: mocks.setAlwaysOnTop,
    setEffects: mocks.setEffects,
    setResizable: mocks.setResizable,
    startDragging: vi.fn(async () => undefined),
  }),
}));
vi.mock('../api', () => ({
  VrpianoApi: {
    getStatus: vi.fn(async () => ({
      running: false,
      paused: false,
      song_name: '',
      song_path: '',
      progress: 0,
      played_notes: 0,
      total_notes: 0,
      duration_ms: 0,
      elapsed_ms: 0,
      last_event: '',
      last_error: '',
      songs_dir: '',
      speed: 1,
      hotkeys_enabled: false,
      hotkeys_available: true,
      last_hotkey: '',
      last_hotkey_at_ms: 0,
    })),
    listSongs: vi.fn(async () => []),
  },
}));

import VrpianoOverlayView from './VrpianoOverlayView.vue';

describe('VrpianoOverlayView appearance controls', () => {
  beforeEach(() => {
    localStorage.clear();
    vi.clearAllMocks();
  });

  it('updates opacity and toggles the native backdrop effect', async () => {
    const wrapper = mount(VrpianoOverlayView);
    await flushPromises();

    expect(mocks.setEffects).toHaveBeenCalledWith({ effects: ['acrylic'] });

    await wrapper.get('button[title="外观设置"]').trigger('click');
    await wrapper.get('input[type="range"]').setValue('0.4');
    expect(wrapper.attributes('style')).toContain('--vrpiano-overlay-opacity: 0.4');

    await wrapper.get('input[type="checkbox"]').setValue(false);
    await flushPromises();
    expect(mocks.clearEffects).toHaveBeenCalled();

    wrapper.unmount();
  });
});
