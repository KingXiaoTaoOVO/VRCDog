<template>
  <div
    class="role-page"
    :style="{ background: themeConfig.colors.bgMain, color: themeConfig.colors.textStrong }"
  >
    <div class="top-actions">
      <button class="theme-button" type="button" @click="cycleLanguage">
        <Globe :size="16" />
        {{ currentLangLabel }}
      </button>
    </div>

    <main class="role-panel">
      <header class="brand-block">
        <img :src="themeConfig.logo" alt="VrcDog">
        <div>
          <h1>{{ themeConfig.appTitle }}</h1>
          <p>{{ t('role.select_mode') }}</p>
        </div>
      </header>

      <div v-if="!selectedRole" class="role-grid">
        <button type="button" class="role-choice" @click="selectRole('client')">
          <Monitor :size="28" />
          <strong>{{ t('role.client_mode') }}</strong>
          <span>{{ t('role.client_desc') }}</span>
        </button>
        <button type="button" class="role-choice" @click="selectRole('server')">
          <Server :size="28" />
          <strong>{{ t('role.server_mode') }}</strong>
          <span>{{ t('role.server_desc') }}</span>
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
    </main>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import {
  ArrowLeft,
  CircleAlert,
  Cloud,
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
import { getLocaleLabel, getNextLocale, setAppLocale } from '../i18n';
import { currentTheme as themeConfig } from '../theme';

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

const cycleLanguage = () => {
  const nextLang = setAppLocale(getNextLocale(locale.value), { notify: true });
  locale.value = nextLang;
  DbApi.saveSetting({ key: 'language', value: JSON.stringify(nextLang) }).catch(() => {});
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
</script>

<style scoped>
.role-page {
  --surface: v-bind('themeConfig.colors.surface');
  --surface-hover: v-bind('themeConfig.colors.surfaceHover');
  --border: v-bind('themeConfig.colors.borderSoft');
  --border-strong: v-bind('themeConfig.colors.borderStrong');
  --text: v-bind('themeConfig.colors.textStrong');
  --muted: v-bind('themeConfig.colors.textSoft');
  --primary: v-bind('themeConfig.colors.primaryBtnBg');
  --primary-hover: v-bind('themeConfig.colors.primaryBtnHover');
  width: 100vw;
  height: 100vh;
  display: grid;
  place-items: center;
  padding: 28px;
  overflow: auto;
}

.top-actions {
  position: fixed;
  top: 18px;
  right: 18px;
}

button,
input {
  font: inherit;
}

.theme-button,
.secondary-button {
  border: 1px solid var(--border);
  color: var(--text);
  background: var(--surface);
}

.theme-button {
  min-height: 38px;
  display: inline-flex;
  align-items: center;
  gap: 8px;
  border-radius: 8px;
  padding: 0 12px;
  cursor: pointer;
}

.role-panel {
  width: min(540px, 100%);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 24px;
  background: var(--surface);
  box-shadow: 0 24px 70px rgba(0, 0, 0, 0.12);
  backdrop-filter: blur(22px);
}

.brand-block {
  display: flex;
  align-items: center;
  gap: 14px;
  margin-bottom: 22px;
}

.brand-block img {
  width: 58px;
  height: 58px;
  border-radius: 8px;
  object-fit: cover;
  border: 1px solid var(--border-strong);
}

.brand-block h1,
.brand-block p {
  margin: 0;
}

.brand-block h1 {
  font-size: 24px;
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
  gap: 12px;
}

.role-choice {
  min-height: 150px;
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 18px;
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  justify-content: center;
  gap: 9px;
  color: var(--text);
  background: var(--surface-hover);
  text-align: left;
  cursor: pointer;
  transition: border-color 160ms ease, transform 160ms ease;
}

.role-choice:hover {
  border-color: var(--primary);
  transform: translateY(-2px);
}

.role-choice svg {
  color: var(--primary);
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
  background: var(--surface-hover);
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
  background: var(--surface-hover);
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
  background: color-mix(in srgb, var(--surface-hover) 72%, transparent);
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

  .role-grid {
    grid-template-columns: 1fr;
  }
}
</style>
