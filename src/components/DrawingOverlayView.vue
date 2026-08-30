<script setup lang="ts">
import { getCurrentWindow } from '@tauri-apps/api/window';
import { isTauri } from '@tauri-apps/api/core';
import {
  CircleStop,
  FolderOpen,
  Image as ImageIcon,
  Pause,
  Pin,
  PinOff,
  Play,
  RefreshCcw,
  ScanLine,
  X,
} from 'lucide-vue-next';
import { useStorage } from '@vueuse/core';
import { ref } from 'vue';
import { useDrawing } from '../composables/useDrawing';

const {
  config,
  sourcePath,
  plan,
  status,
  canvasEl,
  busy,
  error,
  progressPercent,
  statusKey,
  stageLabel,
  chooseImage,
  prepare,
  start,
  togglePause,
  stop,
} = useDrawing();

const positionLocked = useStorage('vrcdog.drawing.overlay.locked', false);
const overlayWindow = getCurrentWindow();

const closeOverlay = () => { void overlayWindow.close(); };
const toggleLock = () => { positionLocked.value = !positionLocked.value; };
const dragHandle = ref<HTMLElement | null>(null);

const startDrag = (event: PointerEvent) => {
  if (positionLocked.value || !dragHandle.value) return;
  dragHandle.value.setPointerCapture(event.pointerId);
  void overlayWindow.startDragging();
};
</script>

<template>
  <div class="drawing-overlay" :class="{ locked: positionLocked }">
    <header ref="dragHandle" class="overlay-title" @pointerdown="startDrag">
      <span class="title-grip"><ScanLine :size="15" /> {{ $t('drawing.title') }}</span>
      <div class="title-actions">
        <button class="title-btn" :title="$t('drawing.reset')" @click="toggleLock">
          <PinOff v-if="positionLocked" :size="15" />
          <Pin v-else :size="15" />
        </button>
        <button class="title-btn" :title="$t('drawing.stop')" @click="closeOverlay"><X :size="16" /></button>
      </div>
    </header>

    <div class="overlay-body">
      <div class="overlay-status" :class="{ active: status.running, paused: status.paused }">
        <span class="dot" />{{ $t(statusKey) }}
        <span v-if="busy && stageLabel" class="stage">{{ stageLabel }}</span>
      </div>

      <button class="ov-wide secondary" :disabled="status.running" @click="chooseImage">
        <FolderOpen :size="16" /> {{ sourcePath ? $t('drawing.open_image') : $t('drawing.empty_title') }}
      </button>

      <label class="ov-field">
        <span>{{ $t('drawing.mode') }}</span>
        <select v-model="config.mode" :disabled="status.running">
          <option value="lineart">{{ $t('drawing.mode_lineart') }}</option>
          <option value="edges">{{ $t('drawing.mode_edges') }}</option>
          <option value="dither">{{ $t('drawing.mode_dither') }}</option>
          <option value="ai">{{ $t('drawing.mode_ai') }}</option>
        </select>
      </label>

      <template v-if="config.mode === 'ai'">
        <label class="ov-field">
          <span>{{ $t('drawing.ai_model') }}</span>
          <select v-model="config.ai_model" :disabled="status.running">
            <option value="image-to-line">{{ $t('drawing.ai_model_imageline') }}</option>
            <option value="anime2sketch">{{ $t('drawing.ai_model_anime2sketch') }}</option>
          </select>
        </label>
        <label class="ov-field">
          <span>{{ $t('drawing.contrast') }} <b>{{ config.contrast.toFixed(2) }}</b></span>
          <input v-model.number="config.contrast" type="range" min="0.5" max="3" step="0.05" :disabled="status.running">
        </label>
      </template>
      <template v-else>
        <label class="ov-field">
          <span>{{ $t('drawing.threshold') }} <b>{{ config.threshold }}</b></span>
          <input v-model.number="config.threshold" type="range" min="10" max="245" step="1" :disabled="status.running">
        </label>
      </template>

      <label class="ov-field">
        <span>{{ $t('drawing.sensitivity') }} <b>{{ config.sensitivity.toFixed(1) }}</b></span>
        <input v-model.number="config.sensitivity" type="range" min="0.2" max="4" step="0.1" :disabled="status.running">
      </label>
      <label class="ov-field">
        <span>{{ $t('drawing.point_delay') }} <b>{{ config.point_delay_ms }} ms</b></span>
        <input v-model.number="config.point_delay_ms" type="range" min="1" max="120" step="1">
      </label>
      <label class="ov-field">
        <span>{{ $t('drawing.canvas_size') }} <b>{{ config.canvas_size_px === 0 ? $t('drawing.auto') : config.canvas_size_px }}</b></span>
        <input v-model.number="config.canvas_size_px" type="range" min="0" max="4096" step="64" :disabled="status.running">
      </label>
      <label class="ov-field">
        <span>{{ $t('drawing.pen_settle') }} <b>{{ config.pen_settle_ms }} ms</b></span>
        <input v-model.number="config.pen_settle_ms" type="range" min="0" max="120" step="1" :disabled="status.running">
      </label>

      <div class="ov-preview">
        <canvas ref="canvasEl" />
        <div v-if="!sourcePath" class="ov-empty"><ImageIcon :size="30" stroke-width="1.5" /></div>
        <div v-else-if="busy" class="ov-empty"><RefreshCcw :size="22" class="spin" /></div>
      </div>

      <div class="ov-metrics">
        <span><b>{{ plan?.strokes.length || 0 }}</b> {{ $t('drawing.strokes') }}</span>
        <span><b>{{ progressPercent }}%</b> {{ $t('drawing.progress') }}</span>
      </div>
      <div class="ov-progress"><span :style="{ width: `${progressPercent}%` }" /></div>
      <div v-if="error || status.last_error" class="ov-error">{{ error || status.last_error }}</div>
    </div>

    <footer class="overlay-footer">
      <button class="ov-wide secondary" :disabled="!sourcePath || busy || status.running" @click="prepare">
        <RefreshCcw :size="16" :class="{ spin: busy }" /> {{ $t('drawing.generate') }}
      </button>
      <button v-if="!status.running" class="ov-wide primary" :disabled="!plan || busy" @click="start">
        <Play :size="17" fill="currentColor" /> {{ $t('drawing.start') }}
      </button>
      <template v-else>
        <button class="ov-wide secondary" @click="togglePause">
          <Play v-if="status.paused" :size="16" fill="currentColor" />
          <Pause v-else :size="16" fill="currentColor" />
          {{ status.paused ? $t('drawing.resume') : $t('drawing.pause') }}
        </button>
        <button class="ov-wide stop" @click="stop"><CircleStop :size="16" /> {{ $t('drawing.stop') }}</button>
      </template>
    </footer>
  </div>
</template>

<style scoped>
.drawing-overlay {
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  border-radius: 16px;
  overflow: hidden;
  background: rgba(18, 22, 33, 0.62);
  backdrop-filter: blur(14px) saturate(1.2);
  -webkit-backdrop-filter: blur(14px) saturate(1.2);
  border: 1px solid rgba(255, 255, 255, 0.16);
  box-shadow: 0 18px 50px rgba(0, 0, 0, 0.45);
  color: #f4f1ea;
  font-family: "PingFang SC", "Microsoft YaHei", "Nunito", sans-serif;
}
.overlay-title {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 12px;
  cursor: grab;
  background: rgba(255, 255, 255, 0.06);
  border-bottom: 1px solid rgba(255, 255, 255, 0.1);
}
.overlay-title.locked { cursor: default; }
.title-grip { display: flex; align-items: center; gap: 7px; font-size: 13px; font-weight: 800; color: #ffd9a8; }
.title-actions { display: flex; gap: 6px; }
.title-btn { width: 30px; height: 30px; display: grid; place-items: center; border-radius: 7px; border: 1px solid rgba(255,255,255,0.14); background: transparent; color: #e8e2d6; }
.title-btn:hover { background: rgba(255,255,255,0.12); }
.overlay-body { flex: 1; min-height: 0; overflow-y: auto; padding: 12px; display: flex; flex-direction: column; gap: 10px; }
.overlay-status { display: inline-flex; align-items: center; gap: 7px; font-size: 11px; font-weight: 800; align-self: flex-start; padding: 4px 10px; border-radius: 999px; background: rgba(255,255,255,0.08); }
.overlay-status .dot { width: 7px; height: 7px; border-radius: 50%; background: #a8978d; }
.overlay-status.active .dot { background: #22c55e; box-shadow: 0 0 0 3px rgba(34,197,94,.18); }
.overlay-status.paused .dot { background: #f59e0b; }
.overlay-status .stage { color: #ffd9a8; font-weight: 700; }
.ov-wide { width: 100%; min-height: 36px; display: inline-flex; align-items: center; justify-content: center; gap: 7px; padding: 0 12px; border-radius: 8px; font-size: 12px; font-weight: 800; }
.primary { background: #f59e0b; color: #1c1206; border: 1px solid #f59e0b; }
.secondary { background: rgba(255,255,255,0.08); color: #f4f1ea; border: 1px solid rgba(255,255,255,0.16); }
.stop { background: #ef4444; color: #fff; border: 1px solid #ef4444; }
.ov-field { display: flex; flex-direction: column; gap: 6px; font-size: 11px; font-weight: 700; color: #d8cfc0; }
.ov-field > span { display: flex; justify-content: space-between; }
.ov-field b { color: #fff; font-family: ui-monospace, monospace; }
.ov-field select { height: 34px; border-radius: 8px; border: 1px solid rgba(255,255,255,0.16); background: rgba(10,12,18,0.7); color: #f4f1ea; font-size: 12px; }
.ov-field input[type="range"] { width: 100%; accent-color: #f59e0b; }
.ov-preview { position: relative; flex: 1; min-height: 160px; border-radius: 10px; border: 1px solid rgba(255,255,255,0.14); background: #fffdf7; overflow: hidden; }
.ov-preview canvas { position: absolute; inset: 0; width: 100%; height: 100%; }
.ov-empty { position: absolute; inset: 0; display: grid; place-items: center; color: #9d806c; }
.ov-metrics { display: flex; justify-content: space-between; font-size: 11px; color: #cfc6b6; }
.ov-metrics b { color: #fff; }
.ov-progress { height: 5px; border-radius: 3px; background: rgba(255,255,255,0.16); overflow: hidden; }
.ov-progress span { display: block; height: 100%; background: #f59e0b; transition: width .2s ease; }
.ov-error { font-size: 11px; color: #fecaca; background: rgba(239,68,68,.16); border-radius: 6px; padding: 7px 9px; }
.overlay-footer { padding: 10px 12px; display: flex; flex-direction: column; gap: 8px; border-top: 1px solid rgba(255,255,255,0.1); background: rgba(255,255,255,0.04); }
.spin { animation: ov-spin 1s linear infinite; }
@keyframes ov-spin { to { transform: rotate(360deg); } }
button:disabled { opacity: .45; cursor: not-allowed; }
</style>
