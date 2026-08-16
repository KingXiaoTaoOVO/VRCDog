import { describe, expect, it } from 'vitest';
import { SUPPORTED_LOCALES } from './languages';
import { drawingMessages } from './drawingMessages';

describe('drawing translations', () => {
  it('covers every supported locale with the complete drawing key set', () => {
    const expected = Object.keys(drawingMessages['en-US'].drawing).sort();
    for (const locale of SUPPORTED_LOCALES) {
      expect(Object.keys(drawingMessages[locale].drawing).sort(), locale).toEqual(expected);
      expect(drawingMessages[locale].sidebar.drawing, locale).toBeTruthy();
    }
  });
});
