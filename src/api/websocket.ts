import { translate } from '../i18n';
import { reactive } from 'vue';
import { VrcApi, DbApi } from './index';
import { useNotificationEngine } from '../stores/notificationEngine';
import { getCookieValue } from './cookies';

export const wsState = reactive({
  connected: false,
  messageCount: 0,
  bytesReceived: 0,
  lastUpdate: '',
  everConnected: false, // 是否曾经成功连接过（区分"从未连接"与"已断开"）
  phase: 'idle' as 'idle' | 'authenticating' | 'connecting' | 'waiting' | 'connected',
  lastError: '',
  reconnectAttempts: 0,
});

let webSocket: WebSocket | null = null;
let lastMessage = '';
let reconnectTimer: ReturnType<typeof setTimeout> | null = null;
let reconnectAttempts = 0;
let reconnectEnabled = false;
let authenticating = false;
let lifecycleId = 0;
const RECONNECT_DELAY_MS = 5000;
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

function scheduleReconnect(reason?: string) {
  if (!reconnectEnabled || reconnectTimer !== null) return;
  reconnectAttempts++;
  wsState.connected = false;
  wsState.phase = 'waiting';
  wsState.reconnectAttempts = reconnectAttempts;
  if (reason) wsState.lastError = reason;
  console.warn(`[WSS] Pipeline unavailable, retrying in ${RECONNECT_DELAY_MS / 1000}s (attempt ${reconnectAttempts})`, reason || '');

  reconnectTimer = setTimeout(() => {
    reconnectTimer = null;
    void initWebsocket();
  }, RECONNECT_DELAY_MS);
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

  socket.onopen = () => {
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
    wsState.connected = false;
    if (webSocket === socket) webSocket = null;
    console.log('[WSS] Pipeline closed', e.code, e.reason);
    if (!reconnectEnabled || attemptLifecycleId !== lifecycleId) return;
    const reason = e.reason || (e.code ? `WebSocket closed (${e.code})` : 'WebSocket closed');
    scheduleReconnect(reason);
  };

  socket.onerror = (err) => {
    console.error('[WSS] Pipeline error', err);
    wsState.lastError = 'Pipeline WebSocket connection error';
    socket.close();
  };

  socket.onmessage = ({ data }) => {
    wsState.messageCount++;
    wsState.bytesReceived += data.length;
    
    if (lastMessage === data) return;
    lastMessage = data;
    
    try {
      const json = JSON.parse(data);
      if (typeof json.content === 'string') {
        try { json.content = JSON.parse(json.content); } catch {}
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

export function onPipelineMessage(handler: PipelineHandler) {
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

function shouldEmitFriendPresenceNotification(type: string, content: any): boolean {
  const userId = content?.userId || content?.user?.id;
  if (!userId) return false;

  const location = type === 'friend-online' ? getFriendLocation(content) : '';
  const key = `${type}:${userId}:${location}`;
  const now = Date.now();
  const last = friendNotifyTimes.get(key) || 0;
  if (now - last < FRIEND_NOTIFY_DEBOUNCE_MS) return false;

  friendNotifyTimes.set(key, now);
  for (const [cachedKey, timestamp] of friendNotifyTimes) {
    if (now - timestamp > FRIEND_NOTIFY_DEBOUNCE_MS * 4) {
      friendNotifyTimes.delete(cachedKey);
    }
  }
  return true;
}

async function emitFriendPresenceNotification(type: string, content: any) {
  if (type !== 'friend-online' && type !== 'friend-offline') return;
  if (!shouldEmitFriendPresenceNotification(type, content)) return;

  const userId = content?.userId || content?.user?.id;
  const displayName = getFriendDisplayName(content);
  const location = getFriendLocation(content);
  const isOnline = type === 'friend-online';
  const title = isOnline ? `${displayName} 已上线` : `${displayName} 已下线`;
  const detail = isOnline
    ? (location && location !== 'offline' ? `位置：${location}` : '好友现在在线')
    : '好友现在离线';

  await DbApi.saveNotification({
    notificationJson: JSON.stringify({
      id: `${type}:${userId}:${Date.now()}`,
      type,
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
  await notify('VRC 好友状态', `${title}${detail ? ` - ${detail}` : ''}`, isOnline ? 'friend_online' : 'friend_offline');
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
    
    // ====== 1. 好友日志记录 ======
    if (type === 'friend-online') {
      await DbApi.addFriendLog({ eventType: 'online', userId: content.userId, displayName: content.user?.displayName || 'Unknown', detail: content.location });
      await emitFriendPresenceNotification(type, content);
    } else if (type === 'friend-offline') {
      await DbApi.addFriendLog({ eventType: 'offline', userId: content.userId, displayName: content.user?.displayName || 'Unknown', detail: null });
      await emitFriendPresenceNotification(type, content);
    } else if (type === 'friend-location') {
      await DbApi.addFriendLog({ eventType: 'location_change', userId: content.userId, displayName: content.user?.displayName || 'Unknown', detail: content.location });
    } else if (type === 'friend-add') {
      await DbApi.addFriendLog({ eventType: 'friend_add', userId: content.userId, displayName: content.user?.displayName || 'Unknown', detail: null });
    } else if (type === 'friend-delete') {
      await DbApi.addFriendLog({ eventType: 'friend_remove', userId: content.userId, displayName: 'Unknown', detail: null });
    }

    // ====== 2. 离线缓存 (SQLite Friend Caching) 实时同步 ======
    if (['friend-online', 'friend-offline', 'friend-location', 'friend-update', 'friend-add'].includes(type) && content.user) {
      await DbApi.saveFriend({
        userId: content.userId || content.user.id,
        displayName: content.user.displayName,
        status: content.user.status || 'offline',
        location: content.user.location || '',
        friendData: JSON.stringify(content.user)
      });
    } else if (type === 'friend-delete') {
      await DbApi.removeFriend({ userId: content.userId });
    }

    // ====== 3. 离线缓存 (SQLite Notifications) 实时同步 ======
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
      // Depending on API, hide/clear might give notificationId
      if (content.notificationId || content.id) {
         await DbApi.deleteNotification({ id: content.notificationId || content.id });
      }
    }

    // ====== 4. 热力图活动记录 (Heatmap Activity) ======
    if (type === 'friend-online' || type === 'friend-location') {
      await DbApi.recordActivity({
        userId: content.userId,
        displayName: content.user?.displayName || 'Unknown',
        status: content.user?.status || 'online',
        location: content.location
      });
    }

  } catch (err) {
    console.warn(translate('debug.log_write_fail'), err);
  }

  for (const handler of handlers) {
    try { handler(json); } catch (e) { console.error('Handler error', e); }
  }
  
  window.dispatchEvent(new CustomEvent('vrc-pipeline-event', { detail: json }));
}
