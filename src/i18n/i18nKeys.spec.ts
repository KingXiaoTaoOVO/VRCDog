import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';
import zhCN from './locales/zh-CN.json';
import { drawingMessages } from './drawingMessages';

/**
 * 静态校验：代码里用到的每一个 `t('字面量 key')` 都必须真的存在于中文语言包里。
 *
 * 为什么需要这条测试：vue-i18n 在 key 缺失时**返回 key 路径本身**（真值），
 * 所以 `t('export.vrcx_export_btn') || '导出 JSON'` 这种写法根本不会兜底，
 * 界面会直接把 `export.vrcx_export_btn` 当文案渲染出来。
 * 本仓库已经出现过 11 个这样的 key（VRCX 导入导出整块、VRCPiano 日志三处、清空日志按钮），
 * 全部是新增功能时只改了组件、没补语言包，而且没有任何自动化能发现。
 *
 * 中文语言包是所有语种的合并基（`mergeMessages(zhCN, ...)`），
 * 因此只要它齐了，其它语种最坏是回退中文而不是显示裸 key。
 */

const ROOT = join(__dirname, '..');

function collectSourceFiles(dir: string, out: string[] = []): string[] {
  for (const entry of readdirSync(dir)) {
    const full = join(dir, entry);
    const st = statSync(full);
    if (st.isDirectory()) {
      if (entry === 'node_modules' || entry === 'dist' || entry === 'locales') continue;
      collectSourceFiles(full, out);
    } else if (/\.vue$/.test(entry) || (/\.ts$/.test(entry) && !/\.spec\.ts$/.test(entry))) {
      out.push(full);
    }
  }
  return out;
}

/**
 * 只匹配「没有任何兜底」的字面量调用：`t('a.b')` / `$t('a.b')`。
 *
 * 刻意排除两类：
 * - `t('a.b', { count })` 这类带参数的调用（参数不是文案兜底，但正则上先放过，
 *   避免误伤；缺 key 时它们同样会渲染裸 key）；
 * - `tt('a.b', '中文兜底')` —— TranslatorView 里自带的正确兜底 helper
 *   （`const tt = (key, fallback) => { const v = t(key); return v === key ? fallback : v }`）。
 *   注意 `tt(` 里也含一个 `t(`，必须用 lookbehind 卡住标识符边界，否则会把这 22 处
 *   有正确兜底的调用误报成缺陷。
 *
 * 而 `t('a.b') || '兜底'` 这种写法**不在排除范围内**：vue-i18n 缺 key 时返回 key 本身
 * （真值），`||` 永远不会生效，所以它等价于「没有兜底」，正是要抓的目标。
 */
const LITERAL_KEY_RE = /(?<![A-Za-z0-9_$])(?:\$t|t)\(\s*['"]([A-Za-z0-9_]+(?:\.[A-Za-z0-9_]+)+)['"]\s*\)/g;

function resolve(obj: Record<string, any>, key: string): unknown {
  return key.split('.').reduce<unknown>((acc, part) => {
    if (acc && typeof acc === 'object' && part in (acc as Record<string, unknown>)) {
      return (acc as Record<string, unknown>)[part];
    }
    return undefined;
  }, obj);
}

describe('i18n key 完整性', () => {
  const effective = { ...(zhCN as Record<string, any>), drawing: drawingMessages['zh-CN'].drawing };

  const used = new Map<string, string>();
  for (const file of collectSourceFiles(ROOT)) {
    const source = readFileSync(file, 'utf8');
    for (const match of source.matchAll(LITERAL_KEY_RE)) {
      if (!used.has(match[1])) used.set(match[1], file.slice(ROOT.length + 1));
    }
  }

  it('确实扫到了字面量 key（防止正则失效导致测试空转）', () => {
    expect(used.size).toBeGreaterThan(500);
  });

  it('代码里用到的每个 key 都能在中文语言包解析出文案', () => {
    const missing: string[] = [];
    for (const [key, where] of used) {
      const value = resolve(effective, key);
      // 数组型文案（星期名、小时坐标）同样是合法值 —— 取用方必须走 tm()/translateList()。
      const ok = (typeof value === 'string' && value.trim() !== '')
        || (Array.isArray(value) && value.length > 0 && value.every((item) => typeof item === 'string' && item.trim() !== ''));
      if (!ok) missing.push(`${key}  <- ${where}`);
    }
    expect(missing, `裸 key 会被直接渲染到界面上：\n${missing.join('\n')}`).toEqual([]);
  });
});
