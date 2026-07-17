import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
  request: vi.fn(),
  getAuth: vi.fn(),
  notify: vi.fn(),
}));

vi.mock('./index', () => ({
  VrcApi: { request: mocks.request },
  DbApi: {
    getAuth: mocks.getAuth,
    addFriendLog: vi.fn(),
    saveFriend: vi.fn(),
    removeFriend: vi.fn(),
    saveNotification: vi.fn(),
    deleteNotification: vi.fn(),
    recordActivity: vi.fn(),
    getAllSettings: vi.fn().mockResolvedValue({}),
  },
}));

vi.mock('../i18n', () => ({
  translate: (key: string) => key,
}));

vi.mock('../stores/notificationEngine', () => ({
  useNotificationEngine: () => ({ notify: mocks.notify }),
}));

class FakeWebSocket {
  static instances: FakeWebSocket[] = [];

  onopen: ((event: Event) => void) | null = null;
  onclose: ((event: CloseEvent) => void) | null = null;
  onerror: ((event: Event) => void) | null = null;
  onmessage: ((event: MessageEvent) => void) | null = null;

  constructor(public readonly url: string) {
    FakeWebSocket.instances.push(this);
  }

  close(code = 1000, reason = '') {
    this.onclose?.({ code, reason } as CloseEvent);
  }

  emitOpen() {
    this.onopen?.(new Event('open'));
  }

  emitClose(code = 1006, reason = '') {
    this.onclose?.({ code, reason } as CloseEvent);
  }
}

describe('VRChat pipeline reconnect lifecycle', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.resetModules();
    mocks.request.mockReset();
    mocks.getAuth.mockReset().mockResolvedValue('["auth=authcookie_test"]');
    mocks.notify.mockReset();
    FakeWebSocket.instances = [];
    vi.stubGlobal('WebSocket', FakeWebSocket);
    vi.spyOn(console, 'warn').mockImplementation(() => {});
    vi.spyOn(console, 'error').mockImplementation(() => {});
    vi.spyOn(console, 'log').mockImplementation(() => {});
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it('keeps retrying every five seconds after more than five failures', async () => {
    mocks.getAuth.mockRejectedValue(new Error('temporary failure'));
    const { closeWebSocket, initWebsocket, wsState } = await import('./websocket');

    await initWebsocket();
    for (let attempt = 1; attempt <= 7; attempt++) {
      expect(mocks.getAuth).toHaveBeenCalledTimes(attempt);
      expect(wsState.phase).toBe('waiting');
      expect(wsState.reconnectAttempts).toBe(attempt);
      await vi.advanceTimersByTimeAsync(5000);
    }

    expect(mocks.getAuth).toHaveBeenCalledTimes(8);
    closeWebSocket();
  });

  it('ignores an authentication response that finishes after logout', async () => {
    let resolveAuth!: (value: unknown) => void;
    mocks.getAuth.mockReturnValue(new Promise((resolve) => {
      resolveAuth = resolve;
    }));
    const { closeWebSocket, initWebsocket, wsState } = await import('./websocket');

    const pendingInit = initWebsocket();
    closeWebSocket();
    resolveAuth('["auth=stale-token"]');
    await pendingInit;
    await vi.advanceTimersByTimeAsync(10_000);

    expect(FakeWebSocket.instances).toHaveLength(0);
    expect(mocks.getAuth).toHaveBeenCalledTimes(1);
    expect(wsState.phase).toBe('idle');
  });

  it('reconnects after a socket error and resets attempts on open', async () => {
    const { closeWebSocket, initWebsocket, wsState } = await import('./websocket');

    await initWebsocket();
    const firstSocket = FakeWebSocket.instances[0];
    expect(firstSocket.url).toBe(
      'wss://pipeline.vrchat.cloud/?authToken=authcookie_test',
    );
    firstSocket.onerror?.(new Event('error'));
    expect(wsState.phase).toBe('waiting');

    await vi.advanceTimersByTimeAsync(5000);
    expect(FakeWebSocket.instances).toHaveLength(2);
    FakeWebSocket.instances[1].emitOpen();

    expect(wsState.connected).toBe(true);
    expect(wsState.phase).toBe('connected');
    expect(wsState.reconnectAttempts).toBe(0);
    closeWebSocket();
  });
});
