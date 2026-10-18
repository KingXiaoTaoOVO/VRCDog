import { describe, expect, it } from 'vitest';
import { katakanaToHiragana, toRomaji } from './transliteration';

describe('transliteration', () => {
  it('converts katakana to hiragana', () => {
    expect(katakanaToHiragana('コンニチハ')).toBe('こんにちは');
    expect(katakanaToHiragana('ラーメン')).toBe('らーめん');
  });

  it('converts hiragana and katakana to romaji', () => {
    expect(toRomaji('こんにちは')).toBe('konnichiha');
    expect(toRomaji('ありがとう')).toBe('arigatou');
    expect(toRomaji('ちょっと')).toBe('chotto');
    expect(toRomaji('ラーメン')).toBe('ra-men');
  });
});
