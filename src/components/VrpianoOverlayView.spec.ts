import { flushPromises, mount } from '@vue/test-utils';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
  clearEffects: vi.fn(async () => undefined),
  emit: vi.fn(async () => undefined),
  listeners: new Map<string, (event: { payload: any }) => void>(),
  listen: vi.fn(async (event: string, handler: (event: { payload: any }) => void) => {
    mocks.listeners.set(event, handler);
    return vi.fn();
  }),
  onMoved: vi.fn(async () => vi.fn()),
  setAlwaysOnTop: vi.fn(async () => undefined),
  setEffects: vi.fn(async () => undefined),
  setResizable: vi.fn(async () => undefined),
  getStatus: vi.fn(),
  listSongs: vi.fn(),
  start: vi.fn(),
  stop: vi.fn(),
  togglePause: vi.fn(),
  previewSong: vi.fn(),
  status: {} as any,
  songs: [] as any[],
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
    getStatus: mocks.getStatus,
    listSongs: mocks.listSongs,
    start: mocks.start,
    stop: mocks.stop,
    togglePause: mocks.togglePause,
    previewSong: mocks.previewSong,
  },
}));

import VrpianoOverlayView from './VrpianoOverlayView.vue';
import { VRPIANO_PREVIEW_SONG_EVENT } from './vrpianoEvents';

describe('VrpianoOverlayView appearance controls', () => {
  afterEach(() => {
    vi.useRealTimers();
  });

  beforeEach(() => {
    localStorage.clear();
    vi.clearAllMocks();
    mocks.listeners.clear();
    mocks.status = {
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
      hotkeys_enabled: true,
      hotkeys_available: true,
      last_hotkey: '',
      last_hotkey_at_ms: 0,
    };
    mocks.songs = [];
    mocks.getStatus.mockImplementation(async () => ({ ...mocks.status }));
    mocks.listSongs.mockImplementation(async () => [...mocks.songs]);
    mocks.start.mockImplementation(async ({ songPath }: { songPath: string }) => {
      mocks.status = { ...mocks.status, running: true, paused: false, song_name: 'Test Song', song_path: songPath };
      return { ...mocks.status };
    });
    mocks.stop.mockImplementation(async () => {
      mocks.status = { ...mocks.status, running: false, paused: false };
      return { ...mocks.status };
    });
    mocks.togglePause.mockImplementation(async () => {
      mocks.status = { ...mocks.status, paused: !mocks.status.paused };
      return { ...mocks.status };
    });
    mocks.previewSong.mockImplementation(async () => undefined);
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

  it('does not reactivate the same hotkey event on every status poll', async () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date('2026-08-14T05:00:00Z'));
    const wrapper = mount(VrpianoOverlayView);
    await flushPromises();

    const onStatus = mocks.listeners.get('vrpiano_status');
    expect(onStatus).toBeTypeOf('function');
    const hotkeyStatus = { ...mocks.status, last_hotkey: 'F1', last_hotkey_at_ms: Date.now() };
    onStatus?.({ payload: hotkeyStatus });
    await wrapper.vm.$nextTick();
    expect(wrapper.get('.hotkey-list span').classes()).toContain('pressed');

    await vi.advanceTimersByTimeAsync(900);
    expect(wrapper.get('.hotkey-list span').classes()).not.toContain('pressed');
    onStatus?.({ payload: hotkeyStatus });
    await wrapper.vm.$nextTick();
    expect(wrapper.get('.hotkey-list span').classes()).not.toContain('pressed');

    wrapper.unmount();
  });

  it('uses one start-pause-resume button and reveals restart only after starting', async () => {
    mocks.songs = [{ id: 'song-1', name: 'Test Song', path: 'C:/songs/test.mid', size: 10, modified_ms: 1 }];
    const wrapper = mount(VrpianoOverlayView);
    await flushPromises();

    expect(wrapper.find('button[title="重新开始"]').exists()).toBe(false);
    const primary = wrapper.get('.play-button');
    expect(primary.text()).toContain('开始');

    await primary.trigger('click');
    await flushPromises();
    expect(mocks.start).toHaveBeenCalledOnce();
    expect(primary.text()).toContain('暂停');
    expect(wrapper.find('button[title="重新开始"]').exists()).toBe(true);

    await primary.trigger('click');
    await flushPromises();
    expect(mocks.togglePause).toHaveBeenCalledOnce();
    expect(primary.text()).toContain('继续');

    wrapper.unmount();
  });

  it('previews a song on double click while the preview switch is enabled', async () => {
    mocks.songs = [{ id: 'song-1', name: 'Test Song', path: 'C:/songs/test.mid', size: 10, modified_ms: 1 }];
    const wrapper = mount(VrpianoOverlayView);
    await flushPromises();

    const songButton = wrapper.get('.playlist-scroll button');
    expect(wrapper.get('[data-testid="preview-toggle"]').text()).toContain('双击试听');
    await songButton.trigger('dblclick');
    await flushPromises();
    expect(mocks.emit).toHaveBeenCalledWith(VRPIANO_PREVIEW_SONG_EVENT, { songPath: 'C:/songs/test.mid' });
    expect(mocks.previewSong).not.toHaveBeenCalled();

    await wrapper.get('[data-testid="preview-toggle"]').trigger('click');
    expect(wrapper.get('[data-testid="preview-toggle"]').text()).toContain('试听关闭');
    mocks.previewSong.mockClear();
    mocks.emit.mockClear();
    await songButton.trigger('dblclick');
    await flushPromises();
    expect(mocks.previewSong).not.toHaveBeenCalled();
    expect(mocks.emit).not.toHaveBeenCalledWith(VRPIANO_PREVIEW_SONG_EVENT, { songPath: 'C:/songs/test.mid' });

    wrapper.unmount();
  });
});
