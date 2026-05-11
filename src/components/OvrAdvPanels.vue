<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import { Ruler, Target } from 'lucide-vue-next';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { ref, onMounted, onUnmounted, watch } from 'vue';

const { t } = useI18n();

const props = defineProps<{
  activeSubTab: string;
  config: any;
}>();

const emit = defineEmits(['update:config']);

// 使用原生的 props 绑定以避免响应式死循环
// ESLint 的警告将通过 template 注释忽略

const updateConfig = (key: string, value: any) => {
  emit('update:config', { ...props.config, [key]: value });
};

// ===== Native Playspace Control (replaces OVRAS INI sync) =====
const heightToggled = ref(false);
const perfStats = ref<any>({
  num_frame_presents: 0,
  num_dropped_frames: 0,
  num_reprojected_frames: 0,
  reprojection_ratio: 0,
});

const applyPlayspaceOffset = async () => {
  try {
    await invoke('ovr_set_playspace_offset', {
      x: props.config.spaceOffsetX || 0,
      y: props.config.spaceOffsetY || 0,
      z: props.config.spaceOffsetZ || 0,
    });
  } catch (e) {
    console.warn('[OvrAdvPanels] Playspace offset error:', e);
  }
};

const applyPlayspaceRotation = async () => {
  try {
    await invoke('ovr_set_playspace_rotation', {
      degrees: props.config.spaceRotation || 0,
    });
  } catch (e) {
    console.warn('[OvrAdvPanels] Playspace rotation error:', e);
  }
};

const toggleHeight = async () => {
  try {
    await invoke('ovr_toggle_height');
    heightToggled.value = !heightToggled.value;
  } catch (e) {
    console.warn('[OvrAdvPanels] Toggle height error:', e);
  }
};

const resetPlayspace = async () => {
  try {
    await invoke('ovr_reset_playspace');
    updateConfig('spaceOffsetX', 0);
    updateConfig('spaceOffsetY', 0);
    updateConfig('spaceOffsetZ', 0);
    updateConfig('spaceRotation', 0);
    heightToggled.value = false;
  } catch (e) {
    console.warn('[OvrAdvPanels] Reset playspace error:', e);
  }
};

const fixFloor = async () => {
  try {
    await invoke('ovr_fix_floor');
  } catch (e) {
    console.warn('[OvrAdvPanels] Fix floor error:', e);
  }
};

// Auto-apply offset when slider changes
watch(() => [props.config.spaceOffsetX, props.config.spaceOffsetY, props.config.spaceOffsetZ], () => {
  applyPlayspaceOffset();
}, { deep: true });

watch(() => props.config.spaceRotation, () => {
  applyPlayspaceRotation();
});

// Listen for performance stats from VR thread
let unlistenPerf: (() => void) | null = null;
onMounted(async () => {
  unlistenPerf = await listen('ovr_perf_stats', (event: any) => {
    perfStats.value = event.payload;
  });
});
onUnmounted(() => {
  if (unlistenPerf) unlistenPerf();
});

</script>

<template>
  <!-- eslint-disable vue/no-mutating-props -->
  <!-- SteamVR 控制 -->
  <div
    v-if="activeSubTab === 'steamvr'"
    class="space-y-5 animate-fade-in"
  >
    <h2 class="text-xl font-extrabold text-primary mb-4 border-primary pb-2">
      {{ t('ovr.steamvr_title') }}
    </h2>
    <div class="space-y-3">
      <!-- 常规设置 -->
      <h3 class="font-bold text-primary mt-4">
        {{ t('ovr.steamvr_misc') }}
      </h3>
      
      <div class="flex items-center justify-between p-4 bg-surface rounded-2xl border-primary shadow-sm">
        <div>
          <h3 class="font-bold text-primary">
            {{ t('ovr.steamvr_timing_overlay') }}
          </h3>
          <p class="text-xs text-primary mt-0.5">
            {{ t('ovr.steamvr_timing_overlay_desc') }}
          </p>
        </div>
        <label class="relative inline-flex items-center cursor-pointer">
          <input
            v-model="config.steamvrTimingOverlay"
            type="checkbox"
            class="sr-only peer"
          >
          <div class="w-11 h-6 bg-background/20 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-border-strong after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-surface after:border-border-soft after:after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-primary/10" />
        </label>
      </div>

      <div class="flex items-center justify-between p-4 bg-surface rounded-2xl border-primary shadow-sm">
        <div>
          <h3 class="font-bold text-primary">
            {{ t('ovr.steamvr_multi_driver') }}
          </h3>
          <p class="text-xs text-primary mt-0.5">
            {{ t('ovr.steamvr_multi_driver_desc') }}
          </p>
        </div>
        <label class="relative inline-flex items-center cursor-pointer">
          <input
            v-model="config.steamvrMultiDriver"
            type="checkbox"
            class="sr-only peer"
          >
          <div class="w-11 h-6 bg-background/20 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-border-strong after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-surface after:border-border-soft after:after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-primary/10" />
        </label>
      </div>

      <div class="flex items-center justify-between p-4 bg-surface rounded-2xl border-primary shadow-sm">
        <div>
          <h3 class="font-bold text-primary">
            {{ t('ovr.steamvr_require_hmd') }}
          </h3>
          <p class="text-xs text-primary mt-0.5">
            {{ t('ovr.steamvr_require_hmd_desc') }}
          </p>
        </div>
        <label class="relative inline-flex items-center cursor-pointer">
          <input
            v-model="config.steamvrRequireHmd"
            type="checkbox"
            class="sr-only peer"
          >
          <div class="w-11 h-6 bg-background/20 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-border-strong after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-surface after:border-border-soft after:after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-primary/10" />
        </label>
      </div>

      <div class="flex items-center justify-between p-4 bg-surface rounded-2xl border-primary shadow-sm">
        <div>
          <h3 class="font-bold text-primary">
            {{ t('ovr.steamvr_disable_notifs') }}
          </h3>
        </div>
        <label class="relative inline-flex items-center cursor-pointer">
          <input
            v-model="config.steamvrDisableNotifs"
            type="checkbox"
            class="sr-only peer"
          >
          <div class="w-11 h-6 bg-background/20 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-border-strong after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-surface after:border-border-soft after:after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-primary/10" />
        </label>
      </div>

      <div class="flex items-center justify-between p-4 bg-surface rounded-2xl border-primary shadow-sm">
        <div>
          <h3 class="font-bold text-primary">
            {{ t('ovr.steamvr_no_fade_grid') }}
          </h3>
        </div>
        <label class="relative inline-flex items-center cursor-pointer">
          <input
            v-model="config.steamvrNoFadeGrid"
            type="checkbox"
            class="sr-only peer"
          >
          <div class="w-11 h-6 bg-background/20 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-border-strong after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-surface after:border-border-soft after:after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-primary/10" />
        </label>
      </div>

      <!-- 摄像头设置 -->
      <h3 class="font-bold text-primary mt-4">
        {{ t('ovr.steamvr_camera') }}
      </h3>
      
      <div class="flex items-center justify-between p-4 bg-surface rounded-2xl border-primary shadow-sm">
        <div>
          <h3 class="font-bold text-primary">
            {{ t('ovr.steamvr_camera_enable') }}
          </h3>
          <p class="text-xs text-primary mt-0.5">
            {{ t('ovr.steamvr_camera_enable_desc') }}
          </p>
        </div>
        <label class="relative inline-flex items-center cursor-pointer">
          <input
            v-model="config.steamvrCameraEnable"
            type="checkbox"
            class="sr-only peer"
          >
          <div class="w-11 h-6 bg-background/20 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-border-strong after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-surface after:border-border-soft after:after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-primary/10" />
        </label>
      </div>
    </div>
  </div>

  <!-- 护栏边界 (Chaperone) -->
  <div
    v-else-if="activeSubTab === 'chaperone'"
    class="space-y-5 animate-fade-in"
  >
    <h2 class="text-xl font-extrabold text-primary mb-4 border-primary pb-2">
      {{ t('ovr.chap_title') }}
    </h2>
    <div class="space-y-3">
      <div class="p-4 bg-surface rounded-2xl border-primary shadow-sm space-y-4">
        <div>
          <label class="block text-sm font-bold text-primary mb-2">{{ t('ovr.chap_visibility') }}: {{ config.chapVisibility }}%</label>
          <input
            v-model.number="config.chapVisibility"
            type="range"
            min="30"
            max="100"
            step="1"
            class="w-full h-2 bg-primary/10 rounded-lg appearance-none cursor-pointer accent-indigo-500"
          >
        </div>
        <div>
          <label class="block text-sm font-bold text-primary mb-2">{{ t('ovr.chap_fade_distance') }}: {{ config.chapFadeDistance }} m</label>
          <input
            v-model.number="config.chapFadeDistance"
            type="range"
            min="0"
            max="2"
            step="0.1"
            class="w-full h-2 bg-primary/10 rounded-lg appearance-none cursor-pointer accent-indigo-500"
          >
        </div>
        <div>
          <label class="block text-sm font-bold text-primary mb-2">{{ t('ovr.chap_height') }}: {{ config.chapHeight }} m</label>
          <input
            v-model.number="config.chapHeight"
            type="range"
            min="0"
            max="4"
            step="0.1"
            class="w-full h-2 bg-primary/10 rounded-lg appearance-none cursor-pointer accent-indigo-500"
          >
        </div>
      </div>

      <div class="flex items-center justify-between p-4 bg-surface rounded-2xl border-primary shadow-sm">
        <div>
          <h3 class="font-bold text-primary">
            {{ t('ovr.chap_force_bounds') }}
          </h3>
          <p class="text-xs text-primary mt-0.5">
            {{ t('ovr.chap_force_bounds_desc') }}
          </p>
        </div>
        <label class="relative inline-flex items-center cursor-pointer">
          <input
            v-model="config.chapForceBounds"
            type="checkbox"
            class="sr-only peer"
          >
          <div class="w-11 h-6 bg-background/20 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-border-strong after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-surface after:border-border-soft after:after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-primary/10" />
        </label>
      </div>

      <div class="flex items-center justify-between p-4 bg-surface rounded-2xl border-primary shadow-sm">
        <div>
          <h3 class="font-bold text-primary">
            {{ t('ovr.chap_disable') }}
          </h3>
          <p class="text-xs text-primary mt-0.5">
            {{ t('ovr.chap_disable_desc') }}
          </p>
        </div>
        <label class="relative inline-flex items-center cursor-pointer">
          <input
            v-model="config.chapDisable"
            type="checkbox"
            class="sr-only peer"
          >
          <div class="w-11 h-6 bg-background/20 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-border-strong after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-surface after:border-border-soft after:after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-primary/10" />
        </label>
      </div>

      <!-- 接近警告与进阶颜色设置 -->
      <h3 class="font-bold text-primary mt-4">
        {{ t('ovr.chap_advanced') }}
      </h3>
      <div class="p-4 bg-surface rounded-2xl border-primary shadow-sm space-y-4">
        <label class="flex items-center space-x-2">
          <input
            v-model="config.chapBeginnerMode"
            type="checkbox"
            class="rounded text-primary"
          >
          <span class="text-sm font-bold text-primary">{{ t('ovr.chap_beginner_mode') }}</span>
        </label>
        <label class="flex items-center space-x-2">
          <input
            v-model="config.chapHapticFeedback"
            type="checkbox"
            class="rounded text-primary"
          >
          <span class="text-sm font-bold text-primary">{{ t('ovr.chap_haptic_feedback') }}</span>
        </label>
        <label class="flex items-center space-x-2">
          <input
            v-model="config.chapAudioWarning"
            type="checkbox"
            class="rounded text-primary"
          >
          <span class="text-sm font-bold text-primary">{{ t('ovr.chap_audio_warning') }}</span>
        </label>
        
        <div class="pt-2 border-primary">
          <h4 class="text-xs font-bold text-primary mb-2">
            {{ t('ovr.chap_color_override') }}
          </h4>
          <div class="flex gap-2">
            <input
              v-model="config.chapColor"
              type="color"
              class="w-full h-8 rounded border-border-soft cursor-pointer"
            >
          </div>
        </div>
      </div>
    </div>
  </div>

  <!-- 空间与运动 -->
  <div
    v-else-if="activeSubTab === 'playspace'"
    class="space-y-5 animate-fade-in"
  >
    <h2 class="text-xl font-extrabold text-primary mb-4 border-primary pb-2">
      {{ t('ovr.space_title') }}
    </h2>
    <div class="space-y-3">
      <!-- 空间偏移 -->
      <h3 class="font-bold text-primary mt-4">
        {{ t('ovr.space_offset_section') }}
      </h3>
      <p class="text-xs text-primary mb-2">
        {{ t('ovr.space_offset_desc') }}
      </p>
      
      <div class="p-4 bg-surface rounded-2xl border-primary shadow-sm space-y-4">
        <div>
          <label class="block text-sm font-bold text-primary mb-2">{{ t('ovr.space_offset_x') }}: {{ config.spaceOffsetX }} m</label>
          <input
            v-model.number="config.spaceOffsetX"
            type="range"
            min="-10"
            max="10"
            step="0.1"
            class="w-full h-2 bg-primary/10 rounded-lg appearance-none cursor-pointer accent-indigo-500"
          >
        </div>
        <div>
          <label class="block text-sm font-bold text-primary mb-2">{{ t('ovr.space_offset_y') }}: {{ config.spaceOffsetY }} m</label>
          <input
            v-model.number="config.spaceOffsetY"
            type="range"
            min="-10"
            max="10"
            step="0.1"
            class="w-full h-2 bg-primary/10 rounded-lg appearance-none cursor-pointer accent-indigo-500"
          >
        </div>
        <div>
          <label class="block text-sm font-bold text-primary mb-2">{{ t('ovr.space_offset_z') }}: {{ config.spaceOffsetZ }} m</label>
          <input
            v-model.number="config.spaceOffsetZ"
            type="range"
            min="-10"
            max="10"
            step="0.1"
            class="w-full h-2 bg-primary/10 rounded-lg appearance-none cursor-pointer accent-indigo-500"
          >
        </div>
      </div>

      <div class="flex items-center justify-between p-4 bg-surface rounded-2xl border-primary shadow-sm">
        <div>
          <h3 class="font-bold text-primary">
            {{ t('ovr.space_adjust_chap') }}
          </h3>
          <p class="text-xs text-primary mt-0.5">
            {{ t('ovr.space_adjust_chap_desc') }}
          </p>
        </div>
        <label class="relative inline-flex items-center cursor-pointer">
          <input
            v-model="config.spaceAdjustChap"
            type="checkbox"
            class="sr-only peer"
          >
          <div class="w-11 h-6 bg-background/20 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-border-strong after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-surface after:border-border-soft after:after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-primary/10" />
        </label>
      </div>

      <!-- 运动模拟 -->
      <h3 class="font-bold text-primary mt-4">
        {{ t('ovr.motion_section') }}
      </h3>
      
      <div class="flex items-center justify-between p-4 bg-surface rounded-2xl border-primary shadow-sm">
        <div>
          <h3 class="font-bold text-primary">
            {{ t('ovr.motion_drag_left') }}
          </h3>
        </div>
        <label class="relative inline-flex items-center cursor-pointer">
          <input
            v-model="config.motionDragLeft"
            type="checkbox"
            class="sr-only peer"
          >
          <div class="w-11 h-6 bg-background/20 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-border-strong after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-surface after:border-border-soft after:after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-primary/10" />
        </label>
      </div>

      <div class="flex items-center justify-between p-4 bg-surface rounded-2xl border-primary shadow-sm">
        <div>
          <h3 class="font-bold text-primary">
            {{ t('ovr.motion_drag_right') }}
          </h3>
        </div>
        <label class="relative inline-flex items-center cursor-pointer">
          <input
            v-model="config.motionDragRight"
            type="checkbox"
            class="sr-only peer"
          >
          <div class="w-11 h-6 bg-background/20 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-border-strong after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-surface after:border-border-soft after:after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-primary/10" />
        </label>
      </div>

      <!-- 空间修复与旋转 (Space Fix & Rotation) -->
      <h3 class="font-bold text-primary mt-4">
        {{ t('ovr.space_fix_section') }}
      </h3>
      
      <div class="grid grid-cols-2 gap-4">
        <button
          class="p-4 bg-primary/10 hover:bg-primary/10 active:bg-primary/10 rounded-2xl border-primary shadow-sm transition-colors text-center text-primary font-bold flex flex-col items-center justify-center gap-2"
          @click="fixFloor"
        >
          <Ruler class="w-6 h-6 text-primary" />
          <span>{{ t('ovr.space_fix_floor') }}</span>
        </button>
        <button
          class="p-4 bg-primary/10 hover:bg-primary/10 active:bg-primary/10 rounded-2xl border-primary shadow-sm transition-colors text-center text-primary font-bold flex flex-col items-center justify-center gap-2"
          @click="resetPlayspace"
        >
          <Target class="w-6 h-6 text-primary" />
          <span>{{ t('ovr.space_fix_center') }}</span>
        </button>
      </div>

      <!-- {{ t('ovr.space_height_toggle_title') }} - Native API -->
      <div class="flex items-center justify-between p-4 bg-surface rounded-2xl border-primary shadow-sm">
        <div>
          <h3 class="font-bold text-primary">
            {{ t('ovr.space_height_toggle_title') }}
          </h3>
          <p class="text-xs text-primary mt-0.5">
            {{ t('ovr.space_height_toggle_desc') }}
          </p>
        </div>
        <button
          :class="['px-4 py-2 font-bold rounded-lg transition-all text-sm', heightToggled ? 'bg-primary/10 text-white shadow-md' : 'bg-primary/10 hover:bg-primary/10 text-primary']"
          @click="toggleHeight"
        >
          {{ heightToggled ? t('ovr.space_height_toggled') : t('ovr.space_height_toggle_btn') }}
        </button>
      </div>
      
      <div class="p-4 bg-surface rounded-2xl border-primary shadow-sm space-y-4">
        <div>
          <label class="block text-sm font-bold text-primary mb-2">{{ t('ovr.space_rotation') }}: {{ config.spaceRotation || 0 }}°</label>
          <input
            v-model.number="config.spaceRotation"
            type="range"
            min="-180"
            max="180"
            step="1"
            class="w-full h-2 bg-primary/10 rounded-lg appearance-none cursor-pointer accent-indigo-500"
          >
        </div>
        <div class="flex justify-between">
          <button
            class="px-3 py-1 bg-surface hover:bg-background/20 text-text-muted text-xs font-bold rounded"
            @click="config.spaceRotation = -90"
          >
            -90°
          </button>
          <button
            class="px-3 py-1 bg-surface hover:bg-background/20 text-text-muted text-xs font-bold rounded"
            @click="config.spaceRotation = 0"
          >
            {{ t('ovr.stats_reset') }}
          </button>
          <button
            class="px-3 py-1 bg-surface hover:bg-background/20 text-text-muted text-xs font-bold rounded"
            @click="config.spaceRotation = 90"
          >
            +90°
          </button>
        </div>
      </div>

      <!-- 高级运动模拟 -->
      <h3 class="font-bold text-primary mt-4">
        {{ t('ovr.motion_advanced') }}
      </h3>
      <div class="p-4 bg-surface rounded-2xl border-primary shadow-sm space-y-4">
        <label class="flex items-center justify-between">
          <span class="text-sm font-bold text-primary">{{ t('ovr.motion_gravity') }}</span>
          <input
            v-model="config.motionGravity"
            type="checkbox"
            class="rounded text-primary"
          >
        </label>
        <div
          v-if="config.motionGravity"
          class="pl-4 border-l-2 border-primary space-y-3 mt-2"
        >
          <label class="block text-xs font-bold text-primary">{{ t('ovr.motion_gravity_strength', { val: config.motionGravityStrength || 9.8 }) }}</label>
          <input
            v-model.number="config.motionGravityStrength"
            type="range"
            min="0"
            max="20"
            step="0.1"
            class="w-full h-2 bg-primary/10 rounded appearance-none cursor-pointer"
          >
          <label class="flex items-center space-x-2">
            <input
              v-model="config.motionGravitySaveMomentum"
              type="checkbox"
              class="rounded text-primary"
            >
            <span class="text-xs font-bold text-primary">{{ t('ovr.motion_save_momentum') }}</span>
          </label>
        </div>

        <label class="flex items-center justify-between border-primary pt-3">
          <span class="text-sm font-bold text-primary">{{ t('ovr.rotation_auto_turn') }}</span>
          <input
            v-model="config.rotationAutoTurn"
            type="checkbox"
            class="rounded text-primary"
          >
        </label>
        <label class="flex items-center justify-between border-primary pt-3">
          <span class="text-sm font-bold text-primary">{{ t('ovr.rotation_redirected_walk') }}</span>
          <input
            v-model="config.rotationRedirectedWalk"
            type="checkbox"
            class="rounded text-primary"
          >
        </label>
      </div>
    </div>
  </div>

  <!-- 音频管理 -->
  <div
    v-else-if="activeSubTab === 'audio'"
    class="space-y-5 animate-fade-in"
  >
    <h2 class="text-xl font-extrabold text-primary mb-4 border-primary pb-2">
      {{ t('ovr.audio_title') }}
    </h2>
    <div class="space-y-3">
      <div class="flex items-center justify-between p-4 bg-surface rounded-2xl border-primary shadow-sm">
        <div>
          <h3 class="font-bold text-primary">
            {{ t('ovr.audio_prox_sensor') }}
          </h3>
          <p class="text-xs text-primary mt-0.5">
            {{ t('ovr.audio_prox_sensor_desc') }}
          </p>
        </div>
        <label class="relative inline-flex items-center cursor-pointer">
          <input
            v-model="config.audioProxSensor"
            type="checkbox"
            class="sr-only peer"
          >
          <div class="w-11 h-6 bg-background/20 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-border-strong after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-surface after:border-border-soft after:after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-primary/10" />
        </label>
      </div>
      
      <div class="flex items-center justify-between p-4 bg-surface rounded-2xl border-primary shadow-sm">
        <div>
          <h3 class="font-bold text-primary">
            {{ t('ovr.audio_ptt') }}
          </h3>
          <p class="text-xs text-primary mt-0.5">
            {{ t('ovr.audio_ptt_desc') }}
          </p>
        </div>
        <label class="relative inline-flex items-center cursor-pointer">
          <input
            v-model="config.audioPTT"
            type="checkbox"
            class="sr-only peer"
          >
          <div class="w-11 h-6 bg-background/20 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-border-strong after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-surface after:border-border-soft after:after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-primary/10" />
        </label>
      </div>
    </div>
  </div>

  <!-- 视频画质 -->
  <div
    v-else-if="activeSubTab === 'video'"
    class="space-y-5 animate-fade-in"
  >
    <h2 class="text-xl font-extrabold text-primary mb-4 border-primary pb-2">
      {{ t('ovr.video_title') }}
    </h2>
    <div class="space-y-3">
      <div class="flex items-center justify-between p-4 bg-surface rounded-2xl border-primary shadow-sm">
        <div>
          <h3 class="font-bold text-primary">
            {{ t('ovr.video_brightness_on') }}
          </h3>
          <p class="text-xs text-primary mt-0.5">
            {{ t('ovr.video_brightness_desc') }}
          </p>
        </div>
        <label class="relative inline-flex items-center cursor-pointer">
          <input
            v-model="config.videoBrightnessOn"
            type="checkbox"
            class="sr-only peer"
          >
          <div class="w-11 h-6 bg-background/20 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-border-strong after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-surface after:border-border-soft after:after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-primary/10" />
        </label>
      </div>

      <div
        v-if="config.videoBrightnessOn"
        class="p-4 bg-surface rounded-2xl border-primary shadow-sm space-y-4"
      >
        <div>
          <label class="block text-sm font-bold text-primary mb-2">{{ t('ovr.video_brightness_value') }}: {{ config.videoBrightnessValue }}%</label>
          <input
            v-model.number="config.videoBrightnessValue"
            type="range"
            min="0"
            max="150"
            step="1"
            class="w-full h-2 bg-primary/10 rounded-lg appearance-none cursor-pointer accent-indigo-500"
          >
        </div>
      </div>
      
      <div class="flex items-center justify-between p-4 bg-surface rounded-2xl border-primary shadow-sm">
        <div>
          <h3 class="font-bold text-primary">
            {{ t('ovr.video_motion_smooth') }}
          </h3>
          <p class="text-xs text-primary mt-0.5">
            {{ t('ovr.video_motion_smooth_desc') }}
          </p>
        </div>
        <label class="relative inline-flex items-center cursor-pointer">
          <input
            v-model="config.videoMotionSmooth"
            type="checkbox"
            class="sr-only peer"
          >
          <div class="w-11 h-6 bg-background/20 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-border-strong after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-surface after:border-border-soft after:after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-primary/10" />
        </label>
      </div>

      <div class="p-4 bg-surface rounded-2xl border-primary shadow-sm space-y-4">
        <label class="flex items-center justify-between">
          <span class="text-sm font-bold text-primary">Advanced SS Filtering</span>
          <input
            v-model="config.videoAdvSSFiltering"
            type="checkbox"
            class="rounded text-primary"
          >
        </label>
        <div class="pt-2 border-primary">
          <label class="block text-xs font-bold text-primary mb-2">SuperSampling Override: {{ config.videoSuperSampling || 100 }}%</label>
          <input
            v-model.number="config.videoSuperSampling"
            type="range"
            min="20"
            max="500"
            step="10"
            class="w-full h-2 bg-primary/10 rounded appearance-none cursor-pointer"
          >
        </div>
        <div class="pt-2 border-primary">
          <label class="flex items-center justify-between">
            <span class="text-sm font-bold text-primary">Use Overlay For Color</span>
            <input
              v-model="config.videoOverlayColor"
              type="checkbox"
              class="rounded text-primary"
            >
          </label>
        </div>
      </div>
    </div>
  </div>

  <!-- 实用工具 -->
  <div
    v-else-if="activeSubTab === 'utilities'"
    class="space-y-5 animate-fade-in"
  >
    <h2 class="text-xl font-extrabold text-primary mb-4 border-primary pb-2">
      {{ t('ovr.util_title') }}
    </h2>
    <div class="space-y-3">
      <div class="flex items-center justify-between p-4 bg-surface rounded-2xl border-primary shadow-sm">
        <div>
          <h3 class="font-bold text-primary">
            {{ t('ovr.util_alarm_enabled') }}
          </h3>
          <p class="text-xs text-primary mt-0.5">
            {{ t('ovr.util_alarm_desc') }}
          </p>
        </div>
        <label class="relative inline-flex items-center cursor-pointer">
          <input
            v-model="config.utilAlarmEnabled"
            type="checkbox"
            class="sr-only peer"
          >
          <div class="w-11 h-6 bg-background/20 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-border-strong after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-surface after:border-border-soft after:after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-primary/10" />
        </label>
      </div>
      
      <div
        v-if="config.utilAlarmEnabled"
        class="p-4 bg-surface rounded-2xl border-primary shadow-sm space-y-4"
      >
        <div>
          <label class="block text-sm font-bold text-primary mb-2">{{ t('ovr.util_alarm_time') }}</label>
          <input
            v-model="config.utilAlarmTime"
            type="time"
            class="w-full px-3 py-2 bg-primary/10 rounded-lg border-primary text-primary"
          >
        </div>
      </div>
      
      <div class="flex items-center justify-between p-4 bg-surface rounded-2xl border-primary shadow-sm">
        <div>
          <h3 class="font-bold text-primary">
            {{ t('ovr.util_tracker_battery') }}
          </h3>
          <p class="text-xs text-primary mt-0.5">
            {{ t('ovr.util_tracker_battery_desc') }}
          </p>
        </div>
        <label class="relative inline-flex items-center cursor-pointer">
          <input
            v-model="config.utilTrackerBattery"
            type="checkbox"
            class="sr-only peer"
          >
          <div class="w-11 h-6 bg-background/20 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-border-strong after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-surface after:border-border-soft after:after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-primary/10" />
        </label>
      </div>

      <div class="p-4 bg-surface rounded-2xl border-primary shadow-sm space-y-4">
        <label class="flex items-center justify-between">
          <span class="text-sm font-bold text-primary">{{ t('ovr.util_media_keys') }}</span>
          <input
            v-model="config.utilMediaKeys"
            type="checkbox"
            class="rounded text-primary"
          >
        </label>
        <p class="text-xs text-primary mt-0.5">
          {{ t('ovr.util_media_keys_desc') }}
        </p>
        
        <label class="flex items-center justify-between border-primary pt-3">
          <span class="text-sm font-bold text-primary">{{ t('ovr.util_keyboard') }}</span>
          <input
            v-model="config.utilKeyboard"
            type="checkbox"
            class="rounded text-primary"
          >
        </label>
      </div>
    </div>
  </div>

  <!-- 性能统计 -->
  <div
    v-else-if="activeSubTab === 'statistics'"
    class="space-y-5 animate-fade-in"
  >
    <h2 class="text-xl font-extrabold text-primary mb-4 border-primary pb-2">
      {{ t('ovr.stats_title') }}
    </h2>
    <div class="space-y-3">
      <div class="p-4 bg-surface rounded-2xl border-primary shadow-sm space-y-4">
        <div class="flex justify-between items-center">
          <div>
            <h3 class="font-bold text-primary">
              {{ t('ovr.stats_hmd_distance') }}
            </h3>
            <p class="text-xs text-primary mt-0.5">
              {{ t('ovr.stats_hmd_distance_desc') }}
            </p>
          </div>
          <span class="text-sm font-bold text-primary">{{ t('ovr.stats_frames', { val: perfStats.num_frame_presents?.toLocaleString() || 0 }) }}</span>
        </div>
        
        <div class="flex justify-between items-center">
          <div>
            <h3 class="font-bold text-primary">
              {{ t('ovr.stats_dropped_frames') }}
            </h3>
            <p class="text-xs text-primary mt-0.5">
              {{ t('ovr.stats_dropped_frames_desc') }}
            </p>
          </div>
          <span
            class="text-sm font-bold"
            :class="perfStats.num_dropped_frames > 100 ? 'text-red-500' : 'text-primary'"
          >{{ perfStats.num_dropped_frames?.toLocaleString() || 0 }}</span>
        </div>
        
        <div class="flex justify-between items-center">
          <div>
            <h3 class="font-bold text-primary">
              {{ t('ovr.stats_reprojection_ratio') }}
            </h3>
            <p class="text-xs text-primary mt-0.5">
              {{ t('ovr.stats_reprojected_count', { val: perfStats.num_reprojected_frames?.toLocaleString() || 0 }) }}
            </p>
          </div>
          <span
            class="text-sm font-bold"
            :class="perfStats.reprojection_ratio > 20 ? 'text-red-500' : perfStats.reprojection_ratio > 5 ? 'text-yellow-500' : 'text-green-500'"
          >
            {{ (perfStats.reprojection_ratio || 0).toFixed(1) }}%
          </span>
        </div>
        
        <div class="pt-4 border-primary flex justify-end">
          <button class="px-4 py-2 bg-primary/10 hover:bg-primary/10 text-primary font-bold rounded-lg transition-colors text-sm">
            {{ t('ovr.stats_reset') }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
