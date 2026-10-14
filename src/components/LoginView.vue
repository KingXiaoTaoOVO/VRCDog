<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue';
import { VrcApi, DbApi } from "../api";
import { Bone, Key, User, Loader2, Globe, ArrowLeft, Trash2, Settings, ArrowDownToLine, Languages, Check, X, ChevronDown, AlertCircle } from 'lucide-vue-next';
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
  <div class="flex items-center justify-center h-screen bg-[var(--theme-bg-main)] relative overflow-hidden">
    <!-- VrcDog 风格左上角工具栏 -->
    <div class="fixed top-4 left-4 z-50 flex gap-2 items-center">
      <!-- 设置 -->
      <button
        class="vrcdog-toolbar-btn cartoon-pill-btn transition-transform hover:scale-105 active:scale-95"
        :title="t('login.toolbar.settings')"
        @click="openSettings"
      >
        <Settings :size="18" />
      </button>
      <!-- 检查更新 -->
      <button
        class="vrcdog-toolbar-btn cartoon-pill-btn relative transition-transform hover:scale-105 active:scale-95"
        :title="t('login.toolbar.check_update')"
        @click="openUpdateDialog"
      >
        <ArrowDownToLine :size="18" />
        <span
          v-if="updateAvailable"
          class="absolute top-1.5 right-1.5 w-2.5 h-2.5 rounded-full bg-red-500 ring-2 ring-[var(--theme-bg-main)] animate-ping"
          :title="t('login.toolbar.update_available', { version: updateAvailableTag })"
        />
        <span
          v-if="updateAvailable"
          class="absolute top-1.5 right-1.5 w-2.5 h-2.5 rounded-full bg-red-500 ring-2 ring-[var(--theme-bg-main)]"
        />
      </button>
      <!-- 语言 -->
      <div class="relative vrcdog-lang-wrap">
        <button
          class="vrcdog-toolbar-btn cartoon-pill-btn transition-transform hover:scale-105 active:scale-95"
          :title="t('login.toolbar.language')"
          @click.stop="showLangMenu = !showLangMenu"
        >
          <Languages :size="18" />
        </button>
        <transition name="vrcdog-menu">
          <div v-if="showLangMenu" class="vrcdog-lang-menu rounded-2xl border-2 border-[var(--theme-border-soft)] shadow-2xl p-1.5 custom-scrollbar">
            <button
              v-for="lang in languages"
              :key="lang.code"
              class="vrcdog-lang-item rounded-xl transition-all hover:translate-x-1"
              :class="{ active: preferredLangCode === lang.code }"
              @click="selectLanguage(lang.code)"
            >
              <Check
                :size="14"
                class="vrcdog-lang-check"
                :style="{ visibility: preferredLangCode === lang.code ? 'visible' : 'hidden' }"
              />
              <span class="vrcdog-lang-label">{{ lang.label }} ({{ lang.code }})</span>
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
      <div class="cartoon-card bg-[var(--theme-surface)]/95 backdrop-blur-xl p-8 rounded-[28px] shadow-2xl w-full max-w-md border-2 border-[var(--theme-border-soft)] flex flex-col">
        <div class="text-center mb-8">
          <h1 class="text-3xl font-extrabold text-[var(--theme-text)] mb-2 flex items-center justify-center gap-2 tracking-tight">
            <Bone class="animate-bounce text-[var(--theme-primary)]" :size="30" /> {{ t('login.title') }}
          </h1>
          <p class="text-zinc-400 text-sm">
            {{ t('login.subtitle') }}
          </p>
        </div>

        <div v-if="!show2FA" class="space-y-5">
          <div>
            <label class="block text-sm font-bold text-text mb-1 flex items-center gap-1.5">
              <User :size="16" class="text-[var(--theme-primary)]" /> {{ t('login.username') }}
            </label>
            <input
              v-model="username"
              type="text"
              :placeholder="t('login.username')"
              class="cartoon-input w-full px-4 py-3 rounded-2xl border-2 border-[var(--theme-border-soft)] focus:border-[var(--theme-primary)] focus:ring-4 focus:ring-[var(--theme-primary)]/15 outline-none transition-all bg-[var(--theme-surface)] text-[var(--theme-text)] placeholder-zinc-500 font-medium"
            >
          </div>
          <div>
            <label class="block text-sm font-bold text-text mb-1 flex items-center gap-1.5">
              <Key :size="16" class="text-[var(--theme-primary)]" /> {{ t('login.password') }}
            </label>
            <input
              v-model="password"
              type="password"
              :placeholder="t('login.password')"
              class="cartoon-input w-full px-4 py-3 rounded-2xl border-2 border-[var(--theme-border-soft)] focus:border-[var(--theme-primary)] focus:ring-4 focus:ring-[var(--theme-primary)]/15 outline-none transition-all bg-[var(--theme-surface)] text-[var(--theme-text)] placeholder-zinc-500 font-medium"
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

          <button
            :disabled="loading"
            class="cartoon-pill-btn w-full mt-4 bg-primary hover:brightness-110 active:brightness-95 text-white font-extrabold py-3.5 px-4 rounded-2xl shadow-[0_8px_20px_rgba(249,115,22,0.35)] transition-all flex items-center justify-center gap-2 disabled:opacity-50 disabled:cursor-not-allowed hover:scale-[1.02] active:scale-[0.98]"
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
              {{ t('login.2fa_desc', { method: twoFactorMethodLabel(selectedTwoFactorMethod) }) }}
            </p>
          </div>
          <div v-if="twoFactorMethods.length > 1" class="grid grid-cols-2 gap-2" role="radiogroup" :aria-label="t('login.2fa_choose_method')">
            <button
              v-for="method in twoFactorMethods"
              :key="method"
              type="button"
              role="radio"
              :aria-checked="selectedTwoFactorMethod === method"
              class="cartoon-pill-btn rounded-xl border-2 px-3 py-2 text-sm font-bold transition-all hover:scale-105 active:scale-95"
              :class="selectedTwoFactorMethod === method
                ? 'border-[var(--theme-primary)] bg-[var(--theme-primary)] text-white shadow-md'
                : 'border-[var(--theme-border-soft)] bg-[var(--theme-surface)] text-[var(--theme-text-muted)] hover:bg-[var(--theme-surface-hover)]'"
              @click="selectedTwoFactorMethod = method; twoFactorCode = ''; errorMsg = ''"
            >
              {{ twoFactorMethodLabel(method) }}
            </button>
          </div>
          <div>
            <input
              v-model="twoFactorCode"
              type="text"
              inputmode="numeric"
              autocomplete="one-time-code"
              maxlength="12"
              :aria-label="t('login.2fa_placeholder')"
              :placeholder="t('login.2fa_placeholder')"
              class="cartoon-input w-full px-4 py-4 rounded-2xl border-2 border-[var(--theme-border-soft)] focus:border-[var(--theme-primary)] focus:ring-4 focus:ring-[var(--theme-primary)]/15 outline-none transition-all bg-[var(--theme-surface)] text-[var(--theme-text)] placeholder-zinc-600 text-center text-2xl tracking-[0.5em] font-mono font-extrabold"
              @keyup.enter="handle2FA"
            >
          </div>
          <button
            :disabled="loading"
            class="cartoon-pill-btn w-full mt-4 bg-emerald-500 hover:brightness-110 active:brightness-95 text-white font-extrabold py-3.5 px-4 rounded-2xl shadow-[0_8px_20px_rgba(16,185,129,0.35)] transition-all flex items-center justify-center gap-2 disabled:opacity-50 disabled:cursor-not-allowed hover:scale-[1.02] active:scale-[0.98]"
            @click="handle2FA"
          >
            <Loader2 v-if="loading" class="animate-spin" :size="20" />
            {{ loading ? t('login.btn_submitting') : t('login.btn_submit_code') }}
          </button>
          <div class="mt-6 flex justify-center">
            <button
              @click="show2FA = false"
              class="cartoon-pill-btn group relative px-6 py-2.5 rounded-2xl flex items-center justify-center gap-2 font-bold transition-all overflow-hidden bg-[var(--theme-surface)] hover:bg-[var(--theme-surface-hover)] border-2 border-[var(--theme-border-soft)] hover:scale-105 active:scale-95"
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
        class="cartoon-card bg-[var(--theme-surface)]/95 backdrop-blur-xl p-6 rounded-[28px] shadow-2xl w-full max-w-xs border-2 border-[var(--theme-border-soft)] flex flex-col"
      >
        <h2 class="text-center text-lg font-extrabold text-[var(--theme-text)] mb-4 flex items-center justify-center gap-1.5">
          <User :size="18" class="text-[var(--theme-primary)]" />
          {{ t('login.saved_accounts') }}
        </h2>
        <div v-if="savedAccounts.length > 0" class="flex-1 overflow-y-auto custom-scrollbar space-y-2 pr-1">
          <div
            v-for="account in savedAccounts"
            :key="account.userId"
            class="vrcdog-saved-account-card flex items-center gap-3 p-3 rounded-2xl cursor-pointer bg-[var(--theme-surface)] hover:bg-[var(--theme-surface-hover)] transition-all duration-200 border-2 border-[var(--theme-border-soft)] hover:border-[var(--theme-primary)]/50 hover:scale-[1.02] active:scale-[0.98] group shadow-sm"
            @click="loginWithSavedAccount(account)"
          >
            <img
              v-if="account.avatarUrl"
              :src="account.avatarUrl"
              class="w-10 h-10 rounded-2xl object-cover flex-shrink-0 bg-zinc-700 ring-2 ring-[var(--theme-border-soft)]"
              @error="($event.target as HTMLImageElement).style.display = 'none'"
            >
            <div v-else class="w-10 h-10 rounded-2xl bg-zinc-700 flex items-center justify-center flex-shrink-0 ring-2 ring-[var(--theme-border-soft)]">
              <User :size="18" class="text-zinc-400" />
            </div>
            <div class="flex-1 min-w-0">
              <p class="text-sm font-bold text-[var(--theme-text)] truncate">{{ account.displayName }}</p>
              <p class="text-xs text-[var(--theme-text-muted)] truncate">{{ account.username }}</p>
            </div>
            <button
              class="opacity-0 group-hover:opacity-100 transition-all p-1.5 rounded-xl hover:bg-red-500/20 text-zinc-500 hover:text-red-400 hover:scale-110 active:scale-95"
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
    <transition name="vrcdog-modal">
      <div v-if="showSettingsDialog" class="vrcdog-dialog-overlay" @click="showSettingsDialog = false">
        <div class="vrcdog-dialog-panel cartoon-card" style="width: 460px;" @click.stop>
          <div class="vrcdog-dialog-header">
            <h3 class="vrcdog-dialog-title flex items-center gap-2 font-extrabold">
              <Settings :size="18" class="text-[var(--theme-primary)]" />
              {{ t('login.settings.title') }}
            </h3>
            <button class="vrcdog-dialog-close transition-transform hover:scale-110 active:scale-95" @click="showSettingsDialog = false"><X :size="18" /></button>
          </div>
          <div class="vrcdog-dialog-body">
            <label class="vrcdog-field-label">{{ t('login.settings.proxy_label') }}</label>
            <input
              v-model="settingsProxy"
              type="text"
              :placeholder="t('login.settings.proxy_placeholder')"
              class="vrcdog-input"
            >
            <label class="vrcdog-checkbox-row mt-4">
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
              class="vrcdog-input mt-2"
            >
          </div>
          <div class="vrcdog-dialog-footer">
            <button class="vrcdog-btn-primary cartoon-pill-btn" @click="saveSettingsAndRestart">{{ t('login.settings.save_restart') }}</button>
            <button class="vrcdog-btn-secondary cartoon-pill-btn" @click="showSettingsDialog = false">{{ t('login.settings.close') }}</button>
          </div>
        </div>
      </div>
    </transition>

    <!-- ============ 更新检查弹窗 ============ -->
    <transition name="vrcdog-modal">
      <div v-if="showUpdateDialog" class="vrcdog-dialog-overlay" @click="showUpdateDialog = false">
        <div class="vrcdog-dialog-panel vrcdog-update-panel cartoon-card" style="width: 520px;" @click.stop>
          <div class="vrcdog-dialog-header">
            <h3 class="vrcdog-dialog-title flex items-center gap-2 font-extrabold">
              <span class="w-3 h-3 rounded-full bg-[var(--theme-primary)]" />
              {{ t('login.update.title') }}
            </h3>
            <button class="vrcdog-dialog-close transition-transform hover:scale-110 active:scale-95" @click="showUpdateDialog = false"><X :size="18" /></button>
          </div>
          <div class="vrcdog-dialog-body vrcdog-update-dialog-body">
            <!-- 稳定版 / 测试版 切换 -->
            <div class="vrcdog-tab-group">
              <button
                class="vrcdog-tab-pill transition-all"
                :class="{ active: updateChannel === 'stable' }"
                @click="switchUpdateChannel('stable')"
              >{{ t('login.update.tab_stable') }}</button>
              <button
                class="vrcdog-tab-pill transition-all"
                :class="{ active: updateChannel === 'beta' }"
                @click="switchUpdateChannel('beta')"
              >{{ t('login.update.tab_beta') }}</button>
            </div>

            <!-- 测试版警告 -->
            <div v-if="updateChannel === 'beta'" class="vrcdog-warning-box">
              <AlertCircle :size="18" class="vrcdog-warning-icon" />
              <div>
                <div class="vrcdog-warning-title">{{ t('login.update.beta_warning_title') }}</div>
                <div class="vrcdog-warning-desc">{{ t('login.update.beta_warning_desc') }}</div>
              </div>
            </div>

            <!-- 版本下拉 -->
            <label class="vrcdog-field-label mt-4">{{ t('login.update.version') }}</label>
            <div class="relative">
              <button
                class="vrcdog-version-trigger transition-all hover:border-[var(--theme-primary)]"
                :disabled="updateLoading || updateChannelReleases.length === 0"
                @click.stop="showUpdateVersionDropdown = !showUpdateVersionDropdown"
              >
                <span>{{ updateLoading ? '...' : (updateSelectedTag || (updateChannelReleases.length === 0 ? t('login.update.no_releases') : '...')) }}</span>
                <ChevronDown :size="16" class="transition-transform duration-200" :class="{ 'rotate-180': showUpdateVersionDropdown }" />
              </button>
              <transition name="vrcdog-menu">
                <div v-if="showUpdateVersionDropdown" class="vrcdog-version-list custom-scrollbar">
                  <button
                    v-for="rel in updateChannelReleases"
                    :key="rel.tag"
                    class="vrcdog-version-item transition-all"
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
            <div v-if="!updateLoading && updateChannel === 'stable'" class="vrcdog-update-status flex items-center gap-2 mt-4">
              <span v-if="isCurrentLatest" class="inline-flex items-center gap-1.5 px-3 py-1 rounded-full text-xs font-semibold bg-emerald-500/10 text-emerald-500 border border-emerald-500/20">
                <Check :size="12" />
                {{ t('login.update.up_to_date') }}
              </span>
              <span v-else class="inline-flex items-center gap-1.5 px-3 py-1 rounded-full text-xs font-semibold bg-amber-500/10 text-amber-500 border border-amber-500/20">
                {{ t('login.update.has_new', { version: updateChannelReleases[0]?.tag || '' }) }}
              </span>
            </div>
          </div>
          <div class="vrcdog-dialog-footer mt-auto">
            <button
              class="vrcdog-btn-primary cartoon-pill-btn"
              :disabled="!updateSelectedTag || updateInstalling"
              @click="downloadUpdate"
            >{{ updateInstalling ? t('login.update.installing', { percent: updateInstallPercent }) : t('login.update.download') }}</button>
          </div>
        </div>
      </div>
    </transition>
  </div>
</template>

<style scoped>
/* ── VrcDog-style toolbar ─────────────────────────────────────── */
.vrcdog-toolbar-btn {
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
.vrcdog-toolbar-btn:hover {
  background: var(--theme-surface-hover);
  color: var(--theme-text);
}

/* ── Language menu ─────────────────────────────────────────── */
.vrcdog-lang-wrap { position: relative; }
.vrcdog-lang-menu {
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
.vrcdog-lang-item {
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
.vrcdog-lang-item:hover { background: var(--theme-surface-hover); }
.vrcdog-lang-check { flex-shrink: 0; color: var(--theme-primary); }
.vrcdog-lang-label {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* ── Dialog overlay & panel ───────────────────────────────── */
.vrcdog-dialog-overlay {
  position: fixed;
  inset: 0;
  z-index: 100;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.55);
  backdrop-filter: blur(4px);
}
.vrcdog-dialog-panel {
  background: var(--theme-surface);
  border: 2px solid var(--theme-border-soft);
  border-radius: 24px;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5);
  display: flex;
  flex-direction: column;
  max-height: 90vh;
}
.vrcdog-update-panel {
  min-height: 440px;
}
.vrcdog-dialog-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 18px 22px 12px 22px;
}
.vrcdog-dialog-title {
  font-size: 18px;
  font-weight: 700;
  color: var(--theme-text);
  margin: 0;
}
.vrcdog-dialog-close {
  width: 32px; height: 32px;
  border-radius: 9999px;
  background: transparent;
  color: var(--theme-text-muted);
  border: 1px solid transparent;
  cursor: pointer;
  display: inline-flex;
  align-items: center; justify-content: center;
}
.vrcdog-dialog-close:hover { background: var(--theme-surface-hover); color: var(--theme-text); }
.vrcdog-dialog-body {
  padding: 4px 22px 16px 22px;
  overflow-y: auto;
}
.vrcdog-update-dialog-body {
  overflow: visible !important;
  position: relative;
}
.vrcdog-dialog-footer {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  padding: 12px 22px 18px 22px;
}

/* ── Form controls ───────────────────────────────────────── */
.vrcdog-field-label {
  display: block;
  font-size: 13px;
  font-weight: 700;
  color: var(--theme-text);
  margin-bottom: 6px;
}
.vrcdog-input {
  width: 100%;
  padding: 10px 14px;
  border: 2px solid var(--theme-border-soft);
  border-radius: 14px;
  background: var(--theme-bg-main);
  color: var(--theme-text);
  font-size: 14px;
  outline: none;
  transition: all 0.2s;
}
.vrcdog-input:focus {
  border-color: var(--theme-primary);
  box-shadow: 0 0 0 4px rgba(249, 115, 22, 0.15);
}
.vrcdog-checkbox-row {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  font-size: 14px;
  color: var(--theme-text);
  cursor: pointer;
}

/* ── Buttons ─────────────────────────────────────────────── */
.vrcdog-btn-primary {
  padding: 10px 22px;
  border-radius: 16px;
  background: var(--theme-primary);
  color: #fff;
  font-size: 14px;
  font-weight: 700;
  border: none;
  cursor: pointer;
  box-shadow: 0 6px 16px rgba(249, 115, 22, 0.3);
  transition: all 0.2s cubic-bezier(0.34, 1.56, 0.64, 1);
}
.vrcdog-btn-primary:hover {
  filter: brightness(1.1);
  transform: scale(1.02);
}
.vrcdog-btn-primary:active {
  transform: scale(0.98);
}
.vrcdog-btn-primary:disabled { opacity: 0.4; cursor: not-allowed; transform: none; box-shadow: none; }
.vrcdog-btn-secondary {
  padding: 10px 20px;
  border-radius: 16px;
  background: transparent;
  color: var(--theme-text);
  font-size: 14px;
  font-weight: 600;
  border: 2px solid var(--theme-border-soft);
  cursor: pointer;
  transition: all 0.15s;
}
.vrcdog-btn-secondary:hover { background: var(--theme-surface-hover); transform: scale(1.02); }
.vrcdog-btn-secondary:active { transform: scale(0.98); }

/* ── Tab pills (stable / beta) ───────────────────────────── */
.vrcdog-tab-group {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 6px;
  padding: 5px;
  border-radius: 16px;
  background: var(--theme-bg-main);
  border: 2px solid var(--theme-border-soft);
  margin-bottom: 12px;
}
.vrcdog-tab-pill {
  padding: 10px 16px;
  border-radius: 12px;
  background: transparent;
  color: var(--theme-text-muted);
  font-size: 14px;
  font-weight: 700;
  border: none;
  cursor: pointer;
  transition: all 0.2s cubic-bezier(0.34, 1.56, 0.64, 1);
}
.vrcdog-tab-pill.active {
  background: var(--theme-surface);
  color: var(--theme-text);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.12);
}

/* ── Beta warning box ────────────────────────────────────── */
.vrcdog-warning-box {
  display: flex;
  gap: 12px;
  padding: 12px 14px;
  border-radius: 16px;
  background: rgba(239, 68, 68, 0.08);
  border: 2px solid rgba(239, 68, 68, 0.35);
  margin-top: 12px;
}
.vrcdog-warning-icon { color: #ef4444; flex-shrink: 0; margin-top: 2px; }
.vrcdog-warning-title { font-weight: 700; color: #ef4444; font-size: 14px; }
.vrcdog-warning-desc { color: #ef4444; font-size: 13px; opacity: 0.9; margin-top: 2px; line-height: 1.4; }

/* ── Version dropdown ────────────────────────────────────── */
.vrcdog-version-trigger {
  width: 100%;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 16px;
  border: 2px solid var(--theme-border-soft);
  border-radius: 16px;
  background: var(--theme-bg-main);
  color: var(--theme-text);
  font-size: 14px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.2s;
}
.vrcdog-version-trigger:disabled { opacity: 0.5; cursor: not-allowed; }
.vrcdog-version-list {
  position: absolute;
  top: calc(100% + 8px);
  left: 0;
  right: 0;
  z-index: 60;
  max-height: 220px;
  overflow-y: auto;
  background: var(--theme-surface);
  border: 2px solid var(--theme-border-soft);
  border-radius: 18px;
  box-shadow: 0 16px 40px rgba(0, 0, 0, 0.45);
  padding: 6px;
  backdrop-filter: blur(16px);
}
.vrcdog-version-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
  padding: 10px 14px;
  font-size: 14px;
  font-weight: 600;
  color: var(--theme-text);
  background: transparent;
  border: none;
  border-radius: 12px;
  cursor: pointer;
  text-align: left;
  transition: all 0.15s cubic-bezier(0.16, 1, 0.3, 1);
}
.vrcdog-version-item:hover {
  background: var(--theme-surface-hover);
  transform: translateX(4px);
}
.vrcdog-version-item.active {
  color: var(--theme-primary);
  background: rgba(249, 115, 22, 0.12);
}

.vrcdog-update-status {
  margin-top: 12px;
  font-size: 13px;
  color: var(--theme-text-muted);
}

/* ── Transitions ────────────────────────────────────────── */
.vrcdog-modal-enter-active, .vrcdog-modal-leave-active { transition: opacity 0.18s; }
.vrcdog-modal-enter-from, .vrcdog-modal-leave-to { opacity: 0; }
.vrcdog-modal-enter-active .vrcdog-dialog-panel,
.vrcdog-modal-leave-active .vrcdog-dialog-panel { transition: transform 0.25s cubic-bezier(0.34, 1.56, 0.64, 1); }
.vrcdog-modal-enter-from .vrcdog-dialog-panel { transform: scale(0.92); }
.vrcdog-modal-leave-to .vrcdog-dialog-panel { transform: scale(0.92); }

.vrcdog-menu-enter-active, .vrcdog-menu-leave-active { transition: opacity 0.15s, transform 0.18s cubic-bezier(0.34, 1.56, 0.64, 1); }
.vrcdog-menu-enter-from, .vrcdog-menu-leave-to { opacity: 0; transform: translateY(-8px) scale(0.96); }
</style>
