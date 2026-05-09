<script setup lang="ts">
import { ref, computed } from 'vue';
import { VrcApi, DbApi, SysApi, GamelogApi } from "../api";
import { invoke } from '@tauri-apps/api/core';
import { Bone, Key, User, Loader2, Globe } from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';
import { getVersion } from '@tauri-apps/api/app';

const { t, locale } = useI18n();
const appVersion = ref('');

const langMap: Record<string, string> = {
  'zh-CN': '简体中文',
  'en-US': 'English',
  'ja-JP': '日本語'
};

const currentLangLabel = computed(() => langMap[locale.value] || 'Language');

const cycleLanguage = () => {
  const keys = Object.keys(langMap);
  const idx = keys.indexOf(locale.value);
  const nextIdx = (idx + 1) % keys.length;
  const nextLang = keys[nextIdx];
  locale.value = nextLang;
  localStorage.setItem('vrcdog-locale', nextLang);
  // Optional: save to DbApi if needed
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
      errorMsg.value = res.error;
    } else if (res.requires_two_factor_auth && res.requires_two_factor_auth.length > 0) {
      show2FA.value = true;
      twoFactorMethods.value = res.requires_two_factor_auth;
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
    } else if (res.current_user) {
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
      emit('login-success', res.current_user);
    }
  } catch (err: any) {
    errorMsg.value = err.message || err;
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
  } catch (e) {}
});
</script>

<template>
  <div class="flex items-center justify-center h-screen bg-orange-50/50 relative overflow-hidden">
    <div class="fixed top-4 right-4 z-50 flex gap-2">
      <button @click="cycleLanguage" class="flex items-center gap-2 px-4 py-2 bg-white/50 hover:bg-white/80 backdrop-blur rounded-xl border border-orange-200 transition-all font-medium text-sm shadow-sm text-orange-900">
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

    <div class="bg-white p-8 rounded-3xl shadow-xl w-full max-w-md border border-orange-100 relative z-10">
      <div class="text-center mb-8">
        <h1 class="text-3xl font-bold text-orange-600 mb-2 font-mono flex items-center justify-center gap-2">
          <Bone class="animate-bounce" /> {{ t('login.title') }}
        </h1>
        <p class="text-orange-900/60 text-sm">
          {{ t('login.subtitle') }}
        </p>
      </div>

      <div
        v-if="!show2FA"
        class="space-y-5"
      >
        <div>
          <label class="block text-sm font-bold text-orange-900 mb-1 flex items-center gap-1">
            <User :size="16" /> {{ t('login.username') }}
          </label>
          <input
            v-model="username"
            type="text"
            placeholder="Username / Email"
            class="w-full px-4 py-3 rounded-xl border-2 border-orange-100 focus:border-orange-400 focus:ring-0 outline-none transition-colors bg-orange-50/30"
          >
        </div>
        <div>
          <label class="block text-sm font-bold text-orange-900 mb-1 flex items-center gap-1">
            <Key :size="16" /> {{ t('login.password') }}
          </label>
          <input
            v-model="password"
            type="password"
            placeholder="Password"
            class="w-full px-4 py-3 rounded-xl border-2 border-orange-100 focus:border-orange-400 focus:ring-0 outline-none transition-colors bg-orange-50/30"
            @keyup.enter="handleLogin"
          >
        </div>

        <div class="relative flex items-center py-2">
          <div class="flex-grow border-t border-orange-200" />
          <span class="flex-shrink-0 mx-4 text-orange-400 text-xs font-bold">{{ t('login.or') }}</span>
          <div class="flex-grow border-t border-orange-200" />
        </div>

        <div>
          <label class="block text-xs font-bold text-orange-900 mb-1">{{ t('login.use_cookie') }}</label>
          <input
            v-model="authCookie"
            type="text"
            placeholder="auth=xxxxxxx..."
            class="w-full px-4 py-2 rounded-xl border-2 border-orange-100 focus:border-orange-400 focus:ring-0 outline-none transition-colors bg-orange-50/30 text-xs"
            @keyup.enter="handleLogin"
          >
        </div>

        <button
          :disabled="loading"
          class="w-full mt-4 bg-orange-500 hover:bg-orange-600 text-white font-bold py-3 px-4 rounded-xl shadow-lg shadow-orange-500/30 transition-all flex items-center justify-center gap-2 disabled:opacity-50 disabled:cursor-not-allowed"
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
        <div class="text-center p-4 bg-orange-50 rounded-xl mb-4">
          <Key
            class="mx-auto text-orange-500 mb-2"
            :size="32"
          />
          <p class="text-orange-900 font-bold">
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
            class="w-full px-4 py-4 rounded-xl border-2 border-orange-100 focus:border-orange-400 focus:ring-0 outline-none transition-colors bg-orange-50/30 text-center text-2xl tracking-widest font-mono font-bold"
            @keyup.enter="handle2FA"
          >
        </div>
        <button
          :disabled="loading"
          class="w-full mt-4 bg-green-500 hover:bg-green-600 text-white font-bold py-3 px-4 rounded-xl shadow-lg shadow-green-500/30 transition-all flex items-center justify-center gap-2 disabled:opacity-50 disabled:cursor-not-allowed"
          @click="handle2FA"
        >
          <Loader2
            v-if="loading"
            class="animate-spin"
            :size="20"
          />
          {{ loading ? t('login.btn_submitting') : t('login.btn_submit_code') }}
        </button>
        <button
          class="w-full text-orange-600 hover:text-orange-800 text-sm font-bold mt-2"
          @click="show2FA = false; errorMsg = ''"
        >
          {{ t('login.btn_cancel') }}
        </button>
      </div>

      <p
        v-if="errorMsg"
        class="mt-4 text-center text-red-500 text-sm font-bold animate-pulse"
      >
        {{ errorMsg }}
      </p>
    </div>
    
    <!-- 左下角版本号 -->
    <div class="absolute bottom-4 left-4 z-50">
      <span class="text-xs font-mono font-bold text-amber-900/40 bg-white/40 backdrop-blur px-2 py-1 rounded-lg">v{{ appVersion }}</span>
    </div>
  </div>
</template>
