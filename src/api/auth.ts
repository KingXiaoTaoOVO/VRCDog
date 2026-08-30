import { request } from './request';

export interface TwoFactorRequestParams {
  code: string;
  authCookie?: string;
}

export const AuthApi = {
  verifyOTP: (params: TwoFactorRequestParams) =>
    request('/auth/twofactorauth/otp/verify', {
      method: 'POST',
      params: { code: params.code },
      authCookie: params.authCookie,
      // A wrong code here is not a real session expiry; don't trigger the
      // global "auth expired -> clear cookies / logout" handler mid-login.
      suppressAuthExpired: true,
    }),

  verifyTOTP: (params: TwoFactorRequestParams) =>
    request('/auth/twofactorauth/totp/verify', {
      method: 'POST',
      params: { code: params.code },
      authCookie: params.authCookie,
      suppressAuthExpired: true,
    }),

  verifyEmailOTP: (params: TwoFactorRequestParams) =>
    request('/auth/twofactorauth/emailotp/verify', {
      method: 'POST',
      params: { code: params.code },
      authCookie: params.authCookie,
      suppressAuthExpired: true,
    }),

  getConfig: () => 
    request('/config', { method: 'GET' }),

  login: async (params: any) => {
    const headers: any = {};
    if (params.username && params.password) {
      // VrcDog uses encodeURIComponent on username and password separately before base64
      const authStr = `${encodeURIComponent(params.username)}:${encodeURIComponent(params.password)}`;
      const b64 = btoa(authStr);
      headers['Authorization'] = `Basic ${b64}`;
    }
    // A 401 here means "2FA required", not a dead session. Suppress the
    // global vrc-auth-expired handler so it doesn't wipe the freshly set
    // `auth` cookie before the user can submit their code.
    return request('/auth/user', { method: 'GET', headers, authCookie: params.authCookie, suppressAuthExpired: true });
  }
};
