import { reactive } from 'vue';
import { VrcApi, DbApi } from './index';

export const wsState = reactive({
  connected: false,
  messageCount: 0,
  bytesReceived: 0,
  lastUpdate: '',
});

let webSocket: WebSocket | null = null;
let lastMessage = '';
let reconnectTimer: number | null = null;

export async function initWebsocket() {
  if (webSocket !== null) return;
  
  try {
    const res: any = await VrcApi.request('/auth', 'GET');
    if (res && res.token) {
      connectWebSocket(res.token);
    }
  } catch (err) {
    console.error('WebSocket init error:', err);
    reconnectTimer = setTimeout(initWebsocket, 5000) as unknown as number;
  }
}

function connectWebSocket(token: string) {
  if (webSocket !== null) return;
  
  const socket = new WebSocket(`wss://pipeline.vrchat.cloud/?auth=${token}`);
  
  socket.onopen = () => {
    wsState.connected = true;
    console.log('[WSS] Pipeline connected');
  };
  
  socket.onclose = (e) => {
    wsState.connected = false;
    if (webSocket === socket) webSocket = null;
    console.log('[WSS] Pipeline closed', e.code, e.reason);
    if (reconnectTimer) clearTimeout(reconnectTimer);
    reconnectTimer = setTimeout(() => {
      if (webSocket === null) initWebsocket();
    }, 5000) as unknown as number;
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
  if (reconnectTimer) {
    clearTimeout(reconnectTimer);
    reconnectTimer = null;
  }
  if (webSocket) {
    webSocket.close();
    webSocket = null;
  }
  wsState.connected = false;
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

async function handlePipeline(json: any) {
  wsState.lastUpdate = new Date().toLocaleTimeString();
  
  try {
    const type = json.type;
    const content = json.content;
    
    // ====== 1. 好友日志记录 ======
    if (type === 'friend-online') {
      await DbApi.addFriendLog({ eventType: 'online', userId: content.userId, displayName: content.user?.displayName || 'Unknown', detail: content.location });
    } else if (type === 'friend-offline') {
      await DbApi.addFriendLog({ eventType: 'offline', userId: content.userId, displayName: content.user?.displayName || 'Unknown', detail: null });
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
      await DbApi.saveNotification({
        notificationJson: JSON.stringify({
          id: content.id,
          type: content.type,
          senderUserId: content.senderUserId,
          senderUsername: content.senderUsername,
          receiverUserId: content.receiverUserId,
          message: content.message || '',
          details: typeof content.details === 'object' ? JSON.stringify(content.details) : (content.details || ''),
          created_at: content.created_at || new Date().toISOString()
        })
      });
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
    console.warn('[WSS] 写入日志或缓存失败', err);
  }

  for (const handler of handlers) {
    try { handler(json); } catch (e) { console.error('Handler error', e); }
  }
  
  window.dispatchEvent(new CustomEvent('vrc-pipeline-event', { detail: json }));
}
