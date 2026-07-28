import { translate } from '../i18n';
import { reactive } from 'vue';
import { VrcApi, DbApi } from './index';
import { useNotificationEngine } from '../stores/notificationEngine';
import { useFriendsStore } from '../stores/friendsStore';
import { getCookieValue } from './cookies';

export const wsState = reactive({
  connected: false,
  messageCount: 0,
  bytesReceived: 0,
  lastUpdate: '',
  everConnected: false,
  phase: 'idle' as 'idle' | 'authenticating' | 'connecting' | 'waiting' | 'connected',
  lastError: '',
  reconnectAttempts: 0,
});

let webSocket: WebSocket | null = null;
let reconnectTimer: ReturnType<typeof setTimeout> | null = null;
let reconnectAttempts = 0;
let reconnectEnabled = false;
let authenticating = false;
let lifecycleId = 0;

const RECONNECT_BASE_DELAY_MS = 2000;
const RECONNECT_MAX_DELAY_MS = 60000;
const MAX_RECONNECT_ATTEMPTS = 100;
const FRIEND_NOTIFY_DEBOUNCE_MS = 45_000;
const friendNotifyTimes = new Map<string, number>();

function errorMessage(err: unknown): string {
  if (err instanceof Error) return err.message;
  return typeof err === 'string' ? err : 'Unknown pipeline error';
}

function clearReconnectTimer() {
  if (reconnectTimer !== null) {
    clearTimeout(reconnectTimer);
    reconnectTimer = null;
  }
}

function getReconnectDelay(): number {
  // First retry starts at the documented base delay, then backs off.
  const exponent = Math.max(0, reconnectAttempts - 1);
  const base = Math.min(RECONNECT_BASE_DELAY_MS * Math.pow(1.5, exponent), RECONNECT_MAX_DELAY_MS);
  const jitter = Math.random() * 0.25 * base;
  return Math.floor(base + jitter);
}

function scheduleReconnect(reason?: string) {
  if (!reconnectEnabled || reconnectTimer !== null) return;
  if (reconnectAttempts >= MAX_RECONNECT_ATTEMPTS) {
    console.error(`[WSS] Max reconnect attempts (${MAX_RECONNECT_ATTEMPTS}) reached, giving up`);
    wsState.lastError = `Max reconnect attempts reached: ${reason || ''}`;
    return;
  }

  reconnectAttempts++;
  wsState.connected = false;
  wsState.phase = 'waiting';
  wsState.reconnectAttempts = reconnectAttempts;
  if (reason) wsState.lastError = reason;

  const delay = getReconnectDelay();
  console.warn(`[WSS] Pipeline unavailable, retrying in ${(delay / 1000).toFixed(1)}s (attempt ${reconnectAttempts})`, reason || '');

  reconnectTimer = setTimeout(() => {
    reconnectTimer = null;
    void initWebsocket();
  }, delay);
}

export async function initWebsocket() {
  reconnectEnabled = true;
  if (webSocket !== null || authenticating) return;

  clearReconnectTimer();
  const attemptLifecycleId = lifecycleId;
  authenticating = true;
  wsState.phase = 'authenticating';

  try {
    let authCookie = await DbApi.getAuth();
    let token = getCookieValue(authCookie, 'auth');

    if (!token) {
      await VrcApi.request('/auth/user', {
        method: 'GET',
        suppressAuthExpired: true,
      });
      authCookie = await DbApi.getAuth();
      token = getCookieValue(authCookie, 'auth');
    }

    if (!reconnectEnabled || attemptLifecycleId !== lifecycleId) return;
    if (!token) throw new Error('VRChat auth cookie is missing');
    connectWebSocket(token, attemptLifecycleId);
  } catch (err) {
    if (!reconnectEnabled || attemptLifecycleId !== lifecycleId) return;
    const message = errorMessage(err);
    console.error('[WSS] Pipeline authentication failed:', err);
    scheduleReconnect(message);
  } finally {
    if (attemptLifecycleId === lifecycleId) authenticating = false;
  }
}

function connectWebSocket(token: string, attemptLifecycleId: number) {
  if (!reconnectEnabled || attemptLifecycleId !== lifecycleId || webSocket !== null) return;

  wsState.phase = 'connecting';
  const socket = new WebSocket(
    `wss://pipeline.vrchat.cloud/?authToken=${encodeURIComponent(token)}`,
  );
  webSocket = socket;

  // Connection timeout - detect hung handshakes
  const handshakeTimeout = setTimeout(() => {
    if (wsState.phase === 'connecting' && socket === webSocket) {
      console.warn('[WSS] WebSocket handshake timeout');
      socket.close();
    }
  }, 15000);

  socket.onopen = () => {
    clearTimeout(handshakeTimeout);
    if (!reconnectEnabled || attemptLifecycleId !== lifecycleId) {
      socket.close();
      return;
    }
    wsState.connected = true;
    wsState.everConnected = true;
    wsState.phase = 'connected';
    wsState.lastError = '';
    reconnectAttempts = 0;
    wsState.reconnectAttempts = 0;
    console.log('[WSS] Pipeline connected');
  };

  socket.onclose = (e) => {
    clearTimeout(handshakeTimeout);
    wsState.connected = false;
    if (webSocket === socket) webSocket = null;
    console.log('[WSS] Pipeline closed', e.code, e.reason);
    if (!reconnectEnabled || attemptLifecycleId !== lifecycleId) return;
    const reason = e.reason || (e.code ? `WebSocket closed (${e.code})` : 'WebSocket closed');
    scheduleReconnect(reason);
  };

  socket.onerror = (err) => {
    clearTimeout(handshakeTimeout);
    console.error('[WSS] Pipeline error', err);
    wsState.lastError = 'Pipeline WebSocket connection error';
    socket.close();
  };

  socket.onmessage = ({ data }) => {
    wsState.messageCount++;
    wsState.bytesReceived += data.length;

    try {
      const json = JSON.parse(data);
      if (typeof json.content === 'string') {
        try { json.content = JSON.parse(json.content); } catch { /* not JSON, keep as string */ }
      }
      handlePipeline(json);
    } catch (e) {
      console.error('[WSS] Parse error', e);
    }
  };
}

export function closeWebSocket() {
  reconnectEnabled = false;
  lifecycleId++;
  authenticating = false;
  clearReconnectTimer();
  const socket = webSocket;
  webSocket = null;
  if (socket) socket.close(1000, 'Client logout');
  wsState.connected = false;
  wsState.phase = 'idle';
  wsState.lastError = '';
  reconnectAttempts = 0;
  wsState.reconnectAttempts = 0;
}

type PipelineHandler = (json: any) => void;
const handlers: PipelineHandler[] = [];
const MAX_HANDLERS = 50;

export function onPipelineMessage(handler: PipelineHandler): () => void {
  if (handlers.length >= MAX_HANDLERS) {
    console.warn('[WSS] Max pipeline handlers reached, consider cleaning up subscriptions');
  }
  handlers.push(handler);
  return () => {
    const idx = handlers.indexOf(handler);
    if (idx > -1) handlers.splice(idx, 1);
  };
}

function getFriendDisplayName(content: any): string {
  return content?.user?.displayName || content?.displayName || 'Unknown';
}

function getFriendLocation(content: any): string {
  return content?.location || content?.user?.location || '';
}

function friendPatchFromPipeline(type: string, content: any): { userId: string; patch: Record<string, unknown> } | null {
  const userId = content?.userId || content?.user?.id;
  if (!userId) return null;

  const user = content?.user || {};
  const location = type === 'friend-offline' ? 'offline' : getFriendLocation(content) || user.location;
  return {
    userId,
    patch: {
      ...user,
      id: userId,
      location,
      status: type === 'friend-offline' ? 'offline' : user.status || 'online',
    },
  };
}

function shouldEmitFriendPresenceNotification(type: string, content: any): boolean {
  const userId = content?.userId || content?.user?.id;
  if (!userId) return false;

  const location = type === 'friend-online' ? getFriendLocation(content) : '';
  const key = `${type}:${userId}:${location}`;
  const now = Date.now();
  const last = friendNotifyTimes.get(key) || 0;
  if (now - last < FRIEND_NOTIFY_DEBOUNCE_MS) return false;

  friendNotifyTimes.set(key, now);

  // Periodic cleanup of stale entries
  if (friendNotifyTimes.size > 200) {
    for (const [cachedKey, timestamp] of friendNotifyTimes) {
      if (now - timestamp > FRIEND_NOTIFY_DEBOUNCE_MS * 4) {
        friendNotifyTimes.delete(cachedKey);
      }
    }
  }

  return true;
}

async function emitFriendPresenceNotification(type: string, content: any) {
  if (type !== 'friend-online' && type !== 'friend-offline' && type !== 'friend-location') return;
  if (!shouldEmitFriendPresenceNotification(type, content)) return;

  const userId = content?.userId || content?.user?.id;
  if (!userId) return;

  const displayName = getFriendDisplayName(content);
  const location = getFriendLocation(content);

  let title: string;
  let detail: string;
  let notifType: string;

  if (type === 'friend-online') {
    title = `${displayName} 已上线`;
    detail = location && location !== 'offline' ? `位置：${location}` : '好友现在在线';
    notifType = 'friend_online';
  } else if (type === 'friend-offline') {
    title = `${displayName} 已下线`;
    detail = '好友现在离线';
    notifType = 'friend_offline';
  } else {
    // friend-location: friend moved to a new world
    if (!location || location === 'offline' || location === 'private') return;
    title = `${displayName} 切换了世界`;
    detail = `新位置：${location}`;
    notifType = 'friend_location';
  }

  try {
    await DbApi.saveNotification({
      notificationJson: JSON.stringify({
        id: `${type}:${userId}:${Date.now()}`,
        type: notifType,
        senderUserId: userId,
        senderUsername: displayName,
        receiverUserId: null,
        message: title,
        details: JSON.stringify({
          source: 'pipeline',
          userId,
          displayName,
          location,
          message: detail
        }),
        created_at: new Date().toISOString()
      })
    });

    window.dispatchEvent(new CustomEvent('vrc-notifications-synced'));

    const settings = await DbApi.getAllSettings().catch(() => ({} as Record<string, unknown>));
    if (settings.notifyFriendsOnline === false || settings.notifyFriendsOnline === 'false') return;

    const { notify } = useNotificationEngine();
    await notify('VRC 好友状态', `${title}${detail ? ` - ${detail}` : ''}`, notifType as any);
  } catch (err) {
    console.warn('[WSS] Failed to emit friend notification:', err);
  }
}

function notificationTitle(content: any) {
  const sender = content?.senderUsername || content?.senderDisplayName || content?.senderUserId || 'VRChat';
  const message = typeof content?.message === 'string' ? content.message.trim() : '';
  if (message) return message;
  switch (content?.type) {
    case 'friendRequest': return `${sender} 发送了好友请求`;
    case 'invite': return `${sender} 邀请你加入房间`;
    case 'requestInvite': return `${sender} 请求加入你的位置`;
    case 'group.invite': return `${sender} 发送了群组邀请`;
    case 'group.request': return `${sender} 发送了群组申请`;
    default: return sender;
  }
}

function notificationBody(content: any) {
  if (typeof content?.details === 'string') {
    try {
      const parsed = JSON.parse(content.details);
      return parsed?.worldName || parsed?.message || parsed?.location || content.details;
    } catch {
      return content.details;
    }
  }
  const details = content?.details || {};
  return details.worldName || details.message || details.location || content?.type || '';
}

async function handlePipeline(json: any) {
  wsState.lastUpdate = new Date().toLocaleTimeString();

  try {
    const type = json.type;
    const content = json.content;

    // ====== 1. Friend log recording ======
    if (type === 'friend-online') {
      const userId = content.userId || content.user?.id;
      const displayName = content.user?.displayName || 'Unknown';
      if (userId) {
        await DbApi.addFriendLog({ eventType: 'online', userId, displayName, detail: content.location });
      }
      await emitFriendPresenceNotification(type, content);
    } else if (type === 'friend-offline') {
      const userId = content.userId || content.user?.id;
      const displayName = content.user?.displayName || 'Unknown';
      if (userId) {
        await DbApi.addFriendLog({ eventType: 'offline', userId, displayName, detail: null });
      }
      await emitFriendPresenceNotification(type, content);
    } else if (type === 'friend-location') {
      const userId = content.userId || content.user?.id;
      const displayName = content.user?.displayName || 'Unknown';
      if (userId) {
        await DbApi.addFriendLog({ eventType: 'location_change', userId, displayName, detail: content.location });
      }
      await emitFriendPresenceNotification(type, content);
    } else if (type === 'friend-add') {
      const userId = content.userId || content.user?.id;
      const displayName = content.user?.displayName || 'Unknown';
      if (userId) {
        await DbApi.addFriendLog({ eventType: 'friend_add', userId, displayName, detail: null });
      }
    } else if (type === 'friend-delete') {
      const userId = content.userId;
      if (userId) {
        await DbApi.addFriendLog({ eventType: 'friend_remove', userId, displayName: 'Unknown', detail: null });
      }
    }

    // ====== 2. Offline cache (SQLite Friend Caching) real-time sync ======
    const isFriendEvent = ['friend-online', 'friend-offline', 'friend-location', 'friend-update', 'friend-add'].includes(type);
    const pipelineFriend = isFriendEvent ? friendPatchFromPipeline(type, content) : null;
    if (pipelineFriend) {
      try {
        const friendsStore = useFriendsStore();
        if (type === 'friend-add' && content.user) {
          friendsStore.addFriend(pipelineFriend.patch as any);
        } else {
          friendsStore.updateFriend(pipelineFriend.userId, pipelineFriend.patch as any);
        }
      } catch { /* store may not be initialized yet */ }

      // Pipeline status events sometimes only include userId and location. Only
      // persist complete snapshots so an abbreviated event cannot erase cached
      // profile fields, while still updating the reactive store above.
      if (content.user) {
        await DbApi.saveFriend({
          userId: pipelineFriend.userId,
          displayName: content.user.displayName,
          status: String(pipelineFriend.patch.status || 'offline'),
          location: String(pipelineFriend.patch.location || ''),
          friendData: JSON.stringify(pipelineFriend.patch)
        });
      }
    } else if (type === 'friend-delete') {
      if (content.userId) {
        await DbApi.removeFriend({ userId: content.userId });
        try {
          const friendsStore = useFriendsStore();
          friendsStore.removeFriend(content.userId);
        } catch { /* store may not be initialized yet */ }
      }
    }

    // ====== 3. Offline cache (SQLite Notifications) real-time sync ======
    if (type === 'notification') {
      const details = typeof content.details === 'object' ? JSON.stringify(content.details || {}) : (content.details || '');
      const hasContent = Boolean((content.message || '').trim() || details.trim() || content.senderUsername);
      if (content.id && content.type && hasContent) {
        await DbApi.saveNotification({
          notificationJson: JSON.stringify({
            id: content.id,
            type: content.type,
            senderUserId: content.senderUserId,
            senderUsername: content.senderUsername,
            receiverUserId: content.receiverUserId,
            message: content.message || '',
            details,
            created_at: content.created_at || new Date().toISOString()
          })
        });
        const { notify } = useNotificationEngine();
        await notify(notificationTitle(content), notificationBody(content), 'invite');
      }
    } else if (type === 'hide-notification' || type === 'clear-notification') {
      if (content.notificationId || content.id) {
         await DbApi.deleteNotification({ id: content.notificationId || content.id });
      }
    }

    // ====== 4. Heatmap activity recording ======
    if (type === 'friend-online' || type === 'friend-location') {
      if (content.userId) {
        await DbApi.recordActivity({
          userId: content.userId,
          displayName: content.user?.displayName || 'Unknown',
          status: content.user?.status || 'online',
          location: content.location
        });
      }
    }

  } catch (err) {
    console.warn(translate('debug.log_write_fail'), err);
  }

  // Execute all registered handlers
  for (const handler of handlers) {
    try { handler(json); } catch (e) { console.error('[WSS] Handler error', e); }
  }

  window.dispatchEvent(new CustomEvent('vrc-pipeline-event', { detail: json }));
}
