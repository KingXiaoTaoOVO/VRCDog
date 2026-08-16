import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

// Event listeners registered by the bridge are captured here so tests can drive
// the frontend `wsState` exactly as the Rust pipeline task would.
const listeners = vi.hoisted(() => ({} as Record<string, (event: { payload: any }) => void>));

const mocks = vi.hoisted(() => ({
  request: vi.fn(),
  getAuth: vi.fn(),
  getSetting: vi.fn(),
  startPipelineWs: vi.fn().mockResolvedValue(undefined),
  stopPipelineWs: vi.fn().mockResolvedValue(undefined),
  getUser: vi.fn(),
  getCachedFriends: vi.fn(),
  addFriendLog: vi.fn(),
  saveNotification: vi.fn(),
  notify: vi.fn(),
  updateFriend: vi.fn(),
  addFriend: vi.fn(),
  removeFriend: vi.fn(),
  allFriends: [] as any[],
  markDataHealthy: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  isTauri: () => true,
  invoke: vi.fn(),
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(async (event: string, handler: (e: { payload: any }) => void) => {
    listeners[event] = handler;
    return () => {
      delete listeners[event];
    };
  }),
}));

vi.mock('./index', () => ({
  VrcApi: {
    request: mocks.request,
    startPipelineWs: mocks.startPipelineWs,
    stopPipelineWs: mocks.stopPipelineWs,
    getUser: mocks.getUser,
  },
  DbApi: {
    getAuth: mocks.getAuth,
    getSetting: mocks.getSetting,
    addFriendLog: mocks.addFriendLog,
    getCachedFriends: mocks.getCachedFriends,
    saveFriend: vi.fn(),
    removeFriend: vi.fn(),
    saveNotification: mocks.saveNotification,
    deleteNotification: vi.fn(),
    recordActivity: vi.fn(),
    getAllSettings: vi.fn().mockResolvedValue({}),
  },
}));

vi.mock('../stores/dataHealth', () => ({
  markDataHealthy: mocks.markDataHealthy,
  dataHealth: { lastSuccessAt: null },
  dataServiceStatus: { value: 'offline' },
  nowTs: { value: 0 },
}));

vi.mock('../i18n', () => ({
  translate: (key: string) => key,
}));

vi.mock('../stores/notificationEngine', () => ({
  useNotificationEngine: () => ({ notify: mocks.notify }),
}));

vi.mock('../stores/friendsStore', () => ({
  useFriendsStore: () => ({
    allFriends: mocks.allFriends,
    updateFriend: mocks.updateFriend,
    addFriend: mocks.addFriend,
    removeFriend: mocks.removeFriend,
  }),
}));

const flush = () => new Promise<void>((resolve) => setTimeout(resolve, 0));

describe('VRChat pipeline native bridge', () => {
  beforeEach(() => {
    vi.resetModules();
    mocks.request.mockReset();
    mocks.getAuth.mockReset().mockResolvedValue('["auth=authcookie_test"]');
    mocks.getSetting.mockReset().mockResolvedValue(null);
    mocks.startPipelineWs.mockReset().mockResolvedValue(undefined);
    mocks.stopPipelineWs.mockReset().mockResolvedValue(undefined);
    mocks.getUser.mockReset().mockRejectedValue(new Error('not found'));
    mocks.getCachedFriends.mockReset().mockResolvedValue([]);
    mocks.addFriendLog.mockReset().mockResolvedValue(undefined);
    mocks.saveNotification.mockReset().mockResolvedValue(undefined);
    mocks.notify.mockReset();
    mocks.updateFriend.mockReset();
    mocks.addFriend.mockReset();
    mocks.removeFriend.mockReset();
    mocks.allFriends = [];
    mocks.markDataHealthy.mockReset();
    listeners['pipeline_ws_status'] = undefined as any;
    listeners['pipeline_ws_message'] = undefined as any;
    vi.spyOn(console, 'error').mockImplementation(() => {});
    vi.spyOn(console, 'warn').mockImplementation(() => {});
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('starts the native pipeline with the auth token from the cookie', async () => {
    const { initWebsocket, wsState } = await import('./websocket');

    await initWebsocket();

    expect(mocks.startPipelineWs).toHaveBeenCalledTimes(1);
    expect(mocks.startPipelineWs).toHaveBeenCalledWith({ authToken: 'authcookie_test', pipelineUrl: 'wss://pipeline.vrchat.cloud' });
    expect(wsState.phase).toBe('authenticating');
  });

  it('forwards a user-configured pipeline URL to the backend', async () => {
    mocks.getSetting.mockResolvedValue('wss://mirror.example.com:8443');
    const { initWebsocket } = await import('./websocket');

    await initWebsocket();

    expect(mocks.startPipelineWs).toHaveBeenCalledWith({ authToken: 'authcookie_test', pipelineUrl: 'wss://mirror.example.com:8443' });
  });

  it('falls back to the default pipeline URL when the saved value is blank', async () => {
    mocks.getSetting.mockResolvedValue('   ');
    const { initWebsocket } = await import('./websocket');

    await initWebsocket();

    expect(mocks.startPipelineWs).toHaveBeenCalledWith({ authToken: 'authcookie_test', pipelineUrl: 'wss://pipeline.vrchat.cloud' });
  });

  it('reflects a connected status emitted by the backend', async () => {
    const { initWebsocket, wsState } = await import('./websocket');

    await initWebsocket();
    listeners['pipeline_ws_status']!({ payload: { phase: 'connected', connected: true, messageCount: 0, reconnectAttempts: 0 } });

    expect(wsState.phase).toBe('connected');
    expect(wsState.connected).toBe(true);
    expect(wsState.everConnected).toBe(true);
  });

  it('reflects reconnect/waiting status with the attempt count', async () => {
    const { initWebsocket, wsState } = await import('./websocket');

    await initWebsocket();
    listeners['pipeline_ws_status']!({ payload: { phase: 'waiting', connected: false, messageCount: 0, reconnectAttempts: 2 } });

    expect(wsState.phase).toBe('waiting');
    expect(wsState.reconnectAttempts).toBe(2);
  });

  it('reflects an unavailable/failed status from the backend', async () => {
    const { initWebsocket, wsState } = await import('./websocket');

    await initWebsocket();
    listeners['pipeline_ws_status']!({ payload: { phase: 'failed', connected: false, messageCount: 0, reconnectAttempts: 5, lastError: 'boom' } });

    expect(wsState.phase).toBe('failed');
    expect(wsState.lastError).toBe('boom');
  });

  it('fails without an auth token and never starts the pipeline', async () => {
    mocks.getAuth.mockResolvedValue(null);
    const { initWebsocket, wsState } = await import('./websocket');

    await initWebsocket();

    expect(mocks.startPipelineWs).not.toHaveBeenCalled();
    expect(wsState.phase).toBe('failed');
  });

  it('parses a pipeline message into a friend location update', async () => {
    const { initWebsocket } = await import('./websocket');

    await initWebsocket();
    listeners['pipeline_ws_message']!({
      payload: JSON.stringify({
        type: 'friend-location',
        content: { userId: 'usr_friend', location: 'wrld_example:123' },
      }),
    });
    await flush();

    expect(mocks.updateFriend).toHaveBeenCalled();
    const call = mocks.updateFriend.mock.calls[0];
    expect(call[0]).toBe('usr_friend');
    expect(call[1].location).toBe('wrld_example:123');
  });

  it('uses the cached friend name when an offline event only contains a user id', async () => {
    mocks.allFriends = [{ id: 'usr_friend', displayName: 'Alice' }];
    const { initWebsocket } = await import('./websocket');

    await initWebsocket();
    listeners['pipeline_ws_message']!({
      payload: JSON.stringify({
        type: 'friend-offline',
        content: { userId: 'usr_friend' },
      }),
    });
    await flush();
    await flush();

    expect(mocks.addFriendLog).toHaveBeenCalledWith(expect.objectContaining({
      eventType: 'offline',
      userId: 'usr_friend',
      displayName: 'Alice',
    }));
    expect(mocks.notify).toHaveBeenCalledWith(
      'VRC 好友状态',
      expect.stringContaining('Alice 已下线'),
      'friend_offline',
    );
    const saved = JSON.parse(mocks.saveNotification.mock.calls[0][0].notificationJson);
    expect(saved.senderUsername).toBe('Alice');
  });

  it('retains a name from a full presence event for a later abbreviated offline event', async () => {
    const { initWebsocket } = await import('./websocket');

    await initWebsocket();
    listeners['pipeline_ws_message']!({
      payload: JSON.stringify({
        type: 'friend-online',
        content: {
          userId: 'usr_friend',
          location: 'wrld_example:123',
          user: { id: 'usr_friend', displayName: 'Alice' },
        },
      }),
    });
    await flush();
    mocks.addFriendLog.mockClear();
    mocks.notify.mockClear();

    listeners['pipeline_ws_message']!({
      payload: JSON.stringify({ type: 'friend-offline', content: { userId: 'usr_friend' } }),
    });
    await flush();
    await flush();

    expect(mocks.addFriendLog).toHaveBeenCalledWith(expect.objectContaining({
      eventType: 'offline',
      displayName: 'Alice',
    }));
    expect(mocks.notify).toHaveBeenCalledWith(
      'VRC 好友状态',
      expect.stringContaining('Alice 已下线'),
      'friend_offline',
    );
  });

  it('marks the data service healthy on any inbound pipeline message', async () => {
    const { initWebsocket } = await import('./websocket');

    await initWebsocket();
    mocks.markDataHealthy.mockClear();
    listeners['pipeline_ws_message']!({
      payload: JSON.stringify({ type: 'notification', content: { id: 'n1' } }),
    });
    await flush();

    expect(mocks.markDataHealthy).toHaveBeenCalled();
  });

  it('stops the backend and resets state on close', async () => {
    const { initWebsocket, closeWebSocket, wsState } = await import('./websocket');

    await initWebsocket();
    listeners['pipeline_ws_status']!({ payload: { phase: 'connected', connected: true, messageCount: 3, reconnectAttempts: 0 } });
    closeWebSocket();
    await flush();

    expect(mocks.stopPipelineWs).toHaveBeenCalledTimes(1);
    expect(wsState.phase).toBe('idle');
    expect(wsState.connected).toBe(false);
    expect(wsState.messageCount).toBe(0);
  });
});
