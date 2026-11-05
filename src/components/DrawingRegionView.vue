<script setup lang="ts">
import { emit } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { onMounted, onUnmounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { Check, X } from 'lucide-vue-next';
import { DrawingApi } from '../api';

const { t } = useI18n();

type Rect = { left: number; top: number; width: number; height: number };

const shot = ref<{ width: number; height: number; dataUrl: string } | null>(null);
const loading = ref(true);
const failure = ref('');
const rect = ref<Rect | null>(null);
const anchor = ref<{ x: number; y: number } | null>(null);
const stageEl = ref<HTMLElement | null>(null);

/** 物理像素 / CSS 像素：截图是物理分辨率，窗口坐标是 CSS 逻辑像素。 */
const pixelRatio = () => window.devicePixelRatio || 1;

const loadShot = async () => {
  loading.value = true;
  failure.value = '';
  try {
    shot.value = await DrawingApi.captureScreen();
  } catch (error) {
    // DXGI Desktop Duplication 同一时刻只能有一个持有者（远程协助推流时就是这种）。
    failure.value = String((error as any)?.message || error);
  } finally {
    loading.value = false;
  }
};

const pointOf = (event: PointerEvent) => {
  const box = stageEl.value?.getBoundingClientRect();
  if (!box) return { x: 0, y: 0 };
  return {
    x: Math.max(0, Math.min(box.width, event.clientX - box.left)),
    y: Math.max(0, Math.min(box.height, event.clientY - box.top)),
  };
};

const onPointerDown = (event: PointerEvent) => {
  const point = pointOf(event);
  anchor.value = point;
  rect.value = { left: point.x, top: point.y, width: 0, height: 0 };
  (event.currentTarget as HTMLElement).setPointerCapture?.(event.pointerId);
};

const onPointerMove = (event: PointerEvent) => {
  if (!anchor.value) return;
  const point = pointOf(event);
  rect.value = {
    left: Math.min(anchor.value.x, point.x),
    top: Math.min(anchor.value.y, point.y),
    width: Math.abs(point.x - anchor.value.x),
    height: Math.abs(point.y - anchor.value.y),
  };
};

const onPointerUp = () => {
  anchor.value = null;
};

/** 选区换算成物理像素 —— 那才是 VRChat 画布上真正要走多少鼠标位移。 */
const selection = () => {
  if (!rect.value) return null;
  const ratio = pixelRatio();
  const width = Math.round(rect.value.width * ratio);
  const height = Math.round(rect.value.height * ratio);
  if (width < 16 || height < 16) return null;
  return { width, height };
};

const selectionLabel = () => {
  const current = selection();
  if (!current) return t('drawing.region_none');
  const plan = planSize.value;
  const aspect = plan ? `${(plan.width / plan.height).toFixed(2)}` : '—';
  return `${current.width} × ${current.height} px · 1:${(current.width / current.height).toFixed(2)} · 图像比例 ${aspect}`;
};

// 打开选区前主窗口已经把 plan 尺寸带过来了（用于提示宽高比是否匹配）。
const planSize = ref<{ width: number; height: number } | null>(null);

const confirm = async () => {
  const current = selection();
  if (!current) return;
  await emit('drawing_region_selected', current);
  await getCurrentWindow().close();
};

const cancel = async () => {
  await getCurrentWindow().close();
};

const onKeydown = (event: KeyboardEvent) => {
  if (event.key === 'Escape') {
    event.preventDefault();
    void cancel();
  } else if (event.key === 'Enter') {
    event.preventDefault();
    void confirm();
  }
};

onMounted(async () => {
  const params = new URLSearchParams(window.location.search);
  const width = Number(params.get('pw') || 0);
  const height = Number(params.get('ph') || 0);
  if (width > 0 && height > 0) planSize.value = { width, height };
  window.addEventListener('keydown', onKeydown);
  await loadShot();
});

onUnmounted(() => {
  window.removeEventListener('keydown', onKeydown);
});
</script>

<template>
  <div class="region-root">
    <div v-if="loading" class="region-status">
      <span>{{ $t('drawing.region_capturing') }}</span>
    </div>
    <div v-else-if="failure" class="region-status error">
      <span>{{ $t('drawing.region_capture_failed') }}</span>
      <code>{{ failure }}</code>
      <button class="region-btn" @click="loadShot">{{ $t('drawing.region_retry') }}</button>
      <button class="region-btn ghost" @click="cancel">{{ $t('drawing.region_cancel') }}</button>
    </div>

    <div
      v-else
      ref="stageEl"
      class="region-stage"
      @pointerdown="onPointerDown"
      @pointermove="onPointerMove"
      @pointerup="onPointerUp"
      @pointercancel="onPointerUp"
    >
      <img v-if="shot" :src="shot.dataUrl" class="region-shot" alt="" draggable="false">
      <div v-if="rect" class="region-rect" :style="{
        left: `${rect.left}px`,
        top: `${rect.top}px`,
        width: `${rect.width}px`,
        height: `${rect.height}px`,
      }" />
      <div class="region-hint">{{ $t('drawing.region_hint') }}</div>
    </div>

    <div class="region-bar">
      <span class="region-size">{{ selectionLabel() }}</span>
      <div class="region-actions">
        <button class="region-btn primary" :disabled="!selection()" @click="confirm">
          <Check :size="14" /> {{ $t('drawing.region_confirm') }}
        </button>
        <button class="region-btn ghost" @click="cancel">
          <X :size="14" /> {{ $t('drawing.region_cancel') }}
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.region-root { position: fixed; inset: 0; background: #0b0d12; color: #e8ecf3; overflow: hidden; user-select: none; }
.region-stage { position: absolute; inset: 0 0 46px 0; cursor: crosshair; touch-action: none; }
.region-shot { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: fill; pointer-events: none; opacity: .92; }
.region-rect { position: absolute; border: 2px solid #df7600; background: rgba(223, 118, 0, .16); box-shadow: 0 0 0 9999px rgba(6, 8, 12, .55); }
.region-hint { position: absolute; left: 50%; top: 18px; transform: translateX(-50%); font-size: 13px; padding: 6px 12px; border-radius: 6px; background: rgba(8, 10, 14, .78); border: 1px solid rgba(255, 255, 255, .14); }
.region-status { position: absolute; inset: 0; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 12px; font-size: 14px; }
.region-status.error code { max-width: 70vw; font-size: 12px; color: #fca5a5; word-break: break-all; }
.region-bar { position: absolute; left: 0; right: 0; bottom: 0; height: 46px; display: flex; align-items: center; justify-content: space-between; gap: 16px; padding: 0 16px; background: #11141b; border-top: 1px solid rgba(255, 255, 255, .12); }
.region-size { font-size: 12px; color: #b9c2d0; font-family: ui-monospace, monospace; }
.region-actions { display: flex; gap: 8px; }
.region-btn { height: 30px; display: inline-flex; align-items: center; gap: 6px; padding: 0 12px; border-radius: 6px; border: 1px solid rgba(255, 255, 255, .18); background: #1b1f28; color: #e8ecf3; font-size: 12px; font-weight: 700; }
.region-btn.primary { background: #df7600; border-color: #df7600; color: #1a0e00; }
.region-btn.ghost { background: transparent; }
.region-btn:disabled { opacity: .45; cursor: not-allowed; }
</style>
