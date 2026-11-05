<script setup lang="ts">
import { isTauri } from '@tauri-apps/api/core';
import { WebviewWindow } from '@tauri-apps/api/webviewWindow';
import {
  CircleStop,
  Eraser,
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
import { onMounted, onUnmounted, ref } from 'vue';
import { listen } from '@tauri-apps/api/event';
import { useDrawing } from '../composables/useDrawing';
import { ModelRuntimeApi, type RuntimeModelStatus } from '../api';

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
  previewTransform,
  mask,
} = useDrawing();

type MaskPoint = { x: number; y: number };

// 顶层 ref 才会在模板里自动解包，这里显式取名，模板就能写 v-model="maskBrush" 而不是 .value。
const maskEnabled = mask.enabled;
const maskTool = mask.tool;
const maskBrush = mask.brushSize;
const maskTolerance = mask.tolerance;

const painting = ref(false);
const lastMaskPoint = ref<MaskPoint | null>(null);
const rectStart = ref<MaskPoint | null>(null);

// 指针坐标 → 图像像素坐标，和笔画渲染共用同一个变换，涂哪里就排除哪里。
const toImagePoint = (event: PointerEvent): MaskPoint => {
  const canvas = canvasEl.value;
  if (!canvas) return { x: 0, y: 0 };
  const rect = canvas.getBoundingClientRect();
  const transform = previewTransform.value;
  if (!transform.scale) return { x: 0, y: 0 };
  return {
    x: (event.clientX - rect.left - transform.offsetX) / transform.scale,
    y: (event.clientY - rect.top - transform.offsetY) / transform.scale,
  };
};

const onMaskPointerDown = async (event: PointerEvent) => {
  if (!mask.enabled.value || status.value.running || !plan.value) return;
  event.preventDefault();
  const point = toImagePoint(event);
  const tool = mask.tool.value;
  painting.value = true;
  lastMaskPoint.value = point;
  rectStart.value = point;
  if (tool === 'keep' || tool === 'erase') mask.paintCircle(point.x, point.y, tool === 'keep');
  else if (tool === 'wand-erase' || tool === 'wand-keep') {
    await mask.wandPick(point.x, point.y, tool === 'wand-keep');
    painting.value = false;
  }
};

const onMaskPointerMove = (event: PointerEvent) => {
  if (!painting.value || !lastMaskPoint.value) return;
  const tool = mask.tool.value;
  if (tool !== 'keep' && tool !== 'erase') return;
  const point = toImagePoint(event);
  mask.paintLine(lastMaskPoint.value, point, tool === 'keep');
  lastMaskPoint.value = point;
};

const onMaskPointerUp = (event: PointerEvent) => {
  if (!painting.value) return;
  painting.value = false;
  const tool = mask.tool.value;
  if ((tool === 'rect-keep' || tool === 'rect-erase') && rectStart.value) {
    mask.paintRect(rectStart.value, toImagePoint(event), tool === 'rect-keep');
  }
  rectStart.value = null;
  lastMaskPoint.value = null;
};

const overlayOpen = ref(false);
const regionOpen = ref(false);
let unlistenRegion: (() => void) | null = null;

/**
 * 打开全屏冻结选区：抓一帧桌面 → 拖框 → 选区的物理像素就是画布尺寸。
 * 截图失败（DXGI 被远程协助占用等）由选区窗口自己提示并允许重试，
 * 这里只负责把窗口拉起来。
 */
const openRegionPicker = async () => {
  if (!isTauri()) return;
  const label = 'drawing-region';
  try {
    const existing = await WebviewWindow.getByLabel(label);
    if (existing) {
      await existing.setFocus();
      return;
    }
  } catch { /* ignore */ }
  const query = plan.value ? `&pw=${plan.value.width}&ph=${plan.value.height}` : '';
  const win = new WebviewWindow(label, {
    url: `/?mode=drawing-region${query}`,
    title: 'VRCDog Region Picker',
    fullscreen: true,
    decorations: false,
    alwaysOnTop: true,
    skipTaskbar: true,
    resizable: false,
  });
  win.once('tauri://created', () => { regionOpen.value = true; });
  win.once('tauri://error', () => { regionOpen.value = false; });
  win.once('tauri://destroyed', () => { regionOpen.value = false; });
};

const clearRegion = () => {
  config.value.canvas_width_px = 0;
  config.value.canvas_height_px = 0;
  needsPrepare.value = Boolean(plan.value);
};

// U²-Netp 抠图权重不打进安装包：这里查状态、按需下载，下载完标记需要重新生成。
const matting = ref<RuntimeModelStatus | null>(null);
const mattingBusy = ref(false);

const refreshMatting = async () => {
  if (!isTauri()) return;
  try {
    matting.value = await ModelRuntimeApi.getMattingStatus();
  } catch {
    matting.value = null;
  }
};

const downloadMatting = async () => {
  mattingBusy.value = true;
  try {
    matting.value = await ModelRuntimeApi.downloadMatting();
    if (plan.value) needsPrepare.value = true;
  } catch (value) {
    error.value = String((value as any)?.message || value);
  } finally {
    mattingBusy.value = false;
  }
};

// 组件重挂载时（用户切走再切回来）overlay 窗口可能仍然存在。
// 不同步状态的话 overlayOpen 恒为 false，再次点击会因 label 冲突触发 tauri://error。
onMounted(async () => {
  if (!isTauri()) return;
  try {
    overlayOpen.value = Boolean(await WebviewWindow.getByLabel('drawing-overlay'));
    regionOpen.value = Boolean(await WebviewWindow.getByLabel('drawing-region'));
  } catch { /* ignore */ }
  void refreshMatting();
  try {
    const unlisten = await listen<{ width: number; height: number }>('drawing_region_selected', (event) => {
      const width = Math.round(event.payload?.width || 0);
      const height = Math.round(event.payload?.height || 0);
      if (width < 16 || height < 16) return;
      config.value.canvas_width_px = width;
      config.value.canvas_height_px = height;
      // 画布尺寸变了 → 展平密度和落点都要重算，标记成需要重新生成。
      needsPrepare.value = Boolean(plan.value);
    });
    unlistenRegion = unlisten;
  } catch { /* 非 Tauri 预览模式下没有事件总线 */ }
});

onUnmounted(() => {
  unlistenRegion?.();
  unlistenRegion = null;
});

const openVrOverlay = () => {
  if (overlayOpen.value) return;
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

        <div
          class="preview-stage"
          :class="{ empty: !plan, masking: maskEnabled && !status.running }"
          @pointerdown="onMaskPointerDown"
          @pointermove="onMaskPointerMove"
          @pointerup="onMaskPointerUp"
          @pointercancel="onMaskPointerUp"
        >
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
                <option value="anime">{{ $t('drawing.mode_anime') }}</option>
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
                <label v-if="config.remove_background" class="field"><span>{{ $t('drawing.bg_tolerance') }} <b>{{ Math.round(config.bg_tolerance * 100) }}%</b></span><input v-model.number="config.bg_tolerance" type="range" min="0.1" max="0.9" step="0.05" :disabled="status.running"></label>
              </div>
            </template>

            <template v-else>
              <div class="field-grid">
                <label class="field"><span>{{ $t('drawing.threshold') }} <b>{{ config.threshold }}</b></span><input v-model.number="config.threshold" type="range" min="10" max="245" step="1" :disabled="status.running"></label>
                <label class="field"><span>{{ $t('drawing.blur') }} <b>{{ config.blur.toFixed(1) }}</b></span><input v-model.number="config.blur" type="range" min="0" max="5" step="0.1" :disabled="status.running"></label>
                <label class="field"><span>{{ $t('drawing.resolution') }} <b>{{ config.max_dimension }}</b></span><input v-model.number="config.max_dimension" type="range" min="128" max="768" step="32" :disabled="status.running"></label>
                <label class="field"><span>{{ $t('drawing.prune') }} <b>{{ config.prune_length }}</b></span><input v-model.number="config.prune_length" type="range" min="0" max="16" step="1" :disabled="status.running"></label>
                <label v-if="config.remove_background" class="field"><span>{{ $t('drawing.bg_tolerance') }} <b>{{ Math.round(config.bg_tolerance * 100) }}%</b></span><input v-model.number="config.bg_tolerance" type="range" min="0.1" max="0.9" step="0.05" :disabled="status.running"></label>
              </div>
            </template>

            <div class="toggle-row">
              <label><input v-model="config.remove_background" type="checkbox" :disabled="status.running"><span>{{ $t('drawing.remove_background') }}</span></label>
              <label><input v-model="config.invert" type="checkbox" :disabled="status.running"><span>{{ $t('drawing.invert') }}</span></label>
              <label><input v-model="config.bridge_gaps" type="checkbox" :disabled="status.running"><span>{{ $t('drawing.bridge_gaps') }}</span></label>
              <label><input v-model="config.optimize_path" type="checkbox" :disabled="status.running"><span>{{ $t('drawing.optimize_path') }}</span></label>
            </div>

            <div v-if="config.remove_background" class="matting-row">
              <span class="matting-state" :class="{ ok: matting?.valid }">
                <Sparkles :size="12" />
                {{ matting?.valid ? $t('drawing.matting_ready') : $t('drawing.matting_model') }}
              </span>
              <button v-if="!matting?.valid" class="secondary-button" :disabled="mattingBusy || status.running" @click="downloadMatting">
                <RefreshCcw :size="14" :class="{ spin: mattingBusy }" />
                {{ mattingBusy ? $t('drawing.matting_downloading') : $t('drawing.matting_missing') }}
              </button>
            </div>
          </section>

          <section class="control-section">
            <div class="control-heading">
              <Eraser :size="17" /><strong>{{ $t('drawing.mask_section') }}</strong>
              <button class="section-reset" :title="$t('drawing.reset_section')" :disabled="status.running" @click="mask.resetMask()">
                <RotateCw :size="14" />
              </button>
            </div>
            <div class="toggle-row" style="margin-top: 0; margin-bottom: 12px;">
              <label><input v-model="maskEnabled" type="checkbox" :disabled="status.running || !plan"><span>{{ $t('drawing.mask_enable') }}</span></label>
            </div>
            <template v-if="maskEnabled && plan">
              <div class="tool-grid">
                <button
                  v-for="entry in ([
                    ['keep', $t('drawing.mask_tool_keep')],
                    ['erase', $t('drawing.mask_tool_erase')],
                    ['wand-erase', $t('drawing.mask_tool_wand_erase')],
                    ['wand-keep', $t('drawing.mask_tool_wand_keep')],
                    ['rect-keep', $t('drawing.mask_tool_rect_keep')],
                    ['rect-erase', $t('drawing.mask_tool_rect_erase')],
                  ] as const)"
                  :key="entry[0]"
                  class="tool-button"
                  :class="{ active: maskTool === entry[0] }"
                  @click="maskTool = entry[0]"
                >{{ entry[1] }}</button>
              </div>
              <div class="field-grid" style="margin-top: 13px;">
                <label class="field"><span>{{ $t('drawing.mask_brush') }} <b>{{ maskBrush }}</b></span><input v-model.number="maskBrush" type="range" min="2" max="120" step="2" :disabled="status.running"></label>
                <label class="field"><span>{{ $t('drawing.mask_tolerance') }} <b>{{ Math.round(maskTolerance * 100) }}%</b></span><input v-model.number="maskTolerance" type="range" min="0.02" max="0.9" step="0.02" :disabled="status.running"></label>
              </div>
              <div class="toggle-row">
                <button class="secondary-button" :disabled="status.running" @click="mask.invertMask()">{{ $t('drawing.mask_invert') }}</button>
                <button class="secondary-button" :disabled="status.running" @click="mask.resetMask()">{{ $t('drawing.mask_clear') }}</button>
                <button v-if="needsPrepare" class="secondary-button" :disabled="busy || status.running" @click="prepare()">{{ $t('drawing.regenerate') }}</button>
              </div>
              <p class="mask-hint">{{ $t('drawing.mask_hint') }}</p>
            </template>
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
              <label><input v-model="config.curve_fit" type="checkbox" :disabled="status.running"><span>{{ $t('drawing.curve_fit') }}</span></label>
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
              <label class="field"><span>{{ $t('drawing.min_button_down') }} <b>{{ config.min_button_down_ms }} ms</b></span><input v-model.number="config.min_button_down_ms" type="range" min="0" max="3000" step="50" :disabled="status.running"></label>
            </div>

            <div class="region-row">
              <button class="secondary-button" :disabled="status.running" @click="openRegionPicker">
                <ScanLine :size="15" /> {{ $t('drawing.region_pick') }}
              </button>
              <span class="region-readout">
                <template v-if="config.canvas_width_px > 0 && config.canvas_height_px > 0">
                  {{ $t('drawing.region_active', { width: config.canvas_width_px, height: config.canvas_height_px }) }}
                  <button class="link-button" :disabled="status.running" @click="clearRegion">{{ $t('drawing.region_clear') }}</button>
                </template>
                <span v-else class="region-none">{{ $t('drawing.region_none') }}</span>
              </span>
            </div>
            <p class="region-note">{{ $t('drawing.region_note') }}</p>
            <div class="toggle-row">
              <label><input v-model="config.focus_vrchat" type="checkbox"><span>{{ $t('drawing.focus_vrchat') }}</span></label>
              <label><input v-model="config.hotkeys_enabled" type="checkbox"><span>{{ $t('drawing.hotkeys') }}</span></label>
            </div>
          </section>
        </div>

        <footer class="control-footer">
          <div class="hotkey-strip">
            <span :class="{ active: status.last_hotkey === 'F9' }"><kbd>F9</kbd>{{ $t('drawing.start') }}</span>
            <span :class="{ active: status.last_hotkey === 'F10' }"><kbd>F10</kbd>{{ $t('drawing.stop') }}</span>
            <span :class="{ active: status.last_hotkey === 'F11' }"><kbd>F11</kbd>{{ $t('drawing.pause_resume') }}</span>
            <span v-if="status.last_event" class="hotkey-hint">({{ status.last_event }})</span>
          </div>
          <div class="primary-actions">
            <button class="secondary-button" :disabled="!sourcePath || busy || status.running" @click="prepare">
              <RefreshCcw :size="17" :class="{ spin: busy }" /> {{ needsPrepare ? $t('drawing.regenerate') : $t('drawing.generate') }}
            </button>
            <button v-if="!status.running" class="primary-button" :disabled="(!plan && !sourcePath) || busy" @click="start">
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
.tool-grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 7px; }
.tool-button { min-height: 32px; padding: 0 10px; border: 1px solid var(--theme-border-soft, #eadfc8); border-radius: 6px; background: var(--theme-bg-main, #fff9eb); color: var(--theme-text-soft, #846251); font-size: 11px; font-weight: 800; }
.tool-button:hover { color: var(--theme-primary, #df7600); border-color: var(--theme-primary, #df7600); }
.tool-button.active { border-color: var(--theme-primary, #df7600); background: var(--theme-primary, #df7600); color: white; }
.mask-hint { margin: 12px 0 0; font-size: 11px; line-height: 1.5; color: var(--theme-text-muted, #a58a79); }
.preview-stage.masking { cursor: crosshair; touch-action: none; }
.matting-row { display: flex; align-items: center; justify-content: space-between; gap: 10px; margin-top: 14px; }
.region-row { display: flex; align-items: center; gap: 12px; margin-top: 14px; flex-wrap: wrap; }
.region-readout { font-size: 11px; font-weight: 700; color: var(--theme-text-soft, #846251); font-family: ui-monospace, monospace; }
.region-readout .region-none { font-weight: 600; color: var(--theme-text-muted, #a58a79); font-family: inherit; }
.link-button { border: 0; background: none; padding: 0 2px; font-size: 11px; font-weight: 800; color: var(--theme-primary, #df7600); text-decoration: underline; cursor: pointer; }
.region-note { margin: 8px 0 0; font-size: 11px; line-height: 1.55; color: var(--theme-text-muted, #a58a79); }
.matting-state { display: inline-flex; align-items: center; gap: 6px; font-size: 11px; font-weight: 700; color: var(--theme-text-muted, #a58a79); }
.matting-state.ok { color: #16a34a; }
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
