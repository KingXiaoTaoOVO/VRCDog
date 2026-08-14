import { flushPromises, mount } from '@vue/test-utils';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => {
  const listeners = new Map<string, (event: { payload: any }) => void>();
  const status = {
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
  };
  return {
    listeners,
    listen: vi.fn(async (event: string, handler: (event: { payload: any }) => void) => {
      listeners.set(event, handler);
      return vi.fn();
    }),
    emit: vi.fn(async () => undefined),
    readSongData: vi.fn(async () => {
      throw new Error('test read');
    }),
    api: {
      init: vi.fn(async () => ({ ...status })),
      listSongs: vi.fn(async () => [{ id: 'song-1', name: 'Test Song', path: 'C:/songs/test.mid', size: 10, modified_ms: 1 }]),
      midishowAccounts: vi.fn(async () => []),
      midishowLoginStatus: vi.fn(async () => ({ state: 'idle', message: '', username: null })),
      readSongData: vi.fn(async () => {
        throw new Error('test read');
      }),
      getStatus: vi.fn(async () => ({ ...status })),
    },
  };
});

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

vi.mock('@tauri-apps/api/core', () => ({
  convertFileSrc: (path: string) => path,
  isTauri: () => false,
}));
vi.mock('@tauri-apps/api/event', () => ({ emit: mocks.emit, listen: mocks.listen }));
vi.mock('@tauri-apps/api/webviewWindow', () => ({
  WebviewWindow: { getByLabel: vi.fn(async () => null) },
}));
vi.mock('@tauri-apps/api/window', () => ({ Effect: { Acrylic: 'acrylic' } }));
vi.mock('@tauri-apps/plugin-dialog', () => ({ open: vi.fn(async () => null) }));
vi.mock('vue-i18n', () => ({ useI18n: () => ({ locale: { value: 'zh-CN' } }) }));
vi.mock('../audio/generalMidi', () => ({
  GENERAL_MIDI_GROUPS: [],
  GeneralMidiSynth: class {},
  getGeneralMidiInstrumentName: () => 'Piano',
  parseGeneralMidi: vi.fn(),
}));
vi.mock('../api', () => ({ VrpianoApi: { ...mocks.api, readSongData: mocks.readSongData }, SysApi: { openUrl: vi.fn() } }));

import VrpianoView from './VrpianoView.vue';
import { VRPIANO_PREVIEW_SONG_EVENT } from './vrpianoEvents';

describe('VrpianoView preview routing', () => {
  beforeEach(() => {
    localStorage.clear();
    mocks.listeners.clear();
    mocks.listen.mockClear();
    mocks.readSongData.mockClear();
  });

  it('routes the overlay preview event to the built-in player loader', async () => {
    const wrapper = mount(VrpianoView);
    await flushPromises();

    const onPreview = mocks.listeners.get(VRPIANO_PREVIEW_SONG_EVENT);
    expect(onPreview).toBeTypeOf('function');
    onPreview?.({ payload: { songPath: 'C:/songs/test.mid' } });
    await flushPromises();

    expect(mocks.readSongData).toHaveBeenCalledWith({ songPath: 'C:/songs/test.mid' });
    wrapper.unmount();
  });
});
