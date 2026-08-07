<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { useI18n } from 'vue-i18n';
import { LogOut, MonitorUp, Settings, X } from 'lucide-vue-next';
import { currentTheme } from '../theme';

const { t } = useI18n();
const isBusy = ref(false);
let blurTimer: ReturnType<typeof setTimeout> | null = null;

const appTitle = computed(() => currentTheme.value.appTitle || 'VrcDog');
const quitLabel = computed(() => t('tray.quit', { app: appTitle.value }));
const logo = computed(() => currentTheme.value.logo);
const themeStyle = computed(() => ({
  '--tray-accent': currentTheme.value.colors.primaryBtnBg,
  '--tray-accent-hover': currentTheme.value.colors.primaryBtnHover,
  '--tray-surface': currentTheme.value.colors.surface,
  '--tray-surface-hover': currentTheme.value.colors.surfaceHover,
  '--tray-border': currentTheme.value.colors.borderStrong,
  '--tray-border-soft': currentTheme.value.colors.borderSoft,
  '--tray-text': currentTheme.value.colors.text,
  '--tray-text-strong': currentTheme.value.colors.textStrong,
  '--tray-text-muted': currentTheme.value.colors.textMuted,
  '--tray-bg': currentTheme.value.colors.bgMain,
}));

const closeMenu = async () => {
  await invoke('tray_close_menu');
};

const runCommand = async (command: 'tray_show_main_window' | 'tray_open_settings' | 'tray_quit_app') => {
  if (isBusy.value) return;
  isBusy.value = true;
  try {
    await invoke(command);
  } finally {
    isBusy.value = false;
  }
};

const handleKeydown = (event: KeyboardEvent) => {
  if (event.key === 'Escape') closeMenu().catch(() => {});
};

const handleBlur = () => {
  if (blurTimer) clearTimeout(blurTimer);
  blurTimer = setTimeout(() => closeMenu().catch(() => {}), 160);
};

onMounted(() => {
  document.documentElement.classList.add('tray-menu-document');
  window.addEventListener('keydown', handleKeydown);
  window.addEventListener('blur', handleBlur);
});

onBeforeUnmount(() => {
  document.documentElement.classList.remove('tray-menu-document');
  window.removeEventListener('keydown', handleKeydown);
  window.removeEventListener('blur', handleBlur);
  if (blurTimer) clearTimeout(blurTimer);
});
</script>

<template>
  <div class="tray-shell" :style="themeStyle">
    <section class="tray-card" :aria-label="t('tray.aria_label')">
      <div class="glass-highlight" aria-hidden="true" />
      <header class="tray-header">
        <img v-if="logo" class="tray-logo" :src="logo" alt="">
        <div class="tray-title-wrap">
          <h1 class="tray-title">{{ appTitle }}</h1>
          <p class="tray-subtitle">{{ t('tray.quick_menu') }}</p>
        </div>
        <button class="icon-button" :title="t('tray.close')" type="button" :aria-label="t('tray.close')" @click="closeMenu">
          <X :size="16" stroke-width="2.4" />
        </button>
      </header>

      <div class="tray-actions">
        <button class="tray-action primary" type="button" :disabled="isBusy" @click="runCommand('tray_show_main_window')">
          <span class="action-icon"><MonitorUp :size="18" /></span>
          <span>{{ t('tray.show_main') }}</span>
        </button>

        <button class="tray-action" type="button" :disabled="isBusy" @click="runCommand('tray_open_settings')">
          <span class="action-icon"><Settings :size="18" /></span>
          <span>{{ t('tray.open_settings') }}</span>
        </button>

        <div class="tray-divider" />

        <button class="tray-action danger" type="button" :disabled="isBusy" @click="runCommand('tray_quit_app')">
          <span class="action-icon"><LogOut :size="18" /></span>
          <span>{{ quitLabel }}</span>
        </button>
      </div>
    </section>
  </div>
</template>

<style scoped>
:global(html.tray-menu-document),
:global(html.tray-menu-document body),
:global(html.tray-menu-document #app) {
  width: 100%;
  height: 100%;
  margin: 0;
  overflow: hidden;
  background: transparent !important;
}

.tray-shell {
  width: 100vw;
  height: 100vh;
  padding: 8px;
  box-sizing: border-box;
  background: transparent;
  color: var(--tray-text);
  font-family: Inter, "Microsoft YaHei", system-ui, -apple-system, BlinkMacSystemFont, sans-serif;
  user-select: none;
}

.tray-card {
  position: relative;
  width: 100%;
  height: 100%;
  box-sizing: border-box;
  overflow: hidden;
  border: 1px solid rgba(255, 255, 255, 0.82);
  border-radius: 8px;
  background: color-mix(in srgb, var(--tray-surface) 74%, rgba(255, 255, 255, 0.62));
  box-shadow:
    0 6px 12px rgba(37, 45, 48, 0.16),
    inset 0 1px 0 rgba(255, 255, 255, 0.94),
    inset 0 0 0 1px rgba(54, 70, 74, 0.05);
  backdrop-filter: blur(28px) saturate(150%);
  -webkit-backdrop-filter: blur(28px) saturate(150%);
}

@supports not ((backdrop-filter: blur(1px)) or (-webkit-backdrop-filter: blur(1px))) {
  .tray-card { background: rgba(255, 250, 240, 0.96); }
}

.glass-highlight {
  position: absolute;
  inset: 0 auto auto 12%;
  width: 70%;
  height: 1px;
  background: linear-gradient(90deg, transparent, rgba(255, 255, 255, 0.96), transparent);
  pointer-events: none;
}

.tray-header {
  position: relative;
  display: flex;
  align-items: center;
  gap: 11px;
  min-height: 64px;
  padding: 12px 13px 8px;
  box-sizing: border-box;
}

.tray-logo {
  width: 38px;
  height: 38px;
  flex: 0 0 auto;
  border-radius: 8px;
  object-fit: cover;
  border: 1px solid rgba(255, 255, 255, 0.84);
  box-shadow: 0 7px 18px rgba(70, 43, 20, 0.16);
}

.tray-title-wrap {
  min-width: 0;
  flex: 1;
  padding-right: 36px;
}

.tray-title {
  margin: 0;
  overflow: hidden;
  color: var(--tray-text-strong);
  font-size: 15px;
  line-height: 20px;
  font-weight: 850;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.tray-subtitle {
  margin: 2px 0 0;
  color: var(--tray-text-muted);
  font-size: 11px;
  line-height: 15px;
  font-weight: 650;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.icon-button {
  position: absolute;
  top: 14px;
  right: 13px;
  z-index: 2;
  width: 32px;
  height: 32px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border: 1px solid rgba(117, 75, 38, 0.14);
  border-radius: 7px;
  color: var(--tray-text-muted);
  background: rgba(255, 255, 255, 0.42);
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.72);
  cursor: pointer;
  transition: transform 140ms ease, background 140ms ease, color 140ms ease;
}

.icon-button:hover {
  transform: rotate(4deg);
  color: var(--tray-text-strong);
  background: rgba(255, 255, 255, 0.68);
}

.tray-actions {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 3px 12px 12px;
}

.tray-action {
  width: 100%;
  min-height: 41px;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 0 12px;
  box-sizing: border-box;
  border: 1px solid rgba(126, 83, 42, 0.08);
  border-radius: 7px;
  color: var(--tray-text);
  background: rgba(255, 255, 255, 0.3);
  font-size: 13px;
  font-weight: 750;
  text-align: left;
  cursor: pointer;
  transition: transform 140ms ease, background 140ms ease, border-color 140ms ease, color 140ms ease, box-shadow 140ms ease;
}

.tray-action:hover {
  transform: translateY(-1px);
  color: var(--tray-text-strong);
  background: rgba(255, 255, 255, 0.58);
  border-color: rgba(126, 83, 42, 0.16);
  box-shadow: 0 7px 16px rgba(76, 47, 21, 0.08);
}

.tray-action:focus-visible,
.icon-button:focus-visible {
  outline: 3px solid color-mix(in srgb, var(--tray-accent) 30%, transparent);
  outline-offset: 2px;
}

.tray-action:disabled {
  cursor: default;
  opacity: 0.62;
  transform: none;
}

.tray-action.primary {
  color: #fffdf8;
  background: var(--tray-accent);
  border-color: rgba(255, 255, 255, 0.34);
  box-shadow: 0 9px 21px color-mix(in srgb, var(--tray-accent) 25%, transparent), inset 0 1px 0 rgba(255, 255, 255, 0.3);
}

.action-icon {
  width: 25px;
  height: 25px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex: 0 0 auto;
}

.tray-divider {
  height: 1px;
  margin: 1px 5px;
  background: linear-gradient(90deg, transparent, rgba(111, 72, 35, 0.18), transparent);
}

.tray-action.danger {
  color: #b84738;
  background: rgba(255, 255, 255, 0.24);
}

.tray-action.danger:hover {
  color: #9f3026;
  background: rgba(255, 244, 241, 0.72);
  border-color: rgba(184, 71, 56, 0.18);
}

@media (prefers-reduced-motion: reduce) {
  .tray-action,
  .icon-button { transition: none; }
}
</style>
