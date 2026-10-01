<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { getVersion } from '@tauri-apps/api/app';
import { useI18n } from 'vue-i18n';
import { ChevronRight, LogOut, MonitorUp, Settings, SlidersHorizontal, X } from 'lucide-vue-next';
import { currentTheme } from '../theme';

const { t } = useI18n();
const isBusy = ref(false);
const appVersion = ref('v5.7.4');
let blurTimer: ReturnType<typeof setTimeout> | null = null;

const appTitle = computed(() => currentTheme.value.appTitle || 'VrcDog');
const quitLabel = computed(() => t('tray.quit', { app: appTitle.value }));
const logo = computed(() => currentTheme.value.logo);

const themeStyle = computed(() => ({
  '--tray-accent': currentTheme.value.colors.primaryBtnBg || '#d97706',
  '--tray-accent-hover': currentTheme.value.colors.primaryBtnHover || '#b45309',
  '--tray-surface': currentTheme.value.colors.surface || 'rgba(255, 252, 240, 0.75)',
  '--tray-surface-hover': currentTheme.value.colors.surfaceHover || 'rgba(255, 252, 240, 0.95)',
  '--tray-border': currentTheme.value.colors.borderStrong || 'rgba(120, 53, 15, 0.16)',
  '--tray-border-soft': currentTheme.value.colors.borderSoft || 'rgba(120, 53, 15, 0.08)',
  '--tray-text': currentTheme.value.colors.text || '#292524',
  '--tray-text-strong': currentTheme.value.colors.textStrong || '#0c0a09',
  '--tray-text-muted': currentTheme.value.colors.textMuted || '#78716c',
  '--tray-bg': currentTheme.value.colors.bgMain || '#faf7ed',
}));

const closeMenu = async () => {
  try {
    await invoke('tray_close_menu');
  } catch (err) {
    console.warn('tray_close_menu error:', err);
  }
};

const runCommand = async (command: 'tray_show_main_window' | 'tray_open_settings' | 'tray_reselect_mode' | 'tray_quit_app') => {
  if (isBusy.value) return;
  isBusy.value = true;
  try {
    await invoke(command);
  } catch (err) {
    console.error(`Command ${command} failed:`, err);
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

onMounted(async () => {
  document.documentElement.classList.add('tray-menu-document');
  window.addEventListener('keydown', handleKeydown);
  window.addEventListener('blur', handleBlur);
  try {
    const ver = await getVersion();
    if (ver) {
      appVersion.value = `v${ver}`;
    }
  } catch {
    // fallback to default version
  }
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
      <!-- Top Subtle Acrylic Reflection Line -->
      <div class="glass-highlight" aria-hidden="true" />

      <!-- Header: Logo + AppTitle & Pulse Status + Close Button -->
      <header class="tray-header">
        <div class="tray-brand">
          <img v-if="logo" class="tray-logo" :src="logo" alt="" />
          <div class="tray-title-wrap">
            <div class="tray-title-row">
              <span class="tray-title">{{ appTitle }}</span>
              <span class="status-badge" :title="t('tray.status_ready')">
                <span class="status-pulse-dot" />
                <span class="status-text">{{ t('tray.status_ready') }}</span>
              </span>
            </div>
            <p class="tray-subtitle">{{ t('tray.quick_menu') }}</p>
          </div>
        </div>
        <button
          class="close-button"
          :title="t('tray.close')"
          type="button"
          :aria-label="t('tray.close')"
          @click="closeMenu"
        >
          <X :size="14" stroke-width="2.2" />
        </button>
      </header>

      <!-- Action Buttons -->
      <div class="tray-actions">
        <!-- 1. Primary: Show Main Window -->
        <button
          class="tray-action primary"
          type="button"
          :disabled="isBusy"
          @click="runCommand('tray_show_main_window')"
        >
          <span class="action-icon primary-icon">
            <MonitorUp :size="16" stroke-width="2.2" />
          </span>
          <span class="action-label-wrap">
            <span class="action-title">{{ t('tray.show_main') }}</span>
          </span>
          <ChevronRight :size="14" stroke-width="2.2" class="action-chevron" />
        </button>

        <!-- 2. Settings -->
        <button
          class="tray-action"
          type="button"
          :disabled="isBusy"
          @click="runCommand('tray_open_settings')"
        >
          <span class="action-icon">
            <Settings :size="16" stroke-width="2" />
          </span>
          <span class="action-label-wrap">
            <span class="action-title">{{ t('tray.open_settings') }}</span>
          </span>
        </button>

        <!-- 3. Switch Mode (PC / VR) -->
        <button
          class="tray-action"
          type="button"
          :disabled="isBusy"
          @click="runCommand('tray_reselect_mode')"
        >
          <span class="action-icon">
            <SlidersHorizontal :size="16" stroke-width="2" />
          </span>
          <span class="action-label-wrap">
            <span class="action-title">{{ t('tray.switch_mode') }}</span>
          </span>
        </button>

        <div class="tray-divider" role="separator" />

        <!-- 4. Quit -->
        <button
          class="tray-action danger"
          type="button"
          :disabled="isBusy"
          @click="runCommand('tray_quit_app')"
        >
          <span class="action-icon danger-icon">
            <LogOut :size="16" stroke-width="2" />
          </span>
          <span class="action-label-wrap">
            <span class="action-title">{{ quitLabel }}</span>
          </span>
        </button>
      </div>

      <!-- Footer Info -->
      <footer class="tray-footer">
        <span class="footer-version">{{ appTitle }} {{ appVersion }}</span>
      </footer>
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
  padding: 0;
  overflow: hidden !important;
  background: transparent !important;
}

.tray-shell {
  width: 100vw;
  height: 100vh;
  padding: 8px;
  box-sizing: border-box;
  background: transparent !important;
  color: var(--tray-text);
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI Variable Text", "Segoe UI", "PingFang SC", "Hiragino Sans GB", "Microsoft YaHei", sans-serif;
  user-select: none;
  overflow: hidden;
}

.tray-card {
  position: relative;
  width: 100%;
  height: 100%;
  box-sizing: border-box;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  border-radius: 14px;
  background: rgba(255, 255, 255, 0.84);
  border: 1px solid rgba(255, 255, 255, 0.68);
  box-shadow:
    0 12px 32px -4px rgba(0, 0, 0, 0.16),
    0 4px 12px -2px rgba(0, 0, 0, 0.08),
    inset 0 1px 0 rgba(255, 255, 255, 0.95),
    inset 0 -1px 0 rgba(0, 0, 0, 0.03);
  backdrop-filter: blur(32px) saturate(180%);
  -webkit-backdrop-filter: blur(32px) saturate(180%);
  transition: transform 160ms ease, box-shadow 160ms ease;
}

@supports not ((backdrop-filter: blur(1px)) or (-webkit-backdrop-filter: blur(1px))) {
  .tray-card {
    background: rgba(255, 255, 255, 0.96);
  }
}

.glass-highlight {
  position: absolute;
  top: 0;
  left: 10%;
  width: 80%;
  height: 1px;
  background: linear-gradient(90deg, transparent, rgba(255, 255, 255, 0.95), transparent);
  pointer-events: none;
}

.tray-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 14px 10px;
  border-bottom: 1px solid rgba(0, 0, 0, 0.05);
}

.tray-brand {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
}

.tray-logo {
  width: 34px;
  height: 34px;
  border-radius: 9px;
  object-fit: cover;
  box-shadow: 0 3px 8px rgba(0, 0, 0, 0.12);
  border: 1px solid rgba(255, 255, 255, 0.8);
}

.tray-title-wrap {
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.tray-title-row {
  display: flex;
  align-items: center;
  gap: 7px;
}

.tray-title {
  font-size: 13.5px;
  font-weight: 700;
  color: var(--tray-text-strong);
  letter-spacing: -0.2px;
}

.status-badge {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 1.5px 6px;
  border-radius: 10px;
  background: rgba(16, 185, 129, 0.12);
  border: 1px solid rgba(16, 185, 129, 0.25);
}

.status-pulse-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: #10b981;
  box-shadow: 0 0 6px #10b981;
  animation: pulse-ring 2s cubic-bezier(0.4, 0, 0.6, 1) infinite;
}

@keyframes pulse-ring {
  0%, 100% {
    opacity: 1;
    transform: scale(1);
  }
  50% {
    opacity: 0.5;
    transform: scale(0.85);
  }
}

.status-text {
  font-size: 10px;
  font-weight: 600;
  color: #065f46;
  white-space: nowrap;
}

.tray-subtitle {
  margin: 1px 0 0;
  font-size: 10.5px;
  color: var(--tray-text-muted);
}

.close-button {
  width: 24px;
  height: 24px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border-radius: 6px;
  border: none;
  background: transparent;
  color: #a8a29e;
  cursor: pointer;
  transition: all 120ms ease;
}

.close-button:hover {
  background: rgba(0, 0, 0, 0.06);
  color: var(--tray-text-strong);
}

.tray-actions {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 10px 10px 6px;
}

.tray-action {
  width: 100%;
  height: 38px;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 0 10px;
  box-sizing: border-box;
  border-radius: 9px;
  border: 1px solid transparent;
  background: rgba(255, 255, 255, 0.45);
  color: var(--tray-text);
  font-size: 12.5px;
  font-weight: 600;
  text-align: left;
  cursor: pointer;
  transition: all 140ms cubic-bezier(0.16, 1, 0.3, 1);
}

.tray-action:hover {
  background: rgba(255, 255, 255, 0.85);
  border-color: rgba(0, 0, 0, 0.06);
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.05);
  transform: translateY(-0.5px);
}

.tray-action:active {
  transform: scale(0.985);
}

.tray-action.primary {
  background: linear-gradient(135deg, color-mix(in srgb, var(--tray-accent) 90%, #fff), var(--tray-accent));
  color: #ffffff;
  box-shadow: 0 4px 12px color-mix(in srgb, var(--tray-accent) 30%, transparent), inset 0 1px 0 rgba(255, 255, 255, 0.3);
}

.tray-action.primary:hover {
  background: linear-gradient(135deg, var(--tray-accent), var(--tray-accent-hover));
  box-shadow: 0 6px 16px color-mix(in srgb, var(--tray-accent) 42%, transparent), inset 0 1px 0 rgba(255, 255, 255, 0.35);
  border-color: transparent;
}

.action-icon {
  width: 22px;
  height: 22px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  color: var(--tray-text-muted);
}

.primary .action-icon {
  color: #ffffff;
}

.action-label-wrap {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.action-title {
  display: block;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.action-chevron {
  opacity: 0.7;
  color: #ffffff;
  margin-left: auto;
}

.tray-divider {
  height: 1px;
  margin: 3px 6px;
  background: linear-gradient(90deg, transparent, rgba(0, 0, 0, 0.08), transparent);
}

.tray-action.danger {
  color: #dc2626;
}

.tray-action.danger .action-icon {
  color: #dc2626;
}

.tray-action.danger:hover {
  background: rgba(254, 242, 242, 0.85);
  border-color: rgba(239, 68, 68, 0.2);
  color: #b91c1c;
}

.tray-footer {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 6px 12px 10px;
  border-top: 1px solid rgba(0, 0, 0, 0.04);
}

.footer-version {
  font-size: 10px;
  font-weight: 500;
  color: var(--tray-text-muted);
  letter-spacing: 0.3px;
  opacity: 0.8;
}

@media (prefers-reduced-motion: reduce) {
  .tray-action,
  .close-button,
  .status-pulse-dot {
    transition: none;
    animation: none;
  }
}
</style>
