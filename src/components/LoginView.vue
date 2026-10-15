<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue';
import { VrcApi, DbApi } from "../api";
import { Key, User, Loader2, ArrowLeft, Trash2, Settings, ArrowDownToLine, Languages, Check, X, ChevronDown, AlertCircle } from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';
import { getLocaleLabel, localeOptions, normalizeLocale, setAppLocale } from '../i18n';
import { getVersion } from '@tauri-apps/api/app';
import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { currentTheme } from '../theme';
import { getCookieValue, mergeCookiesAndSave, normalizeAuthCookieJson, parseCookieInput } from '../api/cookies';
import { useAuthStore } from '../stores/authStore';
import {
  chooseDefaultTwoFactorMethod,
  getTwoFactorErrorMessage,
  getTwoFactorResponseCookie,
  isTwoFactorVerified,
  isValidTwoFactorCode,
  normalizeTwoFactorCode,
  normalizeTwoFactorMethods,
  type TwoFactorMethod,
} from '../api/twoFactor';

const { t, locale } = useI18n({ useScope: 'global' });
let authStore: ReturnType<typeof useAuthStore> | null = null;
try {
  authStore = useAuthStore();
} catch {
  // Fallback for standalone/test environments without Pinia installed
}
const appVersion = ref('');

function formatCookieForDisplay(rawCookie: string | null | undefined): string {
  if (!rawCookie) return '';
  const cookies = parseCookieInput(rawCookie);
  if (cookies.length === 0) return String(rawCookie).trim();
  return cookies.join('; ');
}

// ========== 已保存账号管理 ==========
export interface SavedAccount {
  userId: string;
  displayName: string;
  username: string;
  avatarUrl: string;
  authCookie: string;
  savedPassword?: string;
  updatedAt?: number;
}

const savedAccounts = ref<SavedAccount[]>([]);
// Account persistence is enabled by default so a successful 2FA session can
// be reused on the next launch. The checkbox still lets the user opt out.
const saveCredentials = ref(true);

async function loadSavedAccounts() {
  try {
    const raw = await DbApi.getSetting({ key: 'savedAccounts' });
    if (raw) {
      const parsed = JSON.parse(raw);
      savedAccounts.value = Array.isArray(parsed) ? parsed : [];
    }
  } catch { /* ignore */ }
}

async function persistSavedAccounts() {
  // 安全：桌面端 DbApi.saveSetting 落到应用私有 SQLite，浏览器/Web 构建则落到
  // localStorage（同源任何脚本都可读）。因此 Web 模式下只持久化账号展示信息，
  // 绝不落盘 authCookie 与凭据，避免会话令牌被同源 XSS 或浏览器扩展直接取走。
  const payload = isTauri()
    ? savedAccounts.value
    : savedAccounts.value.map(({ authCookie: _authCookie, savedPassword: _savedPassword, ...rest }) => ({ ...rest, authCookie: '' }));
  await DbApi.saveSetting({ key: 'savedAccounts', value: JSON.stringify(payload) });
}

function hasUsableAuthCookie(rawCookie: string | null | undefined): boolean {
  return parseCookieInput(rawCookie).some(cookie => {
    const equalsIdx = cookie.indexOf('=');
    if (equalsIdx <= 0) return false;
    const name = cookie.slice(0, equalsIdx).trim().toLowerCase();
    const val = cookie.slice(equalsIdx + 1).trim().replace(/^["']|["']$/g, '');
    return name === 'auth' && val.length >= 8 && !['deleted', 'null', 'undefined', 'none'].includes(val.toLowerCase());
  });
}

function prepareManualLoginFromSavedAccount(account: SavedAccount, messageKey: string) {
  username.value = account.username || account.displayName || '';
  password.value = account.savedPassword || '';
  const twoFa = getCookieValue(account.authCookie, 'twoFactorAuth');
  authCookie.value = twoFa ? `twoFactorAuth=${twoFa}` : '';
  saveCredentials.value = true;
  errorMsg.value = t(messageKey);
}

async function updateSavedAccountCookie(userId: string, cookie: string) {
  const idx = savedAccounts.value.findIndex(a => a.userId === userId);
  if (idx < 0) return;
  savedAccounts.value[idx].authCookie = cookie;
  savedAccounts.value[idx].updatedAt = Date.now();
  await persistSavedAccounts();
}

async function saveCurrentAccount(user: any, cookie: string) {
  if (!saveCredentials.value) return;
  if (!user?.id) return;

  const accountUsername = user.username || user.email || username.value.trim() || user.id || '';
  const existingIdx = savedAccounts.value.findIndex(a => a.userId === user.id);
  const existingAcc = existingIdx >= 0 ? savedAccounts.value[existingIdx] : null;

  const pwdToSave = (password.value && saveCredentials.value)
    ? password.value
    : (existingAcc?.savedPassword || undefined);

  const account: SavedAccount = {
    userId: user.id,
    displayName: user.displayName || user.display_name || user.username || 'Unknown',
    username: accountUsername,
    avatarUrl: user.currentAvatarThumbnailImageUrl || user.currentAvatarImageUrl || existingAcc?.avatarUrl || '',
    authCookie: cookie,
    savedPassword: saveCredentials.value ? pwdToSave : undefined,
    updatedAt: Date.now(),
  };

  if (existingIdx >= 0) {
    savedAccounts.value[existingIdx] = account;
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

  authStore?.setLoginGracePeriod(120_000);
  loading.value = true;
  errorMsg.value = '';

  try {
    try { await VrcApi.clearCookies(); } catch {}

    let loggedUser: any = null;
    let freshCookie = savedCookie;

    // 1. 如果保存的 Cookie 含有未过期的 auth 会话令牌，直接用 session 请求验证
    if (hasUsableAuthCookie(savedCookie)) {
      try {
        await DbApi.saveAuth({ cookie: savedCookie });
        try { await VrcApi.applyAuthCookie({ authCookie: savedCookie }); } catch {}

        const verify: any = await VrcApi.request('/auth/user', {
          method: 'GET',
          authCookie: savedCookie,
          suppressAuthExpired: true,
          timeoutMs: 12000,
        });

        const u = verify?.currentUser || verify?.current_user || (verify?.id ? verify : null);
        if (u) {
          loggedUser = u;
          if (verify.auth_cookie) {
            freshCookie = normalizeAuthCookieJson(verify.auth_cookie);
          }
        }
      } catch {
        /* 会话 token 自然过期，自动进入下一步凭据重鉴权 */
      }
    }

    // 2. 如果 Cookie 已失效，但保存了账号凭据与 30 天 twoFactorAuth 令牌，自动执行免密重连
    if (!loggedUser && account.username && account.savedPassword) {
      try {
        const twoFa = getCookieValue(savedCookie, 'twoFactorAuth');
        const cookieToSend = twoFa ? `twoFactorAuth=${twoFa}` : (savedCookie || undefined);

        const res: any = await VrcApi.login({
          username: account.username,
          password: account.savedPassword,
          authCookie: cookieToSend,
        });

        if (res?.requiresTwoFactorAuth || res?.requires_two_factor_auth) {
          // 30 天 2FA 期限已过，提示输入新的 2FA 验证码
          if (res.auth_cookie) {
            authCookie.value = formatCookieForDisplay(res.auth_cookie);
            await mergeCookiesAndSave(res.auth_cookie);
          }
          username.value = account.username;
          password.value = account.savedPassword;
          beginTwoFactor(res.requiresTwoFactorAuth || res.requires_two_factor_auth, res.auth_cookie);
          return;
        }

        const u = res?.currentUser || res?.current_user || (res?.id ? res : null);
        if (u) {
          loggedUser = u;
          if (res.auth_cookie) {
            freshCookie = normalizeAuthCookieJson(res.auth_cookie);
          }
        }
      } catch (authErr) {
        console.warn('[FastLogin] Credentials re-auth failed:', authErr);
      }
    }

    // 3. 登录成功：落盘刷新后的最新 Cookie 并进入主界面
    if (loggedUser) {
      if (freshCookie) {
        authCookie.value = formatCookieForDisplay(freshCookie);
        await DbApi.saveAuth({ cookie: freshCookie });
        await mergeCookiesAndSave(freshCookie);
        try { await VrcApi.applyAuthCookie({ authCookie: freshCookie }); } catch {}
      }

      const idx = savedAccounts.value.findIndex(a => a.userId === account.userId);
      if (idx >= 0) {
        savedAccounts.value[idx].displayName = loggedUser.displayName || loggedUser.display_name || account.displayName;
        savedAccounts.value[idx].username = account.username || loggedUser.username || account.displayName;
        savedAccounts.value[idx].avatarUrl = loggedUser.currentAvatarThumbnailImageUrl || loggedUser.currentAvatarImageUrl || account.avatarUrl;
        savedAccounts.value[idx].authCookie = freshCookie || savedAccounts.value[idx].authCookie;
        savedAccounts.value[idx].updatedAt = Date.now();
        await persistSavedAccounts();
      }

      show2FA.value = false;
      twoFactorCode.value = '';
      errorMsg.value = '';
      emit('login-success', loggedUser);
      return;
    }

    // 4. 若无法自动重连，优雅预填用户名与设备 2FA 凭据，引导用户仅输入密码即可快速登录
    prepareManualLoginFromSavedAccount(account, 'login.saved_cookie_expired');
  } catch (err: any) {
    prepareManualLoginFromSavedAccount(account, 'login.saved_cookie_expired');
  } finally {
    loading.value = false;
  }
}

// ========== 语言切换（VrcDog 对齐：14 种）==========
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
  return normalizeLocale(
    localStorage.getItem('vrcdog-locale-pref')
    || localStorage.getItem('livehime-locale-pref')
    || locale.value,
  );
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
const updateReleases = ref<{ tag: string; name: string; prerelease: boolean; published_at: string; body?: string; installer_url?: string; installer_sha256?: string | null; installer_size?: number | null }[]>([]);
const updateSelectedTag = ref('');
const showUpdateVersionDropdown = ref(false);

// Silent background pre-check on mount: surfaces a small red dot on the
// toolbar's update icon when a newer stable release exists, but never
// opens the modal automatically — clicking the icon is the user's
// explicit "go check" gesture. This is the "auto detect update on the
// update screen" UX the user asked for.
const updateAvailable = ref(false);
const updateAvailableTag = ref('');

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
    // 通过 Rust 后端真实拉取 GitHub Releases（避免 webview CORS/CSP 限制）
    const data = await invoke<any[]>('update_remote_releases');
    updateReleases.value = data
      .filter((r) => !r.draft)
      .map((r: any) => ({
        tag: r.tag,
        name: r.name || r.tag,
        prerelease: !!r.prerelease,
        published_at: r.published_at,
        body: r.body,
        installer_url: r.installer_url,
        installer_sha256: r.installer_sha256 ?? null,
        installer_size: r.installer_size ?? null
      }));
    const list = updateChannelReleases.value;
    if (list.length) updateSelectedTag.value = list[0].tag;
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

const updateInstalling = ref(false);
const updateInstallPercent = ref(0);

// 全自动更新：下载安装包 → SHA-256 校验 → 静默安装 → 自动启动新版本 → 退出旧进程
async function downloadUpdate() {
  const rel = updateReleases.value.find(r => r.tag === updateSelectedTag.value);
  if (!rel || updateInstalling.value) return;
  if (!rel.installer_url) {
    // 极端情况下仓库没有安装包资产，回退到打开 release 页面
    window.open(`https://github.com/KingXiaoTaoOVO/vrcdog-releases/releases/tag/${rel.tag}`, '_blank');
    return;
  }
  updateInstalling.value = true;
  updateInstallPercent.value = 0;
  const unlisten = await listen<any>('app-update://progress', (ev) => {
    const p = ev.payload || {};
    if (p.stage === 'downloading' && p.bytes_total > 0) {
      updateInstallPercent.value = Math.min(100, Math.floor((p.bytes_done / p.bytes_total) * 100));
    }
  });
  try {
    await invoke('update_install_release', {
      downloadUrl: rel.installer_url,
      expectedSha256: rel.installer_sha256 ?? null,
      expectedSize: rel.installer_size ?? null,
    });
    // Rust 端安装完成后会自动启动新版本并退出当前进程；
    // 若 invoke 意外返回则兜底重启
    await invoke('update_restart');
  } catch (e: any) {
    console.error('Auto update failed:', e);
    alert(t('login.update.install_failed', { error: String(e?.message || e) }));
    updateInstalling.value = false;
  } finally {
    unlisten();
  }
}

// ========== 登录逻辑 ==========
const emit = defineEmits(['login-success']);

const username = ref('');
const password = ref('');
const authCookie = ref('');
const loading = ref(false);
const errorMsg = ref('');

const show2FA = ref(false);
const twoFactorMethods = ref<TwoFactorMethod[]>([]);
const selectedTwoFactorMethod = ref<TwoFactorMethod>('totp');
const twoFactorCode = ref('');

/**
 * Detect a "2FA required" outcome from either a thrown VrcRequestError (the
 * login request is rejected with 401 while 2FA is pending) or a structured
 * response. Returns the available 2FA methods, or null if it's a real failure.
 */
function extractTwoFactorMethods(err: any): TwoFactorMethod[] | null {
  const resp = err?.response;
  const fromResp = resp?.requiresTwoFactorAuth || resp?.requires_two_factor_auth;
  if (fromResp) {
    const methods = normalizeTwoFactorMethods(fromResp);
    return methods.length > 0 ? methods : ['emailOtp'];
  }
  const msg = (err?.message || '').toLowerCase();
  if (
    msg.includes('twofactor') ||
    msg.includes('two factor') ||
    msg.includes('hold your horses')
  ) {
    return ['emailOtp'];
  }
  return null;
}

function beginTwoFactor(methods: unknown, cookie?: string) {
  const normalized = normalizeTwoFactorMethods(methods);
  twoFactorMethods.value = normalized.length > 0 ? normalized : ['totp'];
  selectedTwoFactorMethod.value = chooseDefaultTwoFactorMethod(twoFactorMethods.value);
  twoFactorCode.value = '';
  errorMsg.value = '';
  show2FA.value = true;
  if (cookie) {
    authCookie.value = formatCookieForDisplay(cookie);
  }
  if (!authCookie.value) {
    DbApi.getAuth()
      .then((stored) => { if (stored && !authCookie.value) authCookie.value = formatCookieForDisplay(stored); })
      .catch(() => {});
  }
}

function twoFactorMethodLabel(method: TwoFactorMethod): string {
  return method === 'totp' ? t('login.2fa_method_totp') : t('login.2fa_method_email');
}

const handleLogin = async () => {
  if (loading.value) return;

  const rawUser = username.value.trim();
  const rawPass = password.value;
  const rawCookie = authCookie.value.trim();

  if (!rawUser || !rawPass) {
    if (!rawCookie) {
      errorMsg.value = t('login.error_require_credentials');
      return;
    }
  }

  authStore?.setLoginGracePeriod(60_000);
  loading.value = true;
  errorMsg.value = '';

  try {
    let effectiveCookieToSend: string | null = null;
    if (rawUser && rawPass) {
      // If logging in with username and password:
      // Preserve any existing twoFactorAuth cookie so VRChat recognizes this computer and skips the 2FA prompt
      const currentStored = await DbApi.getAuth().catch(() => null);
      let twoFaToken = getCookieValue(rawCookie, 'twoFactorAuth') || getCookieValue(currentStored, 'twoFactorAuth');
      if (!twoFaToken) {
        const matched = savedAccounts.value.find(a =>
          a.username.toLowerCase() === rawUser.toLowerCase() ||
          a.displayName.toLowerCase() === rawUser.toLowerCase()
        );
        if (matched?.authCookie) {
          twoFaToken = getCookieValue(matched.authCookie, 'twoFactorAuth');
        }
      }
      if (twoFaToken) {
        effectiveCookieToSend = `twoFactorAuth=${twoFaToken}`;
      }
    } else if (rawCookie) {
      effectiveCookieToSend = normalizeAuthCookieJson(rawCookie);
    }

    const res: any = await VrcApi.login({
      username: rawUser || null,
      password: rawPass || null,
      authCookie: effectiveCookieToSend
    });

    if (res.error) {
      errorMsg.value = res.error.message || JSON.stringify(res.error);
    } else if (res.requiresTwoFactorAuth || res.requires_two_factor_auth) {
      if (res.auth_cookie) {
        authCookie.value = formatCookieForDisplay(res.auth_cookie);
        await mergeCookiesAndSave(res.auth_cookie);
      }
      beginTwoFactor(res.requiresTwoFactorAuth || res.requires_two_factor_auth, res.auth_cookie);
    } else if (res.id || res.currentUser || res.current_user) {
      if (res.auth_cookie) {
        authCookie.value = formatCookieForDisplay(res.auth_cookie);
        await mergeCookiesAndSave(res.auth_cookie);
      }
      const user = res.currentUser || res.current_user || res;
      // 保存账号
      const durableCookie = await DbApi.getAuth().catch(() => null);
      await saveCurrentAccount(user, normalizeAuthCookieJson(durableCookie || res.auth_cookie || authCookie.value));
      show2FA.value = false;
      twoFactorCode.value = '';
      errorMsg.value = '';
      await loadSavedAccounts();
      emit('login-success', user);
    } else {
      errorMsg.value = `Unhandled login response: ${JSON.stringify(res)}`;
    }
  } catch (err: any) {
    const methods = extractTwoFactorMethods(err);
    if (methods) {
      const challengeCookie = err?.auth_cookie || err?.response?.auth_cookie;
      if (challengeCookie) {
        authCookie.value = formatCookieForDisplay(challengeCookie);
        await mergeCookiesAndSave(challengeCookie);
      }
      beginTwoFactor(methods, challengeCookie);
    } else {
      errorMsg.value = err.message || JSON.stringify(err);
    }
  } finally {
    loading.value = false;
  }
};

const handle2FA = async () => {
  if (loading.value) return;

  const code = normalizeTwoFactorCode(twoFactorCode.value);
  twoFactorCode.value = code;
  if (!isValidTwoFactorCode(code)) {
    errorMsg.value = t('login.error_code_format');
    return;
  }

  authStore?.setLoginGracePeriod(60_000);
  loading.value = true;
  errorMsg.value = '';

  try {
    let currentAuth = authCookie.value;
    if (!currentAuth) {
      currentAuth = (await DbApi.getAuth().catch(() => null)) || '';
      if (currentAuth) authCookie.value = currentAuth;
    }

    const verifyRes: any = await VrcApi.verify2fa({
      code,
      method: selectedTwoFactorMethod.value,
      authCookie: currentAuth || undefined
    });

    if (isTwoFactorVerified(verifyRes)) {
      const responseCookie = getTwoFactorResponseCookie(verifyRes);
      if (responseCookie) {
        const merged = await mergeCookiesAndSave(responseCookie);
        authCookie.value = formatCookieForDisplay(merged || responseCookie);
      }

      const finalCookie = await DbApi.getAuth().catch(() => null);
      const effectiveCookie = normalizeAuthCookieJson(finalCookie || authCookie.value);
      try {
        await VrcApi.applyAuthCookie({ authCookie: effectiveCookie });
      } catch {}

      // Confirm the freshly-2FA-verified session without re-entering the
      // global logout pipeline. The dedicated App.vue handler already
      // validates auth on any vrc-auth-expired event from other paths.
      const user: any = await VrcApi.getCurrentUser({
        authCookie: effectiveCookie,
        suppressAuthExpired: true,
      });
      if (user?.id || user?.currentUser || user?.current_user) {
        const currentUser = user.currentUser || user.current_user || user;
        await saveCurrentAccount(currentUser, effectiveCookie);
        show2FA.value = false;
        twoFactorCode.value = '';
        errorMsg.value = '';
        await loadSavedAccounts();
        emit('login-success', currentUser);
      } else {
        errorMsg.value = t('login.error_verify_failed');
      }
    } else {
      errorMsg.value = getTwoFactorErrorMessage(verifyRes) || t('login.error_code_wrong');
    }
  } catch (err: any) {
    errorMsg.value = err.message || String(err);
  } finally {
    loading.value = false;
  }
};

function closeVrcDogMenusOutside(e: MouseEvent) {
  const el = e.target as HTMLElement;
  if (!el.closest('.vrcdog-lang-wrap')) showLangMenu.value = false;
  if (!el.closest('.vrcdog-version-trigger') && !el.closest('.vrcdog-version-list')) showUpdateVersionDropdown.value = false;
}

onMounted(async () => {
  show2FA.value = false;
  twoFactorCode.value = '';
  try {
    appVersion.value = await getVersion();
    const cookie = await DbApi.getAuth();
    if (cookie) {
      authCookie.value = formatCookieForDisplay(cookie);
    }
  } catch (e) {}
  await loadSavedAccounts();
  document.addEventListener('click', closeVrcDogMenusOutside);
  // Background pre-check: never blocks the page, never opens a dialog.
  // If a newer stable release is found we light up the toolbar dot.
  silentlyCheckUpdate().catch(() => {});
});

/**
 * Background update probe. Pulls the release list, filters to stable,
 * and surfaces a red dot on the toolbar icon when the newest stable
 * tag is newer than the running app version.
 */
async function silentlyCheckUpdate() {
  try {
    const data = await invoke<any[]>('update_remote_releases');
    const stable = (data || []).filter((r) => !r.draft && !r.prerelease);
    if (stable.length === 0) return;
    const latestTag = String(stable[0].tag || '').replace(/^v/, '');
    const currentTag = String(appVersion.value || '').replace(/^v/, '');
    if (latestTag && currentTag && cmpVersions(latestTag, currentTag) > 0) {
      updateAvailable.value = true;
      updateAvailableTag.value = stable[0].tag;
    }
  } catch {
    // best-effort; no UI feedback for failures during background check
  }
}

/** Local semver-style compare (X.Y.Z with optional pre-release tag). */
function cmpVersions(a: string, b: string): number {
  const parse = (s: string) => {
    const [core, pre] = s.split('-', 2);
    const nums = core.split('.').map((n) => parseInt(n, 10) || 0);
    return { nums, pre: pre || '' };
  };
  const A = parse(a);
  const B = parse(b);
  for (let i = 0; i < Math.max(A.nums.length, B.nums.length); i++) {
    const diff = (A.nums[i] || 0) - (B.nums[i] || 0);
    if (diff !== 0) return diff;
  }
  // No pre-release > pre-release (5.0.6 > 5.0.6-beta.1)
  if (A.pre !== B.pre) return A.pre ? -1 : 1;
  return 0;
}

onUnmounted(() => {
  document.removeEventListener('click', closeVrcDogMenusOutside);
});
</script>

<template>
  <div class="login-page">
    <!-- 左上角工具栏 -->
    <div class="login-toolbar">
      <button class="tool-btn" :title="t('login.toolbar.settings')" @click="openSettings">
        <Settings :size="17" />
      </button>
      <button class="tool-btn" :title="t('login.toolbar.check_update')" @click="openUpdateDialog">
        <ArrowDownToLine :size="17" />
        <template v-if="updateAvailable">
          <span class="tool-dot tool-dot-ping" />
          <span class="tool-dot" :title="t('login.toolbar.update_available', { version: updateAvailableTag })" />
        </template>
      </button>
      <div class="action-menu vrcdog-lang-wrap">
        <button class="tool-btn" :title="t('login.toolbar.language')" @click.stop="showLangMenu = !showLangMenu">
          <Languages :size="17" />
        </button>
        <transition name="vrcdog-menu">
          <div v-if="showLangMenu" class="lang-menu custom-scrollbar">
            <button
              v-for="lang in languages"
              :key="lang.code"
              class="lang-item"
              :class="{ active: preferredLangCode === lang.code }"
              @click="selectLanguage(lang.code)"
            >
              <Check
                :size="14"
                class="lang-check"
                :style="{ visibility: preferredLangCode === lang.code ? 'visible' : 'hidden' }"
              />
              <span class="lang-label">{{ lang.label }} ({{ lang.code }})</span>
            </button>
          </div>
        </transition>
      </div>
    </div>

    <!-- 主舞台 -->
    <div class="login-stage" :class="{ single: show2FA }">
      <!-- 左：登录表单 -->
      <section class="panel">
        <header class="brand-head">
          <img class="brand-logo" :src="currentTheme.logo" alt="">
          <div class="brand-copy">
            <h1>{{ t('login.title') }}</h1>
            <p>{{ t('login.subtitle') }}</p>
          </div>
        </header>

        <div v-if="!show2FA" class="panel-body">
          <label class="theme-field">
            <span>{{ t('login.username') }}</span>
            <span class="input-shell">
              <User :size="16" />
              <input
                v-model="username"
                type="text"
                autocomplete="username"
                spellcheck="false"
                :placeholder="t('login.username')"
              >
            </span>
          </label>

          <label class="theme-field">
            <span>{{ t('login.password') }}</span>
            <span class="input-shell">
              <Key :size="16" />
              <input
                v-model="password"
                type="password"
                autocomplete="current-password"
                :placeholder="t('login.password')"
                @keyup.enter="handleLogin"
              >
            </span>
          </label>

          <label class="checkbox-row">
            <input
              v-model="saveCredentials"
              type="checkbox"
            >
            <span>{{ t('login.save_credentials') }}</span>
          </label>

          <button
            :disabled="loading"
            class="primary-button bg-primary"
            @click="handleLogin"
          >
            <Loader2 v-if="loading" class="spin" :size="18" />
            {{ loading ? t('login.btn_logging_in') : t('login.btn_login') }}
          </button>
        </div>

        <div v-else class="panel-body">
          <div class="notice">
            <Key :size="18" class="notice-icon" />
            <div>
              <strong>{{ t('login.2fa_title') }}</strong>
              <small>{{ t('login.2fa_desc', { method: twoFactorMethodLabel(selectedTwoFactorMethod) }) }}</small>
            </div>
          </div>

          <div
            v-if="twoFactorMethods.length > 1"
            class="segmented"
            role="radiogroup"
            :aria-label="t('login.2fa_choose_method')"
          >
            <button
              v-for="method in twoFactorMethods"
              :key="method"
              type="button"
              role="radio"
              :aria-checked="selectedTwoFactorMethod === method"
              class="segment"
              :class="{ active: selectedTwoFactorMethod === method }"
              @click="selectedTwoFactorMethod = method; twoFactorCode = ''; errorMsg = ''"
            >
              {{ twoFactorMethodLabel(method) }}
            </button>
          </div>

          <label class="theme-field">
            <span>{{ t('login.2fa_placeholder') }}</span>
            <span class="input-shell code-shell">
              <input
                v-model="twoFactorCode"
                type="text"
                inputmode="numeric"
                autocomplete="one-time-code"
                maxlength="12"
                :aria-label="t('login.2fa_placeholder')"
                @keyup.enter="handle2FA"
              >
            </span>
          </label>

          <button
            :disabled="loading"
            class="primary-button"
            @click="handle2FA"
          >
            <Loader2 v-if="loading" class="spin" :size="18" />
            {{ loading ? t('login.btn_submitting') : t('login.btn_submit_code') }}
          </button>

          <button class="ghost-button" @click="show2FA = false">
            <ArrowLeft :size="16" />
            <span>{{ t('login.btn_cancel') || 'Cancel' }}</span>
          </button>
        </div>

        <p v-if="errorMsg" class="error-message">
          <AlertCircle :size="15" />
          <span>{{ errorMsg }}</span>
        </p>
      </section>

      <!-- 右：已保存的账号（2FA 阶段隐藏） -->
      <aside v-if="!show2FA" class="panel panel-accounts">
        <header class="panel-head">
          <User :size="16" />
          <h2>{{ t('login.saved_accounts') }}</h2>
        </header>

        <div v-if="savedAccounts.length > 0" class="account-list custom-scrollbar">
          <div
            v-for="account in savedAccounts"
            :key="account.userId"
            class="vrcdog-saved-account-card account-card"
            @click="loginWithSavedAccount(account)"
          >
            <img
              v-if="account.avatarUrl"
              :src="account.avatarUrl"
              class="account-avatar"
              @error="($event.target as HTMLImageElement).style.display = 'none'"
            >
            <span v-else class="account-avatar account-avatar-fallback">
              <User :size="16" />
            </span>
            <span class="account-copy">
              <strong>{{ account.displayName }}</strong>
              <small>{{ account.username }}</small>
            </span>
            <button
              class="account-delete"
              :title="t('login.delete_account')"
              @click.stop="deleteSavedAccount(account.userId)"
            >
              <Trash2 :size="15" />
            </button>
          </div>
        </div>

        <div v-else class="account-empty">
          <p>{{ t('login.no_saved_accounts') }}</p>
        </div>
      </aside>
    </div>

    <!-- 左下角版本号 -->
    <div class="app-version">v{{ appVersion }}</div>

    <!-- ============ 设置弹窗 ============ -->
    <transition name="vrcdog-modal">
      <div v-if="showSettingsDialog" class="modal-overlay" @click="showSettingsDialog = false">
        <div class="modal-panel" style="width: 440px;" @click.stop>
          <header class="modal-head">
            <h3>
              <Settings :size="17" />
              {{ t('login.settings.title') }}
            </h3>
            <button :aria-label="t('a11y.settings')" class="modal-close" @click="showSettingsDialog = false"><X :size="17" /></button>
          </header>
          <div class="modal-body">
            <label class="theme-field">
              <span>{{ t('login.settings.proxy_label') }}</span>
              <span class="input-shell">
                <input
                  v-model="settingsProxy"
                  type="text"
                  :placeholder="t('login.settings.proxy_placeholder')"
                >
              </span>
            </label>
            <label class="checkbox-row">
              <input
                v-model="settingsCustomApiEnabled"
                type="checkbox"
              >
              <span>{{ t('login.settings.custom_api_label') }}</span>
            </label>
            <label v-if="settingsCustomApiEnabled" class="theme-field">
              <span>API Endpoint</span>
              <span class="input-shell">
                <input
                  v-model="settingsCustomApiUrl"
                  type="text"
                  spellcheck="false"
                  placeholder="https://api.vrchat.cloud/api/1"
                >
              </span>
            </label>
          </div>
          <footer class="modal-foot">
            <button class="ghost-button" @click="showSettingsDialog = false">{{ t('login.settings.close') }}</button>
            <button class="primary-button" @click="saveSettingsAndRestart">{{ t('login.settings.save_restart') }}</button>
          </footer>
        </div>
      </div>
    </transition>

    <!-- ============ 更新检查弹窗 ============ -->
    <transition name="vrcdog-modal">
      <div v-if="showUpdateDialog" class="modal-overlay" @click="showUpdateDialog = false">
        <div class="modal-panel" style="width: 500px;" @click.stop>
          <header class="modal-head">
            <h3>
              <ArrowDownToLine :size="17" />
              {{ t('login.update.title') }}
            </h3>
            <button :aria-label="t('a11y.close')" class="modal-close" @click="showUpdateDialog = false"><X :size="17" /></button>
          </header>
          <div class="modal-body">
            <!-- 稳定版 / 测试版 -->
            <div class="segmented">
              <button
                class="segment"
                :class="{ active: updateChannel === 'stable' }"
                @click="switchUpdateChannel('stable')"
              >{{ t('login.update.tab_stable') }}</button>
              <button
                class="segment"
                :class="{ active: updateChannel === 'beta' }"
                @click="switchUpdateChannel('beta')"
              >{{ t('login.update.tab_beta') }}</button>
            </div>

            <!-- 测试版警告 -->
            <div v-if="updateChannel === 'beta'" class="warning-box">
              <AlertCircle :size="17" />
              <div>
                <strong>{{ t('login.update.beta_warning_title') }}</strong>
                <small>{{ t('login.update.beta_warning_desc') }}</small>
              </div>
            </div>

            <!-- 版本下拉 -->
            <label class="theme-field">
              <span>{{ t('login.update.version') }}</span>
              <span class="select-wrap">
                <button
                  class="select-trigger vrcdog-version-trigger"
                  :disabled="updateLoading || updateChannelReleases.length === 0"
                  @click.stop="showUpdateVersionDropdown = !showUpdateVersionDropdown"
                >
                  <span>{{ updateLoading ? '...' : (updateSelectedTag || (updateChannelReleases.length === 0 ? t('login.update.no_releases') : '...')) }}</span>
                  <ChevronDown :size="16" :class="{ rotated: showUpdateVersionDropdown }" />
                </button>
                <transition name="vrcdog-menu">
                  <div v-if="showUpdateVersionDropdown" class="select-list vrcdog-version-list custom-scrollbar">
                    <button
                      v-for="rel in updateChannelReleases"
                      :key="rel.tag"
                      class="select-item"
                      :class="{ active: rel.tag === updateSelectedTag }"
                      @click="updateSelectedTag = rel.tag; showUpdateVersionDropdown = false"
                    >
                      <span>{{ rel.tag }}</span>
                      <Check v-if="rel.tag === updateSelectedTag" :size="14" />
                    </button>
                  </div>
                </transition>
              </span>
            </label>

            <!-- 是否最新 -->
            <div v-if="!updateLoading && updateChannel === 'stable'">
              <span v-if="isCurrentLatest" class="status-pill ok">
                <Check :size="12" />
                {{ t('login.update.up_to_date') }}
              </span>
              <span v-else class="status-pill new">
                {{ t('login.update.has_new', { version: updateChannelReleases[0]?.tag || '' }) }}
              </span>
            </div>
          </div>
          <footer class="modal-foot">
            <button class="ghost-button" @click="showUpdateDialog = false">{{ t('login.settings.close') }}</button>
            <button
              class="primary-button"
              :disabled="!updateSelectedTag || updateInstalling"
              @click="downloadUpdate"
            >{{ updateInstalling ? t('login.update.installing', { percent: updateInstallPercent }) : t('login.update.download') }}</button>
          </footer>
        </div>
      </div>
    </transition>
  </div>
</template>

<style scoped>
/* ══════════════════════════════════════════════════════════════
   精炼暖色系 · 登录 / 账号入口
   设计原则：小圆角 + 1px 细描边 + 层次阴影 + 克制动效
   ══════════════════════════════════════════════════════════════ */
.login-page {
  --surface: var(--theme-surface);
  --surface-hover: var(--theme-surface-hover);
  --border: var(--theme-border-soft);
  --border-strong: var(--theme-border-strong);
  --text: var(--theme-text-strong);
  --text-base: var(--theme-text);
  --muted: var(--theme-text-muted);
  --accent: var(--theme-primary);
  --accent-hover: var(--theme-primary-hover);
  --active-bg: var(--theme-active-bg);
  --glass: var(--theme-glass-effect, blur(24px) saturate(180%));

  position: relative;
  isolation: isolate;
  box-sizing: border-box;
  width: 100%;
  height: 100vh;
  display: grid;
  place-items: center;
  padding: 72px 28px 44px;
  overflow: auto;
  color: var(--text-base);
  background-color: var(--theme-bg-main);
}

/* 背景：四点柔和极光（替代原先的方格网格） */
.login-page::before {
  content: '';
  position: fixed;
  inset: 0;
  z-index: -2;
  pointer-events: none;
  background:
    radial-gradient(64% 60% at 6% -6%, color-mix(in srgb, var(--accent) 26%, transparent) 0%, transparent 72%),
    radial-gradient(56% 50% at 96% 2%, color-mix(in srgb, var(--accent) 14%, transparent) 0%, transparent 70%),
    radial-gradient(72% 64% at 92% 104%, color-mix(in srgb, var(--accent) 18%, transparent) 0%, transparent 74%),
    radial-gradient(54% 48% at 6% 98%, color-mix(in srgb, var(--accent) 11%, transparent) 0%, transparent 70%);
}

/* 背景：中心提亮，把视线收拢到内容区，同时压住光晕让面板更清晰 */
.login-page::after {
  content: '';
  position: fixed;
  inset: 0;
  z-index: -1;
  pointer-events: none;
  background: radial-gradient(78% 66% at 50% 46%,
    color-mix(in srgb, var(--theme-bg-main) 58%, transparent) 0%,
    transparent 74%);
}

/* ── 工具栏 ─────────────────────────────────────────────────── */
.login-toolbar {
  position: fixed;
  z-index: 60;
  top: 18px;
  left: 20px;
  display: flex;
  align-items: center;
  gap: 8px;
}

.tool-btn {
  position: relative;
  width: 38px;
  height: 38px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border: 1px solid var(--border-strong);
  border-radius: 9px;
  color: var(--text-base);
  background: color-mix(in srgb, var(--surface-hover) 86%, transparent);
  box-shadow: 0 6px 18px color-mix(in srgb, var(--text) 8%, transparent), inset 0 1px rgba(255, 255, 255, 0.7);
  backdrop-filter: var(--glass);
  -webkit-backdrop-filter: var(--glass);
  cursor: pointer;
  transition: border-color 150ms ease, background-color 150ms ease, transform 150ms ease;
}

.tool-btn:hover {
  border-color: color-mix(in srgb, var(--accent) 46%, var(--border));
  background: var(--surface-hover);
}

.tool-btn:active { transform: scale(0.96); }

.tool-btn:focus-visible,
.lang-item:focus-visible,
.primary-button:focus-visible,
.ghost-button:focus-visible,
.segment:focus-visible,
.account-card:focus-visible,
.account-delete:focus-visible,
.select-trigger:focus-visible,
.select-item:focus-visible,
.modal-close:focus-visible {
  outline: 3px solid color-mix(in srgb, var(--accent) 22%, transparent);
  outline-offset: 2px;
}

.tool-dot {
  position: absolute;
  top: 7px;
  right: 7px;
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: #ef4444;
  box-shadow: 0 0 0 2px var(--theme-bg-main);
}

.tool-dot-ping {
  animation: tool-ping 1.7s cubic-bezier(0, 0, 0.2, 1) infinite;
}

@keyframes tool-ping {
  0% { transform: scale(1); opacity: 0.65; }
  75%, 100% { transform: scale(2.2); opacity: 0; }
}

/* ── 语言菜单 ───────────────────────────────────────────────── */
.action-menu { position: relative; }

.lang-menu {
  position: absolute;
  z-index: 80;
  top: calc(100% + 9px);
  left: 0;
  box-sizing: border-box;
  width: 268px;
  max-height: 340px;
  overflow-y: auto;
  padding: 6px;
  border: 1px solid var(--border-strong);
  border-radius: 10px;
  background: color-mix(in srgb, var(--surface-hover) 94%, var(--theme-bg-main));
  box-shadow: 0 18px 46px color-mix(in srgb, var(--text) 16%, transparent), inset 0 1px rgba(255, 255, 255, 0.7);
  backdrop-filter: var(--glass);
  -webkit-backdrop-filter: var(--glass);
}

.lang-item {
  width: 100%;
  min-height: 34px;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 0 10px;
  border: 1px solid transparent;
  border-radius: 7px;
  color: var(--text-base);
  background: transparent;
  font-size: 12.5px;
  font-weight: 600;
  text-align: left;
  cursor: pointer;
  transition: background-color 130ms ease, border-color 130ms ease, color 130ms ease;
}

.lang-item:hover {
  border-color: var(--border);
  background: var(--surface-hover);
}

.lang-item.active {
  border-color: color-mix(in srgb, var(--accent) 30%, var(--border));
  color: var(--accent);
  background: var(--active-bg);
}

.lang-check { flex: 0 0 auto; color: var(--accent); }

.lang-label {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* ── 主舞台 ─────────────────────────────────────────────────── */
.login-stage {
  position: relative;
  z-index: 1;
  width: min(880px, 100%);
  display: grid;
  grid-template-columns: minmax(0, 1.3fr) minmax(0, 1fr);
  align-items: stretch;
  gap: 20px;
}

.login-stage.single {
  grid-template-columns: minmax(0, 440px);
  justify-content: center;
}

.panel {
  box-sizing: border-box;
  display: flex;
  flex-direction: column;
  padding: 28px;
  border: 1px solid var(--border-strong);
  border-radius: 16px;
  background:
    linear-gradient(150deg, color-mix(in srgb, var(--accent) 4%, transparent) 0%, transparent 44%),
    color-mix(in srgb, var(--surface) 90%, transparent);
  box-shadow:
    0 26px 70px color-mix(in srgb, var(--text) 13%, transparent),
    0 3px 10px color-mix(in srgb, var(--text) 5%, transparent),
    inset 0 1px rgba(255, 255, 255, 0.7);
  backdrop-filter: var(--glass);
  -webkit-backdrop-filter: var(--glass);
}

/* ── 品牌头部 ───────────────────────────────────────────────── */
.brand-head {
  display: flex;
  align-items: center;
  gap: 13px;
  padding-bottom: 20px;
  margin-bottom: 20px;
  border-bottom: 1px solid var(--border);
}

.brand-logo {
  width: 46px;
  height: 46px;
  flex: 0 0 46px;
  border-radius: 11px;
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
  font-size: 21px;
  font-weight: 800;
  line-height: 1.2;
  letter-spacing: -0.01em;
}

.brand-copy p {
  margin: 0;
  color: var(--muted);
  font-size: 12px;
  font-weight: 600;
}

/* ── 表单 ───────────────────────────────────────────────────── */
.panel-body {
  display: grid;
  gap: 16px;
}

.theme-field {
  display: grid;
  gap: 7px;
  color: var(--text-base);
  font-size: 12px;
  font-weight: 700;
}

.input-shell {
  box-sizing: border-box;
  min-height: 46px;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 0 13px;
  border: 1px solid var(--border);
  border-radius: 10px;
  background: color-mix(in srgb, var(--surface-hover) 62%, transparent);
  transition: border-color 150ms ease, background-color 150ms ease, box-shadow 150ms ease;
}

.input-shell:focus-within {
  border-color: var(--accent);
  background: color-mix(in srgb, var(--surface-hover) 88%, transparent);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent) 15%, transparent);
}

.input-shell > svg {
  flex: 0 0 auto;
  color: var(--muted);
  transition: color 150ms ease;
}

.input-shell:focus-within > svg { color: var(--accent); }

.input-shell input {
  width: 100%;
  min-width: 0;
  padding: 0;
  border: 0;
  outline: 0;
  color: var(--text);
  background: transparent;
  font-size: 13.5px;
  font-weight: 600;
}

.input-shell input::placeholder {
  color: var(--muted);
  font-weight: 500;
  opacity: 0.72;
}

.code-shell input {
  text-align: center;
  font-family: ui-monospace, 'JetBrains Mono', 'Fira Code', monospace;
  font-size: 20px;
  font-weight: 700;
  letter-spacing: 0.4em;
  text-indent: 0.4em;
}

.code-shell input::placeholder {
  font-size: 13px;
  font-weight: 500;
  letter-spacing: normal;
  text-indent: 0;
  opacity: 0.6;
}

.checkbox-row {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  color: var(--text-base);
  font-size: 12.5px;
  font-weight: 600;
  cursor: pointer;
  user-select: none;
}

.checkbox-row input {
  width: 15px;
  height: 15px;
  margin: 0;
  border-radius: 4px;
  accent-color: var(--accent);
  cursor: pointer;
}

/* ── 按钮 ───────────────────────────────────────────────────── */
.primary-button {
  min-height: 46px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 9px;
  padding: 0 18px;
  border: 1px solid transparent;
  border-radius: 10px;
  color: #fff;
  background: var(--accent);
  font-size: 13.5px;
  font-weight: 750;
  letter-spacing: 0.01em;
  cursor: pointer;
  box-shadow: 0 10px 22px color-mix(in srgb, var(--accent) 26%, transparent);
  transition: background-color 160ms ease, transform 160ms ease, box-shadow 160ms ease, opacity 160ms ease;
}

.primary-button:hover:not(:disabled) {
  background: var(--accent-hover);
  transform: translateY(-1px);
  box-shadow: 0 14px 26px color-mix(in srgb, var(--accent) 30%, transparent);
}

.primary-button:active:not(:disabled) { transform: translateY(0) scale(0.99); }

.primary-button:disabled {
  opacity: 0.55;
  cursor: wait;
  box-shadow: none;
}

.ghost-button {
  min-height: 42px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 0 16px;
  border: 1px solid var(--border);
  border-radius: 10px;
  color: var(--text-base);
  background: color-mix(in srgb, var(--surface-hover) 55%, transparent);
  font-size: 12.5px;
  font-weight: 700;
  cursor: pointer;
  transition: background-color 150ms ease, border-color 150ms ease, transform 150ms ease;
}

.ghost-button:hover {
  border-color: var(--border-strong);
  background: var(--surface-hover);
}

.ghost-button:active { transform: scale(0.985); }

.panel-body > .primary-button,
.panel-body > .ghost-button { width: 100%; }

.spin { animation: spin 0.8s linear infinite; }

@keyframes spin { to { transform: rotate(360deg); } }

/* ── 2FA ────────────────────────────────────────────────────── */
.notice {
  display: flex;
  align-items: flex-start;
  gap: 11px;
  padding: 13px 14px;
  border: 1px solid color-mix(in srgb, var(--accent) 26%, var(--border));
  border-radius: 11px;
  background: color-mix(in srgb, var(--active-bg) 62%, transparent);
}

.notice-icon {
  flex: 0 0 auto;
  margin-top: 1px;
  color: var(--accent);
}

.notice strong {
  display: block;
  color: var(--text);
  font-size: 13px;
}

.notice small {
  display: block;
  margin-top: 2px;
  color: var(--muted);
  font-size: 11.5px;
  font-weight: 600;
  line-height: 1.45;
}

.segmented {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(0, 1fr));
  gap: 5px;
  padding: 5px;
  border: 1px solid var(--border);
  border-radius: 10px;
  background: color-mix(in srgb, var(--surface-hover) 52%, transparent);
}

.segment {
  min-height: 38px;
  border: 0;
  border-radius: 7px;
  color: var(--muted);
  background: transparent;
  font-size: 12.5px;
  font-weight: 700;
  cursor: pointer;
  transition: background-color 150ms ease, color 150ms ease, box-shadow 150ms ease;
}

.segment:hover { color: var(--text-base); }

.segment.active {
  color: #fff;
  background: var(--accent);
  box-shadow: 0 5px 14px color-mix(in srgb, var(--accent) 24%, transparent);
}

/* ── 错误提示 ───────────────────────────────────────────────── */
.error-message {
  display: flex;
  align-items: flex-start;
  gap: 7px;
  margin: 16px 0 0;
  padding: 10px 12px;
  border: 1px solid color-mix(in srgb, #dc2626 28%, transparent);
  border-radius: 10px;
  color: #c62828;
  background: color-mix(in srgb, #dc2626 7%, transparent);
  font-size: 12px;
  font-weight: 700;
  line-height: 1.45;
}

.error-message svg {
  flex: 0 0 auto;
  margin-top: 1px;
}

/* ── 已保存账号 ─────────────────────────────────────────────── */
.panel-accounts { padding: 24px; }

.panel-head {
  display: flex;
  align-items: center;
  gap: 8px;
  padding-bottom: 15px;
  margin-bottom: 14px;
  border-bottom: 1px solid var(--border);
}

.panel-head > svg { color: var(--accent); }

.panel-head h2 {
  margin: 0;
  color: var(--text);
  font-size: 13px;
  font-weight: 750;
  letter-spacing: 0.01em;
}

.account-list {
  flex: 1;
  min-height: 0;
  display: grid;
  align-content: start;
  gap: 8px;
  padding-right: 3px;
  overflow-y: auto;
}

.account-card {
  position: relative;
  display: flex;
  align-items: center;
  gap: 11px;
  padding: 10px 11px;
  border: 1px solid var(--border);
  border-radius: 11px;
  background: color-mix(in srgb, var(--surface-hover) 55%, transparent);
  cursor: pointer;
  transition: border-color 170ms ease, background-color 170ms ease, transform 170ms ease, box-shadow 170ms ease;
}

.account-card:hover {
  border-color: color-mix(in srgb, var(--accent) 52%, transparent);
  background: color-mix(in srgb, var(--accent) 6%, var(--surface-hover));
  transform: translateY(-1px);
  box-shadow: 0 8px 20px color-mix(in srgb, var(--accent) 15%, transparent);
}

.account-card:active { transform: translateY(0) scale(0.995); }

.account-avatar {
  width: 38px;
  height: 38px;
  flex: 0 0 38px;
  border-radius: 10px;
  object-fit: cover;
  border: 1px solid var(--border-strong);
  background: color-mix(in srgb, var(--text) 8%, transparent);
}

.account-avatar-fallback {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  color: var(--muted);
}

.account-copy {
  flex: 1;
  min-width: 0;
  display: grid;
  gap: 2px;
}

.account-copy strong,
.account-copy small {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.account-copy strong {
  color: var(--text);
  font-size: 13px;
  font-weight: 700;
}

.account-copy small {
  color: var(--muted);
  font-size: 11px;
  font-weight: 500;
}

.account-delete {
  width: 30px;
  height: 30px;
  flex: 0 0 30px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border: 1px solid transparent;
  border-radius: 8px;
  color: var(--muted);
  background: transparent;
  opacity: 0;
  cursor: pointer;
  transition: opacity 150ms ease, background-color 150ms ease, color 150ms ease, border-color 150ms ease;
}

.account-card:hover .account-delete,
.account-card:focus-within .account-delete { opacity: 1; }

.account-delete:hover {
  border-color: color-mix(in srgb, #dc2626 26%, transparent);
  color: #dc2626;
  background: color-mix(in srgb, #dc2626 10%, transparent);
}

.account-empty {
  flex: 1;
  display: grid;
  place-items: center;
  padding: 18px 6px;
  text-align: center;
}

.account-empty p {
  max-width: 210px;
  margin: 0;
  color: var(--muted);
  font-size: 11.5px;
  font-weight: 600;
  line-height: 1.65;
}

/* ── 版本号 ─────────────────────────────────────────────────── */
.app-version {
  position: fixed;
  z-index: 50;
  bottom: 16px;
  left: 20px;
  padding: 3px 8px;
  border: 1px solid var(--border);
  border-radius: 7px;
  color: var(--muted);
  background: color-mix(in srgb, var(--surface-hover) 70%, transparent);
  font-family: ui-monospace, 'JetBrains Mono', monospace;
  font-size: 11px;
  font-weight: 700;
}

/* ── 弹窗 ───────────────────────────────────────────────────── */
.modal-overlay {
  position: fixed;
  inset: 0;
  z-index: 100;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 24px;
  background: color-mix(in srgb, var(--theme-bg-main) 58%, rgba(0, 0, 0, 0.5));
  backdrop-filter: blur(6px);
  -webkit-backdrop-filter: blur(6px);
}

.modal-panel {
  box-sizing: border-box;
  max-width: 100%;
  max-height: 90vh;
  display: flex;
  flex-direction: column;
  border: 1px solid var(--border-strong);
  border-radius: 16px;
  background: var(--theme-bg-main);
  box-shadow: 0 32px 90px rgba(0, 0, 0, 0.28);
}

.modal-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 20px 22px 14px;
}

.modal-head h3 {
  display: flex;
  align-items: center;
  gap: 9px;
  margin: 0;
  color: var(--text);
  font-size: 15px;
  font-weight: 800;
}

.modal-head h3 > svg { color: var(--accent); }

.modal-close {
  width: 32px;
  height: 32px;
  flex: 0 0 32px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border: 1px solid transparent;
  border-radius: 8px;
  color: var(--muted);
  background: transparent;
  cursor: pointer;
  transition: background-color 150ms ease, color 150ms ease;
}

.modal-close:hover {
  color: var(--text);
  background: var(--surface-hover);
}

.modal-body {
  display: grid;
  gap: 14px;
  padding: 0 22px 4px;
  overflow-y: auto;
}

.modal-foot {
  display: flex;
  justify-content: flex-end;
  gap: 9px;
  padding: 16px 22px 20px;
}

.warning-box {
  display: flex;
  gap: 11px;
  padding: 12px 13px;
  border: 1px solid color-mix(in srgb, #ef4444 30%, transparent);
  border-radius: 11px;
  background: color-mix(in srgb, #ef4444 7%, transparent);
}

.warning-box > svg {
  flex: 0 0 auto;
  margin-top: 1px;
  color: #ef4444;
}

.warning-box strong {
  display: block;
  color: #dc2626;
  font-size: 12.5px;
}

.warning-box small {
  display: block;
  margin-top: 2px;
  color: color-mix(in srgb, #dc2626 88%, transparent);
  font-size: 11.5px;
  font-weight: 600;
  line-height: 1.5;
}

/* ── 版本下拉 ───────────────────────────────────────────────── */
.select-wrap { position: relative; display: block; }

.select-trigger {
  box-sizing: border-box;
  width: 100%;
  min-height: 46px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  padding: 0 13px;
  border: 1px solid var(--border);
  border-radius: 10px;
  color: var(--text);
  background: color-mix(in srgb, var(--surface-hover) 62%, transparent);
  font-size: 13px;
  font-weight: 650;
  cursor: pointer;
  transition: border-color 150ms ease, background-color 150ms ease;
}

.select-trigger:hover:not(:disabled) {
  border-color: color-mix(in srgb, var(--accent) 45%, var(--border));
}

.select-trigger:disabled { opacity: 0.55; cursor: not-allowed; }

.select-trigger svg { transition: transform 200ms ease; }

.select-trigger svg.rotated { transform: rotate(180deg); }

.select-list {
  position: absolute;
  z-index: 90;
  top: calc(100% + 8px);
  left: 0;
  right: 0;
  box-sizing: border-box;
  max-height: 216px;
  overflow-y: auto;
  padding: 6px;
  border: 1px solid var(--border-strong);
  border-radius: 12px;
  background: var(--theme-bg-main);
  box-shadow: 0 20px 50px color-mix(in srgb, var(--text) 18%, transparent);
}

.select-item {
  width: 100%;
  min-height: 36px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  padding: 0 11px;
  border: 1px solid transparent;
  border-radius: 8px;
  color: var(--text-base);
  background: transparent;
  font-size: 12.5px;
  font-weight: 650;
  text-align: left;
  cursor: pointer;
  transition: background-color 130ms ease;
}

.select-item:hover { background: var(--surface-hover); }

.select-item.active {
  color: var(--accent);
  background: var(--active-bg);
}

.status-pill {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 5px 11px;
  border-radius: 999px;
  font-size: 11.5px;
  font-weight: 700;
}

.status-pill.ok {
  border: 1px solid color-mix(in srgb, #059669 24%, transparent);
  color: #059669;
  background: color-mix(in srgb, #059669 10%, transparent);
}

.status-pill.new {
  border: 1px solid color-mix(in srgb, #d97706 26%, transparent);
  color: #b45309;
  background: color-mix(in srgb, #d97706 12%, transparent);
}

/* ── 过渡 ───────────────────────────────────────────────────── */
.vrcdog-modal-enter-active,
.vrcdog-modal-leave-active { transition: opacity 0.18s ease; }

.vrcdog-modal-enter-from,
.vrcdog-modal-leave-to { opacity: 0; }

.vrcdog-modal-enter-active .modal-panel,
.vrcdog-modal-leave-active .modal-panel { transition: transform 0.24s cubic-bezier(0.16, 1, 0.3, 1); }

.vrcdog-modal-enter-from .modal-panel,
.vrcdog-modal-leave-to .modal-panel { transform: translateY(8px) scale(0.985); }

.vrcdog-menu-enter-active,
.vrcdog-menu-leave-active { transition: opacity 0.15s ease, transform 0.18s cubic-bezier(0.16, 1, 0.3, 1); }

.vrcdog-menu-enter-from,
.vrcdog-menu-leave-to { opacity: 0; transform: translateY(-6px) scale(0.985); }

/* ── 响应式 ─────────────────────────────────────────────────── */
@media (max-width: 820px) {
  .login-page { place-items: start center; }

  .login-stage {
    grid-template-columns: minmax(0, 460px);
    justify-content: center;
  }

  .panel-accounts { min-height: 176px; }
}

@media (max-height: 720px) and (min-width: 821px) {
  .login-page { place-items: start center; }
}

@media (max-width: 480px) {
  .login-page { padding: 70px 12px 26px; }
  .panel { padding: 20px; }
  .panel-accounts { padding: 18px; }
  .brand-copy h1 { font-size: 19px; }
  .modal-overlay { padding: 14px; }
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

@media (prefers-reduced-transparency: reduce) {
  .panel,
  .tool-btn,
  .lang-menu {
    background: var(--theme-bg-main);
    backdrop-filter: none;
    -webkit-backdrop-filter: none;
  }
}
</style>
