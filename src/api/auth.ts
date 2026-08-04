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
    }),

  verifyTOTP: (params: TwoFactorRequestParams) =>
    request('/auth/twofactorauth/totp/verify', {
      method: 'POST',
      params: { code: params.code },
      authCookie: params.authCookie,
    }),

  verifyEmailOTP: (params: TwoFactorRequestParams) =>
    request('/auth/twofactorauth/emailotp/verify', {
      method: 'POST',
      params: { code: params.code },
      authCookie: params.authCookie,
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
    return request('/auth/user', { method: 'GET', headers, authCookie: params.authCookie });
  }
};
