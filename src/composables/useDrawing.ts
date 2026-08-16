import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { useStorage } from '@vueuse/core';
import { convertFileSrc, isTauri } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/plugin-dialog';
import {
  DrawingApi,
  type DrawingConfig,
  type DrawingStatus,
  type PreparedDrawing,
} from '../api';

const defaultConfig: DrawingConfig = {
  mode: 'lineart',
  max_dimension: 512,
  threshold: 150,
  blur: 0.8,
  invert: false,
  bridge_gaps: true,
  prune_length: 4,
  min_stroke_length: 7,
  smooth_window: 3,
  simplify_epsilon: 1.35,
  merge_distance: 3,
  optimize_path: true,
  sensitivity: 1.2,
  vertical_stretch: 1,
  max_step_px: 4,
  point_delay_ms: 28,
  lift_delay_ms: 45,
  start_delay_ms: 1500,
  focus_vrchat: true,
  hotkeys_enabled: true,
  ai_model: 'image-to-line',
  contrast: 1,
  artifact_removal: 0.6,
  model_size: 512,
  lift_speed: 1,
};

const emptyStatus = (): DrawingStatus => ({
  prepared: false,
  running: false,
  paused: false,
  progress: 0,
  current_stroke: 0,
  total_strokes: 0,
  total_points: 0,
  source_path: '',
  last_event: '',
  last_error: '',
  hotkeys_enabled: true,
  hotkeys_available: isTauri(),
  last_hotkey: '',
  last_hotkey_at_ms: 0,
  stage: '',
});

const processingSectionKeys: (keyof DrawingConfig)[] = [
  'mode', 'max_dimension', 'threshold', 'blur', 'invert', 'bridge_gaps', 'prune_length',
  'ai_model', 'contrast', 'artifact_removal', 'model_size',
];
const strokeSectionKeys: (keyof DrawingConfig)[] = [
  'min_stroke_length', 'smooth_window', 'simplify_epsilon', 'merge_distance', 'optimize_path',
];
const executionSectionKeys: (keyof DrawingConfig)[] = [
  'sensitivity', 'vertical_stretch', 'max_step_px', 'point_delay_ms', 'lift_delay_ms',
  'start_delay_ms', 'focus_vrchat', 'hotkeys_enabled', 'lift_speed',
];

export function useDrawing() {
  const { t } = useI18n();

  const config = useStorage<DrawingConfig>('vrcdog.drawing.config', defaultConfig, localStorage, { mergeDefaults: true });
  const sourcePath = useStorage('vrcdog.drawing.source', '');
  const plan = ref<PreparedDrawing | null>(null);
  const status = ref<DrawingStatus>(emptyStatus());
  const canvasEl = ref<HTMLCanvasElement | null>(null);
  const busy = ref(false);
  const error = ref('');
  const previewZoom = ref(1);
  const needsPrepare = ref(false);
  let unlistenStatus: UnlistenFn | null = null;
  let configTimer: number | null = null;

  const sourceUrl = computed(() => sourcePath.value && isTauri() ? convertFileSrc(sourcePath.value) : '');
  const progressPercent = computed(() => Math.round(Math.max(0, Math.min(1, status.value.progress)) * 100));
  const statusKey = computed(() => {
    if (status.value.running && status.value.paused) return 'drawing.status_paused';
    if (status.value.running) return 'drawing.status_running';
    if (status.value.prepared) return 'drawing.status_ready';
    return 'drawing.status_empty';
  });
  const stageLabel = computed(() => (status.value.stage ? t(`drawing.stage_${status.value.stage}`) : ''));

  const cleanConfig = (): DrawingConfig => JSON.parse(JSON.stringify(config.value));

  const showError = (value: unknown) => {
    error.value = String((value as any)?.message || value || t('drawing.error_unknown'));
  };

  const renderPlan = () => {
    const target = canvasEl.value;
    const drawing = plan.value;
    if (!target) return;
    const cssWidth = Math.max(320, target.clientWidth || 640);
    const cssHeight = Math.max(260, target.clientHeight || 480);
    const ratio = window.devicePixelRatio || 1;
    target.width = Math.round(cssWidth * ratio);
    target.height = Math.round(cssHeight * ratio);
    const context = target.getContext('2d');
    if (!context) return;
    context.setTransform(ratio, 0, 0, ratio, 0, 0);
    context.clearRect(0, 0, cssWidth, cssHeight);
    context.fillStyle = '#fffdf7';
    context.fillRect(0, 0, cssWidth, cssHeight);
    if (!drawing) return;
    const padding = 24;
    const scale = Math.min((cssWidth - padding * 2) / drawing.width, (cssHeight - padding * 2) / drawing.height) * previewZoom.value;
    const offsetX = (cssWidth - drawing.width * scale) / 2;
    const offsetY = (cssHeight - drawing.height * scale) / 2;
    context.strokeStyle = '#54240f';
    context.lineWidth = Math.max(1, Math.min(2.2, scale));
    context.lineCap = 'round';
    context.lineJoin = 'round';
    for (const stroke of drawing.strokes) {
      if (stroke.points.length < 2) continue;
      context.beginPath();
      context.moveTo(offsetX + stroke.points[0].x * scale, offsetY + stroke.points[0].y * scale);
      for (const point of stroke.points.slice(1)) {
        context.lineTo(offsetX + point.x * scale, offsetY + point.y * scale);
      }
      context.stroke();
    }
  };

  const chooseImage = async () => {
    const selected = await open({
      multiple: false,
      directory: false,
      filters: [{ name: t('drawing.image_files'), extensions: ['png', 'jpg', 'jpeg', 'webp', 'bmp', 'gif'] }],
    });
    if (typeof selected !== 'string') return;
    sourcePath.value = selected;
    needsPrepare.value = true;
    await prepare();
  };

  const prepare = async () => {
    if (!sourcePath.value || busy.value || status.value.running) return;
    busy.value = true;
    error.value = '';
    try {
      plan.value = await DrawingApi.prepare({ sourcePath: sourcePath.value, config: cleanConfig() });
      status.value = await DrawingApi.getStatus();
      needsPrepare.value = false;
      await nextTick();
      renderPlan();
    } catch (value) {
      showError(value);
    } finally {
      busy.value = false;
    }
  };

  const start = async () => {
    error.value = '';
    try {
      await DrawingApi.setConfig({ config: cleanConfig() });
      status.value = await DrawingApi.start();
    } catch (value) { showError(value); }
  };

  const togglePause = async () => {
    try {
      status.value = status.value.paused ? await DrawingApi.resume() : await DrawingApi.pause();
    } catch (value) { showError(value); }
  };

  const stop = async () => {
    try { status.value = await DrawingApi.stop(); }
    catch (value) { showError(value); }
  };

  const resetConfig = () => {
    config.value = { ...defaultConfig };
    needsPrepare.value = Boolean(sourcePath.value);
  };

  const resetSection = (section: 'processing' | 'stroke' | 'execution') => {
    const keys = section === 'processing' ? processingSectionKeys : section === 'stroke' ? strokeSectionKeys : executionSectionKeys;
    const next = { ...config.value };
    for (const key of keys) {
      (next as any)[key] = (defaultConfig as any)[key];
    }
    config.value = next;
    needsPrepare.value = Boolean(plan.value);
  };

  watch(previewZoom, renderPlan);
  watch(plan, () => nextTick(renderPlan));
  watch(config, () => {
    needsPrepare.value = Boolean(plan.value);
    if (configTimer !== null) window.clearTimeout(configTimer);
    configTimer = window.setTimeout(() => {
      configTimer = null;
      void DrawingApi.setConfig({ config: cleanConfig() }).catch(showError);
    }, 250);
  }, { deep: true });

  const resizeObserver = typeof ResizeObserver === 'undefined' ? null : new ResizeObserver(renderPlan);

  onMounted(async () => {
    if (canvasEl.value) resizeObserver?.observe(canvasEl.value);
    try {
      [status.value, plan.value] = await Promise.all([DrawingApi.getStatus(), DrawingApi.getPlan()]);
      if (plan.value) sourcePath.value = plan.value.source_path;
    } catch (value) { showError(value); }
    unlistenStatus = await listen<DrawingStatus>('vrdrawing_status', event => {
      status.value = event.payload;
    });
    await nextTick();
    renderPlan();
  });

  onUnmounted(() => {
    resizeObserver?.disconnect();
    if (configTimer !== null) window.clearTimeout(configTimer);
    unlistenStatus?.();
  });

  return {
    t,
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
    renderPlan,
  };
}
