<script setup lang="ts">
import { onMounted, onBeforeUnmount } from 'vue';
import { ShieldAlert, Monitor, Glasses, ArrowRight } from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';
import { SysApi } from '../../api';
import { currentTheme } from '../../theme';
import { useUiStore } from '../../stores/uiStore';
import { storeToRefs } from 'pinia';

const { t } = useI18n();
const uiStore = useUiStore();
const { serverModePerms, modeSelectionError } = storeToRefs(uiStore);

// While the mode-selection screen is shown, route external "open detail" requests
// to the search tab. Register once on mount and remove on unmount so the listener
// never accumulates (it used to be added on every click) or keeps hijacking
// vrc-open-detail after a mode has already been chosen.
const handleOpenDetail = () => {
  if (uiStore.activeTab !== 'search') {
    uiStore.activeTab = 'search';
  }
};

onMounted(() => {
  window.addEventListener('vrc-open-detail', handleOpenDetail);
});

onBeforeUnmount(() => {
  window.removeEventListener('vrc-open-detail', handleOpenDetail);
});

const selectAppMode = async (mode: 'pc' | 'vr') => {
  uiStore.modeSelectionError = '';
  if (mode === 'vr') {
    try {
      const isSteamVRRunning = await SysApi.checkSteamVR();
      if (!isSteamVRRunning) {
        uiStore.modeSelectionError = t('auto_1c5874ec');
        return;
      }
    } catch (e: any) {
      uiStore.modeSelectionError = e.message || t('auto_a712a9fb');
      return;
    }
    uiStore.activeTab = 'ovr'; 
  } else {
    uiStore.activeTab = 'dashboard'; 
  }
  uiStore.appMode = mode;
};
</script>

<template>
  <div class="mode-page">
    <main class="mode-stage">
      <header class="brand-head">
        <img class="brand-logo" :src="currentTheme.logo" alt="">
        <div class="brand-copy">
          <h1>{{ $t('app.select_mode_title') }}</h1>
          <p>{{ $t('app.select_mode_desc') }}</p>
        </div>
      </header>

      <div class="mode-grid">
        <button
          type="button"
          class="mode-card"
          :class="{ disabled: serverModePerms['pc'] === false }"
          :disabled="serverModePerms['pc'] === false"
          @click="selectAppMode('pc')"
        >
          <span class="mode-icon"><Monitor :size="22" /></span>
          <span class="mode-copy">
            <strong>PC Desktop</strong>
            <small v-if="serverModePerms['pc'] !== false">{{ $t('auto_3c519b1c') }}</small>
            <small v-else class="denied">{{ $t('auto_f31a212e') }}</small>
          </span>
          <ArrowRight :size="18" class="mode-arrow" />
        </button>

        <button
          type="button"
          class="mode-card"
          :class="{ disabled: serverModePerms['vr'] === false }"
          :disabled="serverModePerms['vr'] === false"
          @click="selectAppMode('vr')"
        >
          <span class="mode-icon"><Glasses :size="22" /></span>
          <span class="mode-copy">
            <strong>VR Overlay</strong>
            <small v-if="serverModePerms['vr'] !== false">{{ $t('auto_6138dd05') }}</small>
            <small v-else class="denied">{{ $t('auto_f31a212e') }}</small>
          </span>
          <ArrowRight :size="18" class="mode-arrow" />
        </button>
      </div>

      <p v-if="modeSelectionError" class="mode-error">
        <ShieldAlert :size="15" />
        <span>{{ modeSelectionError }}</span>
      </p>
    </main>
  </div>
</template>

<style scoped>
/* ══════════════════════════════════════════════════════════════
   模式选择 · 精炼暖色系（与 RoleSelectView / LoginView 同一套语言）
   ══════════════════════════════════════════════════════════════ */
.mode-page {
  --border: var(--theme-border-soft);
  --border-strong: var(--theme-border-strong);
  --text: var(--theme-text-strong);
  --text-base: var(--theme-text);
  --muted: var(--theme-text-muted);
  --accent: var(--theme-primary);
  --surface: var(--theme-surface);
  --surface-hover: var(--theme-surface-hover);

  position: relative;
  isolation: isolate;
  box-sizing: border-box;
  width: 100vw;
  min-height: 100dvh;
  display: grid;
  place-items: center;
  padding: 40px 24px;
  overflow: auto;
  color: var(--text-base);
  background-color: var(--theme-bg-main);
  background-image:
    radial-gradient(64% 60% at 6% -6%, color-mix(in srgb, var(--accent) 26%, transparent) 0%, transparent 72%),
    radial-gradient(56% 50% at 96% 2%, color-mix(in srgb, var(--accent) 14%, transparent) 0%, transparent 70%),
    radial-gradient(72% 64% at 92% 104%, color-mix(in srgb, var(--accent) 18%, transparent) 0%, transparent 74%),
    radial-gradient(54% 48% at 6% 98%, color-mix(in srgb, var(--accent) 11%, transparent) 0%, transparent 70%);
}

/* 背景：中心提亮，把视线收拢到内容区 */
.mode-page::before {
  content: '';
  position: fixed;
  inset: 0;
  z-index: -1;
  pointer-events: none;
  background: radial-gradient(80% 68% at 50% 48%,
    color-mix(in srgb, var(--theme-bg-main) 28%, transparent) 0%,
    transparent 76%);
}

.mode-stage {
  position: relative;
  z-index: 1;
  width: min(720px, 100%);
  display: grid;
  gap: 20px;
}

/* ── 品牌头部 ───────────────────────────────────────────────── */
.brand-head {
  display: flex;
  align-items: center;
  gap: 13px;
}

.brand-logo {
  width: 48px;
  height: 48px;
  flex: 0 0 48px;
  border-radius: 12px;
  object-fit: cover;
  border: 1px solid var(--border-strong);
  box-shadow: 0 8px 20px color-mix(in srgb, var(--text) 14%, transparent);
}

.brand-copy {
  min-width: 0;
  display: grid;
  gap: 3px;
}

.brand-copy h1 {
  margin: 0;
  color: var(--text);
  font-size: 22px;
  font-weight: 800;
  line-height: 1.2;
  letter-spacing: -0.01em;
}

.brand-copy p {
  margin: 0;
  color: var(--muted);
  font-size: 12.5px;
  font-weight: 600;
}

/* ── 模式卡片 ───────────────────────────────────────────────── */
.mode-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 12px;
}

.mode-card {
  box-sizing: border-box;
  min-height: 138px;
  display: grid;
  grid-template-columns: minmax(0, 1fr) 20px;
  grid-template-areas:
    'icon arrow'
    'copy arrow';
  align-content: center;
  align-items: center;
  gap: 11px 14px;
  padding: 20px;
  border: 1px solid var(--border);
  border-radius: 14px;
  color: var(--text-base);
  background:
    linear-gradient(150deg,
      color-mix(in srgb, var(--accent) 4%, transparent) 0%,
      transparent 46%),
    color-mix(in srgb, var(--surface) 88%, transparent);
  box-shadow:
    0 14px 36px color-mix(in srgb, var(--text) 7%, transparent),
    0 1px 2px color-mix(in srgb, var(--text) 4%, transparent),
    inset 0 1px rgba(255, 255, 255, 0.6);
  text-align: left;
  cursor: pointer;
  transition: border-color 180ms ease, transform 180ms ease, box-shadow 180ms ease;
}

.mode-card:hover:not(.disabled) {
  border-color: color-mix(in srgb, var(--accent) 50%, transparent);
  transform: translateY(-2px);
  box-shadow:
    0 18px 42px color-mix(in srgb, var(--accent) 15%, transparent),
    0 2px 6px color-mix(in srgb, var(--text) 6%, transparent),
    inset 0 1px rgba(255, 255, 255, 0.7);
}

.mode-card:active:not(.disabled) {
  transform: translateY(0);
}

.mode-card:focus-visible {
  outline: 3px solid color-mix(in srgb, var(--accent) 24%, transparent);
  outline-offset: 2px;
}

.mode-card.disabled {
  opacity: 0.55;
  cursor: not-allowed;
  filter: grayscale(0.5);
}

.mode-icon {
  grid-area: icon;
  width: 42px;
  height: 42px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border: 1px solid color-mix(in srgb, var(--accent) 24%, var(--border));
  border-radius: 11px;
  color: var(--accent);
  background: color-mix(in srgb, var(--accent) 9%, transparent);
}

.mode-copy {
  grid-area: copy;
  min-width: 0;
  display: grid;
  gap: 4px;
}

.mode-copy strong {
  color: var(--text);
  font-size: 15px;
  font-weight: 750;
  line-height: 1.25;
}

.mode-copy small {
  color: var(--muted);
  font-size: 12px;
  font-weight: 600;
  line-height: 1.5;
}

.mode-copy small.denied {
  color: #c62828;
}

.mode-arrow {
  grid-area: arrow;
  align-self: center;
  color: var(--muted);
  transition: color 160ms ease, transform 160ms ease;
}

.mode-card:hover:not(.disabled) .mode-arrow {
  color: var(--accent);
  transform: translateX(3px);
}

/* ── 错误提示 ───────────────────────────────────────────────── */
.mode-error {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  margin: 0;
  padding: 12px 14px;
  border: 1px solid color-mix(in srgb, #dc2626 28%, transparent);
  border-radius: 11px;
  color: #c62828;
  background: color-mix(in srgb, #dc2626 7%, transparent);
  font-size: 12.5px;
  font-weight: 700;
  line-height: 1.5;
}

.mode-error svg {
  flex: 0 0 auto;
  margin-top: 1px;
}

/* ── 响应式 ─────────────────────────────────────────────────── */
@media (max-width: 640px) {
  .mode-page {
    place-items: start center;
    padding: 34px 16px;
  }

  .mode-grid {
    grid-template-columns: 1fr;
  }

  .mode-card {
    min-height: 116px;
    padding: 17px;
  }
}

@media (prefers-reduced-motion: reduce) {
  *,
  *::before,
  *::after {
    transition-duration: 0.01ms !important;
    animation-duration: 0.01ms !important;
    animation-iteration-count: 1 !important;
  }
}
</style>
