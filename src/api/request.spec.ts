import { afterEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
  mergeCookiesAndSave: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  isTauri: () => true,
  invoke: mocks.invoke,
}));

vi.mock('./cookies', () => ({
  mergeCookiesAndSave: mocks.mergeCookiesAndSave,
  normalizeAuthCookieJson: (value: string) => value,
}));

vi.mock('./debugConfig', () => ({ isDebugLogEnabled: () => false }));

import { buildVrchatApiUrl, request } from './request';

describe('request GET coalescing', () => {
  afterEach(() => {
    vi.useRealTimers();
    mocks.invoke.mockReset();
    mocks.mergeCookiesAndSave.mockReset();
  });

  it('uses one native request for concurrent identical GETs', async () => {
    let resolveRequest!: (value: unknown) => void;
    mocks.invoke.mockImplementation((command: string) => {
      if (command === 'vrc_execute') {
        return new Promise((resolve) => { resolveRequest = resolve; });
      }
      return Promise.resolve(null);
    });

    const first = request('https://example.test/worlds', { method: 'GET' });
    const second = request('https://example.test/worlds', { method: 'GET' });

    expect(second).toBe(first);
    expect(mocks.invoke).toHaveBeenCalledTimes(1);

    resolveRequest({ status: 200, data: '{}' });
    await expect(first).resolves.toEqual({});
  });

  it('does not coalesce GETs that carry request headers', async () => {
    mocks.invoke.mockResolvedValue({ status: 200, data: '{}' });

    await Promise.all([
      request('https://example.test/admin', {
        headers: { 'x-vrcdog-admin-password': 'first' },
      }),
      request('https://example.test/admin', {
        headers: { 'x-vrcdog-admin-password': 'second' },
      }),
    ]);

    expect(mocks.invoke).toHaveBeenCalledTimes(2);
  });

  it('does not coalesce GETs that use different execution policies', async () => {
    mocks.invoke.mockResolvedValue({ status: 200, data: '{}' });

    await Promise.all([
      request('https://example.test/state', { timeoutMs: 3000 }),
      request('https://example.test/state', { timeoutMs: 10000 }),
    ]);

    expect(mocks.invoke).toHaveBeenCalledTimes(2);
  });

  it('removes a failed GET from the in-flight cache', async () => {
    mocks.invoke
      .mockRejectedValueOnce(new Error('REQUEST_CONNECT: refused'))
      .mockResolvedValueOnce({ status: 200, data: '{"ok":true}' });

    await expect(request('https://example.test/retry', { maxRetries: 0 })).rejects.toMatchObject({
      code: 'VRCHAT_NETWORK_ERROR',
    });
    await expect(request('https://example.test/retry', { maxRetries: 0 })).resolves.toEqual({ ok: true });
    expect(mocks.invoke).toHaveBeenCalledTimes(2);
  });

  it('does not automatically replay a failed POST', async () => {
    mocks.invoke.mockRejectedValue(new Error('REQUEST_CONNECT: refused'));

    await expect(request('https://example.test/write', {
      method: 'POST',
      params: { value: 1 },
    })).rejects.toMatchObject({ code: 'VRCHAT_NETWORK_ERROR' });

    expect(mocks.invoke).toHaveBeenCalledTimes(1);
  });

  it('retries a replay-safe GET after a temporary connection failure', async () => {
    vi.useFakeTimers();
    mocks.invoke
      .mockRejectedValueOnce(new Error('REQUEST_CONNECT: refused'))
      .mockResolvedValueOnce({ status: 200, data: '{"ok":true}' });

    const operation = request('https://example.test/read', { maxRetries: 1, dedupe: false });
    await vi.advanceTimersByTimeAsync(1000);

    await expect(operation).resolves.toEqual({ ok: true });
    expect(mocks.invoke).toHaveBeenCalledTimes(2);
  });

  it('rejects unsupported absolute URL schemes', async () => {
    await expect(request('file:///sensitive.txt')).rejects.toThrow('Unsupported API URL');
    expect(mocks.invoke).not.toHaveBeenCalled();
  });

  it('passes an explicit auth cookie to a VRChat POST without putting it in the body', async () => {
    mocks.invoke.mockResolvedValue({ status: 200, data: '{"verified":true}' });

    await expect(request('/auth/twofactorauth/emailotp/verify', {
      method: 'POST',
      params: { code: '123456' },
      authCookie: '["auth=session","twoFactorAuth=pending"]',
    })).resolves.toEqual({ verified: true });

    expect(mocks.invoke).toHaveBeenCalledWith('vrc_execute', {
      options: expect.objectContaining({
        url: 'https://api.vrchat.cloud/api/1/auth/twofactorauth/emailotp/verify',
        method: 'POST',
        body: '{"code":"123456"}',
        auth_cookie: '["auth=session","twoFactorAuth=pending"]',
      }),
    });
  });

  it('attaches the merged response cookie to a successful object response', async () => {
    mocks.invoke.mockResolvedValue({
      status: 200,
      data: '{"verified":true}',
      auth_cookie: '["auth=refreshed"]',
    });
    mocks.mergeCookiesAndSave.mockResolvedValue('["auth=refreshed","twoFactorAuth=accepted"]');

    await expect(request('/auth/twofactorauth/totp/verify', {
      method: 'POST',
      params: { code: '654321' },
    })).resolves.toEqual({
      verified: true,
      auth_cookie: '["auth=refreshed","twoFactorAuth=accepted"]',
    });
  });

  it('normalizes relative VRChat endpoints without duplicating api/1', () => {
    expect(buildVrchatApiUrl('/api/1/auth/user')).toBe(
      'https://api.vrchat.cloud/api/1/auth/user',
    );
  });
});
