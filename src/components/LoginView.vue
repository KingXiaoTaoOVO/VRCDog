<script setup lang="ts">
import { ref, computed } from 'vue';
import { VrcApi, DbApi, SysApi, GamelogApi } from "../api";
import { invoke } from '@tauri-apps/api/core';
import { Bone, Key, User, Loader2, Globe, ArrowLeft } from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';
import i18n from '../i18n';
import { getVersion } from '@tauri-apps/api/app';
import { currentTheme } from '../theme';

const { t, locale } = useI18n({ useScope: 'global' });
const appVersion = ref('');



const languages = [
  { code: 'zh-CN', label: 'Chinese' },
  { code: 'en-US', label: 'English' },
  { code: 'ja-JP', label: 'Japanese' }
];

const currentLangLabel = computed(() => {
  const lang = languages.find(l => l.code === locale.value);
  return lang ? lang.label : 'Chinese';
});

const cycleLanguage = () => {
  const currentIndex = languages.findIndex(l => l.code === locale.value);
  const nextIndex = (currentIndex + 1) % languages.length;
  const nextLang = languages[nextIndex].code;
  
  // Update global locale immediately
  if (i18n.global) {
    (i18n.global.locale as any).value = nextLang;
  }
  locale.value = nextLang;
  
  localStorage.setItem('vrcdog-locale', nextLang);
  window.dispatchEvent(new CustomEvent('settings-updated', { detail: { language: nextLang } }));
  DbApi.saveSetting({ key: 'language', value: JSON.stringify(nextLang) }).catch(() => {});
};

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
    // 关键步骤：清空可能残留的旧会话（否则随便输密码也能通过）
    await DbApi.clearAuth();
    await VrcApi.clearCookies();
    // 关键步骤：必须先调用 /config 获取初始 session cookie
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
      
      // [VRCX 对齐] 合并保存中间 cookie（login 阶段产生的 auth cookie）
      if (res.auth_cookie) {
        authCookie.value = res.auth_cookie;
        // 不用 DbApi.saveAuth 而是 merge，确保不覆盖已有的 twoFactorAuth
        try {
          const newCookies: string[] = JSON.parse(res.auth_cookie);
          let existing: string[] = [];
          try {
            const stored = await invoke<string | null>('db_get_auth');
            if (stored) { const p = JSON.parse(stored); if (Array.isArray(p)) existing = p; }
          } catch {}
          const map = new Map<string, string>();
          for (const c of existing) { const n = c.split('=')[0]; if (n) map.set(n, c); }
          for (const c of newCookies) { const n = c.split('=')[0]; if (n) map.set(n, c); }
          await invoke('db_save_auth', { cookie: JSON.stringify(Array.from(map.values())) });
        } catch {
          // fallback: 直接保存
          try { await DbApi.saveAuth({ cookie: res.auth_cookie }); } catch {}
        }
      }
    } else if (res.id || res.currentUser || res.current_user) {
      // [VRCX 对齐] 合并保存 auth cookie
      if (res.auth_cookie) {
        authCookie.value = res.auth_cookie;
        try {
          const newCookies: string[] = JSON.parse(res.auth_cookie);
          let existing: string[] = [];
          try {
            const stored = await invoke<string | null>('db_get_auth');
            if (stored) { const p = JSON.parse(stored); if (Array.isArray(p)) existing = p; }
          } catch {}
          const map = new Map<string, string>();
          for (const c of existing) { const n = c.split('=')[0]; if (n) map.set(n, c); }
          for (const c of newCookies) { const n = c.split('=')[0]; if (n) map.set(n, c); }
          await invoke('db_save_auth', { cookie: JSON.stringify(Array.from(map.values())) });
        } catch {
          try { await DbApi.saveAuth({ cookie: res.auth_cookie }); } catch {}
        }
      }
      const user = res.currentUser || res.current_user || res;
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
      // [VRCX 对齐] 2FA 通过后的 Cookie 已通过 VrcApi.request() 自动合并到 DB
      // verify2fa 内部已改为读取 DB 的完整 Cookie 并使用 mergeCookiesAndSave
      // 获取完整用户信息
      const user: any = await VrcApi.getCurrentUser();
      if (user) {
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

import { onMounted } from 'vue';
onMounted(async () => {
  try {
    appVersion.value = await getVersion();
    const cookie = await DbApi.getAuth();
    if (cookie) {
      authCookie.value = cookie;
    }
  } catch (e) {}
});
</script>

<template>
  <div class="flex items-center justify-center h-screen bg-[var(--theme-bg-main)] relative overflow-hidden">
    <div class="fixed top-4 right-4 z-50 flex gap-2">
      <button
        class="flex items-center gap-2 px-4 py-2 bg-surface hover:bg-surface backdrop-blur rounded-xl border-orange-200 transition-all font-medium text-sm shadow-sm text-text"
        @click="cycleLanguage"
      >
        <Globe class="w-4 h-4" />
        {{ currentLangLabel }}
      </button>
    </div>

    <div class="absolute -top-20 -left-20 text-orange-200/40 transform -rotate-12 pointer-events-none">
      <Bone
        :size="300"
        stroke-width="1"
      />
    </div>
    <div class="absolute -bottom-20 -right-20 text-orange-200/40 transform rotate-12 pointer-events-none">
      <Bone
        :size="300"
        stroke-width="1"
      />
    </div>

    <div class="bg-[var(--theme-surface)]/90 backdrop-blur-xl p-8 rounded-3xl shadow-2xl w-full max-w-md border border-[var(--theme-border-soft)] relative z-10 flex flex-col">
      <div class="text-center mb-8">
        <h1 class="text-3xl font-bold text-white mb-2 font-mono flex items-center justify-center gap-2">
          <Bone class="animate-bounce" /> {{ t('login.title') }}
        </h1>
        <p class="text-zinc-400 text-sm">
          {{ t('login.subtitle') }}
        </p>
      </div>

      <div
        v-if="!show2FA"
        class="space-y-5"
      >
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
          <Loader2
            v-if="loading"
            class="animate-spin"
            :size="20"
          />
          {{ loading ? t('login.btn_logging_in') : t('login.btn_login') }}
        </button>
      </div>

      <div
        v-else
        class="space-y-5"
      >
        <div class="text-center p-4 bg-black/40 rounded-xl mb-4 border border-white/5">
          <Key
            class="mx-auto text-primary mb-2 animate-pulse"
            :size="32"
          />
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
          <Loader2
            v-if="loading"
            class="animate-spin"
            :size="20"
          />
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
    
    <!-- 宸︿笅瑙掔増鏈彿 -->
    <div class="absolute bottom-4 left-4 z-50">
      <span class="text-xs font-mono font-bold text-text-muted/40 bg-surface backdrop-blur px-2 py-1 rounded-lg">v{{ appVersion }}</span>
    </div>
  </div>
</template>

