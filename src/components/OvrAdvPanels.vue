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

const updateConfig = (key: string, value: any) => {
  const keys = key.split('.');
  if (keys.length === 2) {
    emit('update:config', {
      ...props.config,
      [keys[0]]: {
        ...props.config[keys[0]],
        [keys[1]]: value
      }
    });
  } else {
    emit('update:config', { ...props.config, [key]: value });
  }
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
      x: props.config.playspace.offsetX || 0,
      y: props.config.playspace.offsetY || 0,
      z: props.config.playspace.offsetZ || 0,
    });
  } catch (e) {
    console.warn('[OvrAdvPanels] Playspace offset error:', e);
  }
};

const applyPlayspaceRotation = async () => {
  try {
    await invoke('ovr_set_playspace_rotation', {
      degrees: props.config.playspace.rotation || 0,
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
    updateConfig('playspace.offsetX', 0);
    updateConfig('playspace.offsetY', 0);
    updateConfig('playspace.offsetZ', 0);
    updateConfig('playspace.rotation', 0);
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
watch(() => [props.config.playspace.offsetX, props.config.playspace.offsetY, props.config.playspace.offsetZ], () => {
  applyPlayspaceOffset();
}, { deep: true });

watch(() => props.config.playspace.rotation, () => {
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
            v-model="config.steamvr.timingOverlay"
            type="checkbox"
            class="sr-only peer"
          >
          <div class="w-11 h-6 bg-border-strong/35 border border-border-strong peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-white after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-primary peer-checked:border-primary shadow-inner" />
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
            v-model="config.steamvr.multiDriver"
            type="checkbox"
            class="sr-only peer"
          >
          <div class="w-11 h-6 bg-border-strong/35 border border-border-strong peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-white after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-primary peer-checked:border-primary shadow-inner" />
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
            v-model="config.steamvr.requireHmd"
            type="checkbox"
            class="sr-only peer"
          >
          <div class="w-11 h-6 bg-border-strong/35 border border-border-strong peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-white after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-primary peer-checked:border-primary shadow-inner" />
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
            v-model="config.steamvr.disableNotifs"
            type="checkbox"
            class="sr-only peer"
          >
          <div class="w-11 h-6 bg-border-strong/35 border border-border-strong peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-white after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-primary peer-checked:border-primary shadow-inner" />
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
            v-model="config.steamvr.noFadeGrid"
            type="checkbox"
            class="sr-only peer"
          >
          <div class="w-11 h-6 bg-border-strong/35 border border-border-strong peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-white after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-primary peer-checked:border-primary shadow-inner" />
        </label>
      </div>

      <!-- 摄像头设置-->
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
            v-model="config.steamvr.cameraEnable"
            type="checkbox"
            class="sr-only peer"
          >
          <div class="w-11 h-6 bg-border-strong/35 border border-border-strong peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-white after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-primary peer-checked:border-primary shadow-inner" />
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
          <label class="block text-sm font-bold text-primary mb-2">{{ t('ovr.chap_visibility') }}: {{ config.chaperone.visibility }}%</label>
          <input
            v-model.number="config.chaperone.visibility"
            type="range"
            min="30"
            max="100"
            step="1"
            class="w-full h-2 bg-primary/10 rounded-lg appearance-none cursor-pointer accent-primary"
          >
        </div>
        <div>
          <label class="block text-sm font-bold text-primary mb-2">{{ t('ovr.chap_fade_distance') }}: {{ config.chaperone.fadeDistance }} m</label>
          <input
            v-model.number="config.chaperone.fadeDistance"
            type="range"
            min="0"
            max="2"
            step="0.1"
            class="w-full h-2 bg-primary/10 rounded-lg appearance-none cursor-pointer accent-primary"
          >
        </div>
        <div>
          <label class="block text-sm font-bold text-primary mb-2">{{ t('ovr.chap_height') }}: {{ config.chaperone.height }} m</label>
          <input
            v-model.number="config.chaperone.height"
            type="range"
            min="0"
            max="4"
            step="0.1"
            class="w-full h-2 bg-primary/10 rounded-lg appearance-none cursor-pointer accent-primary"
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
            v-model="config.chaperone.forceBounds"
            type="checkbox"
            class="sr-only peer"
          >
          <div class="w-11 h-6 bg-border-strong/35 border border-border-strong peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-white after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-primary peer-checked:border-primary shadow-inner" />
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
            v-model="config.chaperone.disable"
            type="checkbox"
            class="sr-only peer"
          >
          <div class="w-11 h-6 bg-border-strong/35 border border-border-strong peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-white after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-primary peer-checked:border-primary shadow-inner" />
        </label>
      </div>

      <!-- 接近警告与进阶颜色设置-->
      <h3 class="font-bold text-primary mt-4">
        {{ t('ovr.chap_advanced') }}
      </h3>
      <div class="p-4 bg-surface rounded-2xl border-primary shadow-sm space-y-4">
        <label class="flex items-center space-x-2">
          <input
            v-model="config.chaperone.beginnerMode"
            type="checkbox"
            class="rounded text-primary"
          >
          <span class="text-sm font-bold text-primary">{{ t('ovr.chap_beginner_mode') }}</span>
        </label>
        <label class="flex items-center space-x-2">
          <input
            v-model="config.chaperone.hapticFeedback"
            type="checkbox"
            class="rounded text-primary"
          >
          <span class="text-sm font-bold text-primary">{{ t('ovr.chap_haptic_feedback') }}</span>
        </label>
        <label class="flex items-center space-x-2">
          <input
            v-model="config.chaperone.audioWarning"
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
              v-model="config.chaperone.colorHex"
              type="color"
              class="w-full h-8 rounded border-border-soft cursor-pointer"
            >
          </div>
        </div>
      </div>
    </div>
  </div>

  <!-- 空间与运动-->
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
          <label class="block text-sm font-bold text-primary mb-2">{{ t('ovr.space_offset_x') }}: {{ config.playspace.offsetX }} m</label>
          <input
            v-model.number="config.playspace.offsetX"
            type="range"
            min="-10"
            max="10"
            step="0.1"
            class="w-full h-2 bg-primary/10 rounded-lg appearance-none cursor-pointer accent-primary"
          >
        </div>
        <div>
          <label class="block text-sm font-bold text-primary mb-2">{{ t('ovr.space_offset_y') }}: {{ config.playspace.offsetY }} m</label>
          <input
            v-model.number="config.playspace.offsetY"
            type="range"
            min="-10"
            max="10"
            step="0.1"
            class="w-full h-2 bg-primary/10 rounded-lg appearance-none cursor-pointer accent-primary"
          >
        </div>
        <div>
          <label class="block text-sm font-bold text-primary mb-2">{{ t('ovr.space_offset_z') }}: {{ config.playspace.offsetZ }} m</label>
          <input
            v-model.number="config.playspace.offsetZ"
            type="range"
            min="-10"
            max="10"
            step="0.1"
            class="w-full h-2 bg-primary/10 rounded-lg appearance-none cursor-pointer accent-primary"
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
            v-model="config.general.spaceAdjustChap"
            type="checkbox"
            class="sr-only peer"
          >
          <div class="w-11 h-6 bg-border-strong/35 border border-border-strong peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-white after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-primary peer-checked:border-primary shadow-inner" />
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
            v-model="config.playspace.dragLeft"
            type="checkbox"
            class="sr-only peer"
          >
          <div class="w-11 h-6 bg-border-strong/35 border border-border-strong peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-white after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-primary peer-checked:border-primary shadow-inner" />
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
            v-model="config.playspace.dragRight"
            type="checkbox"
            class="sr-only peer"
          >
          <div class="w-11 h-6 bg-border-strong/35 border border-border-strong peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-white after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-primary peer-checked:border-primary shadow-inner" />
        </label>
      </div>

      <!-- 空间修复与旋转(Space Fix & Rotation) -->
      <h3 class="font-bold text-primary mt-4">
        {{ t('ovr.space_fix_section') }}
      </h3>
      
      <div class="grid grid-cols-2 gap-4">
        <button
          class="p-4 bg-primary text-white hover:bg-primary-hover active:bg-primary-hover rounded-2xl border border-primary shadow-sm transition-colors text-center font-bold flex flex-col items-center justify-center gap-2"
          @click="fixFloor"
        >
          <Ruler class="w-6 h-6 text-white" />
          <span>{{ t('ovr.space_fix_floor') }}</span>
        </button>
        <button
          class="p-4 bg-primary text-white hover:bg-primary-hover active:bg-primary-hover rounded-2xl border border-primary shadow-sm transition-colors text-center font-bold flex flex-col items-center justify-center gap-2"
          @click="resetPlayspace"
        >
          <Target class="w-6 h-6 text-white" />
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
          :class="['px-4 py-2 font-bold rounded-lg transition-all text-sm', heightToggled ? 'bg-primary text-white shadow-md' : 'bg-primary text-white hover:bg-primary-hover']"
          @click="toggleHeight"
        >
          {{ heightToggled ? t('ovr.space_height_toggled') : t('ovr.space_height_toggle_btn') }}
        </button>
      </div>
      
      <div class="p-4 bg-surface rounded-2xl border-primary shadow-sm space-y-4">
        <div>
          <label class="block text-sm font-bold text-primary mb-2">{{ t('ovr.space_rotation') }}: {{ config.playspace.rotation || 0 }}°</label>
          <input
            v-model.number="config.playspace.rotation"
            type="range"
            min="-180"
            max="180"
            step="1"
            class="w-full h-2 bg-primary/10 rounded-lg appearance-none cursor-pointer accent-primary"
          >
        </div>
        <div class="flex justify-between">
          <button
            class="px-3 py-1 bg-surface-hover hover:bg-primary hover:text-white text-text-muted text-xs font-bold rounded border border-border-soft shadow-sm"
            @click="config.playspace.rotation = -90"
          >
            -90°
          </button>
          <button
            class="px-3 py-1 bg-surface-hover hover:bg-primary hover:text-white text-text-muted text-xs font-bold rounded border border-border-soft shadow-sm"
            @click="config.playspace.rotation = 0"
          >
            {{ t('ovr.stats_reset') }}
          </button>
          <button
            class="px-3 py-1 bg-surface-hover hover:bg-primary hover:text-white text-text-muted text-xs font-bold rounded border border-border-soft shadow-sm"
            @click="config.playspace.rotation = 90"
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
            v-model="config.general.motionGravityOn"
            type="checkbox"
            class="rounded text-primary"
          >
        </label>
        <div
          v-if="config.general.motionGravityOn"
          class="pl-4 border-l-2 border-primary space-y-3 mt-2"
        >
          <label class="block text-xs font-bold text-primary">{{ t('ovr.motion_gravity_strength', { val: config.general.motionGravityStrength || 9.8 }) }}</label>
          <input
            v-model.number="config.general.motionGravityStrength"
            type="range"
            min="0"
            max="20"
            step="0.1"
            class="w-full h-2 bg-primary/10 rounded appearance-none cursor-pointer"
          >
          <label class="flex items-center space-x-2">
            <input
              v-model="config.general.motionSaveMomentum"
              type="checkbox"
              class="rounded text-primary"
            >
            <span class="text-xs font-bold text-primary">{{ t('ovr.motion_save_momentum') }}</span>
          </label>
        </div>

        <label class="flex items-center justify-between border-primary pt-3">
          <span class="text-sm font-bold text-primary">{{ t('ovr.rotation_auto_turn') }}</span>
          <input
            v-model="config.general.rotationAutoTurn"
            type="checkbox"
            class="rounded text-primary"
          >
        </label>
        <label class="flex items-center justify-between border-primary pt-3">
          <span class="text-sm font-bold text-primary">{{ t('ovr.rotation_redirected_walk') }}</span>
          <input
            v-model="config.general.rotationRedirectedWalk"
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
            v-model="config.audio.proxSensor"
            type="checkbox"
            class="sr-only peer"
          >
          <div class="w-11 h-6 bg-border-strong/35 border border-border-strong peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-white after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-primary peer-checked:border-primary shadow-inner" />
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
            v-model="config.audio.pTT"
            type="checkbox"
            class="sr-only peer"
          >
          <div class="w-11 h-6 bg-border-strong/35 border border-border-strong peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-white after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-primary peer-checked:border-primary shadow-inner" />
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
            v-model="config.video.brightnessOn"
            type="checkbox"
            class="sr-only peer"
          >
          <div class="w-11 h-6 bg-border-strong/35 border border-border-strong peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-white after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-primary peer-checked:border-primary shadow-inner" />
        </label>
      </div>

      <div
        v-if="config.video.brightnessOn"
        class="p-4 bg-surface rounded-2xl border-primary shadow-sm space-y-4"
      >
        <div>
          <label class="block text-sm font-bold text-primary mb-2">{{ t('ovr.video_brightness_value') }}: {{ config.video.brightnessValue }}%</label>
          <input
            v-model.number="config.video.brightnessValue"
            type="range"
            min="0"
            max="150"
            step="1"
            class="w-full h-2 bg-primary/10 rounded-lg appearance-none cursor-pointer accent-primary"
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
            v-model="config.video.motionSmooth"
            type="checkbox"
            class="sr-only peer"
          >
          <div class="w-11 h-6 bg-border-strong/35 border border-border-strong peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-white after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-primary peer-checked:border-primary shadow-inner" />
        </label>
      </div>

      <div class="p-4 bg-surface rounded-2xl border-primary shadow-sm space-y-4">
        <label class="flex items-center justify-between">
          <span class="text-sm font-bold text-primary">{{ t('ovr.video_adv_ss_filter') }}</span>
          <input
            v-model="config.video.advSSFilter"
            type="checkbox"
            class="rounded text-primary"
          >
        </label>
        <div class="pt-2 border-primary">
          <label class="block text-xs font-bold text-primary mb-2">{{ t('ovr.video_super_sampling') }}: {{ config.video.superSampling || 100 }}%</label>
          <input
            v-model.number="config.video.superSampling"
            type="range"
            min="20"
            max="500"
            step="10"
            class="w-full h-2 bg-primary/10 rounded appearance-none cursor-pointer"
          >
        </div>
        <div class="pt-2 border-primary">
          <label class="flex items-center justify-between">
            <span class="text-sm font-bold text-primary">{{ t('ovr.video_overlay_color') }}</span>
            <input
              v-model="config.video.overlayColor"
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
            v-model="config.utilities.alarmEnabled"
            type="checkbox"
            class="sr-only peer"
          >
          <div class="w-11 h-6 bg-border-strong/35 border border-border-strong peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-white after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-primary peer-checked:border-primary shadow-inner" />
        </label>
      </div>
      
      <div
        v-if="config.utilities.alarmEnabled"
        class="p-4 bg-surface rounded-2xl border-primary shadow-sm space-y-4"
      >
        <div>
          <label class="block text-sm font-bold text-primary mb-2">{{ t('ovr.util_alarm_time') }}</label>
          <input
            v-model="config.utilities.alarmTime"
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
            v-model="config.utilities.trackerBattery"
            type="checkbox"
            class="sr-only peer"
          >
          <div class="w-11 h-6 bg-border-strong/35 border border-border-strong peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-white after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-primary peer-checked:border-primary shadow-inner" />
        </label>
      </div>

      <div class="p-4 bg-surface rounded-2xl border-primary shadow-sm space-y-4">
        <label class="flex items-center justify-between">
          <span class="text-sm font-bold text-primary">{{ t('ovr.util_media_keys') }}</span>
          <input
            v-model="config.utilities.mediaKeys"
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
            v-model="config.utilities.keyboard"
            type="checkbox"
            class="rounded text-primary"
          >
        </label>
      </div>
    </div>
  </div>

  <!-- 鎬ц兘缁熻 -->
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
          <button class="px-4 py-2 bg-primary text-white hover:bg-primary-hover font-bold rounded-lg transition-colors text-sm">
            {{ t('ovr.stats_reset') }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

