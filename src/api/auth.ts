import { request } from './request';

export const AuthApi = {
  verifyOTP: (params: { code: string }) => 
    request('/auth/twofactorauth/otp/verify', { method: 'POST', params }),

  verifyTOTP: (params: { code: string }) => 
    request('/auth/twofactorauth/totp/verify', { method: 'POST', params }),

  verifyEmailOTP: (params: { code: string }) => 
    request('/auth/twofactorauth/emailotp/verify', { method: 'POST', params }),

  getConfig: () => 
    request('/config', { method: 'GET' }),

  login: async (params: any) => {
    const headers: any = {};
    if (params.username && params.password) {
      const b64 = btoa(`${encodeURIComponent(params.username)}:${encodeURIComponent(params.password)}`);
      headers['Authorization'] = `Basic ${b64}`;
    }
    return request('/auth/user', { method: 'GET', headers, authCookie: params.authCookie });
  }
};
