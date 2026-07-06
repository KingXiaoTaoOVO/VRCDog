<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue';
import { VrcApi, DbApi, SysApi, GamelogApi } from "../api";
import { Bone, Key, User, Loader2, Globe, ArrowLeft, Trash2, Settings, ArrowDownToLine, Languages, Check, X, ChevronDown, AlertCircle } from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';
import { getLocaleLabel, localeOptions, normalizeLocale, setAppLocale } from '../i18n';
import { getVersion } from '@tauri-apps/api/app';
import { currentTheme } from '../theme';
import { mergeCookiesAndSave, normalizeAuthCookieJson, parseCookieInput } from '../api/cookies';

const { t, locale } = useI18n({ useScope: 'global' });
const appVersion = ref('');

// ========== 已保存账号管理 ==========
interface SavedAccount {
  userId: string;
  displayName: string;
  username: string;
  avatarUrl: string;
  authCookie: string;
}

const savedAccounts = ref<SavedAccount[]>([]);
const saveCredentials = ref(false);

async function loadSavedAccounts() {
  try {
    const raw = await DbApi.getSetting({ key: 'savedAccounts' });
    if (raw) {
      savedAccounts.value = JSON.parse(raw);
    }
  } catch { /* ignore */ }
}

async function persistSavedAccounts() {
  await DbApi.saveSetting({ key: 'savedAccounts', value: JSON.stringify(savedAccounts.value) });
}

function hasUsableAuthCookie(rawCookie: string | null | undefined): boolean {
  return parseCookieInput(rawCookie).some(cookie => /^auth=.+/i.test(cookie));
}

function prepareManualLoginFromSavedAccount(account: SavedAccount, messageKey: string) {
  username.value = account.username || account.displayName || '';
  password.value = '';
  authCookie.value = '';
  saveCredentials.value = true;
  errorMsg.value = t(messageKey);
}

async function updateSavedAccountCookie(userId: string, cookie: string) {
  const idx = savedAccounts.value.findIndex(a => a.userId === userId);
  if (idx < 0) return;
  savedAccounts.value[idx].authCookie = cookie;
  await persistSavedAccounts();
}

async function saveCurrentAccount(user: any, cookie: string) {
  if (!saveCredentials.value) return;
  if (!user?.id) return;

  const existing = savedAccounts.value.findIndex(a => a.userId === user.id);
  const account: SavedAccount = {
    userId: user.id,
    displayName: user.displayName || user.display_name || user.username || 'Unknown',
    username: user.username || user.id,
    avatarUrl: user.currentAvatarThumbnailImageUrl || user.currentAvatarImageUrl || '',
    authCookie: cookie
  };

  if (existing >= 0) {
    savedAccounts.value[existing] = account;
  } else {
    savedAccounts.value.push(account);
  }
  await persistSavedAccounts();
}

async function deleteSavedAccount(userId: string) {
  savedAccounts.value = savedAccounts.value.filter(a => a.userId !== userId);
  await persistSavedAccounts();
}

async function loginWithSavedAccount(account: SavedAccount) {
  const savedCookie = normalizeAuthCookieJson(account.authCookie);
  if (!hasUsableAuthCookie(savedCookie)) {
    prepareManualLoginFromSavedAccount(account, 'login.saved_cookie_missing');
    return;
  }

  loading.value = true;
  errorMsg.value = '';
  try {
    // 先把该账号保存的 cookie 注入本地 auth 存储和后端 cookie jar。
    // 不清空、不调用 /config，避免覆盖掉可用的保存会话。
    await DbApi.saveAuth({ cookie: savedCookie });
    try { await VrcApi.applyAuthCookie({ authCookie: savedCookie }); } catch {}

    const res: any = await VrcApi.login({
      username: null,
      password: null,
      authCookie: savedCookie
    });

    if (res.error) {
      const msg = res.error.message || JSON.stringify(res.error);
      if (/missing credentials|401|unauthorized|expired|invalid/i.test(msg)) {
        prepareManualLoginFromSavedAccount(account, 'login.saved_cookie_expired');
      } else {
        errorMsg.value = msg;
      }
    } else if (res.requiresTwoFactorAuth || res.requires_two_factor_auth) {
      show2FA.value = true;
      twoFactorMethods.value = Array.isArray(res.requiresTwoFactorAuth) ? res.requiresTwoFactorAuth : Array.isArray(res.requires_two_factor_auth) ? res.requires_two_factor_auth : ['totp'];
      if (res.auth_cookie) {
        authCookie.value = res.auth_cookie;
        await mergeCookiesAndSave(res.auth_cookie);
      }
    } else if (res.id || res.currentUser || res.current_user) {
      if (res.auth_cookie) {
        authCookie.value = res.auth_cookie;
        await mergeCookiesAndSave(res.auth_cookie);
      }
      const user = res.currentUser || res.current_user || res;
      // 更新保存的账号信息
      const idx = savedAccounts.value.findIndex(a => a.userId === account.userId);
      if (idx >= 0) {
        savedAccounts.value[idx].displayName = user.displayName || user.display_name || account.displayName;
        savedAccounts.value[idx].username = user.username || account.username;
        savedAccounts.value[idx].avatarUrl = user.currentAvatarThumbnailImageUrl || user.currentAvatarImageUrl || account.avatarUrl;
        savedAccounts.value[idx].authCookie = normalizeAuthCookieJson(res.auth_cookie || savedCookie);
        await persistSavedAccounts();
      }
      emit('login-success', user);
    } else {
      errorMsg.value = `Unhandled login response: ${JSON.stringify(res)}`;
    }
  } catch (err: any) {
    const msg = err.message || JSON.stringify(err);
    if (msg.includes("hold your horses") || msg.toLowerCase().includes("twofactor")) {
      show2FA.value = true;
      twoFactorMethods.value = ["emailOtp"];
    } else if (/missing credentials|401|unauthorized|expired|invalid/i.test(msg)) {
      prepareManualLoginFromSavedAccount(account, 'login.saved_cookie_expired');
    } else {
      errorMsg.value = msg;
    }
  } finally {
    loading.value = false;
  }
}

// ========== 语言切换（VRCX 对齐：14 种）==========
const languages = localeOptions.map(({ label, value }) => ({ label, code: value }));

const currentLangLabel = computed(() => {
  return getLocaleLabel(locale.value);
});

const showLangMenu = ref(false);

function selectLanguage(code: string) {
  showLangMenu.value = false;
  const lang = languages.find(l => l.code === code);
  if (!lang) return;

  // Persist and broadcast the normalized app locale.
  const effectiveLocale = setAppLocale(code, { notify: true });
  locale.value = effectiveLocale;

  DbApi.saveSetting({ key: 'language', value: JSON.stringify(effectiveLocale) }).catch(() => {});
}

// 用户首选语言（用于在菜单里高亮，即使没实现也显示对勾）
const preferredLangCode = computed(() => {
  return normalizeLocale(localStorage.getItem('vrcdog-locale-pref') || locale.value);
});

// ========== 设置弹窗（代理 + 自定义 API） ==========
const showSettingsDialog = ref(false);
const settingsProxy = ref('');
const settingsCustomApiEnabled = ref(false);
const settingsCustomApiUrl = ref('');

async function openSettings() {
  try {
    const proxy = await DbApi.getSetting({ key: 'proxyUrl' });
    if (proxy) settingsProxy.value = JSON.parse(proxy);
  } catch { settingsProxy.value = ''; }
  try {
    const en = await DbApi.getSetting({ key: 'customApiEnabled' });
    settingsCustomApiEnabled.value = en === 'true' || en === '"true"';
  } catch { settingsCustomApiEnabled.value = false; }
  try {
    const url = await DbApi.getSetting({ key: 'customApiUrl' });
    if (url) settingsCustomApiUrl.value = JSON.parse(url);
  } catch { settingsCustomApiUrl.value = ''; }
  showSettingsDialog.value = true;
}

async function saveSettingsAndRestart() {
  try {
    await DbApi.saveSetting({ key: 'proxyUrl', value: JSON.stringify(settingsProxy.value || '') });
    await DbApi.saveSetting({ key: 'proxyEnabled', value: settingsProxy.value ? 'true' : 'false' });
    await DbApi.saveSetting({ key: 'customApiEnabled', value: settingsCustomApiEnabled.value ? 'true' : 'false' });
    await DbApi.saveSetting({ key: 'customApiUrl', value: JSON.stringify(settingsCustomApiUrl.value || '') });
    // 通知后端立即应用代理
    try {
      const ac = await DbApi.getAuth();
      await VrcApi.setProxy({ proxyUrl: settingsProxy.value || null, authCookie: ac });
    } catch {}
    showSettingsDialog.value = false;
    // 软重载（在 Tauri 里相当于刷新 webview）
    setTimeout(() => window.location.reload(), 300);
  } catch (e: any) {
    console.error('Failed to save settings', e);
  }
}

// ========== 更新检查弹窗 ==========
const showUpdateDialog = ref(false);
const updateChannel = ref<'stable' | 'beta'>('stable');
const updateLoading = ref(false);
const updateReleases = ref<{ tag: string; name: string; prerelease: boolean; published_at: string; body?: string; assets: any[] }[]>([]);
const updateSelectedTag = ref('');
const showUpdateVersionDropdown = ref(false);

const updateChannelReleases = computed(() =>
  updateReleases.value.filter(r => updateChannel.value === 'beta' ? r.prerelease : !r.prerelease)
);

const isCurrentLatest = computed(() => {
  const list = updateChannelReleases.value;
  if (!list.length) return false;
  return list[0].tag === `v${appVersion.value}` || list[0].tag === appVersion.value;
});

async function openUpdateDialog() {
  showUpdateDialog.value = true;
  if (updateReleases.value.length === 0) {
    await fetchReleases();
  }
}

async function fetchReleases() {
  updateLoading.value = true;
  try {
    const res = await fetch('https://api.github.com/repos/KingXiaoTaoOVO/VRCDog/releases?per_page=30', {
      headers: { 'Accept': 'application/vnd.github+json' }
    });
    if (res.ok) {
      const data = await res.json();
      updateReleases.value = data.map((r: any) => ({
        tag: r.tag_name,
        name: r.name || r.tag_name,
        prerelease: !!r.prerelease,
        published_at: r.published_at,
        body: r.body,
        assets: r.assets || []
      }));
      const list = updateChannelReleases.value;
      if (list.length) updateSelectedTag.value = list[0].tag;
    }
  } catch (e) {
    console.warn('Failed to fetch releases', e);
  } finally {
    updateLoading.value = false;
  }
}

function switchUpdateChannel(ch: 'stable' | 'beta') {
  updateChannel.value = ch;
  const list = updateChannelReleases.value;
  if (list.length) updateSelectedTag.value = list[0].tag;
}

async function downloadUpdate() {
  const rel = updateReleases.value.find(r => r.tag === updateSelectedTag.value);
  if (!rel) return;
  // 优先找 .exe 安装包，没有就打开 release 页面
  const exeAsset = rel.assets.find((a: any) => a.name.endsWith('.exe') || a.name.endsWith('.msi'));
  const url = exeAsset?.browser_download_url || `https://github.com/KingXiaoTaoOVO/VRCDog/releases/tag/${rel.tag}`;
  window.open(url, '_blank');
}

// ========== 登录逻辑 ==========
const emit = defineEmits(['login-success']);

const username = ref('');
const password = ref('');
const authCookie = ref('');
const loading = ref(false);
const errorMsg = ref('');

const show2FA = ref(false);
const twoFactorMethods = ref<string[]>([]);
const twoFactorCode = ref('');

const handleLogin = async () => {
  if (!username.value || !password.value) {
    if (!authCookie.value) {
      errorMsg.value = t('login.error_require_credentials');
      return;
    }
  }

  loading.value = true;
  errorMsg.value = '';

  try {
    await DbApi.clearAuth();
    await VrcApi.clearCookies();
    try { await VrcApi.fetchConfig(); } catch (e: any) {
      errorMsg.value = t('login.error_network', { err: e });
      loading.value = false;
      return;
    }

    const res: any = await VrcApi.login({
      username: username.value || null,
      password: password.value || null,
      authCookie: authCookie.value || null
    });

    if (res.error) {
      errorMsg.value = res.error.message || JSON.stringify(res.error);
    } else if (res.requiresTwoFactorAuth || res.requires_two_factor_auth) {
      show2FA.value = true;
      twoFactorMethods.value = Array.isArray(res.requiresTwoFactorAuth) ? res.requiresTwoFactorAuth : Array.isArray(res.requires_two_factor_auth) ? res.requires_two_factor_auth : ['totp'];
      
      if (res.auth_cookie) {
        authCookie.value = res.auth_cookie;
        await mergeCookiesAndSave(res.auth_cookie);
      }
    } else if (res.id || res.currentUser || res.current_user) {
      if (res.auth_cookie) {
        authCookie.value = res.auth_cookie;
        await mergeCookiesAndSave(res.auth_cookie);
      }
      const user = res.currentUser || res.current_user || res;
      // 保存账号
      await saveCurrentAccount(user, normalizeAuthCookieJson(res.auth_cookie || authCookie.value));
      emit('login-success', user);
    } else {
      errorMsg.value = `Unhandled login response: ${JSON.stringify(res)}`;
    }
  } catch (err: any) {
    const msg = err.message || JSON.stringify(err);
    if (msg.includes("hold your horses") || msg.toLowerCase().includes("twofactor")) {
      show2FA.value = true;
      twoFactorMethods.value = ["emailOtp"];
    } else {
      errorMsg.value = msg;
    }
  } finally {
    loading.value = false;
  }
};

const handle2FA = async () => {
  if (!twoFactorCode.value) return;

  loading.value = true;
  errorMsg.value = '';

  try {
    const method = twoFactorMethods.value.includes('totp') ? 'totp'
                  : twoFactorMethods.value.includes('emailOtp') ? 'emailOtp'
                  : 'otp';
    const verifyRes: any = await VrcApi.verify2fa({
      code: twoFactorCode.value,
      method,
      authCookie: authCookie.value
    });

    const verified = verifyRes?.verified === true;
    if (verified) {
      const user: any = await VrcApi.getCurrentUser();
      if (user) {
        // 保存账号
        const finalCookie = await DbApi.getAuth().catch(() => null);
        await saveCurrentAccount(user, normalizeAuthCookieJson(finalCookie || authCookie.value));
        emit('login-success', user);
      } else {
        errorMsg.value = t('login.error_verify_failed');
      }
    } else {
      errorMsg.value = t('login.error_code_wrong');
    }
  } catch (err: any) {
    errorMsg.value = err.message || err;
  } finally {
    loading.value = false;
  }
};

function closeVrcxMenusOutside(e: MouseEvent) {
  const el = e.target as HTMLElement;
  if (!el.closest('.vrcx-lang-wrap')) showLangMenu.value = false;
  if (!el.closest('.vrcx-version-trigger') && !el.closest('.vrcx-version-list')) showUpdateVersionDropdown.value = false;
}

onMounted(async () => {
  try {
    appVersion.value = await getVersion();
    const cookie = await DbApi.getAuth();
    if (cookie) {
      authCookie.value = cookie;
    }
  } catch (e) {}
  await loadSavedAccounts();
  document.addEventListener('click', closeVrcxMenusOutside);
});

onUnmounted(() => {
  document.removeEventListener('click', closeVrcxMenusOutside);
});
</script>

<template>
  <div class="flex items-center justify-center h-screen bg-[var(--theme-bg-main)] relative overflow-hidden">
    <!-- VRCX 风格左上角工具栏 -->
    <div class="fixed top-4 left-4 z-50 flex gap-1 items-center">
      <!-- 设置 -->
      <button
        class="vrcx-toolbar-btn"
        :title="t('login.toolbar.settings')"
        @click="openSettings"
      >
        <Settings :size="18" />
      </button>
      <!-- 检查更新 -->
      <button
        class="vrcx-toolbar-btn"
        :title="t('login.toolbar.check_update')"
        @click="openUpdateDialog"
      >
        <ArrowDownToLine :size="18" />
      </button>
      <!-- 语言 -->
      <div class="relative vrcx-lang-wrap">
        <button
          class="vrcx-toolbar-btn"
          :title="t('login.toolbar.language')"
          @click.stop="showLangMenu = !showLangMenu"
        >
          <Languages :size="18" />
        </button>
        <transition name="vrcx-menu">
          <div v-if="showLangMenu" class="vrcx-lang-menu">
            <button
              v-for="lang in languages"
              :key="lang.code"
              class="vrcx-lang-item"
              :class="{ active: preferredLangCode === lang.code }"
              @click="selectLanguage(lang.code)"
            >
              <Check
                :size="14"
                class="vrcx-lang-check"
                :style="{ visibility: preferredLangCode === lang.code ? 'visible' : 'hidden' }"
              />
              <span class="vrcx-lang-label">{{ lang.label }} ({{ lang.code }})</span>
            </button>
          </div>
        </transition>
      </div>
    </div>

    <div class="absolute -top-20 -left-20 text-orange-200/40 transform -rotate-12 pointer-events-none">
      <Bone :size="300" stroke-width="1" />
    </div>
    <div class="absolute -bottom-20 -right-20 text-orange-200/40 transform rotate-12 pointer-events-none">
      <Bone :size="300" stroke-width="1" />
    </div>

    <!-- 主容器：登录表单 + 已保存账号 -->
    <div class="flex gap-6 items-stretch justify-center relative z-10 max-w-4xl w-full px-4">
      <!-- 左侧：登录表单 -->
      <div class="bg-[var(--theme-surface)]/90 backdrop-blur-xl p-8 rounded-3xl shadow-2xl w-full max-w-md border border-[var(--theme-border-soft)] flex flex-col">
        <div class="text-center mb-8">
          <h1 class="text-3xl font-bold text-[var(--theme-text)] mb-2 font-mono flex items-center justify-center gap-2">
            <Bone class="animate-bounce" /> {{ t('login.title') }}
          </h1>
          <p class="text-zinc-400 text-sm">
            {{ t('login.subtitle') }}
          </p>
        </div>

        <div v-if="!show2FA" class="space-y-5">
          <div>
            <label class="block text-sm font-bold text-text mb-1 flex items-center gap-1">
              <User :size="16" /> {{ t('login.username') }}
            </label>
            <input
              v-model="username"
              type="text"
              :placeholder="t('login.username')"
              class="w-full px-4 py-3 rounded-xl border-2 border-[var(--theme-border-soft)] focus:border-[var(--theme-primary)] focus:ring-0 outline-none transition-colors bg-[var(--theme-surface)] text-[var(--theme-text)]"
            >
          </div>
          <div>
            <label class="block text-sm font-bold text-text mb-1 flex items-center gap-1">
              <Key :size="16" /> {{ t('login.password') }}
            </label>
            <input
              v-model="password"
              type="password"
              :placeholder="t('login.password')"
              class="w-full px-4 py-3 rounded-xl border-2 border-[var(--theme-border-soft)] focus:border-[var(--theme-primary)] focus:ring-0 outline-none transition-colors bg-[var(--theme-surface)] text-[var(--theme-text)]"
              @keyup.enter="handleLogin"
            >
          </div>

          <!-- 保存登录数据复选框 -->
          <label class="inline-flex items-center gap-2 text-sm text-[var(--theme-text-muted)] cursor-pointer select-none">
            <input
              v-model="saveCredentials"
              type="checkbox"
              class="w-4 h-4 rounded border-[var(--theme-border-soft)] accent-[var(--theme-primary)] focus:ring-[var(--theme-primary)] bg-[var(--theme-surface)]"
            >
            {{ t('login.save_credentials') }}
          </label>

          <div class="relative flex items-center py-4">
            <div class="flex-grow border-t border-[var(--theme-border-soft)]" />
            <span class="flex-shrink-0 mx-4 text-zinc-500 text-xs font-bold uppercase tracking-widest">{{ t('login.or') }}</span>
            <div class="flex-grow border-t border-[var(--theme-border-soft)]" />
          </div>

          <div>
            <label class="block text-xs font-bold text-text mb-1">{{ t('login.use_cookie') }}</label>
            <input
              v-model="authCookie"
              type="text"
              :placeholder="t('login.cookie_ph')"
              class="w-full px-4 py-2 rounded-xl border border-[var(--theme-border-soft)] focus:border-[var(--theme-primary)] focus:ring-0 outline-none transition-colors bg-[var(--theme-surface)] text-[var(--theme-text)] placeholder-zinc-500 text-xs"
              @keyup.enter="handleLogin"
            >
          </div>

          <button
            :disabled="loading"
            class="w-full mt-4 bg-primary hover:bg-primary/80 text-white font-bold py-3 px-4 rounded-xl shadow-[0_0_15px_rgba(var(--theme-primary),0.3)] transition-all flex items-center justify-center gap-2 disabled:opacity-50 disabled:cursor-not-allowed"
            @click="handleLogin"
          >
            <Loader2 v-if="loading" class="animate-spin" :size="20" />
            {{ loading ? t('login.btn_logging_in') : t('login.btn_login') }}
          </button>
        </div>

        <div v-else class="space-y-5">
          <div class="text-center p-4 bg-[var(--theme-surface-hover)] rounded-xl mb-4 border border-[var(--theme-border-soft)]">
            <Key class="mx-auto text-primary mb-2 animate-pulse" :size="32" />
            <p class="text-text font-bold">
              {{ t('login.2fa_title') }}
            </p>
            <p class="text-sm text-orange-600/80">
              {{ t('login.2fa_desc', { method: twoFactorMethods.includes('totp') ? t('login.2fa_method_totp') : t('login.2fa_method_email') }) }}
            </p>
          </div>
          <div>
            <input
              v-model="twoFactorCode"
              type="text"
              :placeholder="t('login.2fa_placeholder')"
              class="w-full px-4 py-4 rounded-xl border border-[var(--theme-border-soft)] focus:border-[var(--theme-primary)] focus:ring-0 outline-none transition-colors bg-[var(--theme-surface)] text-[var(--theme-text)] placeholder-zinc-600 text-center text-2xl tracking-[0.5em] font-mono font-bold"
              @keyup.enter="handle2FA"
            >
          </div>
          <button
            :disabled="loading"
            class="w-full mt-4 bg-green-500/90 hover:bg-green-500 text-white font-bold py-3 px-4 rounded-xl shadow-[0_0_15px_rgba(34,197,94,0.3)] transition-all flex items-center justify-center gap-2 disabled:opacity-50 disabled:cursor-not-allowed"
            @click="handle2FA"
          >
            <Loader2 v-if="loading" class="animate-spin" :size="20" />
            {{ loading ? t('login.btn_submitting') : t('login.btn_submit_code') }}
          </button>
          <div class="mt-6 flex justify-center">
            <button
              @click="show2FA = false"
              class="group relative px-6 py-2.5 rounded-xl flex items-center justify-center gap-2 font-bold transition-all duration-300 overflow-hidden bg-[var(--theme-surface)] hover:bg-[var(--theme-surface-hover)] border border-[var(--theme-border-soft)]"
            >
              <div class="absolute inset-0 bg-gradient-to-r from-transparent via-[var(--theme-surface-hover)] to-transparent -translate-x-[100%] group-hover:animate-[shimmer_1.5s_infinite]" />
              <ArrowLeft class="w-4 h-4 text-zinc-400 group-hover:-translate-x-1 transition-transform" />
              <span class="text-[var(--theme-text-muted)] group-hover:text-[var(--theme-text)] transition-colors">{{ t('login.btn_cancel') || 'Cancel' }}</span>
            </button>
          </div>
        </div>

        <p
          v-if="errorMsg"
          class="mt-4 text-center text-red-500 text-sm font-bold animate-pulse"
        >
          {{ errorMsg }}
        </p>
      </div>

      <!-- 右侧：已保存的账号（2FA 阶段隐藏） -->
      <div
        v-if="!show2FA"
        class="bg-[var(--theme-surface)]/90 backdrop-blur-xl p-6 rounded-3xl shadow-2xl w-full max-w-xs border border-[var(--theme-border-soft)] flex flex-col"
      >
        <h2 class="text-center text-lg font-bold text-[var(--theme-text)] mb-4">
          {{ t('login.saved_accounts') }}
        </h2>
        <div v-if="savedAccounts.length > 0" class="flex-1 overflow-y-auto space-y-2">
          <div
            v-for="account in savedAccounts"
            :key="account.userId"
            class="flex items-center gap-3 p-3 rounded-xl cursor-pointer hover:bg-[var(--theme-surface-hover)] transition-colors border border-transparent hover:border-[var(--theme-border-soft)] group"
            @click="loginWithSavedAccount(account)"
          >
            <img
              v-if="account.avatarUrl"
              :src="account.avatarUrl"
              class="w-10 h-10 rounded-full object-cover flex-shrink-0 bg-zinc-700"
              @error="($event.target as HTMLImageElement).style.display = 'none'"
            >
            <div v-else class="w-10 h-10 rounded-full bg-zinc-700 flex items-center justify-center flex-shrink-0">
              <User :size="18" class="text-zinc-400" />
            </div>
            <div class="flex-1 min-w-0">
              <p class="text-sm font-bold text-[var(--theme-text)] truncate">{{ account.displayName }}</p>
              <p class="text-xs text-[var(--theme-text-muted)] truncate">{{ account.username }}</p>
            </div>
            <button
              class="opacity-0 group-hover:opacity-100 transition-opacity p-1.5 rounded-lg hover:bg-red-500/20 text-zinc-500 hover:text-red-400"
              @click.stop="deleteSavedAccount(account.userId)"
              :title="t('login.delete_account')"
            >
              <Trash2 :size="16" />
            </button>
          </div>
        </div>
        <div v-else class="flex-1 flex items-center justify-center">
          <p class="text-sm text-[var(--theme-text-muted)] text-center">{{ t('login.no_saved_accounts') }}</p>
        </div>
      </div>
    </div>
    
    <!-- 左下角版本号 -->
    <div class="absolute bottom-4 left-4 z-50">
      <span class="text-xs font-mono font-bold text-text-muted/40 bg-surface backdrop-blur px-2 py-1 rounded-lg">v{{ appVersion }}</span>
    </div>

    <!-- ============ 设置弹窗 ============ -->
    <transition name="vrcx-modal">
      <div v-if="showSettingsDialog" class="vrcx-dialog-overlay" @click="showSettingsDialog = false">
        <div class="vrcx-dialog-panel" style="width: 460px;" @click.stop>
          <div class="vrcx-dialog-header">
            <h3 class="vrcx-dialog-title">{{ t('login.settings.title') }}</h3>
            <button class="vrcx-dialog-close" @click="showSettingsDialog = false"><X :size="18" /></button>
          </div>
          <div class="vrcx-dialog-body">
            <label class="vrcx-field-label">{{ t('login.settings.proxy_label') }}</label>
            <input
              v-model="settingsProxy"
              type="text"
              :placeholder="t('login.settings.proxy_placeholder')"
              class="vrcx-input"
            >
            <label class="vrcx-checkbox-row mt-4">
              <input
                v-model="settingsCustomApiEnabled"
                type="checkbox"
                class="w-4 h-4 rounded accent-[var(--theme-primary)]"
              >
              <span>{{ t('login.settings.custom_api_label') }}</span>
            </label>
            <input
              v-if="settingsCustomApiEnabled"
              v-model="settingsCustomApiUrl"
              type="text"
              placeholder="https://api.vrchat.cloud/api/1"
              class="vrcx-input mt-2"
            >
          </div>
          <div class="vrcx-dialog-footer">
            <button class="vrcx-btn-primary" @click="saveSettingsAndRestart">{{ t('login.settings.save_restart') }}</button>
            <button class="vrcx-btn-secondary" @click="showSettingsDialog = false">{{ t('login.settings.close') }}</button>
          </div>
        </div>
      </div>
    </transition>

    <!-- ============ 更新检查弹窗 ============ -->
    <transition name="vrcx-modal">
      <div v-if="showUpdateDialog" class="vrcx-dialog-overlay" @click="showUpdateDialog = false">
        <div class="vrcx-dialog-panel" style="width: 520px;" @click.stop>
          <div class="vrcx-dialog-header">
            <h3 class="vrcx-dialog-title">{{ t('login.update.title') }}</h3>
            <button class="vrcx-dialog-close" @click="showUpdateDialog = false"><X :size="18" /></button>
          </div>
          <div class="vrcx-dialog-body">
            <!-- 稳定版 / 测试版 切换 -->
            <div class="vrcx-tab-group">
              <button
                class="vrcx-tab-pill"
                :class="{ active: updateChannel === 'stable' }"
                @click="switchUpdateChannel('stable')"
              >{{ t('login.update.tab_stable') }}</button>
              <button
                class="vrcx-tab-pill"
                :class="{ active: updateChannel === 'beta' }"
                @click="switchUpdateChannel('beta')"
              >{{ t('login.update.tab_beta') }}</button>
            </div>

            <!-- 测试版警告 -->
            <div v-if="updateChannel === 'beta'" class="vrcx-warning-box">
              <AlertCircle :size="18" class="vrcx-warning-icon" />
              <div>
                <div class="vrcx-warning-title">{{ t('login.update.beta_warning_title') }}</div>
                <div class="vrcx-warning-desc">{{ t('login.update.beta_warning_desc') }}</div>
              </div>
            </div>

            <!-- 版本下拉 -->
            <label class="vrcx-field-label mt-4">{{ t('login.update.version') }}</label>
            <div class="relative">
              <button
                class="vrcx-version-trigger"
                :disabled="updateLoading || updateChannelReleases.length === 0"
                @click.stop="showUpdateVersionDropdown = !showUpdateVersionDropdown"
              >
                <span>{{ updateLoading ? '...' : (updateSelectedTag || (updateChannelReleases.length === 0 ? t('login.update.no_releases') : '...')) }}</span>
                <ChevronDown :size="16" />
              </button>
              <transition name="vrcx-menu">
                <div v-if="showUpdateVersionDropdown" class="vrcx-version-list">
                  <button
                    v-for="rel in updateChannelReleases"
                    :key="rel.tag"
                    class="vrcx-version-item"
                    :class="{ active: rel.tag === updateSelectedTag }"
                    @click="updateSelectedTag = rel.tag; showUpdateVersionDropdown = false"
                  >
                    <span>{{ rel.tag }}</span>
                    <Check v-if="rel.tag === updateSelectedTag" :size="14" />
                  </button>
                </div>
              </transition>
            </div>

            <!-- 是否最新 -->
            <p v-if="!updateLoading && updateChannel === 'stable'" class="vrcx-update-status">
              <span v-if="isCurrentLatest">{{ t('login.update.up_to_date') }}</span>
              <span v-else style="color: #f97316;">{{ t('login.update.has_new', { version: updateChannelReleases[0]?.tag || '' }) }}</span>
            </p>
          </div>
          <div class="vrcx-dialog-footer">
            <button
              class="vrcx-btn-primary"
              :disabled="!updateSelectedTag"
              @click="downloadUpdate"
            >{{ t('login.update.download') }}</button>
          </div>
        </div>
      </div>
    </transition>
  </div>
</template>

<style scoped>
/* ── VRCX-style toolbar ─────────────────────────────────────── */
.vrcx-toolbar-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 36px;
  height: 36px;
  border-radius: 9999px;
  background: transparent;
  color: var(--theme-text-muted);
  border: 1px solid transparent;
  transition: background 0.15s, color 0.15s;
  cursor: pointer;
}
.vrcx-toolbar-btn:hover {
  background: var(--theme-surface-hover);
  color: var(--theme-text);
}

/* ── Language menu ─────────────────────────────────────────── */
.vrcx-lang-wrap { position: relative; }
.vrcx-lang-menu {
  position: absolute;
  top: 44px;
  left: 0;
  z-index: 9999;
  width: 270px;
  max-height: 360px;
  overflow-y: auto;
  background: var(--theme-surface);
  border: 1px solid var(--theme-border-soft);
  border-radius: 10px;
  box-shadow: 0 12px 32px rgba(0, 0, 0, 0.35);
  padding: 6px 0;
}
.vrcx-lang-item {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
  padding: 8px 14px;
  font-size: 14px;
  color: var(--theme-text);
  background: transparent;
  border: none;
  cursor: pointer;
  text-align: left;
}
.vrcx-lang-item:hover { background: var(--theme-surface-hover); }
.vrcx-lang-check { flex-shrink: 0; color: var(--theme-primary); }
.vrcx-lang-label {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* ── Dialog overlay & panel ───────────────────────────────── */
.vrcx-dialog-overlay {
  position: fixed;
  inset: 0;
  z-index: 100;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.55);
  backdrop-filter: blur(4px);
}
.vrcx-dialog-panel {
  background: var(--theme-surface);
  border: 1px solid var(--theme-border-soft);
  border-radius: 16px;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5);
  display: flex;
  flex-direction: column;
  max-height: 90vh;
}
.vrcx-dialog-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 18px 22px 12px 22px;
}
.vrcx-dialog-title {
  font-size: 18px;
  font-weight: 600;
  color: var(--theme-text);
  margin: 0;
}
.vrcx-dialog-close {
  width: 32px; height: 32px;
  border-radius: 9999px;
  background: transparent;
  color: var(--theme-text-muted);
  border: 1px solid transparent;
  cursor: pointer;
  display: inline-flex;
  align-items: center; justify-content: center;
}
.vrcx-dialog-close:hover { background: var(--theme-surface-hover); color: var(--theme-text); }
.vrcx-dialog-body {
  padding: 4px 22px 16px 22px;
  overflow-y: auto;
}
.vrcx-dialog-footer {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  padding: 12px 22px 18px 22px;
}

/* ── Form controls ───────────────────────────────────────── */
.vrcx-field-label {
  display: block;
  font-size: 13px;
  font-weight: 600;
  color: var(--theme-text);
  margin-bottom: 6px;
}
.vrcx-input {
  width: 100%;
  padding: 10px 14px;
  border: 1px solid var(--theme-border-soft);
  border-radius: 8px;
  background: var(--theme-bg-main);
  color: var(--theme-text);
  font-size: 14px;
  outline: none;
  transition: border-color 0.15s;
}
.vrcx-input:focus { border-color: var(--theme-primary); }
.vrcx-checkbox-row {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  font-size: 14px;
  color: var(--theme-text);
  cursor: pointer;
}

/* ── Buttons ─────────────────────────────────────────────── */
.vrcx-btn-primary {
  padding: 8px 18px;
  border-radius: 8px;
  background: var(--theme-text);
  color: var(--theme-bg-main);
  font-size: 14px;
  font-weight: 600;
  border: none;
  cursor: pointer;
  transition: opacity 0.15s;
}
.vrcx-btn-primary:hover { opacity: 0.85; }
.vrcx-btn-primary:disabled { opacity: 0.4; cursor: not-allowed; }
.vrcx-btn-secondary {
  padding: 8px 18px;
  border-radius: 8px;
  background: transparent;
  color: var(--theme-text);
  font-size: 14px;
  font-weight: 600;
  border: 1px solid var(--theme-border-soft);
  cursor: pointer;
}
.vrcx-btn-secondary:hover { background: var(--theme-surface-hover); }

/* ── Tab pills (stable / beta) ───────────────────────────── */
.vrcx-tab-group {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 4px;
  padding: 4px;
  border-radius: 10px;
  background: var(--theme-bg-main);
  border: 1px solid var(--theme-border-soft);
  margin-bottom: 8px;
}
.vrcx-tab-pill {
  padding: 10px 16px;
  border-radius: 8px;
  background: transparent;
  color: var(--theme-text-muted);
  font-size: 14px;
  font-weight: 500;
  border: none;
  cursor: pointer;
  transition: background 0.15s, color 0.15s;
}
.vrcx-tab-pill.active {
  background: var(--theme-surface);
  color: var(--theme-text);
}

/* ── Beta warning box ────────────────────────────────────── */
.vrcx-warning-box {
  display: flex;
  gap: 12px;
  padding: 12px 14px;
  border-radius: 10px;
  background: rgba(239, 68, 68, 0.08);
  border: 1px solid rgba(239, 68, 68, 0.4);
  margin-top: 12px;
}
.vrcx-warning-icon { color: #ef4444; flex-shrink: 0; margin-top: 2px; }
.vrcx-warning-title { font-weight: 600; color: #ef4444; font-size: 14px; }
.vrcx-warning-desc { color: #ef4444; font-size: 13px; opacity: 0.85; margin-top: 2px; line-height: 1.4; }

/* ── Version dropdown ────────────────────────────────────── */
.vrcx-version-trigger {
  width: 100%;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 14px;
  border: 1px solid var(--theme-border-soft);
  border-radius: 8px;
  background: var(--theme-bg-main);
  color: var(--theme-text);
  font-size: 14px;
  cursor: pointer;
}
.vrcx-version-trigger:disabled { opacity: 0.5; cursor: not-allowed; }
.vrcx-version-list {
  position: absolute;
  top: 44px;
  left: 0;
  right: 0;
  z-index: 10;
  max-height: 320px;
  overflow-y: auto;
  background: var(--theme-surface);
  border: 1px solid var(--theme-border-soft);
  border-radius: 8px;
  box-shadow: 0 12px 32px rgba(0, 0, 0, 0.4);
  padding: 4px 0;
}
.vrcx-version-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
  padding: 8px 14px;
  font-size: 14px;
  color: var(--theme-text);
  background: transparent;
  border: none;
  cursor: pointer;
  text-align: left;
}
.vrcx-version-item:hover { background: var(--theme-surface-hover); }
.vrcx-version-item.active { color: var(--theme-primary); }

.vrcx-update-status {
  margin-top: 12px;
  font-size: 13px;
  color: var(--theme-text-muted);
}

/* ── Transitions ────────────────────────────────────────── */
.vrcx-modal-enter-active, .vrcx-modal-leave-active { transition: opacity 0.18s; }
.vrcx-modal-enter-from, .vrcx-modal-leave-to { opacity: 0; }
.vrcx-modal-enter-active .vrcx-dialog-panel,
.vrcx-modal-leave-active .vrcx-dialog-panel { transition: transform 0.18s; }
.vrcx-modal-enter-from .vrcx-dialog-panel { transform: scale(0.96); }
.vrcx-modal-leave-to .vrcx-dialog-panel { transform: scale(0.96); }

.vrcx-menu-enter-active, .vrcx-menu-leave-active { transition: opacity 0.12s, transform 0.12s; }
.vrcx-menu-enter-from, .vrcx-menu-leave-to { opacity: 0; transform: translateY(-4px); }
</style>
