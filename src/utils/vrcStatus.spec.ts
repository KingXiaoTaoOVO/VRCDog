import { describe, expect, it } from 'vitest';
import zhCN from '../i18n/locales/zh-CN.json';
import { VRC_STATUSES, vrcStatusLabelKey } from './vrcStatus';

/**
 * 回归的缺陷：侧边栏原本写 `$t('status.' + status.replace(' ', '_'))`，
 * `String.replace` 用字符串模式只替换**第一个**空格，
 * 于是 "do not disturb" 拼出 `status.do_not disturb` 这种必然不存在的 key，
 * vue-i18n 又把缺失 key 原样返回 —— 用户会在侧边栏看到裸字符串。
 */
const resolve = (key: string): unknown =>
  key.split('.').reduce<any>((acc, part) => (acc && typeof acc === 'object' ? acc[part] : undefined), zhCN);

describe('vrcStatusLabelKey', () => {
  it('每个 VRChat 状态都能解析出中文文案，而不是裸 key', () => {
    for (const status of VRC_STATUSES) {
      const key = vrcStatusLabelKey(status);
      expect(key, `${status} 拼出的 key 形态不对`).toMatch(/^status\.[a-z_]+$/);
      const label = resolve(key);
      expect(typeof label, `${key} 在中文语言包里不存在`).toBe('string');
      expect((label as string).trim(), `${key} 文案为空`).not.toBe('');
      expect(label, `${key} 解析回了裸 key`).not.toBe(key);
    }
  });

  it('"do not disturb" 不再被半截替换成非法 key', () => {
    const naive = `status.${'do not disturb'.replace(' ', '_')}`;
    expect(naive).toBe('status.do_not disturb'); // 旧写法的样子
    expect(vrcStatusLabelKey('do not disturb')).toBe('status.do_not_disturb');
  });

  it('大小写与多余空白都归一', () => {
    expect(vrcStatusLabelKey('Do Not Disturb')).toBe('status.do_not_disturb');
    expect(vrcStatusLabelKey('  join   me ')).toBe('status.join_me');
    expect(vrcStatusLabelKey('BUSY')).toBe('status.busy');
  });

  it('未知/空状态回退到一定存在的 key', () => {
    for (const bad of ['sleeping', '', '   ', null, undefined]) {
      expect(typeof resolve(vrcStatusLabelKey(bad))).toBe('string');
      expect(vrcStatusLabelKey(bad)).toBe('status.offline');
    }
  });
});
