import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
  request: vi.fn(),
  getAuth: vi.fn(),
  notify: vi.fn(),
  updateFriend: vi.fn(),
  addFriend: vi.fn(),
  removeFriend: vi.fn(),
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

vi.mock('../stores/friendsStore', () => ({
  useFriendsStore: () => ({
    updateFriend: mocks.updateFriend,
    addFriend: mocks.addFriend,
    removeFriend: mocks.removeFriend,
  }),
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

  emitMessage(data: string) {
    this.onmessage?.({ data } as MessageEvent);
  }
}

describe('VRChat pipeline reconnect lifecycle', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.resetModules();
    mocks.request.mockReset();
    mocks.getAuth.mockReset().mockResolvedValue('["auth=authcookie_test"]');
    mocks.notify.mockReset();
    mocks.updateFriend.mockReset();
    mocks.addFriend.mockReset();
    mocks.removeFriend.mockReset();
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

  it('keeps retrying with exponential backoff after failures', async () => {
    mocks.getAuth.mockRejectedValue(new Error('temporary failure'));
    const { closeWebSocket, initWebsocket, wsState } = await import('./websocket');

    await initWebsocket();
    expect(mocks.getAuth).toHaveBeenCalledTimes(1);
    expect(wsState.phase).toBe('waiting');
    expect(wsState.reconnectAttempts).toBe(1);

    // Advance by 2.5s — first backoff fires at ~2s
    await vi.advanceTimersByTimeAsync(2_500);
    expect(mocks.getAuth).toHaveBeenCalledTimes(2);
    expect(wsState.phase).toBe('waiting');
    expect(wsState.reconnectAttempts).toBe(2);

    // Advance by 4s — second backoff fires at ~3s (2*1.5)
    await vi.advanceTimersByTimeAsync(4_000);
    expect(mocks.getAuth).toHaveBeenCalledTimes(3);
    expect(wsState.reconnectAttempts).toBe(3);

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

    // Backoff is 2000*1.5^1 + jitter = ~3000-3900ms for first reconnect
    await vi.advanceTimersByTimeAsync(4_500);
    expect(FakeWebSocket.instances).toHaveLength(2);
    FakeWebSocket.instances[1].emitOpen();

    expect(wsState.connected).toBe(true);
    expect(wsState.phase).toBe('connected');
    expect(wsState.reconnectAttempts).toBe(0);
    closeWebSocket();
  });

  it('updates a friend location from a compact pipeline event', async () => {
    const { closeWebSocket, initWebsocket } = await import('./websocket');

    await initWebsocket();
    FakeWebSocket.instances[0].emitOpen();
    FakeWebSocket.instances[0].emitMessage(JSON.stringify({
      type: 'friend-location',
      content: { userId: 'usr_friend', location: 'wrld_example:123' },
    }));
    await vi.advanceTimersByTimeAsync(0);

    expect(mocks.updateFriend).toHaveBeenCalledWith('usr_friend', {
      id: 'usr_friend',
      location: 'wrld_example:123',
      status: 'online',
    });
    closeWebSocket();
  });
});
