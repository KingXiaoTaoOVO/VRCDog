import { describe, expect, it } from 'vitest';
import {
  createVrpianoOverlayPanelStyle,
  DEFAULT_VRPIANO_OVERLAY_BLUR,
  isVrpianoOverlayBlurEnabled,
  normalizeVrpianoOverlayBlur,
  normalizeVrpianoOverlayOpacity,
} from './vrpianoOverlayAppearance';

describe('VRPiano overlay appearance', () => {
  it('clamps opacity and preserves the default for missing values', () => {
    expect(normalizeVrpianoOverlayOpacity(null)).toBe(0.88);
    expect(normalizeVrpianoOverlayOpacity(0.1)).toBe(0.3);
    expect(normalizeVrpianoOverlayOpacity(2)).toBe(1);
  });

  it('uses the default native blur when no saved preference exists', () => {
    expect(normalizeVrpianoOverlayBlur(null)).toBe(DEFAULT_VRPIANO_OVERLAY_BLUR);
    expect(isVrpianoOverlayBlurEnabled(null)).toBe(true);
    expect(isVrpianoOverlayBlurEnabled(0)).toBe(false);
  });

  it('exposes opacity and blur as live panel styles', () => {
    expect(createVrpianoOverlayPanelStyle(0.6, 30)).toEqual({
      '--vrpiano-overlay-opacity': '0.6',
      '--vrpiano-overlay-blur': '30px',
      backdropFilter: 'blur(30px) saturate(160%)',
      WebkitBackdropFilter: 'blur(30px) saturate(160%)',
    });
  });
});
