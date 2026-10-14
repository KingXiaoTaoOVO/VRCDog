import { beforeEach, describe, expect, it } from 'vitest';
import {
  setVrcdogClientToken,
  vrcdogClientTokenHeaders,
  vrcdogClientTokenState,
} from './clientToken';

// 服务端 `require_client_token` 只认 `x-vrcdog-client-token` 头。
// 这组测试锁定两条不变式：
//   1) 发给「已注册的那台服务器」的请求必须自动带上 token（否则问卷/权限接口静默 401）；
//   2) 发给任何其他 origin 的请求绝不能带上 token（这是设备身份凭证，不能外泄）。
describe('vrcdog 服务端 client token 的 origin 绑定', () => {
  beforeEach(() => {
    setVrcdogClientToken('', '');
  });

  it('命中同一 origin（含不同路径 / 查询串）时附加 token', () => {
    setVrcdogClientToken('tok-123', 'http://127.0.0.1:11451');

    expect(vrcdogClientTokenHeaders('http://127.0.0.1:11451/api/client/features/u1'))
      .toEqual({ 'x-vrcdog-client-token': 'tok-123' });
    expect(vrcdogClientTokenHeaders('http://127.0.0.1:11451/api/client/surveys/u1?x=1'))
      .toEqual({ 'x-vrcdog-client-token': 'tok-123' });
    // 末尾斜杠、默认端口等写法差异不应导致漏带头
    expect(vrcdogClientTokenHeaders('http://127.0.0.1:11451/'))
      .toEqual({ 'x-vrcdog-client-token': 'tok-123' });
  });

  it('不会把 token 带到别的 origin', () => {
    setVrcdogClientToken('tok-123', 'http://127.0.0.1:11451');

    expect(vrcdogClientTokenHeaders('https://evil.example/api/client/features/u1')).toEqual({});
    expect(vrcdogClientTokenHeaders('http://127.0.0.1:11452/api/client/heartbeat')).toEqual({});
    expect(vrcdogClientTokenHeaders('https://api.vrchat.cloud/api/1/auth/user')).toEqual({});
    // 端口相同、scheme 不同 => origin 不同
    expect(vrcdogClientTokenHeaders('https://127.0.0.1:11451/api/client/features/u1')).toEqual({});
  });

  it('登出 / 清空 token 后不再携带任何身份', () => {
    setVrcdogClientToken('tok-123', 'http://127.0.0.1:11451');
    expect(vrcdogClientTokenState().hasToken).toBe(true);

    setVrcdogClientToken('', '');
    expect(vrcdogClientTokenState()).toEqual({ hasToken: false, origin: '' });
    expect(vrcdogClientTokenHeaders('http://127.0.0.1:11451/api/client/features/u1')).toEqual({});
  });

  it('非法 URL 一律不带头', () => {
    setVrcdogClientToken('tok-123', 'http://127.0.0.1:11451');
    expect(vrcdogClientTokenHeaders('not a url')).toEqual({});
    expect(vrcdogClientTokenHeaders('')).toEqual({});
    expect(vrcdogClientTokenHeaders('javascript:alert(1)')).toEqual({});
  });
});
