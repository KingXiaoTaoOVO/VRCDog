import { translate } from '../i18n';
import { reactive } from 'vue';
import { VrcApi, DbApi } from './index';
import { useNotificationEngine } from '../stores/notificationEngine';

export const wsState = reactive({
  connected: false,
  messageCount: 0,
  bytesReceived: 0,
  lastUpdate: '',
  everConnected: false, // 是否曾经成功连接过（区分"从未连接"与"已断开"）
});

let webSocket: WebSocket | null = null;
let lastMessage = '';
let reconnectTimer: number | null = null;
let reconnectAttempts = 0;
let wsShuttingDown = false; // 阻止 initWebsocket 在 closeWebSocket 之后继续执行
const MAX_RECONNECT_ATTEMPTS = 5; // 最多重连 5 次，之后彻底停止
const RECONNECT_BASE_MS = 5000; // 首次重连间隔 5 秒
const RECONNECT_MAX_MS = 60000; // 最长间隔 60 秒
const FRIEND_NOTIFY_DEBOUNCE_MS = 45_000;
const friendNotifyTimes = new Map<string, number>();

/**
 * 指数退避 + 随机抖动，防止同时冲击服务器
 * 第 1 次: 5s
 * 第 2 次: 10s
 * 第 3 次: 20s
 * 第 4 次: 40s
 * 第 5 次: 60s (max)
 */
function getReconnectDelay(): number {
  const delay = Math.min(RECONNECT_BASE_MS * Math.pow(2, reconnectAttempts - 1), RECONNECT_MAX_MS);
  // 添加 ±20% 随机抖动
  const jitter = delay * 0.2 * (Math.random() * 2 - 1);
  return Math.round(delay + jitter);
}

export async function initWebsocket() {
  if (webSocket !== null) return;
  // 防止 closeWebSocket 之后 in-flight 的异步操作继续连接
  if (wsShuttingDown) return;
  
  // 超过最大重连次数，不再尝试
  if (reconnectAttempts >= MAX_RECONNECT_ATTEMPTS) {
    console.warn(`[WSS] Max reconnect attempts (${MAX_RECONNECT_ATTEMPTS}) reached, giving up`);
    // 如果曾经连接成功过，everConnected 保持 true 以区分"从未连接"与"已断开"
    return;
  }
  
  try {
    // ⚠️ 重要: 使用 suppressAuthExpired=true 防止 /auth 返回 401 时触发用户登出
    // WebSocket pipeline 的 /auth 调用只是为了获取 WS token，
    // 它的 401 不代表用户认证失效（可能是 pipeline 临时不可用）
    const res: any = await VrcApi.request('/auth', { method: 'GET', suppressAuthExpired: true });
    if (res && res.token) {
      // ⚠️ 注意：不再在此处重置 reconnectAttempts！
      // 只有 WebSocket 真正连接成功（onopen）时才重置计数器
      connectWebSocket(res.token);
    } else {
      // /auth 返回成功但没有 token（可能 pipeline 暂时无响应），触发重试
      throw new Error('/auth returned no token');
    }
  } catch (err) {
    console.error('WebSocket init error:', err);
    reconnectAttempts++;
    if (reconnectAttempts < MAX_RECONNECT_ATTEMPTS) {
      const delay = getReconnectDelay();
      console.log(`[WSS] Will retry init in ${Math.round(delay / 1000)}s (attempt ${reconnectAttempts}/${MAX_RECONNECT_ATTEMPTS})`);
      reconnectTimer = setTimeout(initWebsocket, delay) as unknown as number;
    } else {
      console.warn('[WSS] WebSocket pipeline unavailable, will not retry');
      // everConnected 在曾经成功连接后不再重置为 false，以保留"曾经连接过"的状态
    }
  }
}

function connectWebSocket(token: string) {
  if (webSocket !== null) return;
  
  const socket = new WebSocket(`wss://pipeline.vrchat.cloud/?auth=${token}`);
  
  socket.onopen = () => {
    // 如果在连接建立过程中调用了 closeWebSocket，立即关闭此连接
    if (wsShuttingDown) {
      socket.close();
      return;
    }
    wsState.connected = true;
    wsState.everConnected = true;
    // ✅ 只有 WebSocket 真正连接成功时才重置重连计数器
    reconnectAttempts = 0;
    console.log('[WSS] Pipeline connected');
  };
  
  socket.onclose = (e) => {
    wsState.connected = false;
    if (webSocket === socket) webSocket = null;
    console.log('[WSS] Pipeline closed', e.code, e.reason);
    if (reconnectTimer) clearTimeout(reconnectTimer);
    // 意外断开才重连（code 1000 = 正常关闭），且不超过最大次数
    if (e.code !== 1000 && reconnectAttempts < MAX_RECONNECT_ATTEMPTS) {
      reconnectAttempts++;
      const delay = getReconnectDelay();
      console.log(`[WSS] Will reconnect in ${Math.round(delay / 1000)}s (attempt ${reconnectAttempts}/${MAX_RECONNECT_ATTEMPTS})`);
      reconnectTimer = setTimeout(() => {
        if (webSocket === null) initWebsocket();
      }, delay) as unknown as number;
    } else if (e.code === 1000) {
      console.log('[WSS] Clean close, no reconnect');
    } else {
      console.warn('[WSS] Max reconnect attempts reached');
    }
  };
  
  socket.onerror = (err) => {
    console.error('[WSS] Pipeline error', err);
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
  
  webSocket = socket;
}

export function closeWebSocket() {
  wsShuttingDown = true;
  if (reconnectTimer) {
    clearTimeout(reconnectTimer);
    reconnectTimer = null;
  }
  if (webSocket) {
    webSocket.close();
    webSocket = null;
  }
  wsState.connected = false;
  reconnectAttempts = 0; // 重置重连计数器，下次登录时可以重新尝试
  // 延迟清除关闭标记，确保 in-flight 的 initWebsocket 能检测到
  setTimeout(() => { wsShuttingDown = false; }, 500);
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
