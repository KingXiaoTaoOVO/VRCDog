import { beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({ request: vi.fn() }));

vi.mock('./request', () => ({ request: mocks.request }));

import { AuthApi } from './auth';

describe('AuthApi two-factor requests', () => {
  beforeEach(() => {
    mocks.request.mockReset().mockResolvedValue({ verified: true });
  });

  it.each([
    ['emailOtp', AuthApi.verifyEmailOTP, '/auth/twofactorauth/emailotp/verify'],
    ['totp', AuthApi.verifyTOTP, '/auth/twofactorauth/totp/verify'],
    ['otp', AuthApi.verifyOTP, '/auth/twofactorauth/otp/verify'],
  ])('routes %s to the correct endpoint and keeps the cookie out of the body', async (_name, verify, endpoint) => {
    await verify({
      code: '123456',
      authCookie: '["auth=pending","twoFactorAuth=session"]',
    });

    expect(mocks.request).toHaveBeenCalledWith(endpoint, {
      method: 'POST',
      params: { code: '123456' },
      authCookie: '["auth=pending","twoFactorAuth=session"]',
      suppressAuthExpired: true,
    });
  });
});
