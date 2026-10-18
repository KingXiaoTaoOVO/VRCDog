import { beforeAll, describe, expect, it } from 'vitest';

/**
 * 数组型文案（星期名、小时坐标）必须用 tm() 取。
 *
 * 回归的缺陷：`ChartsView.vue` 原本写的是
 * `v-for="(label, idx) in t('charts.hours_short')"`，而 vue-i18n 的 `t()` 对数组消息
 * 返回的是 **key 字符串本身**，Vue 的 v-for 会逐字符遍历它，
 * 于是小时坐标渲染成 "c h a r t s . h o u r s _ s h o r t" 这样一串垃圾标签。
 * 同文件里 `Array.isArray(t('charts.days'))` 也恒为 false，数组文案从来没生效过。
 */

let translateList: (key: string, fallback: string[]) => string[];

beforeAll(async () => {
  // i18n/index 在模块加载时会读 localStorage 里的语言偏好；jsdom worker 里这个全局
  // 不一定存在，先补一个最小实现再动态 import。
  if (typeof globalThis.localStorage === 'undefined') {
    const store = new Map<string, string>();
    (globalThis as any).localStorage = {
      getItem: (k: string) => (store.has(k) ? store.get(k)! : null),
      setItem: (k: string, v: string) => void store.set(k, String(v)),
      removeItem: (k: string) => void store.delete(k),
      clear: () => store.clear(),
    };
  }
  const mod = await import('./index');
  translateList = mod.translateList;
});

describe('translateList 取数组型文案', () => {
  it('返回真正的数组，而不是会被逐字符遍历的 key 字符串', () => {
    const hours = translateList('charts.hours_short', ['X']);
    expect(Array.isArray(hours)).toBe(true);
    expect(hours).toHaveLength(6);
    expect(hours.every((item) => typeof item === 'string' && item.trim() !== '')).toBe(true);
    expect(hours).not.toContain('charts.hours_short');

    expect(translateList('charts.days', ['X'])).toHaveLength(7);
    expect(translateList('dashboard.days', ['X'])).toHaveLength(7);
  });

  it('key 不存在时退回调用方给的兜底数组', () => {
    expect(translateList('charts.this_key_does_not_exist', ['a', 'b'])).toEqual(['a', 'b']);
  });

  it('兜底数组原样返回，不会退化成单字符', () => {
    const fallback = ['0', '4', '8', '12', '16', '20'];
    expect(translateList('charts.missing_axis', fallback)).toBe(fallback);
  });
});
