import { describe, expect, it } from 'vitest';
import { sanitizeArgs, sanitizeResponse } from './request';

// 回归用例：修复前 REDACT_KEY_RE 使用 `^(...)$` 全词匹配，`auth_cookie` /
// `authToken` / `client_token` 这类真实字段名全部漏网，导致 VRChat 会话 Cookie
// 明文进入调试控制台并可被导出。
describe('debug-log redaction', () => {
  it('redacts snake_case and camelCase credential keys', () => {
    const redacted = sanitizeArgs({
      options: {
        auth_cookie: 'auth=eyJhbGciOiJIUzI1NiJ9.secret-session-value',
        authToken: 'eyJhbGciOiJIUzI1NiJ9.another-secret',
        client_token: 'client-token-value',
        twoFactorAuth: '123456',
      },
    }) as any;

    expect(redacted.options.auth_cookie).toBe('***');
    expect(redacted.options.authToken).toBe('***');
    expect(redacted.options.client_token).toBe('***');
    expect(redacted.options.twoFactorAuth).toBe('***');
  });

  it('redacts credentials nested inside a JSON string body', () => {
    const redacted = sanitizeArgs({
      options: {
        method: 'POST',
        url: 'https://api.vrchat.cloud/api/1/auth/user',
        body: JSON.stringify({ username: 'someone', password: 'hunter2', token: 'abc' }),
      },
    }) as any;

    const body = JSON.parse(redacted.options.body);
    expect(body.password).toBe('***');
    expect(body.token).toBe('***');
    // 非敏感字段保留，保证调试控制台仍然可用
    expect(body.username).toBe('someone');
  });

  it('scrubs cookie pairs embedded in plain strings', () => {
    const redacted = sanitizeResponse({
      message: 'sent with auth=eyJhbGciOiJIUzI1NiJ9.payload.sig; twoFactorAuth=987654',
    }) as any;

    expect(redacted.message).not.toContain('eyJhbGciOiJIUzI1NiJ9');
    expect(redacted.message).not.toContain('987654');
    expect(redacted.message).toContain('auth=***');
  });

  it('keeps non-sensitive fields readable', () => {
    const redacted = sanitizeResponse({
      id: 'usr_1234',
      displayName: 'Doggo',
      online: true,
      count: 3,
      tags: ['a', 'b'],
    }) as any;

    expect(redacted).toEqual({
      id: 'usr_1234',
      displayName: 'Doggo',
      online: true,
      count: 3,
      tags: ['a', 'b'],
    });
  });

  it('does not crash on cyclic-free deep structures or primitives', () => {
    expect(sanitizeResponse(null)).toBeNull();
    expect(sanitizeResponse(undefined)).toBeUndefined();
    expect(sanitizeResponse(42)).toBe(42);
    expect(sanitizeArgs()).toBeUndefined();
  });
});
