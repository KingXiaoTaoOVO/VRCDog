import { beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  isTauri: () => true,
  invoke: mocks.invoke,
}));

import {
  cleanCookieSegment,
  getCookieValue,
  mergeCookiesAndSave,
  normalizeAuthCookieJson,
  parseCookieInput,
} from './cookies';

describe('cookies module', () => {
  beforeEach(() => {
    mocks.invoke.mockReset();
  });

  describe('parseCookieInput', () => {
    it('parses valid cookie key=value segments', () => {
      const parsed = parseCookieInput('auth=authcookie_12345; twoFactorAuth=twoFactor_67890');
      expect(parsed).toEqual(['auth=authcookie_12345', 'twoFactorAuth=twoFactor_67890']);
    });

    it('parses JSON string array', () => {
      const parsed = parseCookieInput('["auth=authcookie_12345", "twoFactorAuth=twoFactor_67890"]');
      expect(parsed).toEqual(['auth=authcookie_12345', 'twoFactorAuth=twoFactor_67890']);
    });

    it('strips quotes from values', () => {
      const parsed = parseCookieInput('auth="authcookie_12345"; twoFactorAuth=\'twoFactor_67890\'');
      expect(parsed).toEqual(['auth=authcookie_12345', 'twoFactorAuth=twoFactor_67890']);
    });

    it('rejects empty, deleted, null, undefined or none values', () => {
      expect(parseCookieInput('auth=""; twoFactorAuth="deleted"')).toEqual([]);
      expect(parseCookieInput('auth=null; twoFactorAuth=none')).toEqual([]);
      expect(parseCookieInput('auth=undefined')).toEqual([]);
    });

    it('wraps non-empty bare tokens', () => {
      expect(parseCookieInput('authcookie_bare_12345')).toEqual(['auth=authcookie_bare_12345']);
    });

    it('rejects empty or corrupt bare tokens', () => {
      expect(parseCookieInput('')).toEqual([]);
      expect(parseCookieInput('""')).toEqual([]);
      expect(parseCookieInput('deleted')).toEqual([]);
      expect(parseCookieInput('auth=')).toEqual([]);
    });
  });

  describe('mergeCookiesAndSave', () => {
    it('preserves existing valid auth cookie when incoming token is invalid or empty', async () => {
      mocks.invoke.mockImplementation(async (cmd: string) => {
        if (cmd === 'db_get_auth') return '["auth=authcookie_valid_123456789"]';
        if (cmd === 'db_save_auth') return null;
        return null;
      });

      const result = await mergeCookiesAndSave('["auth=\"\""]');
      // The incoming cookie was invalid, so parseCookieInput returned empty, merge returns null without touching db
      expect(result).toBeNull();
      expect(mocks.invoke).not.toHaveBeenCalledWith('db_save_auth', expect.anything());
    });

    it('merges new valid token and updates db', async () => {
      mocks.invoke.mockImplementation(async (cmd: string) => {
        if (cmd === 'db_get_auth') return '["auth=authcookie_valid_123456789"]';
        if (cmd === 'db_save_auth') return null;
        return null;
      });

      const result = await mergeCookiesAndSave('["twoFactorAuth=twofactor_valid_987654321"]');
      expect(result).toBe('["auth=authcookie_valid_123456789","twoFactorAuth=twofactor_valid_987654321"]');
      expect(mocks.invoke).toHaveBeenCalledWith('db_save_auth', {
        cookie: '["auth=authcookie_valid_123456789","twoFactorAuth=twofactor_valid_987654321"]',
      });
    });

    it('allows valid updated auth token to overwrite old auth token', async () => {
      mocks.invoke.mockImplementation(async (cmd: string) => {
        if (cmd === 'db_get_auth') return '["auth=authcookie_old_123456789"]';
        if (cmd === 'db_save_auth') return null;
        return null;
      });

      const result = await mergeCookiesAndSave('["auth=authcookie_new_987654321"]');
      expect(result).toBe('["auth=authcookie_new_987654321"]');
      expect(mocks.invoke).toHaveBeenCalledWith('db_save_auth', {
        cookie: '["auth=authcookie_new_987654321"]',
      });
    });
  });
});
