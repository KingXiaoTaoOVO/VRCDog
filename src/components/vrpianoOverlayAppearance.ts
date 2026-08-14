export const VRPIANO_OVERLAY_OPACITY_KEY = 'vrcdog.vrpiano.overlay.opacity';
export const VRPIANO_OVERLAY_BLUR_KEY = 'vrcdog.vrpiano.overlay.blur';

export const DEFAULT_VRPIANO_OVERLAY_OPACITY = 0.88;
export const DEFAULT_VRPIANO_OVERLAY_BLUR = 20;

export const normalizeVrpianoOverlayOpacity = (value: unknown) => {
  const parsed = value === null || value === '' ? Number.NaN : Number(value);
  const opacity = Number.isFinite(parsed) ? parsed : DEFAULT_VRPIANO_OVERLAY_OPACITY;
  return Math.min(1, Math.max(0.3, opacity));
};

export const normalizeVrpianoOverlayBlur = (value: unknown) => {
  const parsed = value === null || value === '' ? Number.NaN : Number(value);
  const blur = Number.isFinite(parsed) ? parsed : DEFAULT_VRPIANO_OVERLAY_BLUR;
  return Math.min(40, Math.max(0, blur));
};

export const isVrpianoOverlayBlurEnabled = (value: unknown) => normalizeVrpianoOverlayBlur(value) > 0;

export const createVrpianoOverlayPanelStyle = (opacityValue: unknown, blurValue: unknown) => {
  const opacity = normalizeVrpianoOverlayOpacity(opacityValue);
  const blur = normalizeVrpianoOverlayBlur(blurValue);
  const backdropFilter = `blur(${blur}px) saturate(160%)`;

  return {
    '--vrpiano-overlay-opacity': String(opacity),
    '--vrpiano-overlay-blur': `${blur}px`,
    backdropFilter,
    WebkitBackdropFilter: backdropFilter,
  };
};
