import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

/**
 * 翻译官页面曾经整页写死深色（slate-900 / slate-950 / amber-*），
 * 而四套主题全是浅色亚克力，于是浅色页面上出现一整块深色污区。
 * styles.css 里已经为同类问题把 `dark:` 变体改成显式类名触发，这里守住另一半：
 * 页面骨架的中性色与强调色必须走主题 token，语义状态色（emerald / indigo / cyan / rose / red）不受限。
 */
const source = readFileSync(fileURLToPath(new URL('./TranslatorView.vue', import.meta.url)), 'utf-8');
const template = source.slice(source.indexOf('<template>'));

const NEUTRAL = /(?:bg|text|border|placeholder|ring|shadow|divide|from|via|to)-(?:slate|gray|zinc|neutral)-\d{2,3}/g;
const HARDCODED_ACCENT = /(?:bg|text|border|accent|ring|shadow)-(?:amber|yellow|orange)-\d{2,3}/g;
const HEX_IN_TEMPLATE = /(?:fill|stroke|color)="#[0-9a-fA-F]{3,8}"/g;

describe('translator view theming', () => {
  it('keeps neutral chrome on theme tokens', () => {
    expect(template.match(NEUTRAL) ?? []).toEqual([]);
  });

  it('keeps the accent on the theme primary instead of a fixed amber', () => {
    expect(template.match(HARDCODED_ACCENT) ?? []).toEqual([]);
  });

  it('does not paint the routing diagram with literal colors', () => {
    // SVG 的静态属性写死颜色就不会跟随主题；绑定值里的语义状态色（如 '#10b981'）是允许的。
    expect(template.match(HEX_IN_TEMPLATE) ?? []).toEqual([]);
  });
});
