import { effectScope, ref, watch } from 'vue';
import { convertFileSrc } from '@tauri-apps/api/core';
import type { VrpianoSong } from '../api';

/**
 * Shared VRPiano per-song icon state + helpers.
 *
 * `songIcons` is a module-level singleton ref so the main VRPiano view
 * and the floating overlay view render the EXACT same icon for each
 * song (1:1 match). Persistence to localStorage is also centralized
 * here so the two surfaces never drift.
 *
 * Resolution hierarchy (mirrors VrpianoView's render):
 *   1. Custom icon set by the user (image data/url or emoji/text)
 *   2. Cover fetched from Midishow (song.cover_path -> asset://)
 *   3. Default Music glyph fallback
 */
const ICON_STORAGE_KEY = 'vrcdog.vrpiano.songIcons.v1';

const songIcons = ref<Record<string, string>>({});

// Load once at module init so both views start with the same data.
try {
  if (typeof localStorage !== 'undefined') {
    const raw = localStorage.getItem(ICON_STORAGE_KEY);
    if (raw) songIcons.value = JSON.parse(raw);
  }
} catch {
  /* ignore corrupted state */
}

// Auto-persist on any change. Wrapped in a detached effectScope so the
// watcher survives for the whole app lifetime without parent-scope
// warnings (called at module top level, outside any component).
effectScope(true).run(() => {
  watch(
    songIcons,
    (next) => {
      try {
        if (typeof localStorage !== 'undefined') {
          localStorage.setItem(ICON_STORAGE_KEY, JSON.stringify(next));
        }
      } catch {
        /* quota or disabled storage */
      }
    },
    { deep: true },
  );
});

export function useVrpianoIcons() {
  return { songIcons };
}

export function songIcon(song: VrpianoSong): string {
  return songIcons.value[song.path] || '';
}

export function isImageIcon(value: string): boolean {
  return /^(data:image\/|https?:\/\/|blob:)/i.test(value);
}

export function songCover(song: VrpianoSong): string {
  const cover = song.cover_path;
  if (!cover) return '';
  try {
    return convertFileSrc(cover);
  } catch {
    return '';
  }
}