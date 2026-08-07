<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue';
import { isTauri } from '@tauri-apps/api/core';
import { emit, listen, type UnlistenFn } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useStorage } from '@vueuse/core';
import {
  ChevronLeft,
  ChevronRight,
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
});

const status = ref<VrpianoStatus>(emptyStatus());
const songs = ref<VrpianoSong[]>([]);
const busy = ref(false);
const error = ref('');
const settingsOpen = ref(false);
const recentHotkey = ref('');
const overlayOpacity = useStorage('vrcdog.vrpiano.overlay.opacity', 0.88);
const overlayBlur = useStorage('vrcdog.vrpiano.overlay.blur', 20);
const positionLocked = useStorage('vrcdog.vrpiano.overlay.locked', false);

let pollTimer: number | null = null;
let hotkeyTimer: number | null = null;
let unlistenStatus: UnlistenFn | null = null;
let unlistenClose: UnlistenFn | null = null;
let unlistenMoved: UnlistenFn | null = null;

const panelStyle = computed(() => {
  const opacity = Math.min(1, Math.max(0.3, Number(overlayOpacity.value) || 0.88));
  const blur = Math.min(40, Math.max(0, Number(overlayBlur.value) || 0));
  return {
    backgroundColor: `color-mix(in srgb, var(--theme-bg-main) ${Math.round(opacity * 100)}%, transparent)`,
    backdropFilter: `blur(${blur}px) saturate(160%)`,
  };
});

const progress = computed(() => Math.min(1, Math.max(0, Number(status.value.progress) || 0)));
const currentIndex = computed(() => {
  const byPath = songs.value.findIndex((song) => song.path === status.value.song_path);
  if (byPath >= 0) return byPath;
  return songs.value.findIndex((song) => song.name === status.value.song_name);
});
const currentSong = computed(() => songs.value[currentIndex.value] || songs.value[0] || null);
const playbackLabel = computed(() => {
  if (status.value.paused) return '已暂停';
  if (status.value.running) return '演奏中';
  return '待命';
});
const progressStyle = computed(() => ({ width: `${Math.round(progress.value * 10000) / 100}%` }));
const hotkeys = [
  { key: 'F1', label: '开始' },
  { key: 'F2', label: '停止' },
  { key: 'F3', label: '加速' },
  { key: 'F4', label: '减速' },
  { key: 'F5', label: '默认' },
];

const formatTime = (ms: number) => {
  const seconds = Math.max(0, Math.round((Number(ms) || 0) / 1000));
  return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, '0')}`;
};

const applyStatus = (next: VrpianoStatus) => {
  status.value = next;
  if (next.last_hotkey && next.last_hotkey_at_ms) {
    recentHotkey.value = next.last_hotkey;
    if (hotkeyTimer !== null) window.clearTimeout(hotkeyTimer);
    hotkeyTimer = window.setTimeout(() => {
      recentHotkey.value = '';
      hotkeyTimer = null;
    }, 850);
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
  throw new Error('等待当前曲目停止超时');
};

const playSong = async (song: VrpianoSong) => {
  if (busy.value) return;
  busy.value = true;
  error.value = '';
  try {
    if (status.value.running) {
      await VrpianoApi.stop();
      await waitUntilStopped();
    }
    applyStatus(await VrpianoApi.start({
      songPath: song.path,
      delaySecs: 0,
      speed: status.value.speed || 1,
    }));
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
  const next = Math.min(1, Math.max(0.3, Number(value) || 0.88));
  if (Math.abs(next - Number(overlayOpacity.value)) > 1e-6) overlayOpacity.value = next;
});

watch(overlayBlur, (value) => {
  const next = Math.min(40, Math.max(0, Number(value) || 0));
  if (Math.abs(next - Number(overlayBlur.value)) > 1e-6) overlayBlur.value = next;
});

const handleOpacityInput = (event: Event) => {
  const target = event.target as HTMLInputElement;
  const next = Math.min(1, Math.max(0.3, Number(target.value) || 0.88));
  overlayOpacity.value = next;
};

const handleBlurInput = (event: Event) => {
  const target = event.target as HTMLInputElement;
  const next = Math.min(40, Math.max(0, Number(target.value) || 0));
  overlayBlur.value = next;
};

onMounted(async () => {
  try {
    const [nextStatus, nextSongs] = await Promise.all([VrpianoApi.getStatus(), VrpianoApi.listSongs()]);
    applyStatus(nextStatus);
    songs.value = nextSongs;
  } catch (cause) {
    error.value = cause instanceof Error ? cause.message : String(cause);
  }

  unlistenStatus = await listen<VrpianoStatus>('vrpiano_status', (event) => applyStatus(event.payload));
  unlistenClose = await listen('cmd-close-vrpiano-overlay', closeOverlay);
  pollTimer = window.setInterval(refresh, 1_000);

  if (isTauri()) {
    const appWindow = getCurrentWindow();
    await appWindow.setAlwaysOnTop(true).catch(() => undefined);
    await appWindow.setResizable(!positionLocked.value).catch(() => undefined);
    unlistenMoved = await appWindow.onMoved((event) => {
      if (positionLocked.value) return;
      const position = (event as any).payload || event;
      localStorage.setItem('vrcdog.vrpiano.overlay.position', JSON.stringify({ x: position.x, y: position.y }));
    });
  }
});

onUnmounted(() => {
  if (pollTimer !== null) window.clearInterval(pollTimer);
  if (hotkeyTimer !== null) window.clearTimeout(hotkeyTimer);
  unlistenStatus?.();
  unlistenClose?.();
  unlistenMoved?.();
});
</script>

<template>
  <main class="overlay-shell" :style="panelStyle" @mousedown="startDrag">
    <header class="overlay-header">
      <div class="brand">
        <span class="brand-icon"><Music2 :size="17" /></span>
        <div>
          <strong>VRPiano</strong>
          <small :class="{ active: status.running && !status.paused }">{{ playbackLabel }}</small>
        </div>
      </div>
      <div class="window-actions" data-no-drag>
        <button :title="positionLocked ? '解锁悬浮窗位置' : '锁定悬浮窗位置'" @click="togglePositionLock">
          <Pin v-if="positionLocked" :size="15" />
          <PinOff v-else :size="15" />
        </button>
        <button title="外观设置" :class="{ active: settingsOpen }" @click="settingsOpen = !settingsOpen">
          <Settings2 :size="15" />
        </button>
        <button title="关闭悬浮窗" @click="closeOverlay"><X :size="15" /></button>
      </div>
    </header>

    <section v-if="settingsOpen" class="appearance-settings" data-no-drag @mousedown.stop @pointerdown.stop @click.stop>
      <label>
        <span>背景透明度</span>
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
        <span>背景模糊度</span>
        <input
          v-model.number="overlayBlur"
          type="range"
          min="0"
          max="40"
          step="2"
          class="overlay-slider"
          data-no-drag
          @mousedown.stop
          @pointerdown.stop
          @click.stop
          @input="handleBlurInput"
        >
        <b>{{ overlayBlur }}px</b>
      </label>
    </section>

    <section class="now-playing">
      <div class="song-copy">
        <small>当前曲目</small>
        <strong :title="status.song_name || currentSong?.name">{{ status.song_name || currentSong?.name || '未选择曲目' }}</strong>
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

    <nav class="transport" data-no-drag aria-label="播放控制">
      <button title="上一首" :disabled="busy || !songs.length" @click="moveSong(-1)"><ChevronLeft :size="21" /></button>
      <button class="play-button" :title="status.running && !status.paused ? '暂停' : '播放'" :disabled="busy || !songs.length" @click="togglePlayback">
        <Pause v-if="status.running && !status.paused" :size="23" />
        <Play v-else :size="23" />
      </button>
      <button title="重新播放" :disabled="busy || !currentSong" @click="restartSong"><RotateCcw :size="19" /></button>
      <button title="下一首" :disabled="busy || !songs.length" @click="moveSong(1)"><ChevronRight :size="21" /></button>
    </nav>

    <section class="hotkey-panel" :class="{ enabled: status.hotkeys_enabled }">
      <div class="hotkey-title">
        <Keyboard :size="14" />
        <strong>全局快捷键</strong>
        <span>{{ status.hotkeys_enabled ? '已开启' : '已关闭' }}</span>
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
      <div class="playlist-title"><ListMusic :size="15" /><strong>歌单</strong><span>{{ songs.length }} 首</span></div>
      <div class="playlist-scroll" data-no-drag>
        <button
          v-for="(song, index) in songs"
          :key="song.path"
          :class="{ active: index === currentIndex }"
          :disabled="busy"
          @click="playSong(song)"
        >
          <span>{{ index + 1 }}</span>
          <strong :title="song.name">{{ song.name }}</strong>
          <small v-if="index === currentIndex">{{ status.paused ? '暂停' : status.running ? '播放中' : '当前' }}</small>
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

.brand div {
  min-width: 0;
  display: grid;
  line-height: 1.1;
}

.brand strong {
  font-size: 14px;
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
  width: 50px;
  height: 44px;
  color: white;
  border-color: var(--theme-primary);
  background: var(--theme-primary);
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

.hotkey-title span {
  margin-left: auto;
  color: var(--theme-text-muted);
  font-size: 10px;
  font-weight: 800;
}

.hotkey-panel.enabled .hotkey-title span {
  color: #059669;
}

.hotkey-list {
  margin-top: 7px;
  display: grid;
  grid-template-columns: repeat(5, minmax(0, 1fr));
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

.playlist-scroll button > span {
  color: var(--theme-text-muted);
  font-size: 9px;
  font-variant-numeric: tabular-nums;
  font-weight: 800;
  text-align: center;
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
