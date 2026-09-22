<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { isTauri } from '@tauri-apps/api/core';
import { emit, listen, type UnlistenFn } from '@tauri-apps/api/event';
import { Effect, EffectState, getCurrentWindow } from '@tauri-apps/api/window';
import { useStorage } from '@vueuse/core';
import {
  ChevronLeft,
  ChevronRight,
  Headphones,
  Keyboard,
  ListMusic,
  Music2,
  Pause,
  Pin,
  PinOff,
  Play,
  RotateCcw,
  Settings2,
  X,
} from 'lucide-vue-next';
import { VrpianoApi, type VrpianoSong, type VrpianoStatus } from '../api';
import {
  createVrpianoOverlayPanelStyle,
  DEFAULT_VRPIANO_OVERLAY_BLUR,
  DEFAULT_VRPIANO_OVERLAY_OPACITY,
  isVrpianoOverlayBlurEnabled,
  normalizeVrpianoOverlayBlur,
  normalizeVrpianoOverlayOpacity,
  VRPIANO_OVERLAY_BLUR_KEY,
  VRPIANO_OVERLAY_OPACITY_KEY,
} from './vrpianoOverlayAppearance';
import { VRPIANO_PREVIEW_SONG_EVENT } from './vrpianoEvents';
import { songIcon, songCover, isImageIcon } from '../composables/useVrpianoIcons';

const { t } = useI18n();

const emptyStatus = (): VrpianoStatus => ({
  running: false,
  paused: false,
  song_name: '',
  song_path: '',
  progress: 0,
  played_notes: 0,
  total_notes: 0,
  duration_ms: 0,
  elapsed_ms: 0,
  last_event: 'VRPiano ready',
  last_error: '',
  songs_dir: '',
  speed: 1,
  hotkeys_enabled: false,
  hotkeys_available: true,
  last_hotkey: '',
  last_hotkey_at_ms: 0,
  midi_connected: false,
  midi_device_name: null,
  recording: false,
  recorded_midi_path: null,
  channels: Array.from({ length: 16 }, () => ({ muted: false, solo: false, volume: 127 })),
  voice_listening: false,
  tts_enabled: false,
  last_transcription: '',
  vrchat_osc_enabled: false,
  vrchat_osc_host: '',
  vrchat_osc_port: 9000,
  vrchat_osc_running: false,
  vrchat_osc_connected: false,
  vrchat_osc_last_error: '',
});

const status = ref<VrpianoStatus>(emptyStatus());
const songs = ref<VrpianoSong[]>([]);
const busy = ref(false);
const error = ref('');
const settingsOpen = ref(false);
const recentHotkey = ref('');
const overlayOpacity = useStorage(VRPIANO_OVERLAY_OPACITY_KEY, DEFAULT_VRPIANO_OVERLAY_OPACITY);
const overlayBlur = useStorage(VRPIANO_OVERLAY_BLUR_KEY, DEFAULT_VRPIANO_OVERLAY_BLUR);
const positionLocked = useStorage('vrcdog.vrpiano.overlay.locked', false);
const previewEnabled = useStorage('vrcdog.vrpiano.overlay.preview-enabled', false);
const previewingPath = ref('');
const outputMode = useStorage<'keyboard' | 'midi' | 'osc'>('vrcdog.vrpiano.outputMode.v1', 'keyboard');
const selectedMidiDevice = useStorage('vrcdog.vrpiano.selectedMidiDevice.v1', '');
const vrchatOscHost = useStorage('vrcdog.vrpiano.vrchatOscHost.v1', '127.0.0.1');
const vrchatOscPort = useStorage('vrcdog.vrpiano.vrchatOscPort.v1', 9000);
const vrchatOscMode = useStorage<'piano' | 'avatar'>('vrcdog.vrpiano.oscMode.v1', 'piano');
const vrchatOscAvatarPrefix = useStorage('vrcdog.vrpiano.oscAvatarPrefix.v1', '/avatar/parameters/note');
const hotkeysEnabled = useStorage('vrcdog.vrpiano.hotkeysEnabled.v1', true);

const modeBadgeText = computed(() => {
  if (outputMode.value === 'osc') return t('vrpiano_overlay.mode_osc_contactless');
  if (outputMode.value === 'midi') return 'MIDI';
  return t('vrpiano_overlay.mode_keyboard');
});

const cycleOutputMode = async () => {
  const modes: Array<'keyboard' | 'midi' | 'osc'> = ['keyboard', 'midi', 'osc'];
  const currentIndex = modes.indexOf(outputMode.value);
  outputMode.value = modes[(currentIndex + 1) % modes.length];
};

let pollTimer: number | null = null;
let hotkeyTimer: number | null = null;
let unlistenStatus: UnlistenFn | null = null;
let unlistenClose: UnlistenFn | null = null;
let unlistenMoved: UnlistenFn | null = null;
let unlistenFocus: UnlistenFn | null = null;
let nativeBackdropEnabled: boolean | null = null;
let appearanceRefreshTimer: number | null = null;
let lastHandledHotkeyEvent = '';
let songClickTimer: number | null = null;

const panelStyle = computed(() => createVrpianoOverlayPanelStyle(overlayOpacity.value, overlayBlur.value));
const blurEnabled = computed({
  get: () => isVrpianoOverlayBlurEnabled(overlayBlur.value),
  set: (enabled: boolean) => {
    overlayBlur.value = enabled ? DEFAULT_VRPIANO_OVERLAY_BLUR : 0;
  },
});

const syncNativeBackdrop = async (value: unknown, force = false) => {
  if (!isTauri()) return;
  const enabled = isVrpianoOverlayBlurEnabled(value);
  if (!force && nativeBackdropEnabled === enabled) return;

  const appWindow = getCurrentWindow();
  if (enabled) {
    await appWindow.setEffects({
      effects: [Effect.Acrylic],
      state: EffectState.Active,
    });
  }
  else await appWindow.clearEffects();
  nativeBackdropEnabled = enabled;
};

const reapplySavedAppearance = () => {
  const opacity = normalizeVrpianoOverlayOpacity(overlayOpacity.value);
  const blur = normalizeVrpianoOverlayBlur(overlayBlur.value);
  overlayOpacity.value = opacity;
  overlayBlur.value = blur;

  if (!isTauri()) return;
  if (appearanceRefreshTimer !== null) window.clearTimeout(appearanceRefreshTimer);
  appearanceRefreshTimer = window.setTimeout(() => {
    appearanceRefreshTimer = null;
    void syncNativeBackdrop(blur, true).catch(() => {
      nativeBackdropEnabled = null;
    });
  }, 0);
};

const handleVisibilityChange = () => reapplySavedAppearance();

const progress = computed(() => Math.min(1, Math.max(0, Number(status.value.progress) || 0)));
const currentIndex = computed(() => {
  const byPath = songs.value.findIndex((song) => song.path === status.value.song_path);
  if (byPath >= 0) return byPath;
  return songs.value.findIndex((song) => song.name === status.value.song_name);
});
const currentSong = computed(() => songs.value[currentIndex.value] || songs.value[0] || null);
const hasStartedPlayback = computed(() => Boolean(status.value.song_path));
const primaryPlaybackLabel = computed(() => {
  if (status.value.paused) return t('vrpiano_overlay.resume');
  if (status.value.running) return t('vrpiano_overlay.pause');
  return t('vrpiano_overlay.start');
});
const playbackLabel = computed(() => {
  if (status.value.paused) return t('vrpiano_overlay.paused');
  if (status.value.running) return t('vrpiano_overlay.playing');
  return t('vrpiano_overlay.idle');
});
const progressStyle = computed(() => ({ width: `${Math.round(progress.value * 10000) / 100}%` }));
const hotkeys = computed(() => [
  { key: 'F1', label: primaryPlaybackLabel.value },
  ...(hasStartedPlayback.value ? [{ key: 'F2', label: t('vrpiano_overlay.restart') }] : []),
  { key: 'F3', label: t('vrpiano_overlay.speed_up') },
  { key: 'F4', label: t('vrpiano_overlay.speed_down') },
  { key: 'F5', label: t('vrpiano_overlay.default') },
]);

const formatTime = (ms: number) => {
  const seconds = Math.max(0, Math.round((Number(ms) || 0) / 1000));
  return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, '0')}`;
};

const applyStatus = (next: VrpianoStatus) => {
  status.value = next;
  if (next.last_hotkey && next.last_hotkey_at_ms) {
    const eventId = `${next.last_hotkey}:${next.last_hotkey_at_ms}`;
    if (eventId === lastHandledHotkeyEvent) return;
    lastHandledHotkeyEvent = eventId;

    const eventAge = Math.max(0, Date.now() - next.last_hotkey_at_ms);
    if (eventAge >= 850) return;
    recentHotkey.value = next.last_hotkey;
    if (hotkeyTimer !== null) window.clearTimeout(hotkeyTimer);
    hotkeyTimer = window.setTimeout(() => {
      recentHotkey.value = '';
      hotkeyTimer = null;
    }, 850 - eventAge);
  }
};

const refresh = async () => {
  try {
    applyStatus(await VrpianoApi.getStatus());
    error.value = status.value.last_error || '';
  } catch (cause) {
    error.value = String(cause);
  }
};

const waitUntilStopped = async () => {
  for (let attempt = 0; attempt < 30; attempt += 1) {
    const next = await VrpianoApi.getStatus();
    applyStatus(next);
    if (!next.running) return;
    await new Promise((resolve) => window.setTimeout(resolve, 40));
  }
  throw new Error(t('vrpiano_overlay.wait_stop_timeout'));
};

const playSong = async (song: VrpianoSong) => {
  if (busy.value) return;
  busy.value = true;
  error.value = '';
  previewingPath.value = '';
  try {
    if (status.value.running) {
      await VrpianoApi.stop();
      await waitUntilStopped();
    }
    const currentSpeed = status.value.speed || 1;
    if (outputMode.value === 'osc') {
      applyStatus(await VrpianoApi.startVrchatOsc({
        songPath: song.path,
        delaySecs: 0,
        speed: currentSpeed,
        host: vrchatOscHost.value,
        port: vrchatOscPort.value,
        mode: vrchatOscMode.value,
        avatarPrefix: vrchatOscAvatarPrefix.value,
      }));
    } else if (outputMode.value === 'midi') {
      applyStatus(await VrpianoApi.start({
        songPath: song.path,
        delaySecs: 0,
        speed: currentSpeed,
        outputMode: 'midi',
        midiDeviceId: selectedMidiDevice.value || undefined,
      }));
    } else {
      applyStatus(await VrpianoApi.start({
        songPath: song.path,
        delaySecs: 0,
        speed: currentSpeed,
        outputMode: 'keyboard',
      }));
    }
  } catch (cause) {
    error.value = cause instanceof Error ? cause.message : String(cause);
  } finally {
    busy.value = false;
  }
};

const togglePlayback = async () => {
  if (busy.value) return;
  if (!status.value.running) {
    if (currentSong.value) await playSong(currentSong.value);
    return;
  }
  busy.value = true;
  try {
    applyStatus(await VrpianoApi.togglePause());
  } catch (cause) {
    error.value = cause instanceof Error ? cause.message : String(cause);
  } finally {
    busy.value = false;
  }
};

const moveSong = async (direction: -1 | 1) => {
  if (!songs.value.length) return;
  const base = currentIndex.value >= 0 ? currentIndex.value : 0;
  const index = (base + direction + songs.value.length) % songs.value.length;
  await playSong(songs.value[index]);
};

const restartSong = async () => {
  if (currentSong.value) await playSong(currentSong.value);
};

const previewSong = async (song: VrpianoSong) => {
  if (!previewEnabled.value || busy.value) return;
  if (songClickTimer !== null) {
    window.clearTimeout(songClickTimer);
    songClickTimer = null;
  }
  previewingPath.value = song.path;
  error.value = '';
  try {
    await emit(VRPIANO_PREVIEW_SONG_EVENT, { songPath: song.path });
  } catch (cause) {
    error.value = cause instanceof Error ? cause.message : String(cause);
  }
};

const handleSongClick = (song: VrpianoSong) => {
  if (songClickTimer !== null) window.clearTimeout(songClickTimer);
  if (previewEnabled.value) {
    songClickTimer = window.setTimeout(() => {
      songClickTimer = null;
      void previewSong(song);
    }, 220);
  } else {
    previewingPath.value = '';
    void playSong(song);
  }
};

const handleSongDblClick = (song: VrpianoSong) => {
  if (songClickTimer !== null) {
    window.clearTimeout(songClickTimer);
    songClickTimer = null;
  }
  previewingPath.value = '';
  void playSong(song);
};

const toggleHotkeys = async () => {
  const nextEnabled = !status.value.hotkeys_enabled;
  hotkeysEnabled.value = nextEnabled;
  try {
    const targetSongPath = currentSong.value?.path || status.value.song_path || songs.value[0]?.path || '';
    applyStatus(await VrpianoApi.setHotkeys({
      enabled: nextEnabled,
      songPath: targetSongPath,
      delaySecs: 0,
      speed: status.value.speed || 1,
      outputMode: outputMode.value === 'osc' ? 'osc' : outputMode.value === 'midi' ? 'midi' : 'keyboard',
      midiDeviceId: outputMode.value === 'midi' ? selectedMidiDevice.value : undefined,
      oscHost: vrchatOscHost.value,
      oscPort: vrchatOscPort.value,
    }));
  } catch (cause) {
    error.value = cause instanceof Error ? cause.message : String(cause);
  }
};

const startDrag = async (event: MouseEvent) => {
  if (positionLocked.value || !isTauri()) return;
  const target = event.target as HTMLElement;
  if (target.closest('button, input, select, [data-no-drag]')) return;
  await getCurrentWindow().startDragging().catch(() => undefined);
};

const togglePositionLock = async () => {
  positionLocked.value = !positionLocked.value;
  if (isTauri()) await getCurrentWindow().setResizable(!positionLocked.value).catch(() => undefined);
};

const closeOverlay = async () => {
  await emit('vrpiano-overlay-closed').catch(() => undefined);
  if (isTauri()) await getCurrentWindow().destroy().catch(() => undefined);
};

watch(overlayOpacity, (value) => {
  const next = normalizeVrpianoOverlayOpacity(value);
  if (Math.abs(next - Number(overlayOpacity.value)) > 1e-6) overlayOpacity.value = next;
});

watch(overlayBlur, (value) => {
  const next = normalizeVrpianoOverlayBlur(value);
  if (Math.abs(next - Number(overlayBlur.value)) > 1e-6) overlayBlur.value = next;
  void syncNativeBackdrop(next).catch(() => {
    nativeBackdropEnabled = null;
  });
});

watch(outputMode, async (newMode) => {
  if (status.value.hotkeys_enabled) {
    const targetSongPath = currentSong.value?.path || status.value.song_path || songs.value[0]?.path || '';
    try {
      applyStatus(await VrpianoApi.setHotkeys({
        enabled: true,
        songPath: targetSongPath,
        delaySecs: 0,
        speed: status.value.speed || 1,
        outputMode: newMode,
        midiDeviceId: newMode === 'midi' ? selectedMidiDevice.value : undefined,
        oscHost: vrchatOscHost.value,
        oscPort: vrchatOscPort.value,
      }));
    } catch {}
  }
});

const handleOpacityInput = (event: Event) => {
  const target = event.target as HTMLInputElement;
  overlayOpacity.value = normalizeVrpianoOverlayOpacity(target.value);
};

onMounted(async () => {
  if (isTauri()) {
    const appWindow = getCurrentWindow();
    await Promise.all([
      appWindow.setAlwaysOnTop(true).catch(() => undefined),
      appWindow.setResizable(!positionLocked.value).catch(() => undefined),
      syncNativeBackdrop(overlayBlur.value).catch(() => undefined),
    ]);
    unlistenMoved = await appWindow.onMoved((event) => {
      if (positionLocked.value) return;
      const position = (event as any).payload || event;
      localStorage.setItem('vrcdog.vrpiano.overlay.position', JSON.stringify({ x: position.x, y: position.y }));
    });
    unlistenFocus = await appWindow.onFocusChanged(() => reapplySavedAppearance());
  }

  document.addEventListener('visibilitychange', handleVisibilityChange);

  try {
    const [nextStatus, nextSongs] = await Promise.all([VrpianoApi.getStatus(), VrpianoApi.listSongs()]);
    applyStatus(nextStatus);
    songs.value = nextSongs;
    if (hotkeysEnabled.value && !nextStatus.hotkeys_enabled) {
      const targetSongPath = currentSong.value?.path || nextStatus.song_path || nextSongs[0]?.path || '';
      applyStatus(await VrpianoApi.setHotkeys({
        enabled: true,
        songPath: targetSongPath,
        delaySecs: 0,
        speed: nextStatus.speed || 1,
        outputMode: outputMode.value === 'osc' ? 'osc' : outputMode.value === 'midi' ? 'midi' : 'keyboard',
        midiDeviceId: outputMode.value === 'midi' ? selectedMidiDevice.value : undefined,
        oscHost: vrchatOscHost.value,
        oscPort: vrchatOscPort.value,
      }));
    }
  } catch (cause) {
    error.value = cause instanceof Error ? cause.message : String(cause);
  }

  unlistenStatus = await listen<VrpianoStatus>('vrpiano_status', (event) => applyStatus(event.payload));
  unlistenClose = await listen('cmd-close-vrpiano-overlay', closeOverlay);
  pollTimer = window.setInterval(refresh, 1_000);

});

onUnmounted(() => {
  if (pollTimer !== null) window.clearInterval(pollTimer);
  if (hotkeyTimer !== null) window.clearTimeout(hotkeyTimer);
  if (songClickTimer !== null) window.clearTimeout(songClickTimer);
  if (appearanceRefreshTimer !== null) window.clearTimeout(appearanceRefreshTimer);
  document.removeEventListener('visibilitychange', handleVisibilityChange);
  unlistenStatus?.();
  unlistenClose?.();
  unlistenMoved?.();
  unlistenFocus?.();
});
</script>

<template>
  <main class="overlay-shell" :style="panelStyle" @mousedown="startDrag">
    <header class="overlay-header">
      <div class="brand">
        <span class="brand-icon"><Music2 :size="17" /></span>
        <div class="brand-meta">
          <div class="brand-line">
            <strong>VRPiano</strong>
            <span
              class="mode-badge clickable"
              :class="outputMode"
              :title="t('vrpiano_overlay.switch_mode_hint')"
              data-no-drag
              @click.stop="cycleOutputMode"
            >
              {{ modeBadgeText }}
            </span>
          </div>
          <small :class="{ active: status.running && !status.paused }">{{ playbackLabel }}</small>
        </div>
      </div>
      <div class="window-actions" data-no-drag>
        <button :title="positionLocked ? t('vrpiano_overlay.unlock_position') : t('vrpiano_overlay.lock_position')" @click="togglePositionLock">
          <Pin v-if="positionLocked" :size="15" />
          <PinOff v-else :size="15" />
        </button>
        <button :title="t('vrpiano_overlay.appearance')" :class="{ active: settingsOpen }" @click="settingsOpen = !settingsOpen">
          <Settings2 :size="15" />
        </button>
        <button :title="t('vrpiano_overlay.close_overlay')" @click="closeOverlay"><X :size="15" /></button>
      </div>
    </header>

    <section v-if="settingsOpen" class="appearance-settings" data-no-drag @mousedown.stop @pointerdown.stop @click.stop>
      <label>
        <span>{{ t('vrpiano_overlay.opacity_label') }}</span>
        <input
          v-model.number="overlayOpacity"
          type="range"
          min="0.3"
          max="1"
          step="0.05"
          class="overlay-slider"
          data-no-drag
          @mousedown.stop
          @pointerdown.stop
          @click.stop
          @input="handleOpacityInput"
        >
        <b>{{ Math.round(overlayOpacity * 100) }}%</b>
      </label>
      <label>
        <span>{{ t('vrpiano_overlay.blur_label') }}</span>
        <input
          v-model="blurEnabled"
          type="checkbox"
          class="overlay-toggle"
          data-no-drag
          @mousedown.stop
          @pointerdown.stop
          @click.stop
        >
        <b>{{ blurEnabled ? t('vrpiano_overlay.on') : t('vrpiano_overlay.off') }}</b>
      </label>
      <div class="setting-row" data-no-drag>
        <span>{{ t('vrpiano.output_mode') }}</span>
        <div class="mode-toggle-group">
          <button
            class="mode-toggle-btn"
            :class="{ active: outputMode === 'keyboard' }"
            @click="outputMode = 'keyboard'"
          >
            {{ t('vrpiano_overlay.mode_keyboard') }}
          </button>
          <button
            class="mode-toggle-btn"
            :class="{ active: outputMode === 'midi' }"
            @click="outputMode = 'midi'"
          >
            MIDI
          </button>
          <button
            class="mode-toggle-btn"
            :class="{ active: outputMode === 'osc' }"
            @click="outputMode = 'osc'"
          >
            {{ t('vrpiano_overlay.mode_osc_contactless') }}
          </button>
        </div>
      </div>
    </section>

    <section class="now-playing">
      <div class="song-copy">
        <small>{{ t('vrpiano_overlay.current_song') }}</small>
        <strong :title="status.song_name || currentSong?.name">{{ status.song_name || currentSong?.name || t('vrpiano_overlay.no_song_selected') }}</strong>
      </div>
      <span class="speed">{{ Number(status.speed || 1).toFixed(2) }}x</span>
    </section>

    <section class="progress-panel">
      <div class="progress-meta">
        <span>{{ formatTime(status.elapsed_ms) }}</span>
        <span>{{ Math.round(progress * 100) }}%</span>
        <span>{{ formatTime(status.duration_ms) }}</span>
      </div>
      <div class="progress-track"><div class="progress-fill" :style="progressStyle" /></div>
    </section>

    <nav class="transport" data-no-drag :aria-label="t('vrpiano_overlay.playback_controls')">
      <button :title="t('vrpiano_overlay.prev')" :disabled="busy || !songs.length" @click="moveSong(-1)"><ChevronLeft :size="21" /></button>
      <button class="play-button" :title="primaryPlaybackLabel" :disabled="busy || !songs.length" @click="togglePlayback">
        <Pause v-if="status.running && !status.paused" :size="23" />
        <Play v-else :size="23" />
        <span>{{ primaryPlaybackLabel }}</span>
      </button>
      <button v-if="hasStartedPlayback" class="restart-button" :title="t('vrpiano_overlay.restart')" :disabled="busy || !currentSong" @click="restartSong">
        <RotateCcw :size="18" />
        <span>{{ t('vrpiano_overlay.restart') }}</span>
      </button>
      <button :title="t('vrpiano_overlay.next')" :disabled="busy || !songs.length" @click="moveSong(1)"><ChevronRight :size="21" /></button>
    </nav>

    <section class="hotkey-panel" :class="{ enabled: status.hotkeys_enabled }">
      <div
        class="hotkey-title clickable"
        role="button"
        tabindex="0"
        :title="status.hotkeys_enabled ? t('vrpiano_overlay.disable_hotkeys') : t('vrpiano_overlay.enable_hotkeys')"
        @click="toggleHotkeys"
        @keydown.enter="toggleHotkeys"
        @keydown.space.prevent="toggleHotkeys"
      >
        <Keyboard :size="14" />
        <strong>{{ t('vrpiano_overlay.global_hotkeys') }}</strong>
        <span class="hotkey-switch" :class="{ on: status.hotkeys_enabled }">
          {{ status.hotkeys_enabled ? t('vrpiano_overlay.on') : t('vrpiano_overlay.off') }}
        </span>
      </div>
      <div class="hotkey-list">
        <span
          v-for="item in hotkeys"
          :key="item.key"
          :class="{ pressed: recentHotkey === item.key }"
          :title="`${item.key} ${item.label}`"
        >
          <kbd>{{ item.key }}</kbd>{{ item.label }}
        </span>
      </div>
    </section>

    <section class="playlist">
      <div class="playlist-title">
        <ListMusic :size="15" />
        <strong>{{ t('vrpiano_overlay.playlist') }}</strong>
        <button
          class="preview-toggle"
          :class="{ enabled: previewEnabled }"
          type="button"
          role="switch"
          :aria-checked="previewEnabled"
          data-testid="preview-toggle"
          :title="previewEnabled ? t('vrpiano_overlay.disable_preview') : t('vrpiano_overlay.enable_preview')"
          @click="previewEnabled = !previewEnabled"
        >
          <Headphones :size="13" />
          <span>{{ previewEnabled ? t('vrpiano_overlay.preview_on') : t('vrpiano_overlay.preview_off') }}</span>
        </button>
        <span>{{ songs.length }} {{ t('vrpiano_overlay.songs_unit') }}</span>
      </div>
      <div class="playlist-scroll" data-no-drag>
        <button
          v-for="(song, index) in songs"
          :key="song.path"
          :class="{ active: index === currentIndex, previewing: song.path === previewingPath }"
          :disabled="busy"
          :title="previewEnabled ? t('vrpiano_overlay.click_to_preview_dblclick_to_play') : t('vrpiano_overlay.click_to_play')"
          @click="handleSongClick(song)"
          @dblclick.prevent="handleSongDblClick(song)"
        >
          <span class="song-note" :class="{ custom: Boolean(songIcon(song) || songCover(song)) }">
            <img v-if="isImageIcon(songIcon(song))" :src="songIcon(song)" alt="">
            <img v-else-if="songCover(song)" :src="songCover(song)" alt="">
            <span v-else-if="songIcon(song)">{{ songIcon(song) }}</span>
            <b v-else class="song-index">{{ index + 1 }}</b>
          </span>
          <strong :title="song.name">{{ song.name }}</strong>
          <small v-if="song.path === previewingPath" class="preview-badge">
            <Headphones :size="11" />
            {{ t('vrpiano_overlay.previewing') }}
          </small>
          <small v-else-if="index === currentIndex">{{ status.paused ? t('vrpiano_overlay.pause') : status.running ? t('vrpiano_overlay.playing') : t('vrpiano_overlay.current') }}</small>
        </button>
      </div>
    </section>

    <footer v-if="error" class="error-line" :title="error">{{ error }}</footer>
  </main>
</template>

<style scoped>
:global(html.vrpiano-overlay-mode),
:global(body.vrpiano-overlay-mode),
:global(body.vrpiano-overlay-mode #app) {
  width: 100%;
  height: 100%;
  overflow: hidden;
  background: transparent !important;
}

button,
input {
  font: inherit;
}

.overlay-shell,
.overlay-shell * {
  box-sizing: border-box;
}

.overlay-shell {
  position: relative;
  isolation: isolate;
  width: 100vw;
  height: 100vh;
  min-width: 320px;
  min-height: 420px;
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 12px;
  overflow: hidden;
  border: 1px solid var(--theme-border-strong);
  border-radius: 8px;
  color: var(--theme-text-strong);
  box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--theme-primary) 8%, transparent), 0 14px 36px rgba(0, 0, 0, 0.18);
  user-select: none;
}

.overlay-shell::before {
  content: '';
  position: absolute;
  inset: 0;
  z-index: -1;
  border-radius: inherit;
  background: color-mix(in srgb, var(--theme-bg-main) calc(var(--vrpiano-overlay-opacity, 0.88) * 100%), transparent);
  backdrop-filter: blur(var(--vrpiano-overlay-blur, 20px)) saturate(160%);
  -webkit-backdrop-filter: blur(var(--vrpiano-overlay-blur, 20px)) saturate(160%);
  pointer-events: none;
}

.overlay-header,
.brand,
.window-actions,
.now-playing,
.progress-meta,
.transport,
.hotkey-title,
.playlist-title {
  display: flex;
  align-items: center;
}

.overlay-header {
  min-height: 34px;
  justify-content: space-between;
  gap: 10px;
  cursor: move;
}

.brand {
  min-width: 0;
  gap: 9px;
}

.brand-icon {
  width: 30px;
  height: 30px;
  flex: 0 0 auto;
  display: grid;
  place-items: center;
  border-radius: 7px;
  color: white;
  background: var(--theme-primary);
}

.brand-meta {
  min-width: 0;
  display: grid;
  line-height: 1.1;
}

.brand-line {
  display: flex;
  align-items: center;
  gap: 6px;
}

.brand strong {
  font-size: 14px;
}

.mode-badge {
  padding: 1px 5px;
  font-size: 9px;
  font-weight: 800;
  border-radius: 4px;
  background: color-mix(in srgb, var(--theme-primary) 18%, transparent);
  color: var(--theme-primary);
  border: 1px solid color-mix(in srgb, var(--theme-primary) 32%, transparent);
}

.mode-badge.osc {
  background: color-mix(in srgb, #8b5cf6 18%, transparent);
  color: #8b5cf6;
  border-color: color-mix(in srgb, #8b5cf6 32%, transparent);
}

.mode-badge.midi {
  background: color-mix(in srgb, #06b6d4 18%, transparent);
  color: #06b6d4;
  border-color: color-mix(in srgb, #06b6d4 32%, transparent);
}

.mode-badge.clickable {
  cursor: pointer;
  user-select: none;
  transition: transform 0.1s ease, filter 0.1s ease;
}

.mode-badge.clickable:hover {
  filter: brightness(1.25);
  transform: scale(1.05);
}

.mode-badge.clickable:active {
  transform: scale(0.95);
}

.brand small {
  margin-top: 3px;
  color: var(--theme-text-muted);
  font-size: 10px;
  font-weight: 800;
}

.brand small.active {
  color: #059669;
}

.window-actions {
  gap: 4px;
}

.window-actions button,
.transport button {
  border: 1px solid var(--theme-border-soft);
  display: grid;
  place-items: center;
  color: var(--theme-text);
  background: color-mix(in srgb, var(--theme-surface-hover) 82%, transparent);
  cursor: pointer;
}

.window-actions button {
  width: 29px;
  height: 29px;
  border-radius: 6px;
}

.window-actions button:hover,
.window-actions button.active,
.transport button:hover:not(:disabled) {
  color: var(--theme-primary);
  border-color: color-mix(in srgb, var(--theme-primary) 42%, transparent);
  background: var(--theme-active-bg);
}

.appearance-settings,
.now-playing,
.progress-panel,
.hotkey-panel,
.playlist {
  border: 1px solid var(--theme-border-soft);
  border-radius: 7px;
  background: color-mix(in srgb, var(--theme-surface) 74%, transparent);
}

.appearance-settings {
  padding: 9px 10px;
  display: grid;
  gap: 8px;
}

.appearance-settings label {
  min-width: 0;
  display: grid;
  grid-template-columns: 74px minmax(0, 1fr) 42px;
  align-items: center;
  gap: 8px;
  color: var(--theme-text-soft);
  font-size: 10px;
  font-weight: 800;
}

.appearance-settings .setting-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  font-size: 10px;
  font-weight: 800;
  color: var(--theme-text-soft);
}

.appearance-settings .mode-toggle-group {
  display: flex;
  gap: 3px;
  background: rgba(0, 0, 0, 0.28);
  padding: 2px;
  border-radius: 5px;
}

.appearance-settings .mode-toggle-btn {
  border: none;
  background: transparent;
  color: var(--theme-text-muted);
  font-size: 9px;
  font-weight: 800;
  padding: 2px 6px;
  border-radius: 3px;
  cursor: pointer;
  transition: all 0.15s ease;
}

.appearance-settings .mode-toggle-btn:hover {
  color: var(--theme-text-strong);
}

.appearance-settings .mode-toggle-btn.active {
  background: color-mix(in srgb, var(--theme-primary) 35%, rgba(255, 255, 255, 0.12));
  color: var(--theme-primary);
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.3);
}

.appearance-settings input {
  min-width: 0;
  accent-color: var(--theme-primary);
}

.appearance-settings .overlay-slider {
  appearance: none;
  -webkit-appearance: none;
  width: 100%;
  height: 14px;
  margin: 0;
  padding: 0;
  cursor: pointer;
  pointer-events: auto;
  background: transparent;
  user-select: none;
  -webkit-user-select: none;
}

.appearance-settings .overlay-slider::-webkit-slider-runnable-track {
  height: 4px;
  border-radius: 999px;
  background: color-mix(in srgb, var(--theme-primary) 22%, var(--theme-border-soft));
}

.appearance-settings .overlay-slider::-moz-range-track {
  height: 4px;
  border-radius: 999px;
  background: color-mix(in srgb, var(--theme-primary) 22%, var(--theme-border-soft));
}

.appearance-settings .overlay-slider::-webkit-slider-thumb {
  appearance: none;
  -webkit-appearance: none;
  width: 14px;
  height: 14px;
  margin-top: -5px;
  border: 0;
  border-radius: 999px;
  background: var(--theme-primary);
  box-shadow: 0 0 0 4px color-mix(in srgb, var(--theme-primary) 18%, transparent);
  cursor: pointer;
  pointer-events: auto;
}

.appearance-settings .overlay-slider::-moz-range-thumb {
  width: 14px;
  height: 14px;
  border: 0;
  border-radius: 999px;
  background: var(--theme-primary);
  box-shadow: 0 0 0 4px color-mix(in srgb, var(--theme-primary) 18%, transparent);
  cursor: pointer;
  pointer-events: auto;
}

.appearance-settings .overlay-toggle {
  appearance: none;
  -webkit-appearance: none;
  position: relative;
  width: 32px;
  height: 18px;
  margin: 0;
  border: 1px solid var(--theme-border-strong);
  border-radius: 9px;
  background: var(--theme-surface-hover);
  cursor: pointer;
  transition: background 160ms ease, border-color 160ms ease;
}

.appearance-settings .overlay-toggle::after {
  content: '';
  position: absolute;
  top: 2px;
  left: 2px;
  width: 12px;
  height: 12px;
  border-radius: 50%;
  background: var(--theme-text-muted);
  transition: transform 160ms ease, background 160ms ease;
}

.appearance-settings .overlay-toggle:checked {
  border-color: var(--theme-primary);
  background: color-mix(in srgb, var(--theme-primary) 30%, transparent);
}

.appearance-settings .overlay-toggle:checked::after {
  background: var(--theme-primary);
  transform: translateX(14px);
}

.appearance-settings .overlay-toggle:focus-visible {
  outline: 2px solid color-mix(in srgb, var(--theme-primary) 45%, transparent);
  outline-offset: 2px;
}

.appearance-settings b {
  color: var(--theme-text-muted);
  text-align: right;
}

.now-playing {
  min-height: 58px;
  justify-content: space-between;
  gap: 12px;
  padding: 9px 11px;
}

.song-copy {
  min-width: 0;
  display: grid;
  gap: 4px;
}

.song-copy small {
  color: var(--theme-text-muted);
  font-size: 10px;
  font-weight: 750;
}

.song-copy strong {
  overflow: hidden;
  font-size: 14px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.speed {
  flex: 0 0 auto;
  padding: 4px 7px;
  border-radius: 5px;
  color: var(--theme-primary);
  background: var(--theme-active-bg);
  font-size: 11px;
  font-weight: 900;
}

.progress-panel {
  padding: 8px 10px 10px;
}

.progress-meta {
  justify-content: space-between;
  color: var(--theme-text-muted);
  font-size: 10px;
  font-variant-numeric: tabular-nums;
  font-weight: 800;
}

.progress-track {
  height: 6px;
  margin-top: 7px;
  overflow: hidden;
  border-radius: 3px;
  background: color-mix(in srgb, var(--theme-text) 12%, transparent);
}

.progress-fill {
  height: 100%;
  border-radius: inherit;
  background: var(--theme-primary);
  transition: width 180ms linear;
}

.transport {
  min-height: 50px;
  justify-content: center;
  gap: 9px;
}

.transport button {
  width: 40px;
  height: 40px;
  border-radius: 7px;
}

.transport .play-button {
  width: 64px;
  height: 38px;
  display: inline-flex;
  gap: 4px;
  color: white;
  border-color: var(--theme-primary);
  background: var(--theme-primary);
  font-size: 10px;
  font-weight: 900;
}

.transport .play-button:hover:not(:disabled) {
  color: white;
  border-color: var(--theme-primary);
  background: color-mix(in srgb, var(--theme-primary) 88%, black);
}

.transport .restart-button {
  width: 76px;
  height: 38px;
  display: inline-flex;
  gap: 4px;
  font-size: 9px;
  font-weight: 850;
}

.transport button:disabled {
  opacity: 0.45;
  cursor: default;
}

.hotkey-panel {
  padding: 8px 10px 9px;
  opacity: 0.72;
}

.hotkey-panel.enabled {
  opacity: 1;
}

.hotkey-title {
  gap: 6px;
  font-size: 11px;
}

.hotkey-title.clickable {
  cursor: pointer;
  border-radius: 4px;
  padding: 2px 4px;
  margin: -2px -4px;
  transition: background 120ms ease;
}

.hotkey-title.clickable:hover {
  background: color-mix(in srgb, var(--theme-primary) 10%, transparent);
}

.hotkey-title span {
  margin-left: auto;
  color: var(--theme-text-muted);
  font-size: 10px;
  font-weight: 800;
}

.hotkey-switch {
  padding: 1px 6px;
  border-radius: 999px;
  background: color-mix(in srgb, var(--theme-text-muted) 16%, transparent);
  transition: background 140ms ease, color 140ms ease;
}

.hotkey-switch.on {
  color: #059669;
  background: color-mix(in srgb, #059669 16%, transparent);
}

.hotkey-panel.enabled .hotkey-title span {
  color: #059669;
}

.hotkey-list {
  margin-top: 7px;
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(48px, 1fr));
  gap: 5px;
}

.hotkey-list span {
  min-width: 0;
  display: grid;
  place-items: center;
  gap: 2px;
  padding: 4px 2px;
  border: 1px solid var(--theme-border-soft);
  border-radius: 5px;
  color: var(--theme-text-muted);
  font-size: 8px;
  font-weight: 800;
}

.hotkey-list kbd {
  color: var(--theme-text-strong);
  font-size: 10px;
  font-weight: 900;
}

.hotkey-list span.pressed {
  color: var(--theme-primary);
  border-color: var(--theme-primary);
  background: var(--theme-active-bg);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--theme-primary) 14%, transparent);
}

.playlist {
  min-height: 0;
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.playlist-title {
  min-height: 34px;
  flex: 0 0 auto;
  gap: 7px;
  padding: 0 10px;
  border-bottom: 1px solid var(--theme-border-soft);
  font-size: 11px;
}

.playlist-title span {
  margin-left: auto;
  color: var(--theme-text-muted);
  font-size: 10px;
  font-weight: 800;
}

.playlist-title .preview-toggle {
  min-width: 0;
  display: inline-flex;
  align-items: center;
  gap: 4px;
  margin-left: auto;
  padding: 4px 6px;
  border: 1px solid var(--theme-border-soft);
  border-radius: 5px;
  color: var(--theme-text-muted);
  background: transparent;
  cursor: pointer;
  font-size: 9px;
  font-weight: 850;
}

.playlist-title .preview-toggle + span {
  margin-left: 4px;
}

.playlist-title .preview-toggle span {
  margin-left: 0;
  color: inherit;
  font-size: inherit;
}

.playlist-title .preview-toggle:hover,
.playlist-title .preview-toggle.enabled {
  color: var(--theme-primary);
  border-color: color-mix(in srgb, var(--theme-primary) 42%, transparent);
  background: var(--theme-active-bg);
}

.playlist-scroll {
  min-height: 0;
  flex: 1;
  overflow-y: auto;
  padding: 5px;
  scrollbar-width: thin;
  scrollbar-color: var(--theme-border-strong) transparent;
}

.playlist-scroll button {
  width: 100%;
  min-height: 34px;
  display: grid;
  grid-template-columns: 24px minmax(0, 1fr) auto;
  align-items: center;
  gap: 7px;
  padding: 4px 8px;
  border: 1px solid transparent;
  border-radius: 5px;
  color: var(--theme-text);
  background: transparent;
  text-align: left;
  cursor: pointer;
}

.playlist-scroll button:hover,
.playlist-scroll button.active {
  border-color: var(--theme-border-soft);
  background: var(--theme-active-bg);
}

.playlist-scroll button.previewing {
  border-color: color-mix(in srgb, var(--theme-primary) 50%, transparent);
  background: color-mix(in srgb, var(--theme-primary) 12%, transparent);
}

.playlist-scroll button > span {
  color: var(--theme-text-muted);
  font-size: 9px;
  font-variant-numeric: tabular-nums;
  font-weight: 800;
  text-align: center;
}

.playlist-scroll .song-note {
  width: 22px;
  height: 22px;
  display: grid;
  place-items: center;
  border-radius: 5px;
  overflow: hidden;
  color: white;
  background: var(--theme-primary);
  font-size: 13px;
  font-weight: 900;
  line-height: 1;
}

.playlist-scroll .song-note.custom {
  color: var(--theme-text);
  background: var(--theme-surface);
  border: 1px solid var(--theme-border-soft);
}

.playlist-scroll .song-note img {
  width: 100%;
  height: 100%;
  border-radius: inherit;
  object-fit: cover;
}

.playlist-scroll .song-note .song-index {
  font-size: 10px;
  font-weight: 900;
  color: var(--theme-text-muted);
}

.playlist-scroll button strong {
  min-width: 0;
  overflow: hidden;
  font-size: 11px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.playlist-scroll button small {
  color: var(--theme-primary);
  font-size: 9px;
  font-weight: 900;
}

.preview-badge {
  display: inline-flex;
  align-items: center;
  gap: 3px;
  color: var(--theme-primary) !important;
  font-weight: 850;
}

.error-line {
  flex: 0 0 auto;
  overflow: hidden;
  color: #dc2626;
  font-size: 10px;
  font-weight: 750;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
