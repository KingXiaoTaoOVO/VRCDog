<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import { currentTheme } from '../theme';
import { Ruler, Target } from 'lucide-vue-next';

const { t } = useI18n();

const props = defineProps<{
  vrDashboardTab: string;
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

</script>

<template>
  <!-- SteamVR -->
  <div
    v-if="vrDashboardTab === 'steamvr'"
    class="vr-dash-section animate-fade-in"
  >
    <div class="vr-dash-row">
      <span>{{ t('ovr.steamvr_timing_overlay') }}</span>
      <div
        class="vr-dash-switch"
        :class="{ 'on': config.steamvr.timingOverlay }"
        @click="updateConfig('steamvr.timingOverlay', !config.steamvr.timingOverlay)"
      >
        <div class="vr-dash-switch-knob" />
      </div>
    </div>
    <div class="vr-dash-row">
      <span>{{ t('ovr.steamvr_camera_enable') }}</span>
      <div
        class="vr-dash-switch"
        :class="{ 'on': config.steamvr.cameraEnable }"
        @click="updateConfig('steamvr.cameraEnable', !config.steamvr.cameraEnable)"
      >
        <div class="vr-dash-switch-knob" />
      </div>
    </div>
  </div>

  <!-- Chaperone -->
  <div
    v-else-if="vrDashboardTab === 'chaperone'"
    class="vr-dash-section animate-fade-in"
  >
    <div class="vr-dash-row">
      <span>{{ t('ovr.chap_visibility') }}</span>
      <span
        class="vr-dash-value"
        @click="updateConfig('chaperone.visibility', config.chaperone.visibility >= 100 ? 30 : config.chaperone.visibility + 10)"
      >{{ config.chaperone.visibility }}%</span>
    </div>
    <div class="vr-dash-row">
      <span>{{ t('ovr.chap_force_bounds') }}</span>
      <div
        class="vr-dash-switch"
        :class="{ 'on': config.chaperone.forceBounds }"
        @click="updateConfig('chaperone.forceBounds', !config.chaperone.forceBounds)"
      >
        <div class="vr-dash-switch-knob" />
      </div>
    </div>
    <div class="vr-dash-row mt-2">
      <span style="font-size: 11px; opacity: 0.8">Advanced / Warnings</span>
      <div class="flex gap-2">
        <div
          class="vr-dash-switch"
          :class="{ 'on': config.chaperone.hapticFeedback }"
          @click="updateConfig('chaperone.hapticFeedback', !config.chaperone.hapticFeedback)"
        >
          <div class="vr-dash-switch-knob" />
        </div>
        <span style="font-size: 11px">{{ t('ovr.chap_haptic_feedback') }}</span>
      </div>
    </div>
  </div>

  <!-- Playspace -->
  <div
    v-else-if="vrDashboardTab === 'playspace'"
    class="vr-dash-section animate-fade-in"
  >
    <div class="vr-dash-row">
      <span>{{ t('ovr.space_adjust_chap') }}</span>
      <div
        class="vr-dash-switch"
        :class="{ 'on': config.general.spaceAdjustChap }"
        @click="updateConfig('general.spaceAdjustChap', !config.general.spaceAdjustChap)"
      >
        <div class="vr-dash-switch-knob" />
      </div>
    </div>
    <div class="vr-dash-row">
      <span>{{ t('ovr.motion_drag_left') }}</span>
      <div
        class="vr-dash-switch"
        :class="{ 'on': config.playspace.dragLeft }"
        @click="updateConfig('playspace.dragLeft', !config.playspace.dragLeft)"
      >
        <div class="vr-dash-switch-knob" />
      </div>
    </div>
    <div class="vr-dash-row mt-2">
      <span style="font-size: 11px; opacity: 0.8">{{ t('ovr.space_fix_section') }}</span>
      <div class="grid grid-cols-2 gap-4">
        <button
          class="vr-dash-btn"
          @click="console.log('fix floor')"
        >
          <Ruler class="w-5 h-5 inline-block mr-2" /> {{ t('ovr.space_fix_floor') }}
        </button>
        <button
          class="vr-dash-btn"
          @click="console.log('fix center')"
        >
          <Target class="w-5 h-5 inline-block mr-2" /> {{ t('ovr.space_fix_center') }}
        </button>
      </div>
    </div>
    <div
      class="vr-dash-row mt-2"
      style="flex-direction: column; align-items: flex-start; gap: 8px;"
    >
      <div
        class="flex justify-between"
        style="width: 100%"
      >
        <span style="font-size: 11px; opacity: 0.8">Rotation ({{ config.playspace.rotation || 0 }}°)</span>
        <div class="flex gap-2">
          <button
            class="vr-dash-btn"
            @click="updateConfig('playspace.rotation', -90)"
          >
            -90°
          </button>
          <button
            class="vr-dash-btn"
            @click="updateConfig('playspace.rotation', 0)"
          >{{ t('ovr.stats_reset') }}</button>
          <button
            class="vr-dash-btn"
            @click="updateConfig('playspace.rotation', 90)"
          >
            +90°
          </button>
        </div>
      </div>
      <div
        class="flex justify-between"
        style="width: 100%; align-items: center"
      >
        <span style="font-size: 11px; opacity: 0.8">{{ t('ovr.motion_gravity') }}</span>
        <div
          class="vr-dash-switch"
          :class="{ 'on': config.general.motionGravityOn }"
          @click="updateConfig('general.motionGravityOn', !config.general.motionGravityOn)"
        >
          <div class="vr-dash-switch-knob" />
        </div>
      </div>
    </div>
  </div>

  <!-- Audio -->
  <div
    v-else-if="vrDashboardTab === 'audio'"
    class="vr-dash-section animate-fade-in"
  >
    <div class="vr-dash-row">
      <span>{{ t('ovr.audio_prox_sensor') }}</span>
      <div
        class="vr-dash-switch"
        :class="{ 'on': config.audio.proxSensor }"
        @click="updateConfig('audio.proxSensor', !config.audio.proxSensor)"
      >
        <div class="vr-dash-switch-knob" />
      </div>
    </div>
    <div class="vr-dash-row">
      <span>{{ t('ovr.audio_ptt') }}</span>
      <div
        class="vr-dash-switch"
        :class="{ 'on': config.audio.pTT }"
        @click="updateConfig('audio.pTT', !config.audio.pTT)"
      >
        <div class="vr-dash-switch-knob" />
      </div>
    </div>
  </div>

  <!-- Video -->
  <div
    v-else-if="vrDashboardTab === 'video'"
    class="vr-dash-section animate-fade-in"
  >
    <div class="vr-dash-row">
      <span>{{ t('ovr.video_brightness_on') }}</span>
      <div
        class="vr-dash-switch"
        :class="{ 'on': config.video.brightnessOn }"
        @click="updateConfig('video.brightnessOn', !config.video.brightnessOn)"
      >
        <div class="vr-dash-switch-knob" />
      </div>
    </div>
    <div class="vr-dash-row">
      <span>{{ t('ovr.video_motion_smooth') }}</span>
      <div
        class="vr-dash-switch"
        :class="{ 'on': config.video.motionSmooth }"
        @click="updateConfig('video.motionSmooth', !config.video.motionSmooth)"
      >
        <div class="vr-dash-switch-knob" />
      </div>
    </div>
    <div class="vr-dash-row mt-2">
      <span style="font-size: 11px; opacity: 0.8">{{ t('ovr.video_super_sampling') }}</span>
      <span
        class="vr-dash-value"
        @click="updateConfig('video.superSampling', config.video.superSampling >= 500 ? 100 : (config.video.superSampling || 100) + 50)"
      >{{ config.video.superSampling || 100 }}%</span>
    </div>
  </div>

  <!-- Utilities -->
  <div
    v-else-if="vrDashboardTab === 'utilities'"
    class="vr-dash-section animate-fade-in"
  >
    <div class="vr-dash-row">
      <span>{{ t('ovr.util_alarm_enabled') }}</span>
      <div
        class="vr-dash-switch"
        :class="{ 'on': config.utilities.alarmEnabled }"
        @click="updateConfig('utilities.alarmEnabled', !config.utilities.alarmEnabled)"
      >
        <div class="vr-dash-switch-knob" />
      </div>
    </div>
    <div class="vr-dash-row">
      <span>{{ t('ovr.util_tracker_battery') }}</span>
      <div
        class="vr-dash-switch"
        :class="{ 'on': config.utilities.trackerBattery }"
        @click="updateConfig('utilities.trackerBattery', !config.utilities.trackerBattery)"
      >
        <div class="vr-dash-switch-knob" />
      </div>
    </div>
    <div class="vr-dash-row mt-2">
      <span style="font-size: 11px; opacity: 0.8">{{ t('ovr.util_media_keys') }}</span>
      <div
        class="vr-dash-switch"
        :class="{ 'on': config.utilities.mediaKeys }"
        @click="updateConfig('utilities.mediaKeys', !config.utilities.mediaKeys)"
      >
        <div class="vr-dash-switch-knob" />
      </div>
    </div>
  </div>

  <!-- Statistics -->
  <div
    v-else-if="vrDashboardTab === 'statistics'"
    class="vr-dash-section animate-fade-in"
  >
    <div class="vr-dash-row">
      <span>{{ t('ovr.stats_hmd_distance') }}</span>
      <span class="vr-dash-value">0 {{ t('ovr.stats_unit_m') }}</span>
    </div>
    <div class="vr-dash-row">
      <span>{{ t('ovr.stats_reprojection_ratio') }}</span>
      <span class="vr-dash-value">0%</span>
    </div>
  </div>
</template>

<style scoped>
.animate-fade-in { animation: fadeIn 0.2s; }
@keyframes fadeIn { from { opacity: 0; } to { opacity: 1; } }

.vr-dash-section {
  display: flex;
  flex-direction: column;
  gap: 16px;
}
.vr-dash-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  color: v-bind('currentTheme.colors.textStrong');
  font-size: 13px;
  font-weight: 600;
  padding-bottom: 8px;
  border-bottom: 1px dashed v-bind('currentTheme.colors.borderSoft');
}
.vr-dash-value {
  color: v-bind('currentTheme.colors.textSoft');
  font-weight: 800;
  font-size: 12px;
  background: v-bind('currentTheme.colors.bgMain');
  padding: 4px 10px;
  border-radius: 6px;
  border: 1px solid v-bind('currentTheme.colors.borderStrong');
  cursor: pointer;
}
.vr-dash-switch {
  width: 32px; height: 18px;
  background: rgba(0, 0, 0, 0.1);
  border-radius: 10px;
  position: relative;
  cursor: pointer;
  transition: background 0.3s;
  box-shadow: inset 0 1px 3px rgba(0,0,0,0.1);
}
.vr-dash-switch.on { background: v-bind('currentTheme.colors.primaryBtnBg'); }
.vr-dash-switch-knob {
  width: 14px; height: 14px;
  background: white;
  border-radius: 50%;
  position: absolute;
  top: 2px; left: 2px;
  transition: transform 0.3s cubic-bezier(0.34, 1.56, 0.64, 1);
  box-shadow: 0 1px 3px rgba(0,0,0,0.2);
}
.vr-dash-switch.on .vr-dash-switch-knob { transform: translateX(14px); }
.vr-dash-btn {
  background: v-bind('currentTheme.colors.bgMain');
  color: v-bind('currentTheme.colors.textStrong');
  border: 1px solid v-bind('currentTheme.colors.borderStrong');
  padding: 4px 8px;
  border-radius: 6px;
  font-size: 11px;
  font-weight: 700;
  cursor: pointer;
}
.vr-dash-btn:active {
  background: rgba(0,0,0,0.1);
}
.mt-2 { margin-top: 8px; }
.flex { display: flex; }
.gap-2 { gap: 8px; }
</style>
