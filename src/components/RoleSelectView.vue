<template>
  <div
    class="role-page"
    :style="{ background: themeConfig.colors.bgMain, color: themeConfig.colors.textStrong }"
  >
    <nav ref="controlsRef" class="top-actions" aria-label="Display preferences" @keydown.esc="activeMenu = null">
      <div class="action-menu">
        <button
          class="control-button theme-control"
          type="button"
          :aria-expanded="activeMenu === 'theme'"
          aria-haspopup="listbox"
          @click="toggleMenu('theme')"
        >
          <img :src="themeConfig.logo" alt="">
          <span>{{ t(themeConfig.name) }}</span>
          <ChevronDown :size="15" :class="{ rotated: activeMenu === 'theme' }" aria-hidden="true" />
        </button>

        <Transition name="popover">
          <div v-if="activeMenu === 'theme'" class="control-popover theme-popover" role="listbox">
            <button
              v-for="themeItem in themeOptions"
              :key="themeItem.id"
              class="theme-option"
              :class="{ selected: currentThemeId === themeItem.id }"
              type="button"
              role="option"
              :aria-selected="currentThemeId === themeItem.id"
              @click="selectTheme(themeItem.id as ThemeId)"
            >
              <img :src="themeItem.logo" alt="">
              <span>
                <strong>{{ t(themeItem.name) }}</strong>
                <small>{{ themeItem.appTitle }}</small>
              </span>
              <Check v-if="currentThemeId === themeItem.id" :size="16" aria-hidden="true" />
            </button>
          </div>
        </Transition>
      </div>

      <div class="action-menu">
        <button
          class="control-button"
          type="button"
          :aria-expanded="activeMenu === 'language'"
          aria-haspopup="listbox"
          @click="toggleMenu('language')"
        >
          <Globe :size="17" />
          <span>{{ currentLangLabel }}</span>
          <ChevronDown :size="15" :class="{ rotated: activeMenu === 'language' }" aria-hidden="true" />
        </button>

        <Transition name="popover">
          <div v-if="activeMenu === 'language'" class="control-popover language-popover" role="listbox">
            <button
              v-for="option in localeOptions"
              :key="option.value"
              class="language-option"
              :class="{ selected: locale === option.value }"
              type="button"
              role="option"
              :aria-selected="locale === option.value"
              @click="selectLanguage(option.value)"
            >
              <span>{{ option.label }}</span>
              <Check v-if="locale === option.value" :size="15" aria-hidden="true" />
            </button>
          </div>
        </Transition>
      </div>
    </nav>

    <main class="role-panel">
      <section class="brand-stage">
        <div class="mascot-frame">
          <img :src="themeConfig.logo" :alt="themeConfig.appTitle">
        </div>
        <div class="brand-block">
          <h1>{{ themeConfig.appTitle }}</h1>
          <p>{{ t(themeConfig.name) }}</p>
        </div>
      </section>

      <section class="role-workspace">
        <header v-if="!selectedRole" class="workspace-header">
          <h2>{{ t('role.select_mode') }}</h2>
        </header>

        <div v-if="!selectedRole" class="role-grid">
        <button type="button" class="role-choice" @click="selectRole('client')">
          <span class="choice-copy">
            <strong>{{ t('role.client_mode') }}</strong>
            <small>{{ t('role.client_desc') }}</small>
          </span>
          <ArrowRight :size="19" class="choice-arrow" aria-hidden="true" />
        </button>
        <button type="button" class="role-choice" @click="selectRole('server')">
          <span class="choice-copy">
            <strong>{{ t('role.server_mode') }}</strong>
            <small>{{ t('role.server_desc') }}</small>
          </span>
          <ArrowRight :size="19" class="choice-arrow" aria-hidden="true" />
        </button>
      </div>

      <section v-else-if="selectedRole === 'client'" class="config-section">
        <div class="section-title">
          <Monitor :size="18" />
          <div>
            <strong>{{ t('role.client_mode') }}</strong>
            <span>{{ t('role.client_url_hint') }}</span>
          </div>
        </div>

        <label class="theme-field">
          <span>{{ t('role.server_address') }}</span>
          <div class="input-shell">
            <Link2 :size="16" />
            <input
              v-model="serverUrl"
              type="url"
              :placeholder="t('role.server_address_ph')"
              spellcheck="false"
              @keydown.enter="connectToServer"
            >
          </div>
          <small>{{ t('role.client_url_saved_desc') }}</small>
        </label>

        <p v-if="connectError" class="error-message">
          <CircleAlert :size="15" />
          {{ connectError }}
        </p>

        <div class="action-row">
          <button type="button" class="secondary-button" @click="selectedRole = null">
            <ArrowLeft :size="17" />
            {{ t('role.back') }}
          </button>
          <button
            type="button"
            class="primary-button"
            :disabled="isConnecting"
            @click="connectToServer"
          >
            <Loader2 v-if="isConnecting" :size="17" class="spin" />
            <Power v-else :size="17" />
            {{ t('role.connect_server') }}
          </button>
        </div>
      </section>

      <section v-else class="config-section">
        <div class="section-title">
          <Server :size="18" />
          <div>
            <strong>{{ t('role.server_mode') }}</strong>
            <span>{{ serverStage === 'password' ? t('role.server_desc') : t('role.server_target_desc') }}</span>
          </div>
        </div>

        <label v-if="serverStage === 'password'" class="theme-field">
          <span>{{ t('role.server_password') }}</span>
          <div class="input-shell">
            <LockKeyhole :size="16" />
            <input
              v-model="serverPassword"
              type="password"
              :placeholder="t('role.server_password_ph')"
              @keydown.enter="verifyServerPassword"
            >
          </div>
        </label>

        <template v-else>
          <div class="server-mode-switch" role="tablist" :aria-label="t('role.server_target')">
            <button
              v-if="isTauri()"
              type="button"
              role="tab"
              :aria-selected="serverTargetMode === 'local'"
              :class="{ active: serverTargetMode === 'local' }"
              @click="serverTargetMode = 'local'"
            >
              <Monitor :size="17" />
              {{ t('role.local_service') }}
            </button>
            <button
              type="button"
              role="tab"
              :aria-selected="serverTargetMode === 'remote'"
              :class="{ active: serverTargetMode === 'remote' }"
              @click="serverTargetMode = 'remote'"
            >
              <Cloud :size="17" />
              {{ t('role.remote_service') }}
            </button>
          </div>

          <div class="mode-description">
            <Monitor v-if="serverTargetMode === 'local'" :size="17" />
            <Cloud v-else :size="17" />
            <span>
              {{ serverTargetMode === 'local' ? t('role.local_admin_desc') : t('role.remote_admin_desc') }}
            </span>
          </div>

          <label v-if="serverTargetMode === 'remote'" class="theme-field">
            <span>{{ t('role.remote_server_address') }}</span>
            <div class="input-shell">
              <Link2 :size="16" />
              <input
                v-model="remoteAdminUrl"
                type="url"
                :placeholder="t('role.server_address_ph')"
                spellcheck="false"
                @keydown.enter="enterServerDashboard"
              >
            </div>
          </label>
        </template>

        <p v-if="startError" class="error-message">
          <CircleAlert :size="15" />
          {{ startError }}
        </p>

        <div class="action-row">
          <button type="button" class="secondary-button" @click="backFromServer">
            <ArrowLeft :size="17" />
            {{ t('role.back') }}
          </button>
          <button
            type="button"
            class="primary-button"
            :disabled="isStarting"
            @click="serverStage === 'password' ? verifyServerPassword() : enterServerDashboard()"
          >
            <Loader2 v-if="isStarting" :size="17" class="spin" />
            <Server v-else :size="17" />
            {{ serverStage === 'password' ? t('role.verify_password') : t('role.open_dashboard') }}
          </button>
        </div>
      </section>
      </section>
    </main>
  </div>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue';
import {
  ArrowRight,
  ArrowLeft,
  Check,
  CircleAlert,
  Cloud,
  ChevronDown,
  Globe,
  Link2,
  Loader2,
  LockKeyhole,
  Monitor,
  Power,
  Server,
} from 'lucide-vue-next';
import { useStorage } from '@vueuse/core';
import { useI18n } from 'vue-i18n';
import { SysApi, DbApi, VrcApi } from '../api';
import { isTauri } from '@tauri-apps/api/core';
import { getLocaleLabel, localeOptions, setAppLocale, type AppLocale } from '../i18n';
import {
  currentTheme as themeConfig,
  currentThemeId,
  setTheme,
  themes,
  type ThemeId,
} from '../theme';

const emit = defineEmits<{
  'role-selected': [payload: {
    role: 'client' | 'server';
    url?: string;
    serverMode?: 'local' | 'remote';
    password?: string;
  }];
}>();

const { t, locale } = useI18n();
const currentLangLabel = computed(() => getLocaleLabel(locale.value));
const controlsRef = ref<HTMLElement | null>(null);
const activeMenu = ref<'theme' | 'language' | null>(null);
const themeOptions = computed(() => Object.values(themes));
const selectedRole = ref<'client' | 'server' | null>(null);
const serverUrl = useStorage('vrc_server_url', 'http://127.0.0.1:11451');
const serverPassword = ref('');
const serverStage = ref<'password' | 'target'>('password');
const serverTargetMode = ref<'local' | 'remote'>('local');
const remoteAdminUrl = useStorage('vrc_server_admin_url', 'http://127.0.0.1:11451');
const isConnecting = ref(false);
const isStarting = ref(false);
const connectError = ref('');
const startError = ref('');

const toggleMenu = (menu: 'theme' | 'language') => {
  activeMenu.value = activeMenu.value === menu ? null : menu;
};

const selectTheme = (themeId: ThemeId) => {
  setTheme(themeId);
  activeMenu.value = null;
  DbApi.saveSetting({ key: 'theme', value: JSON.stringify(themeId) }).catch(() => {});
  window.dispatchEvent(new CustomEvent('settings-updated', { detail: { theme: themeId } }));
};

const selectLanguage = (next: AppLocale) => {
  const nextLang = setAppLocale(next, { notify: true });
  locale.value = nextLang;
  activeMenu.value = null;
  DbApi.saveSetting({ key: 'language', value: JSON.stringify(nextLang) }).catch(() => {});
};

const handleWindowPointerDown = (event: PointerEvent) => {
  if (!controlsRef.value?.contains(event.target as Node)) activeMenu.value = null;
};

const selectRole = (role: 'client' | 'server') => {
  selectedRole.value = role;
  serverStage.value = 'password';
  connectError.value = '';
  startError.value = '';
};

const normalizeServerUrl = (value: string) => {
  let normalized = value.trim();
  if (!/^https?:\/\//i.test(normalized)) normalized = `http://${normalized}`;
  normalized = normalized.replace('0.0.0.0', '127.0.0.1').replace(/\/+$/, '');
  return normalized;
};

const connectClient = async () => {
  if (!serverUrl.value.trim()) throw new Error(t('role.error_require_url'));
  const finalUrl = normalizeServerUrl(serverUrl.value);
  serverUrl.value = finalUrl;
  await SysApi.saveClientServerConfig({ serverUrl: finalUrl }).catch(() => {});
  await DbApi.saveSetting({ key: 'clientServerUrl', value: JSON.stringify(finalUrl) }).catch(() => {});
  await SysApi.pingServer({ url: finalUrl });
  emit('role-selected', { role: 'client', url: finalUrl });
};

const connectToServer = async () => {
  isConnecting.value = true;
  connectError.value = '';
  try {
    await connectClient();
  } catch (error: any) {
    connectError.value = `${t('role.connection_failed')}${error?.message || error}`;
  } finally {
    isConnecting.value = false;
  }
};

const verifyServerPassword = async () => {
  if (!serverPassword.value) {
    startError.value = t('role.error_require_pwd');
    return;
  }

  isStarting.value = true;
  startError.value = '';
  try {
    await SysApi.verifyServerPassword({ password: serverPassword.value });
    serverStage.value = 'target';
  } catch (error: any) {
    startError.value = error?.message || String(error);
  } finally {
    isStarting.value = false;
  }
};

const enterServerDashboard = async () => {
  isStarting.value = true;
  startError.value = '';
  try {
    let url: string | undefined;
    if (serverTargetMode.value === 'remote') {
      if (!remoteAdminUrl.value.trim()) {
        throw new Error(t('role.error_require_url'));
      }
      url = normalizeServerUrl(remoteAdminUrl.value);
      remoteAdminUrl.value = url;
      await SysApi.pingServer({ url });
      await VrcApi.request(`${url}/api/admin/auth`, {
        method: 'POST',
        params: { password: serverPassword.value },
      });
    }
    emit('role-selected', {
      role: 'server',
      serverMode: serverTargetMode.value,
      url,
      password: serverPassword.value,
    });
  } catch (error: any) {
    startError.value = `${t('role.connection_failed')}${error?.message || error}`;
  } finally {
    isStarting.value = false;
  }
};

const backFromServer = () => {
  startError.value = '';
  if (serverStage.value === 'target') {
    serverStage.value = 'password';
    return;
  }
  selectedRole.value = null;
};

onMounted(async () => {
  window.addEventListener('pointerdown', handleWindowPointerDown);
  try {
    const saved = await SysApi.getClientServerConfig();
    if (saved?.server_url) serverUrl.value = normalizeServerUrl(saved.server_url);
  } catch {
    try {
      const saved = await DbApi.getSetting({ key: 'clientServerUrl' });
      if (saved) serverUrl.value = normalizeServerUrl(JSON.parse(saved));
    } catch {
      // localStorage value from useStorage remains the fallback.
    }
  }
});

onBeforeUnmount(() => {
  window.removeEventListener('pointerdown', handleWindowPointerDown);
});
</script>

<style scoped>
@layer legacy {
.role-page {
  --surface: v-bind('themeConfig.colors.surface');
  --surface-hover: v-bind('themeConfig.colors.surfaceHover');
  --border: rgba(125, 79, 35, 0.14);
  --border-strong: rgba(112, 67, 28, 0.24);
  --text: v-bind('themeConfig.colors.textStrong');
  --muted: v-bind('themeConfig.colors.textSoft');
  --primary: v-bind('themeConfig.colors.primaryBtnBg');
  --primary-hover: v-bind('themeConfig.colors.primaryBtnHover');
  position: relative;
  isolation: isolate;
  width: 100vw;
  min-height: 100dvh;
  display: grid;
  place-items: center;
  box-sizing: border-box;
  padding: 68px 28px 32px;
  overflow: auto;
  background: #e9eff1 !important;
}

.role-page::after {
  content: '';
  position: fixed;
  inset: 0;
  z-index: -2;
  pointer-events: none;
  background: rgba(255, 255, 255, 0.18);
}

.top-actions {
  position: fixed;
  z-index: 20;
  top: 18px;
  right: 18px;
}

.mascot-watermark {
  position: fixed;
  left: max(28px, 7vw);
  bottom: -56px;
  z-index: -1;
  width: min(360px, 34vw);
  aspect-ratio: 1;
  border-radius: 50%;
  object-fit: cover;
  opacity: 0.075;
  filter: saturate(0.75);
  pointer-events: none;
}

button,
input,
select {
  font: inherit;
}

.language-select,
.secondary-button {
  border: 1px solid rgba(120, 75, 33, 0.14);
  color: var(--text);
  background: rgba(255, 252, 245, 0.7);
}

.language-select {
  position: relative;
  width: min(190px, calc(100vw - 32px));
  min-height: 42px;
  display: grid;
  grid-template-columns: auto minmax(0, 1fr) auto;
  align-items: center;
  gap: 8px;
  border-radius: 8px;
  padding: 0 12px;
  box-shadow: 0 10px 28px rgba(35, 49, 55, 0.1), inset 0 1px 0 rgba(255, 255, 255, 0.9);
  backdrop-filter: blur(20px) saturate(145%);
  -webkit-backdrop-filter: blur(20px) saturate(145%);
}

.language-select select {
  width: 100%;
  min-width: 0;
  height: 40px;
  border: 0;
  outline: 0;
  appearance: none;
  color: var(--text);
  background: transparent;
  font-size: 13px;
  font-weight: 700;
  text-overflow: ellipsis;
  cursor: pointer;
}

.language-select > svg { color: var(--primary); }
.language-select > svg:last-child { pointer-events: none; }

.language-select:focus-within {
  border-color: var(--primary);
  box-shadow: 0 0 0 4px color-mix(in srgb, var(--primary) 15%, transparent), 0 10px 28px rgba(90, 54, 22, 0.1);
}

.role-panel {
  position: relative;
  width: min(720px, 100%);
  box-sizing: border-box;
  overflow: hidden;
  border: 1px solid rgba(255, 255, 255, 0.82);
  border-radius: 8px;
  padding: 32px;
  background: rgba(255, 255, 255, 0.58);
  box-shadow: 0 28px 80px rgba(34, 48, 54, 0.15), inset 0 1px 0 rgba(255, 255, 255, 0.94);
  backdrop-filter: blur(30px) saturate(150%);
  -webkit-backdrop-filter: blur(30px) saturate(150%);
}

.panel-glint {
  position: absolute;
  inset: 0 auto auto 12%;
  width: 68%;
  height: 1px;
  background: linear-gradient(90deg, transparent, rgba(255, 255, 255, 0.98), transparent);
  pointer-events: none;
}

.brand-block {
  display: flex;
  align-items: center;
  gap: 16px;
  margin-bottom: 26px;
}

.brand-block img {
  width: 68px;
  height: 68px;
  border-radius: 8px;
  object-fit: cover;
  border: 2px solid rgba(255, 255, 255, 0.82);
  box-shadow: 0 12px 26px rgba(93, 56, 23, 0.17);
}

.brand-block h1,
.brand-block p {
  margin: 0;
}

.brand-block h1 {
  font-size: 28px;
  letter-spacing: 0;
}

.brand-block p,
.role-choice span,
.section-title span,
.theme-field small {
  color: var(--muted);
}

.role-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 16px;
}

.role-choice {
  position: relative;
  min-height: 174px;
  overflow: hidden;
  border: 1px solid rgba(123, 77, 34, 0.12);
  border-radius: 8px;
  padding: 22px;
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  justify-content: center;
  gap: 10px;
  color: var(--text);
  background: rgba(255, 255, 255, 0.48);
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.8);
  text-align: left;
  cursor: pointer;
  transition: border-color 160ms ease, transform 160ms ease, background 160ms ease, box-shadow 160ms ease;
}

.role-choice:first-child {
  border-top: 3px solid #0f766e;
}

.role-choice:last-child {
  border-top: 3px solid #c2410c;
}

.role-choice:hover {
  border-color: color-mix(in srgb, var(--primary) 54%, rgba(255, 255, 255, 0.5));
  transform: translateY(-3px);
  background: rgba(255, 255, 255, 0.62);
  box-shadow: 0 15px 32px rgba(87, 51, 20, 0.11), inset 0 1px 0 rgba(255, 255, 255, 0.94);
}

.role-choice:focus-visible {
  outline: 4px solid color-mix(in srgb, var(--primary) 18%, transparent);
  outline-offset: 2px;
}

.config-section {
  display: grid;
  gap: 16px;
}

.section-title {
  display: flex;
  align-items: center;
  gap: 10px;
}

.section-title > svg {
  color: var(--primary);
}

.section-title div {
  display: grid;
  gap: 2px;
}

.section-title span {
  font-size: 12px;
}

.theme-field {
  display: grid;
  gap: 7px;
  font-size: 13px;
  font-weight: 700;
}

.theme-field small {
  font-size: 11px;
  font-weight: 500;
}

.input-shell {
  min-height: 44px;
  display: flex;
  align-items: center;
  gap: 9px;
  padding: 0 12px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: rgba(255, 255, 255, 0.38);
}

.input-shell:focus-within {
  border-color: var(--primary);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--primary) 16%, transparent);
}

.input-shell svg {
  flex-shrink: 0;
  color: var(--muted);
}

.input-shell input {
  width: 100%;
  min-width: 0;
  border: 0;
  outline: 0;
  color: var(--text);
  background: transparent;
}

.server-mode-switch {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 6px;
  padding: 5px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: rgba(255, 255, 255, 0.38);
}

.server-mode-switch button {
  min-height: 40px;
  border: 0;
  border-radius: 6px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  color: var(--muted);
  background: transparent;
  font-weight: 800;
  cursor: pointer;
}

.server-mode-switch button.active {
  color: white;
  background: var(--primary);
  box-shadow: 0 5px 16px color-mix(in srgb, var(--primary) 24%, transparent);
}

.mode-description {
  min-height: 46px;
  display: flex;
  align-items: center;
  gap: 9px;
  padding: 10px 12px;
  border: 1px solid var(--border);
  border-radius: 8px;
  color: var(--muted);
  background: rgba(255, 255, 255, 0.3);
  font-size: 12px;
  line-height: 1.5;
}

.mode-description svg {
  flex-shrink: 0;
  color: var(--primary);
}

.error-message {
  margin: 0;
  display: flex;
  align-items: center;
  gap: 7px;
  color: #dc2626;
  font-size: 12px;
  font-weight: 700;
}

.action-row {
  display: grid;
  grid-template-columns: minmax(110px, 0.7fr) minmax(0, 1.3fr);
  gap: 10px;
}

.secondary-button,
.primary-button {
  min-height: 42px;
  border-radius: 8px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  font-weight: 800;
  cursor: pointer;
}

.secondary-button:hover {
  background: var(--surface-hover);
}

.primary-button {
  border: 0;
  color: white;
  background: var(--primary);
}

.primary-button:hover:not(:disabled) {
  background: var(--primary-hover);
}

.primary-button:disabled {
  cursor: wait;
  opacity: 0.65;
}

.spin {
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

@media (max-width: 560px) {
  .role-page {
    place-items: start center;
    padding: 72px 14px 20px;
  }

  .role-panel {
    padding: 18px;
  }

  .brand-block img {
    width: 56px;
    height: 56px;
  }

  .brand-block h1 { font-size: 23px; }

  .role-grid {
    grid-template-columns: 1fr;
  }

  .role-choice { min-height: 132px; }

  .mascot-watermark { width: 220px; }
}

@media (max-height: 620px) and (min-width: 561px) {
  .role-page { place-items: start center; }
  .role-panel { padding: 22px; }
  .brand-block { margin-bottom: 18px; }
  .role-choice { min-height: 142px; }
}
}
</style>

<style scoped>
.role-page {
  --page-bg: v-bind('themeConfig.colors.bgMain');
  --surface: v-bind('themeConfig.colors.surface');
  --surface-hover: v-bind('themeConfig.colors.surfaceHover');
  --border: v-bind('themeConfig.colors.borderSoft');
  --border-strong: v-bind('themeConfig.colors.borderStrong');
  --text: v-bind('themeConfig.colors.textStrong');
  --text-base: v-bind('themeConfig.colors.text');
  --muted: v-bind('themeConfig.colors.textMuted');
  --accent: v-bind('themeConfig.colors.primaryBtnBg');
  --accent-hover: v-bind('themeConfig.colors.primaryBtnHover');
  --active-bg: v-bind('themeConfig.colors.activeBg');
  --glass: v-bind('themeConfig.colors.glassEffect');
  position: relative;
  isolation: isolate;
  width: 100vw;
  min-height: 100dvh;
  box-sizing: border-box;
  display: grid;
  place-items: center;
  padding: 82px 28px 38px;
  overflow: auto;
  color: var(--text);
  background-color: var(--page-bg) !important;
  background-image:
    linear-gradient(var(--border) 1px, transparent 1px),
    linear-gradient(90deg, var(--border) 1px, transparent 1px) !important;
  background-size: 44px 44px !important;
}

.role-page::before {
  content: '';
  position: fixed;
  inset: 0;
  z-index: -1;
  pointer-events: none;
  background: color-mix(in srgb, var(--page-bg) 86%, transparent);
  mask-image: linear-gradient(to bottom, transparent, black 38%, black 72%, transparent);
}

button,
input {
  font: inherit;
}

button {
  -webkit-tap-highlight-color: transparent;
}

.top-actions {
  position: fixed;
  z-index: 50;
  top: 18px;
  right: 20px;
  display: flex;
  align-items: center;
  gap: 8px;
}

.action-menu {
  position: relative;
}

.control-button {
  min-height: 42px;
  max-width: 210px;
  border: 1px solid var(--border-strong);
  border-radius: 8px;
  padding: 0 12px;
  display: inline-flex;
  align-items: center;
  gap: 8px;
  color: var(--text-base);
  background: color-mix(in srgb, var(--surface-hover) 88%, transparent);
  box-shadow: 0 8px 24px color-mix(in srgb, var(--text) 10%, transparent), inset 0 1px rgba(255, 255, 255, 0.72);
  backdrop-filter: var(--glass);
  -webkit-backdrop-filter: var(--glass);
  cursor: pointer;
  transition: border-color 150ms ease, background-color 150ms ease, transform 150ms ease;
}

.control-button:hover,
.control-button[aria-expanded='true'] {
  border-color: color-mix(in srgb, var(--accent) 45%, var(--border));
  background: var(--surface-hover);
}

.control-button:active {
  transform: scale(0.98);
}

.control-button > img {
  width: 24px;
  height: 24px;
  flex: 0 0 24px;
  border-radius: 6px;
  object-fit: cover;
}

.control-button > span {
  min-width: 0;
  overflow: hidden;
  font-size: 13px;
  font-weight: 700;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.control-button > svg:first-child {
  flex: 0 0 auto;
  color: var(--accent);
}

.control-button > svg:last-child {
  flex: 0 0 auto;
  color: var(--muted);
  transition: transform 150ms ease;
}

.control-button > svg.rotated {
  transform: rotate(180deg);
}

.control-button:focus-visible,
.theme-option:focus-visible,
.language-option:focus-visible,
.role-choice:focus-visible,
.secondary-button:focus-visible,
.primary-button:focus-visible,
.server-mode-switch button:focus-visible {
  outline: 3px solid color-mix(in srgb, var(--accent) 22%, transparent);
  outline-offset: 2px;
}

.control-popover {
  position: absolute;
  z-index: 70;
  top: calc(100% + 9px);
  right: 0;
  box-sizing: border-box;
  border: 1px solid var(--border-strong);
  border-radius: 8px;
  padding: 8px;
  color: var(--text-base);
  background: color-mix(in srgb, var(--surface-hover) 92%, var(--page-bg));
  box-shadow: 0 18px 50px color-mix(in srgb, var(--text) 17%, transparent), inset 0 1px rgba(255, 255, 255, 0.72);
  backdrop-filter: var(--glass);
  -webkit-backdrop-filter: var(--glass);
}

.theme-popover {
  width: 330px;
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 5px;
}

.theme-option {
  min-width: 0;
  min-height: 58px;
  border: 1px solid transparent;
  border-radius: 6px;
  padding: 8px;
  display: grid;
  grid-template-columns: 36px minmax(0, 1fr) 16px;
  align-items: center;
  gap: 9px;
  color: var(--text-base);
  background: transparent;
  text-align: left;
  cursor: pointer;
}

.theme-option:hover {
  border-color: var(--border);
  background: var(--surface-hover);
}

.theme-option.selected {
  border-color: color-mix(in srgb, var(--accent) 36%, var(--border));
  background: var(--active-bg);
}

.theme-option img {
  width: 36px;
  height: 36px;
  border-radius: 7px;
  object-fit: cover;
  box-shadow: 0 3px 10px color-mix(in srgb, var(--text) 12%, transparent);
}

.theme-option > span {
  min-width: 0;
  display: grid;
  gap: 2px;
}

.theme-option strong,
.theme-option small {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.theme-option strong {
  font-size: 12px;
}

.theme-option small {
  color: var(--muted);
  font-size: 10px;
}

.theme-option > svg,
.language-option > svg {
  color: var(--accent);
}

.language-popover {
  width: 326px;
  max-height: min(330px, calc(100vh - 82px));
  overflow-y: auto;
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 4px;
  scrollbar-width: thin;
  scrollbar-color: var(--border-strong) transparent;
}

.language-option {
  min-width: 0;
  min-height: 38px;
  border: 1px solid transparent;
  border-radius: 6px;
  padding: 0 10px;
  display: grid;
  grid-template-columns: minmax(0, 1fr) 16px;
  align-items: center;
  gap: 6px;
  color: var(--text-base);
  background: transparent;
  font-size: 12px;
  font-weight: 650;
  text-align: left;
  cursor: pointer;
}

.language-option span {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.language-option:hover {
  border-color: var(--border);
  background: var(--surface-hover);
}

.language-option.selected {
  border-color: color-mix(in srgb, var(--accent) 36%, var(--border));
  background: var(--active-bg);
}

.popover-enter-active,
.popover-leave-active {
  transition: opacity 140ms ease, transform 140ms ease;
  transform-origin: top right;
}

.popover-enter-from,
.popover-leave-to {
  opacity: 0;
  transform: translateY(-5px) scale(0.98);
}

.role-panel {
  position: relative;
  width: min(980px, calc(100vw - 56px));
  min-height: 500px;
  box-sizing: border-box;
  overflow: hidden;
  display: grid;
  grid-template-columns: minmax(300px, 0.84fr) minmax(430px, 1.16fr);
  border: 1px solid var(--border-strong);
  border-radius: 16px;
  padding: 0;
  background:
    linear-gradient(135deg,
      color-mix(in srgb, var(--accent) 4%, transparent) 0%,
      transparent 38%,
      color-mix(in srgb, var(--accent) 3%, transparent) 100%),
    color-mix(in srgb, var(--surface) 88%, transparent);
  box-shadow:
    0 32px 90px color-mix(in srgb, var(--text) 16%, transparent),
    0 4px 12px color-mix(in srgb, var(--text) 6%, transparent),
    inset 0 1px rgba(255, 255, 255, 0.72),
    inset 0 0 0 1px color-mix(in srgb, var(--accent) 6%, transparent);
  backdrop-filter: var(--glass);
  -webkit-backdrop-filter: var(--glass);
}

.brand-stage {
  position: relative;
  min-width: 0;
  overflow: hidden;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 27px;
  padding: 48px 38px;
  border-right: 1px solid var(--border);
  background:
    radial-gradient(120% 90% at 50% 0%,
      color-mix(in srgb, var(--accent) 12%, transparent) 0%,
      transparent 60%),
    color-mix(in srgb, var(--active-bg) 70%, var(--surface));
}

.brand-stage::before {
  content: '';
  position: absolute;
  pointer-events: none;
  width: 184px;
  height: 184px;
  top: calc(50% - 120px);
  left: calc(50% - 120px);
  border-radius: 24px;
  background: var(--accent);
  opacity: 0.08;
  filter: blur(36px);
  transform: rotate(-8deg);
}

.mascot-frame {
  position: relative;
  z-index: 1;
  width: 158px;
  aspect-ratio: 1;
  box-sizing: border-box;
  border: 7px solid color-mix(in srgb, var(--surface-hover) 82%, white);
  border-radius: 8px;
  overflow: hidden;
  background: var(--surface-hover);
  box-shadow: 0 20px 38px color-mix(in srgb, var(--text) 18%, transparent);
  transform: rotate(2deg);
}

.mascot-frame img {
  width: 100%;
  height: 100%;
  display: block;
  object-fit: cover;
}

.brand-block {
  position: relative;
  z-index: 1;
  display: grid;
  justify-items: center;
  gap: 5px;
  margin: 0;
  text-align: center;
}

.brand-block h1,
.brand-block p {
  margin: 0;
}

.brand-block h1 {
  color: var(--text);
  font-size: 34px;
  line-height: 1.1;
  letter-spacing: 0;
}

.brand-block p {
  color: var(--muted);
  font-size: 12px;
  font-weight: 750;
}

.role-workspace {
  min-width: 0;
  display: flex;
  flex-direction: column;
  justify-content: center;
  padding: 46px 44px;
}

.workspace-header {
  margin-bottom: 24px;
}

.workspace-header h2 {
  margin: 0;
  color: var(--text);
  font-size: 22px;
  line-height: 1.3;
  letter-spacing: 0;
}

.role-grid {
  display: grid;
  grid-template-columns: 1fr;
  gap: 12px;
}

.role-choice {
  position: relative;
  min-height: 102px;
  border: 1px solid var(--border);
  border-radius: 14px;
  padding: 18px 20px;
  display: grid;
  grid-template-columns: minmax(0, 1fr) 22px;
  align-items: center;
  gap: 15px;
  color: var(--text-base);
  background:
    linear-gradient(135deg,
      color-mix(in srgb, var(--surface-hover) 70%, transparent) 0%,
      color-mix(in srgb, var(--surface) 55%, transparent) 100%);
  box-shadow:
    0 1px 2px color-mix(in srgb, var(--text) 4%, transparent),
    inset 0 1px rgba(255, 255, 255, 0.55);
  text-align: left;
  cursor: pointer;
  isolation: isolate;
  transition:
    transform 240ms cubic-bezier(0.34, 1.56, 0.64, 1),
    border-color 220ms ease,
    background-color 220ms ease,
    box-shadow 240ms ease;
}

.role-choice:hover {
  border-color: color-mix(in srgb, var(--accent) 55%, transparent);
  background:
    linear-gradient(135deg,
      color-mix(in srgb, var(--accent) 8%, var(--surface-hover)) 0%,
      color-mix(in srgb, var(--accent) 3%, var(--surface)) 100%);
  box-shadow:
    0 0 0 1px color-mix(in srgb, var(--accent) 30%, transparent),
    0 0 18px 2px color-mix(in srgb, var(--accent) 22%, transparent),
    0 14px 30px color-mix(in srgb, var(--accent) 16%, transparent),
    0 2px 6px color-mix(in srgb, var(--text) 6%, transparent),
    inset 0 1px rgba(255, 255, 255, 0.62);
  transform: translateY(-2px);
}

.role-choice:focus-visible {
  outline: none;
  border-color: var(--accent);
  background:
    linear-gradient(135deg,
      color-mix(in srgb, var(--accent) 10%, var(--surface-hover)) 0%,
      color-mix(in srgb, var(--accent) 4%, var(--surface)) 100%);
  box-shadow:
    0 0 0 2px color-mix(in srgb, var(--accent) 40%, transparent),
    0 0 28px 5px color-mix(in srgb, var(--accent) 42%, transparent),
    0 16px 36px color-mix(in srgb, var(--accent) 24%, transparent),
    inset 0 1px rgba(255, 255, 255, 0.66);
  transform: translateY(-2px);
  animation: role-choice-glow 1.6s ease-in-out infinite;
}

@keyframes role-choice-glow {
  0%, 100% {
    box-shadow:
      0 0 0 2px color-mix(in srgb, var(--accent) 40%, transparent),
      0 0 28px 5px color-mix(in srgb, var(--accent) 42%, transparent),
      0 16px 36px color-mix(in srgb, var(--accent) 24%, transparent),
      inset 0 1px rgba(255, 255, 255, 0.66);
  }
  50% {
    box-shadow:
      0 0 0 2px color-mix(in srgb, var(--accent) 55%, transparent),
      0 0 36px 7px color-mix(in srgb, var(--accent) 55%, transparent),
      0 16px 36px color-mix(in srgb, var(--accent) 26%, transparent),
      inset 0 1px rgba(255, 255, 255, 0.7);
  }
}

.role-choice:active {
  transform: translateY(0) scale(0.99);
}

.choice-copy {
  min-width: 0;
  display: grid;
  gap: 6px;
}

.choice-copy strong {
  color: var(--text);
  font-size: 15px;
  line-height: 1.2;
}

.choice-copy small {
  overflow: hidden;
  color: var(--muted);
  font-size: 12px;
  line-height: 1.45;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.choice-arrow {
  color: var(--muted);
  transition: color 160ms ease, transform 160ms ease;
}

.role-choice:hover .choice-arrow {
  color: var(--accent);
  transform: translateX(3px);
}

.config-section {
  width: 100%;
  display: grid;
  gap: 17px;
}

.section-title {
  display: flex;
  align-items: center;
  gap: 11px;
  padding-bottom: 15px;
  border-bottom: 1px solid var(--border);
}

.section-title > svg {
  color: var(--accent);
}

.section-title div {
  min-width: 0;
  display: grid;
  gap: 3px;
}

.section-title strong {
  color: var(--text);
  font-size: 15px;
}

.section-title span,
.theme-field small {
  color: var(--muted);
  font-size: 11px;
  line-height: 1.45;
}

.theme-field {
  display: grid;
  gap: 7px;
  color: var(--text-base);
  font-size: 12px;
  font-weight: 700;
}

.input-shell {
  min-height: 44px;
  box-sizing: border-box;
  display: flex;
  align-items: center;
  gap: 9px;
  padding: 0 12px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: color-mix(in srgb, var(--surface-hover) 68%, transparent);
}

.input-shell:focus-within {
  border-color: var(--accent);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent) 14%, transparent);
}

.input-shell svg {
  flex: 0 0 auto;
  color: var(--muted);
}

.input-shell input {
  width: 100%;
  min-width: 0;
  border: 0;
  outline: 0;
  color: var(--text);
  background: transparent;
  font-size: 13px;
}

.input-shell input::placeholder {
  color: var(--muted);
}

.server-mode-switch {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 5px;
  padding: 5px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: color-mix(in srgb, var(--surface-hover) 52%, transparent);
}

.server-mode-switch button {
  min-height: 40px;
  border: 0;
  border-radius: 6px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  color: var(--muted);
  background: transparent;
  font-size: 12px;
  font-weight: 750;
  cursor: pointer;
}

.server-mode-switch button.active {
  color: white;
  background: var(--accent);
  box-shadow: 0 5px 14px color-mix(in srgb, var(--accent) 24%, transparent);
}

.mode-description {
  min-height: 46px;
  box-sizing: border-box;
  display: flex;
  align-items: center;
  gap: 9px;
  padding: 10px 12px;
  border: 1px solid var(--border);
  border-radius: 8px;
  color: var(--muted);
  background: color-mix(in srgb, var(--active-bg) 54%, transparent);
  font-size: 11px;
  line-height: 1.5;
}

.mode-description svg {
  flex: 0 0 auto;
  color: var(--accent);
}

.error-message {
  margin: 0;
  display: flex;
  align-items: center;
  gap: 7px;
  color: #c62828;
  font-size: 12px;
  font-weight: 700;
}

.action-row {
  display: grid;
  grid-template-columns: minmax(104px, 0.72fr) minmax(0, 1.28fr);
  gap: 9px;
  padding-top: 2px;
}

.secondary-button,
.primary-button {
  min-height: 43px;
  border-radius: 8px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  font-size: 12px;
  font-weight: 750;
  cursor: pointer;
  transition: background-color 150ms ease, transform 150ms ease, box-shadow 150ms ease;
}

.secondary-button {
  border: 1px solid var(--border);
  color: var(--text-base);
  background: color-mix(in srgb, var(--surface-hover) 58%, transparent);
}

.secondary-button:hover {
  background: var(--surface-hover);
}

.primary-button {
  border: 1px solid transparent;
  color: white;
  background: var(--accent);
  box-shadow: 0 8px 18px color-mix(in srgb, var(--accent) 22%, transparent);
}

.primary-button:hover:not(:disabled) {
  background: var(--accent-hover);
  transform: translateY(-1px);
}

.secondary-button:active,
.primary-button:active:not(:disabled) {
  transform: scale(0.985);
}

.primary-button:disabled {
  cursor: wait;
  opacity: 0.62;
}

.spin {
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

@media (max-width: 760px) {
  .role-page {
    place-items: start center;
    padding: 82px 20px 28px;
  }

  .role-panel {
    width: min(620px, 100%);
    min-height: 0;
    grid-template-columns: 1fr;
  }

  .brand-stage {
    min-height: 132px;
    box-sizing: border-box;
    flex-direction: row;
    justify-content: flex-start;
    gap: 22px;
    padding: 24px 30px;
    border-right: 0;
    border-bottom: 1px solid var(--border);
  }

  .brand-stage::before {
    width: 88px;
    height: 88px;
    top: 20px;
    left: 20px;
  }

  .mascot-frame {
    width: 82px;
    flex: 0 0 82px;
    border-width: 4px;
  }

  .brand-block {
    justify-items: start;
    text-align: left;
  }

  .brand-block h1 {
    font-size: 28px;
  }

  .role-workspace {
    padding: 30px;
  }
}

@media (max-width: 520px) {
  .role-page {
    padding: 78px 12px 18px;
    background-size: 34px 34px !important;
  }

  .top-actions {
    top: 12px;
    right: 12px;
    left: 12px;
    justify-content: flex-end;
  }

  .control-button {
    max-width: calc(50vw - 18px);
    min-height: 40px;
    padding: 0 9px;
  }

  .theme-popover,
  .language-popover {
    position: fixed;
    top: 61px;
    right: 12px;
    width: min(330px, calc(100vw - 24px));
  }

  .role-panel {
    width: 100%;
  }

  .brand-stage {
    min-height: 110px;
    gap: 17px;
    padding: 19px 20px;
  }

  .mascot-frame {
    width: 68px;
    flex-basis: 68px;
  }

  .brand-block h1 {
    font-size: 24px;
  }

  .role-workspace {
    padding: 24px 18px;
  }

  .workspace-header {
    margin-bottom: 18px;
  }

  .workspace-header h2 {
    font-size: 19px;
  }

  .role-choice {
    min-height: 92px;
    grid-template-columns: minmax(0, 1fr) 18px;
    gap: 11px;
    padding: 14px 13px;
  }

  .choice-copy small {
    white-space: normal;
  }

  .action-row {
    grid-template-columns: 1fr;
  }
}

@media (max-height: 650px) and (min-width: 761px) {
  .role-page {
    place-items: start center;
    padding-top: 72px;
  }

  .role-panel {
    min-height: 440px;
  }

  .mascot-frame {
    width: 136px;
  }

  .role-workspace {
    padding-block: 34px;
  }
}

@media (prefers-reduced-motion: reduce) {
  *,
  *::before,
  *::after {
    scroll-behavior: auto !important;
    transition-duration: 0.01ms !important;
    animation-duration: 0.01ms !important;
    animation-iteration-count: 1 !important;
  }
}

@media (prefers-reduced-transparency: reduce) {
  .role-panel,
  .control-button,
  .control-popover {
    background: var(--page-bg);
    backdrop-filter: none;
    -webkit-backdrop-filter: none;
  }
}
</style>
