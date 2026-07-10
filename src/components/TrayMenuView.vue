<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { LogOut, MonitorUp, Settings } from 'lucide-vue-next';
import { currentTheme } from '../theme';

const isBusy = ref(false);
let blurTimer: ReturnType<typeof setTimeout> | null = null;

const appTitle = computed(() => currentTheme.value.appTitle || 'VrcDog');
const quitLabel = computed(() => `退出 ${appTitle.value}`);
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
  '--tray-glass': currentTheme.value.colors.glassEffect,
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
  if (event.key === 'Escape') {
    closeMenu().catch(() => {});
  }
};

const handleBlur = () => {
  if (blurTimer) clearTimeout(blurTimer);
  blurTimer = setTimeout(() => {
    closeMenu().catch(() => {});
  }, 160);
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
    <section class="tray-card" aria-label="VrcDog tray menu">
      <header class="tray-header">
        <img v-if="logo" class="tray-logo" :src="logo" alt="">
        <div class="tray-title-wrap">
          <h1 class="tray-title">
            {{ appTitle }}
          </h1>
          <p class="tray-subtitle">
            快速菜单
          </p>
        </div>
        <button class="icon-button" title="关闭" type="button" aria-label="关闭" @click="closeMenu">
          <span class="close-mark">X</span>
        </button>
      </header>

      <div class="tray-actions">
        <button class="tray-action primary" type="button" :disabled="isBusy" @click="runCommand('tray_show_main_window')">
          <span class="action-icon"><MonitorUp :size="17" /></span>
          <span>显示主面板</span>
        </button>

        <button class="tray-action" type="button" :disabled="isBusy" @click="runCommand('tray_open_settings')">
          <span class="action-icon"><Settings :size="17" /></span>
          <span>打开设置</span>
        </button>

        <div class="tray-divider"></div>

        <button class="tray-action danger" type="button" :disabled="isBusy" @click="runCommand('tray_quit_app')">
          <span class="action-icon"><LogOut :size="17" /></span>
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
  background: var(--theme-bg-main, #faf7ed);
}

.tray-shell {
  width: 100vw;
  height: 100vh;
  padding: 0;
  box-sizing: border-box;
  background: var(--tray-bg);
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
  border: 1px solid var(--tray-border);
  border-radius: 0;
  background:
    linear-gradient(135deg, rgba(255, 255, 255, 0.66), rgba(255, 255, 255, 0.18)),
    var(--tray-bg);
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.52);
}

.tray-header {
  display: flex;
  align-items: center;
  gap: 9px;
  height: 58px;
  padding: 10px 10px 8px;
  box-sizing: border-box;
}

.tray-logo {
  width: 34px;
  height: 34px;
  flex: 0 0 auto;
  border-radius: 8px;
  object-fit: cover;
  border: 1px solid var(--tray-border-soft);
  box-shadow: 0 6px 14px rgba(15, 23, 42, 0.12);
}

.tray-title-wrap {
  min-width: 0;
  flex: 1;
}

.tray-title {
  margin: 0;
  color: var(--tray-text-strong);
  font-size: 14px;
  line-height: 18px;
  font-weight: 800;
  letter-spacing: 0;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.tray-subtitle {
  margin: 1px 0 0;
  color: var(--tray-text-muted);
  font-size: 11px;
  line-height: 14px;
  font-weight: 600;
}

.icon-button {
  position: absolute;
  top: 12px;
  right: 10px;
  z-index: 2;
  width: 32px;
  height: 32px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex: 0 0 auto;
  border: 1px solid var(--tray-border);
  border-radius: 8px;
  color: #fff;
  background: rgba(69, 26, 3, 0.82);
  box-shadow: 0 6px 16px rgba(15, 23, 42, 0.16);
  cursor: pointer;
  transition: background 140ms ease, color 140ms ease, border-color 140ms ease;
}

.close-mark {
  display: block;
  font-size: 18px;
  line-height: 1;
  font-weight: 800;
}

.icon-button:hover {
  color: var(--tray-text-strong);
  background: var(--tray-surface-hover);
  border-color: var(--tray-border-soft);
}

.tray-actions {
  display: flex;
  flex-direction: column;
  gap: 5px;
  padding: 4px 10px 10px;
}

.tray-action {
  width: 100%;
  height: 38px;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 0 10px;
  box-sizing: border-box;
  border: 1px solid transparent;
  border-radius: 8px;
  color: var(--tray-text);
  background: transparent;
  font-size: 13px;
  font-weight: 700;
  text-align: left;
  cursor: pointer;
  transition: transform 140ms ease, background 140ms ease, border-color 140ms ease, color 140ms ease, box-shadow 140ms ease;
}

.tray-action:hover {
  transform: translateY(-1px);
  color: var(--tray-text-strong);
  background: var(--tray-surface-hover);
  border-color: var(--tray-border-soft);
  box-shadow: 0 8px 18px rgba(15, 23, 42, 0.08);
}

.tray-action:disabled {
  cursor: default;
  opacity: 0.62;
  transform: none;
}

.tray-action.primary {
  color: #fff;
  background: var(--tray-accent);
  border-color: color-mix(in srgb, var(--tray-accent) 72%, #fff);
  box-shadow: 0 10px 22px color-mix(in srgb, var(--tray-accent) 32%, transparent);
}

.tray-action.primary:hover {
  color: #fff;
  background: var(--tray-accent-hover);
}

.action-icon {
  width: 22px;
  height: 22px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex: 0 0 auto;
}

.tray-divider {
  height: 1px;
  margin: 3px 2px;
  background: var(--tray-border-soft);
}

.tray-action.danger {
  color: #dc2626;
}

.tray-action.danger:hover {
  color: #b91c1c;
  background: rgba(239, 68, 68, 0.1);
  border-color: rgba(239, 68, 68, 0.22);
}
</style>
