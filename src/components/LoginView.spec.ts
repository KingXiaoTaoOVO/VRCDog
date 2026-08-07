import { flushPromises, mount } from '@vue/test-utils';
import { beforeEach, describe, expect, it, vi } from 'vitest';

// jsdom does not expose a global `localStorage`, so provide a minimal in-memory mock.
vi.hoisted(() => {
  const values = new Map<string, string>();
  const storage = {
    getItem: (key: string) => values.get(key) ?? null,
    setItem: (key: string, value: string) => values.set(key, String(value)),
    removeItem: (key: string) => values.delete(key),
    clear: () => values.clear(),
    key: (index: number) => Array.from(values.keys())[index] ?? null,
    get length() {
      return values.size;
    },
  };

  Object.defineProperty(globalThis, 'localStorage', {
    configurable: true,
    value: storage,
  });
});

const mocks = vi.hoisted(() => ({
  clearAuth: vi.fn(),
  clearCookies: vi.fn(),
  fetchConfig: vi.fn(),
  getAuth: vi.fn(),
  getCurrentUser: vi.fn(),
  getSetting: vi.fn(),
  login: vi.fn(),
  mergeCookiesAndSave: vi.fn(),
  saveAuth: vi.fn(),
  saveSetting: vi.fn(),
  verify2fa: vi.fn(),
}));

vi.mock('../api', () => ({
  DbApi: {
    clearAuth: mocks.clearAuth,
    getAuth: mocks.getAuth,
    getSetting: mocks.getSetting,
    saveAuth: mocks.saveAuth,
    saveSetting: mocks.saveSetting,
  },
  VrcApi: {
    applyAuthCookie: vi.fn(),
    clearCookies: mocks.clearCookies,
    fetchConfig: mocks.fetchConfig,
    getCurrentUser: mocks.getCurrentUser,
    login: mocks.login,
    verify2fa: mocks.verify2fa,
  },
}));

vi.mock('../api/cookies', () => ({
  mergeCookiesAndSave: mocks.mergeCookiesAndSave,
  normalizeAuthCookieJson: (value: string | null | undefined) => value || '[]',
  parseCookieInput: () => [],
}));

vi.mock('@tauri-apps/api/app', () => ({ getVersion: vi.fn().mockResolvedValue('5.0.1') }));

import i18n, { setAppLocale } from '../i18n';
import LoginView from './LoginView.vue';

const mountLogin = () => mount(LoginView, {
  global: {
    plugins: [i18n],
    stubs: {
      Bone: true,
      Key: true,
      User: true,
      Loader2: true,
      ArrowLeft: true,
      Trash2: true,
      Settings: true,
      ArrowDownToLine: true,
      Languages: true,
      Check: true,
      X: true,
      ChevronDown: true,
      AlertCircle: true,
    },
  },
});

async function enterTwoFactor(wrapper: ReturnType<typeof mountLogin>) {
  const inputs = wrapper.findAll('input');
  await inputs[0].setValue('test-user');
  await inputs[1].setValue('test-password');
  await wrapper.find('button.bg-primary').trigger('click');
  await flushPromises();
}

describe('LoginView two-factor flow', () => {
  beforeEach(() => {
    setAppLocale('zh-CN', { persist: false });
    mocks.clearAuth.mockReset().mockResolvedValue(undefined);
    mocks.clearCookies.mockReset().mockResolvedValue(undefined);
    mocks.fetchConfig.mockReset().mockResolvedValue({});
    mocks.getAuth.mockReset().mockResolvedValue(null);
    mocks.getCurrentUser.mockReset().mockResolvedValue({ id: 'usr_test', displayName: 'Test' });
    mocks.getSetting.mockReset().mockResolvedValue(null);
    mocks.login.mockReset().mockResolvedValue({
      requiresTwoFactorAuth: ['totp', 'emailOtp'],
      auth_cookie: '["auth=pending","twoFactorAuth=session"]',
    });
    mocks.mergeCookiesAndSave.mockReset().mockImplementation(async (cookie: string) => cookie);
    mocks.saveAuth.mockReset().mockResolvedValue(undefined);
    mocks.saveSetting.mockReset().mockResolvedValue(undefined);
    mocks.verify2fa.mockReset().mockResolvedValue({ verified: true });
  });

  it('defaults to email when multiple methods are available and carries the pending cookie', async () => {
    const wrapper = mountLogin();
    await enterTwoFactor(wrapper);

    const radios = wrapper.findAll('[role="radio"]');
    expect(radios).toHaveLength(2);
    expect(radios[1].attributes('aria-checked')).toBe('true');

    const codeInput = wrapper.get('input[autocomplete="one-time-code"]');
    await codeInput.setValue(' 123 456 ');
    await wrapper.findAll('button').find((button) => button.text().includes('提交验证码'))!.trigger('click');
    await flushPromises();

    expect(mocks.verify2fa).toHaveBeenCalledWith({
      code: '123456',
      method: 'emailOtp',
      authCookie: '["auth=pending","twoFactorAuth=session"]',
    });
    expect(mocks.getCurrentUser).toHaveBeenCalledWith({
      authCookie: '["auth=pending","twoFactorAuth=session"]',
    });
    expect(wrapper.emitted('login-success')).toBeTruthy();
  });

  it('allows switching to authenticator before submitting', async () => {
    const wrapper = mountLogin();
    await enterTwoFactor(wrapper);

    await wrapper.findAll('[role="radio"]')[0].trigger('click');
    await wrapper.get('input[autocomplete="one-time-code"]').setValue('654321');
    await wrapper.findAll('button').find((button) => button.text().includes('提交验证码'))!.trigger('click');
    await flushPromises();

    expect(mocks.verify2fa).toHaveBeenCalledWith(expect.objectContaining({
      code: '654321',
      method: 'totp',
    }));
  });

  it('does not send a malformed verification code', async () => {
    const wrapper = mountLogin();
    await enterTwoFactor(wrapper);

    await wrapper.get('input[autocomplete="one-time-code"]').setValue('12345');
    await wrapper.findAll('button').find((button) => button.text().includes('提交验证码'))!.trigger('click');
    await flushPromises();

    expect(mocks.verify2fa).not.toHaveBeenCalled();
    expect(wrapper.text()).toContain('请输入 6 位数字验证码');
  });

  it('shows a server verification message when verified is false', async () => {
    mocks.verify2fa.mockResolvedValue({ verified: false, message: 'Verification session expired' });
    const wrapper = mountLogin();
    await enterTwoFactor(wrapper);

    await wrapper.get('input[autocomplete="one-time-code"]').setValue('123456');
    await wrapper.findAll('button').find((button) => button.text().includes('提交验证码'))!.trigger('click');
    await flushPromises();

    expect(wrapper.text()).toContain('Verification session expired');
    expect(mocks.getCurrentUser).not.toHaveBeenCalled();
  });
});
