import { describe, expect, it } from 'vitest';
import {
  chooseDefaultTwoFactorMethod,
  getTwoFactorErrorMessage,
  getTwoFactorResponseCookie,
  isTwoFactorVerified,
  isValidTwoFactorCode,
  normalizeTwoFactorCode,
  normalizeTwoFactorMethod,
  normalizeTwoFactorMethods,
} from './twoFactor';

describe('two-factor helpers', () => {
  it('normalizes VRChat method spelling variants and removes duplicates', () => {
    expect(normalizeTwoFactorMethod('email_otp')).toBe('emailOtp');
    expect(normalizeTwoFactorMethod('EmailOtp')).toBe('emailOtp');
    expect(normalizeTwoFactorMethod('authenticator_app')).toBe('totp');
    expect(normalizeTwoFactorMethods(['totp', 'emailotp', 'email_otp'])).toEqual(['totp', 'emailOtp']);
  });

  it('defaults to email when both email and authenticator are available', () => {
    expect(chooseDefaultTwoFactorMethod(['totp', 'emailOtp'])).toBe('emailOtp');
    expect(chooseDefaultTwoFactorMethod(['totp'])).toBe('totp');
  });

  it('cleans copied whitespace and validates six digits', () => {
    expect(normalizeTwoFactorCode(' 123 456\n')).toBe('123456');
    expect(isValidTwoFactorCode(' 123 456\n')).toBe(true);
    expect(isValidTwoFactorCode('12345')).toBe(false);
    expect(isValidTwoFactorCode('12345a')).toBe(false);
  });

  it('accepts boolean and string verification success', () => {
    expect(isTwoFactorVerified({ verified: true })).toBe(true);
    expect(isTwoFactorVerified({ verified: 'TRUE' })).toBe(true);
    expect(isTwoFactorVerified({ verified: false })).toBe(false);
  });

  it('extracts response cookies and server error messages', () => {
    expect(getTwoFactorResponseCookie({ auth_cookie: '["auth=new"]' })).toBe('["auth=new"]');
    expect(getTwoFactorErrorMessage({ error: { message: 'Code expired' } })).toBe('Code expired');
    expect(getTwoFactorErrorMessage({ message: 'Try again' })).toBe('Try again');
  });
});
