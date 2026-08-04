export type TwoFactorMethod = 'totp' | 'emailOtp' | 'otp';

export interface TwoFactorVerifyResponse {
  verified?: boolean | string;
  auth_cookie?: string;
  authCookie?: string;
  error?: unknown;
  message?: unknown;
  details?: unknown;
  [key: string]: unknown;
}

function compactMethod(value: unknown): string {
  return String(value ?? '').trim().toLowerCase().replace(/[\s_-]+/g, '');
}

export function normalizeTwoFactorMethod(value: unknown): TwoFactorMethod | null {
  switch (compactMethod(value)) {
    case 'emailotp':
    case 'emailcode':
      return 'emailOtp';
    case 'totp':
    case 'authenticator':
    case 'authenticatorapp':
      return 'totp';
    case 'otp':
      return 'otp';
    default:
      return null;
  }
}

export function normalizeTwoFactorMethods(values: unknown): TwoFactorMethod[] {
  const source = Array.isArray(values) ? values : [values];
  const result: TwoFactorMethod[] = [];
  for (const value of source) {
    const method = normalizeTwoFactorMethod(value);
    if (method && !result.includes(method)) result.push(method);
  }
  return result;
}

export function chooseDefaultTwoFactorMethod(methods: unknown): TwoFactorMethod {
  const normalized = normalizeTwoFactorMethods(methods);
  return normalized.includes('emailOtp') ? 'emailOtp' : normalized[0] || 'totp';
}

export function normalizeTwoFactorCode(value: unknown): string {
  return String(value ?? '').replace(/\s+/g, '');
}

export function isValidTwoFactorCode(value: unknown): boolean {
  return /^\d{6}$/.test(normalizeTwoFactorCode(value));
}

export function isTwoFactorVerified(response: TwoFactorVerifyResponse | null | undefined): boolean {
  const value = response?.verified;
  return value === true || (typeof value === 'string' && value.trim().toLowerCase() === 'true');
}

export function getTwoFactorResponseCookie(response: TwoFactorVerifyResponse | null | undefined): string | null {
  return response?.auth_cookie || response?.authCookie || null;
}

export function getTwoFactorErrorMessage(response: TwoFactorVerifyResponse | null | undefined): string | null {
  const error = response?.error;
  const candidates = [
    typeof error === 'object' && error ? (error as any).message : undefined,
    typeof error === 'object' && error ? (error as any).details : undefined,
    typeof error === 'string' ? error : undefined,
    response?.message,
    response?.details,
  ];
  return candidates.find((value): value is string => typeof value === 'string' && value.trim().length > 0)?.trim() || null;
}
