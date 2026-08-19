<script setup lang="ts">
import { isTauri } from '@tauri-apps/api/core';
import { WebviewWindow } from '@tauri-apps/api/webviewWindow';
import {
  CircleStop,
  FolderOpen,
  Gauge,
  Headset,
  Image as ImageIcon,
  Pause,
  Play,
  RefreshCcw,
  RotateCcw,
  RotateCw,
  ScanLine,
  Sparkles,
} from 'lucide-vue-next';
import { onMounted, ref } from 'vue';
import { useDrawing } from '../composables/useDrawing';

const {
  config,
  sourcePath,
  plan,
  status,
  canvasEl,
  busy,
  error,
  previewZoom,
  needsPrepare,
  sourceUrl,
  progressPercent,
  statusKey,
  stageLabel,
  chooseImage,
  prepare,
  start,
  togglePause,
  stop,
  resetConfig,
  resetSection,
} = useDrawing();

const overlayOpen = ref(false);

// 组件重挂载时（用户切走再切回来）overlay 窗口可能仍然存在。
// 不同步状态的话 overlayOpen 恒为 false，再次点击会因 label 冲突触发 tauri://error。
onMounted(async () => {
  if (!isTauri()) return;
  try {
    overlayOpen.value = Boolean(await WebviewWindow.getByLabel('drawing-overlay'));
  } catch { /* ignore */ }
});

const openVrOverlay = () => {
  const overlay = new WebviewWindow('drawing-overlay', {
    url: '/?mode=drawing-overlay',
    title: 'VRCDog Drawing Overlay',
    transparent: true,
    decorations: false,
    shadow: false,
    alwaysOnTop: true,
    skipTaskbar: true,
    resizable: true,
    width: 460,
    height: 720,
    minWidth: 320,
    minHeight: 480,
  });
  overlay.once('tauri://created', () => { overlayOpen.value = true; });
  overlay.once('tauri://error', () => { overlayOpen.value = false; });
  overlay.onCloseRequested(() => { overlayOpen.value = false; });
};
</script>

<template>
  <div class="drawing-page">
    <header class="drawing-header">
      <div>
        <div class="drawing-kicker"><ScanLine :size="15" /> {{ $t('drawing.kicker') }}</div>
        <h1>{{ $t('drawing.title') }}</h1>
        <p>{{ $t('drawing.subtitle') }}</p>
      </div>
      <div class="header-actions">
        <span class="status-chip" :class="{ active: status.running, paused: status.paused }">
          <span class="status-dot" />{{ $t(statusKey) }}
        </span>
        <button v-if="isTauri()" class="icon-button" :title="$t('drawing.open_vr')" :disabled="overlayOpen" @click="openVrOverlay">
          <Headset :size="18" />
        </button>
        <button class="icon-button" :title="$t('drawing.reset')" :disabled="status.running" @click="resetConfig">
          <RotateCcw :size="18" />
        </button>
      </div>
    </header>

    <div class="drawing-workspace">
      <section class="preview-section">
        <div class="section-toolbar">
          <div>
            <strong>{{ $t('drawing.preview') }}</strong>
            <span v-if="plan">{{ plan.width }} x {{ plan.height }}</span>
          </div>
          <div class="toolbar-actions">
            <label class="zoom-control" :title="$t('drawing.preview_zoom')">
              <Gauge :size="16" />
              <input v-model.number="previewZoom" type="range" min="0.6" max="1.8" step="0.1">
              <span>{{ previewZoom.toFixed(1) }}x</span>
            </label>
            <button class="secondary-button" :disabled="status.running" @click="chooseImage">
              <FolderOpen :size="17" /> {{ $t('drawing.open_image') }}
            </button>
          </div>
        </div>

        <div class="preview-stage" :class="{ empty: !plan }">
          <img v-if="sourceUrl && !plan" :src="sourceUrl" alt="" class="source-preview">
          <canvas ref="canvasEl" />
          <div v-if="!sourcePath" class="empty-state">
            <ImageIcon :size="44" stroke-width="1.5" />
            <strong>{{ $t('drawing.empty_title') }}</strong>
          </div>
          <div v-else-if="busy" class="processing-state">
            <RefreshCcw :size="28" class="spin" />
            <strong>{{ stageLabel || $t('drawing.processing') }}</strong>
          </div>
        </div>

        <div v-if="busy && stageLabel" class="stage-line">{{ stageLabel }} · {{ progressPercent }}%</div>
        <div class="metrics-row">
          <div><span>{{ $t('drawing.strokes') }}</span><strong>{{ plan?.strokes.length || 0 }}</strong></div>
          <div><span>{{ $t('drawing.points') }}</span><strong>{{ plan?.total_points || 0 }}</strong></div>
          <div><span>{{ $t('drawing.progress') }}</span><strong>{{ progressPercent }}%</strong></div>
        </div>
        <div class="progress-track"><span :style="{ width: `${progressPercent}%` }" /></div>
        <div v-if="error || status.last_error" class="error-banner">{{ error || status.last_error }}</div>
      </section>

      <aside class="control-panel">
        <div class="panel-scroll">
          <section class="control-section">
            <div class="control-heading">
              <Sparkles :size="17" /><strong>{{ $t('drawing.processing_section') }}</strong>
              <button class="section-reset" :title="$t('drawing.reset_section')" :disabled="status.running" @click="resetSection('processing')">
                <RotateCw :size="14" />
              </button>
            </div>
            <label class="field full">
              <span>{{ $t('drawing.mode') }}</span>
              <select v-model="config.mode" :disabled="status.running">
                <option value="lineart">{{ $t('drawing.mode_lineart') }}</option>
                <option value="edges">{{ $t('drawing.mode_edges') }}</option>
                <option value="dither">{{ $t('drawing.mode_dither') }}</option>
                <option value="ai">{{ $t('drawing.mode_ai') }}</option>
              </select>
            </label>

            <template v-if="config.mode === 'ai'">
              <div class="field-grid">
                <label class="field full">
                  <span>{{ $t('drawing.ai_model') }}</span>
                  <select v-model="config.ai_model" :disabled="status.running">
                    <option value="image-to-line">{{ $t('drawing.ai_model_imageline') }}</option>
                    <option value="anime2sketch">{{ $t('drawing.ai_model_anime2sketch') }}</option>
                  </select>
                </label>
                <label class="field"><span>{{ $t('drawing.contrast') }} <b>{{ config.contrast.toFixed(2) }}</b></span><input v-model.number="config.contrast" type="range" min="0.5" max="3" step="0.05" :disabled="status.running"></label>
                <label class="field"><span>{{ $t('drawing.artifact_removal') }} <b>{{ Math.round(config.artifact_removal * 100) }}%</b></span><input v-model.number="config.artifact_removal" type="range" min="0" max="1" step="0.05" :disabled="status.running"></label>
                <label class="field"><span>{{ $t('drawing.model_size') }} <b>{{ config.model_size }}</b></span><input v-model.number="config.model_size" type="range" min="128" max="1024" step="32" :disabled="status.running"></label>
              </div>
            </template>

            <template v-else>
              <div class="field-grid">
                <label class="field"><span>{{ $t('drawing.threshold') }} <b>{{ config.threshold }}</b></span><input v-model.number="config.threshold" type="range" min="10" max="245" step="1" :disabled="status.running"></label>
                <label class="field"><span>{{ $t('drawing.blur') }} <b>{{ config.blur.toFixed(1) }}</b></span><input v-model.number="config.blur" type="range" min="0" max="5" step="0.1" :disabled="status.running"></label>
                <label class="field"><span>{{ $t('drawing.resolution') }} <b>{{ config.max_dimension }}</b></span><input v-model.number="config.max_dimension" type="range" min="128" max="768" step="32" :disabled="status.running"></label>
                <label class="field"><span>{{ $t('drawing.prune') }} <b>{{ config.prune_length }}</b></span><input v-model.number="config.prune_length" type="range" min="0" max="16" step="1" :disabled="status.running"></label>
              </div>
            </template>

            <div class="toggle-row">
              <label><input v-model="config.invert" type="checkbox" :disabled="status.running"><span>{{ $t('drawing.invert') }}</span></label>
              <label><input v-model="config.bridge_gaps" type="checkbox" :disabled="status.running"><span>{{ $t('drawing.bridge_gaps') }}</span></label>
              <label><input v-model="config.optimize_path" type="checkbox" :disabled="status.running"><span>{{ $t('drawing.optimize_path') }}</span></label>
            </div>
          </section>

          <section class="control-section">
            <div class="control-heading">
              <ScanLine :size="17" /><strong>{{ $t('drawing.stroke_section') }}</strong>
              <button class="section-reset" :title="$t('drawing.reset_section')" :disabled="status.running" @click="resetSection('stroke')">
                <RotateCw :size="14" />
              </button>
            </div>
            <div class="field-grid">
              <label class="field"><span>{{ $t('drawing.min_length') }} <b>{{ config.min_stroke_length }}</b></span><input v-model.number="config.min_stroke_length" type="range" min="2" max="80" step="1" :disabled="status.running"></label>
              <label class="field"><span>{{ $t('drawing.smoothing') }} <b>{{ config.smooth_window }}</b></span><input v-model.number="config.smooth_window" type="range" min="1" max="11" step="2" :disabled="status.running"></label>
              <label class="field"><span>{{ $t('drawing.simplify') }} <b>{{ config.simplify_epsilon.toFixed(1) }}</b></span><input v-model.number="config.simplify_epsilon" type="range" min="0" max="6" step="0.1" :disabled="status.running"></label>
              <label class="field"><span>{{ $t('drawing.merge') }} <b>{{ config.merge_distance.toFixed(1) }}</b></span><input v-model.number="config.merge_distance" type="range" min="0" max="12" step="0.5" :disabled="status.running"></label>
            </div>
            <div class="toggle-row">
              <label><input v-model="config.two_opt_path" type="checkbox" :disabled="status.running"><span>{{ $t('drawing.two_opt_path') }}</span></label>
            </div>
          </section>

          <section class="control-section">
            <div class="control-heading">
              <Gauge :size="17" /><strong>{{ $t('drawing.execution_section') }}</strong>
              <button class="section-reset" :title="$t('drawing.reset_section')" :disabled="status.running" @click="resetSection('execution')">
                <RotateCw :size="14" />
              </button>
            </div>
            <div class="field-grid">
              <label class="field"><span>{{ $t('drawing.sensitivity') }} <b>{{ config.sensitivity.toFixed(1) }}</b></span><input v-model.number="config.sensitivity" type="range" min="0.2" max="4" step="0.1" :disabled="status.running"></label>
              <label class="field"><span>{{ $t('drawing.vertical_stretch') }} <b>{{ config.vertical_stretch.toFixed(2) }}</b></span><input v-model.number="config.vertical_stretch" type="range" min="0.5" max="2" step="0.05" :disabled="status.running"></label>
              <label class="field"><span>{{ $t('drawing.lift_speed') }} <b>{{ Math.round(config.lift_speed * 100) }}%</b></span><input v-model.number="config.lift_speed" type="range" min="0.2" max="3" step="0.1" :disabled="status.running"></label>
              <label class="field"><span>{{ $t('drawing.point_delay') }} <b>{{ config.point_delay_ms }} ms</b></span><input v-model.number="config.point_delay_ms" type="range" min="1" max="120" step="1"></label>
              <label class="field"><span>{{ $t('drawing.start_delay') }} <b>{{ (config.start_delay_ms / 1000).toFixed(1) }} s</b></span><input v-model.number="config.start_delay_ms" type="range" min="0" max="10000" step="250"></label>
              <label class="field"><span>{{ $t('drawing.canvas_size') }} <b>{{ config.canvas_size_px === 0 ? $t('drawing.auto') : config.canvas_size_px }}</b></span><input v-model.number="config.canvas_size_px" type="range" min="0" max="4096" step="64" :disabled="status.running"></label>
              <label class="field"><span>{{ $t('drawing.pen_settle') }} <b>{{ config.pen_settle_ms }} ms</b></span><input v-model.number="config.pen_settle_ms" type="range" min="0" max="120" step="1" :disabled="status.running"></label>
            </div>
            <div class="toggle-row">
              <label><input v-model="config.focus_vrchat" type="checkbox"><span>{{ $t('drawing.focus_vrchat') }}</span></label>
              <label><input v-model="config.hotkeys_enabled" type="checkbox"><span>{{ $t('drawing.hotkeys') }}</span></label>
            </div>
          </section>
        </div>

        <footer class="control-footer">
          <div class="hotkey-strip">
            <span><kbd>F9</kbd>{{ $t('drawing.start') }}</span>
            <span><kbd>F10</kbd>{{ $t('drawing.stop') }}</span>
            <span><kbd>F11</kbd>{{ $t('drawing.pause_resume') }}</span>
          </div>
          <div class="primary-actions">
            <button class="secondary-button" :disabled="!sourcePath || busy || status.running" @click="prepare">
              <RefreshCcw :size="17" :class="{ spin: busy }" /> {{ needsPrepare ? $t('drawing.regenerate') : $t('drawing.generate') }}
            </button>
            <button v-if="!status.running" class="primary-button" :disabled="!plan || busy || needsPrepare" @click="start">
              <Play :size="19" fill="currentColor" /> {{ $t('drawing.start') }}
            </button>
            <template v-else>
              <button class="secondary-button" @click="togglePause">
                <Play v-if="status.paused" :size="18" fill="currentColor" />
                <Pause v-else :size="18" fill="currentColor" />
                {{ status.paused ? $t('drawing.resume') : $t('drawing.pause') }}
              </button>
              <button class="stop-button" @click="stop"><CircleStop :size="18" /> {{ $t('drawing.stop') }}</button>
            </template>
          </div>
        </footer>
      </aside>
    </div>
  </div>
</template>

<style scoped>
.drawing-page { height: 100%; min-height: 0; display: flex; flex-direction: column; color: var(--theme-text, #54240f); background: var(--theme-bg-main, #fff9eb); }
.drawing-header { min-height: 92px; padding: 18px 24px; display: flex; align-items: center; justify-content: space-between; gap: 20px; border-bottom: 1px solid var(--theme-border-soft, #eadfc8); background: var(--theme-surface, #fffdf7); }
.drawing-kicker { display: flex; align-items: center; gap: 7px; color: var(--theme-primary, #df7600); font-size: 12px; font-weight: 800; }
h1 { margin: 3px 0 1px; font-size: 22px; line-height: 1.2; letter-spacing: 0; color: var(--theme-text-strong, #401b0b); }
.drawing-header p { margin: 0; font-size: 12px; color: var(--theme-text-soft, #846251); }
.header-actions, .toolbar-actions, .primary-actions { display: flex; align-items: center; gap: 9px; }
.status-chip { height: 34px; display: inline-flex; align-items: center; gap: 8px; padding: 0 12px; border: 1px solid var(--theme-border-soft, #eadfc8); border-radius: 6px; background: var(--theme-bg-main, #fff9eb); font-size: 12px; font-weight: 800; }
.status-dot { width: 8px; height: 8px; border-radius: 50%; background: #a8978d; }
.status-chip.active .status-dot { background: #16a34a; box-shadow: 0 0 0 3px rgba(22,163,74,.13); }
.status-chip.paused .status-dot { background: #d97706; }
.icon-button { width: 36px; height: 36px; display: grid; place-items: center; border: 1px solid var(--theme-border-soft, #eadfc8); border-radius: 6px; background: var(--theme-surface, #fffdf7); color: var(--theme-text-soft, #846251); }
.icon-button:hover:not(:disabled) { color: var(--theme-primary, #df7600); border-color: var(--theme-primary, #df7600); }
.drawing-workspace { min-height: 0; flex: 1; display: grid; grid-template-columns: minmax(360px, 1.2fr) minmax(390px, .8fr); }
.preview-section { min-width: 0; min-height: 0; padding: 18px 20px; display: flex; flex-direction: column; border-right: 1px solid var(--theme-border-soft, #eadfc8); }
.section-toolbar { min-height: 38px; display: flex; align-items: center; justify-content: space-between; gap: 16px; margin-bottom: 12px; }
.section-toolbar > div:first-child { display: flex; align-items: baseline; gap: 10px; }
.section-toolbar strong, .control-heading strong { font-size: 13px; color: var(--theme-text-strong, #401b0b); }
.section-toolbar span { font-size: 11px; color: var(--theme-text-muted, #a58a79); font-family: ui-monospace, monospace; }
.zoom-control { height: 34px; display: flex; align-items: center; gap: 7px; color: var(--theme-text-soft, #846251); }
.zoom-control input { width: 92px; }
.preview-stage { position: relative; flex: 1; min-height: 300px; overflow: hidden; border: 1px solid var(--theme-border-soft, #eadfc8); border-radius: 6px; background-color: #fffdf7; background-image: linear-gradient(#efe7d7 1px, transparent 1px), linear-gradient(90deg, #efe7d7 1px, transparent 1px); background-size: 20px 20px; }
.preview-stage canvas { position: absolute; inset: 0; width: 100%; height: 100%; display: block; }
.source-preview { position: absolute; inset: 20px; width: calc(100% - 40px); height: calc(100% - 40px); object-fit: contain; opacity: .6; }
.empty-state, .processing-state { position: absolute; inset: 0; z-index: 2; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 10px; color: #9d806c; background: rgba(255,253,247,.86); }
.empty-state strong, .processing-state strong { font-size: 13px; }
.stage-line { margin-top: 8px; font-size: 11px; font-weight: 700; color: var(--theme-primary, #df7600); font-family: ui-monospace, monospace; }
.metrics-row { display: grid; grid-template-columns: repeat(3, 1fr); margin-top: 12px; border: 1px solid var(--theme-border-soft, #eadfc8); border-radius: 6px; background: var(--theme-surface, #fffdf7); }
.metrics-row > div { min-height: 48px; display: flex; align-items: center; justify-content: space-between; padding: 0 14px; border-right: 1px solid var(--theme-border-soft, #eadfc8); }
.metrics-row > div:last-child { border-right: 0; }
.metrics-row span { font-size: 11px; color: var(--theme-text-muted, #a58a79); }
.metrics-row strong { font-size: 14px; color: var(--theme-text-strong, #401b0b); }
.progress-track { height: 5px; margin-top: 8px; overflow: hidden; border-radius: 3px; background: var(--theme-border-soft, #eadfc8); }
.progress-track span { display: block; height: 100%; background: var(--theme-primary, #df7600); transition: width .2s ease; }
.error-banner { margin-top: 8px; padding: 9px 12px; border: 1px solid #fecaca; border-radius: 6px; background: #fff1f2; color: #b42318; font-size: 12px; }
.control-panel { min-width: 0; min-height: 0; display: flex; flex-direction: column; background: var(--theme-surface, #fffdf7); }
.panel-scroll { min-height: 0; flex: 1; overflow-y: auto; }
.control-section { padding: 17px 20px; border-bottom: 1px solid var(--theme-border-soft, #eadfc8); }
.control-heading { display: flex; align-items: center; gap: 8px; margin-bottom: 14px; color: var(--theme-primary, #df7600); }
.control-heading .section-reset { margin-left: auto; width: 26px; height: 26px; display: grid; place-items: center; border: 1px solid var(--theme-border-soft, #eadfc8); border-radius: 6px; background: var(--theme-bg-main, #fff9eb); color: var(--theme-text-soft, #846251); }
.control-heading .section-reset:hover:not(:disabled) { color: var(--theme-primary, #df7600); border-color: var(--theme-primary, #df7600); }
.field-grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 13px 18px; }
.field { min-width: 0; display: flex; flex-direction: column; gap: 7px; }
.field.full { margin-bottom: 14px; grid-column: 1 / -1; }
.field > span { display: flex; justify-content: space-between; gap: 8px; font-size: 11px; font-weight: 700; color: var(--theme-text-soft, #846251); }
.field b { color: var(--theme-text-strong, #401b0b); font-family: ui-monospace, monospace; font-weight: 800; }
select { height: 36px; padding: 0 10px; border: 1px solid var(--theme-border-soft, #eadfc8); border-radius: 6px; background: var(--theme-bg-main, #fff9eb); color: var(--theme-text-strong, #401b0b); font-size: 12px; }
input[type="range"] { width: 100%; accent-color: var(--theme-primary, #df7600); }
.toggle-row { display: flex; flex-wrap: wrap; gap: 8px 16px; margin-top: 14px; }
.toggle-row label { display: inline-flex; align-items: center; gap: 7px; font-size: 11px; font-weight: 700; color: var(--theme-text-soft, #846251); }
.toggle-row input { accent-color: var(--theme-primary, #df7600); }
.control-footer { padding: 12px 20px 16px; border-top: 1px solid var(--theme-border-soft, #eadfc8); background: var(--theme-surface, #fffdf7); }
.hotkey-strip { display: flex; gap: 12px; margin-bottom: 10px; font-size: 10px; color: var(--theme-text-muted, #a58a79); }
.hotkey-strip span { display: inline-flex; align-items: center; gap: 5px; }
kbd { min-width: 28px; padding: 2px 5px; border: 1px solid var(--theme-border-soft, #eadfc8); border-bottom-width: 2px; border-radius: 4px; background: var(--theme-bg-main, #fff9eb); color: var(--theme-text-strong, #401b0b); text-align: center; }
.primary-actions { justify-content: flex-end; }
.primary-button, .secondary-button, .stop-button { min-height: 36px; display: inline-flex; align-items: center; justify-content: center; gap: 7px; padding: 0 14px; border-radius: 6px; font-size: 12px; font-weight: 800; transition: background-color .15s ease, border-color .15s ease, opacity .15s ease; }
.primary-button { border: 1px solid var(--theme-primary, #df7600); background: var(--theme-primary, #df7600); color: white; }
.primary-button:hover:not(:disabled) { background: var(--theme-primary-hover, #d97706); }
.secondary-button { border: 1px solid var(--theme-border-soft, #eadfc8); background: var(--theme-bg-main, #fff9eb); color: var(--theme-text-strong, #401b0b); }
.stop-button { border: 1px solid #ef4444; background: #ef4444; color: white; }
button:disabled, input:disabled, select:disabled { opacity: .45; cursor: not-allowed; }
.spin { animation: spin 1s linear infinite; }
@keyframes spin { to { transform: rotate(360deg); } }
@media (max-width: 940px) {
  .drawing-workspace { grid-template-columns: 1fr; overflow-y: auto; }
  .preview-section { min-height: 560px; border-right: 0; border-bottom: 1px solid var(--theme-border-soft, #eadfc8); }
  .control-panel { min-height: 620px; }
}
@media (max-width: 620px) {
  .drawing-header { align-items: flex-start; padding: 14px; }
  .drawing-header p { display: none; }
  .drawing-workspace { min-width: 320px; }
  .preview-section, .control-section { padding-left: 14px; padding-right: 14px; }
  .section-toolbar { align-items: flex-start; flex-direction: column; }
  .field-grid { grid-template-columns: 1fr; }
  .metrics-row > div { padding: 0 8px; }
  .hotkey-strip { flex-wrap: wrap; }
}
</style>
