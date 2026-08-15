import { flushPromises, mount } from '@vue/test-utils';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const eventMocks = vi.hoisted(() => {
  const handlers = new Map<string, (event: any) => void>();
  return {
    handlers,
    listen: vi.fn(async (event: string, handler: (payload: any) => void) => {
      handlers.set(event, handler);
      return vi.fn();
    }),
  };
});

vi.hoisted(() => {
  const values = new Map<string, string>();
  const storage = {
    getItem: (key: string) => values.get(key) ?? null,
    setItem: (key: string, value: string) => values.set(key, String(value)),
    removeItem: (key: string) => values.delete(key),
    clear: () => values.clear(),
    key: (index: number) => Array.from(values.keys())[index] ?? null,
    get length() {
      return values.size;
    },
  };

  Object.defineProperty(globalThis, 'localStorage', {
    configurable: true,
    value: storage,
  });
});

vi.mock('@tauri-apps/api/event', () => ({
  listen: eventMocks.listen,
}));

vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => ({
    destroy: vi.fn(),
    startDragging: vi.fn(),
  }),
}));

vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (key: string) => key,
  }),
}));

vi.mock('../api', () => ({
  VrctApi: {
    getHistory: vi.fn(async () => []),
  },
}));

import OverlayView from './OverlayView.vue';

describe('OverlayView background opacity', () => {
  beforeEach(() => {
    localStorage.clear();
    eventMocks.handlers.clear();
    eventMocks.listen.mockClear();
  });

  it('updates the transparent background from live settings events', async () => {
    const wrapper = mount(OverlayView);
    await flushPromises();

    expect(wrapper.attributes('style')).toContain('rgba(0, 0, 0, 0.82)');

    eventMocks.handlers.get('translation-overlay-settings')?.({
      payload: { backgroundOpacity: 0.25 },
    });
    await flushPromises();

    expect(wrapper.attributes('style')).toContain('rgba(0, 0, 0, 0.25)');
  });
});
