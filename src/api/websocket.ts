import { translate } from '../i18n';
import { reactive } from 'vue';
import { VrcApi, DbApi } from './index';
import { isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useNotificationEngine } from '../stores/notificationEngine';
import { useFriendsStore } from '../stores/friendsStore';
import { markDataHealthy } from '../stores/dataHealth';
import { getCookieValue } from './cookies';
import { normalizeNotificationForDb } from './notificationNormalization';

const DEFAULT_PIPELINE_URL = 'wss://pipeline.vrchat.cloud';

export const wsState = reactive({
  connected: false,
  messageCount: 0,
  bytesReceived: 0,
  lastUpdate: '',
  everConnected: false,
  phase: 'idle' as 'idle' | 'authenticating' | 'connecting' | 'waiting' | 'failed' | 'connected',
  lastError: '',
  reconnectAttempts: 0,
});

type PipelineHandler = (json: any) => void;
const handlers: PipelineHandler[] = [];
const MAX_HANDLERS = 50;

const FRIEND_NOTIFY_DEBOUNCE_MS = 45_000;
const friendNotifyTimes = new Map<string, number>();
const notificationToastTimes = new Map<string, number>();
const friendDisplayNameCache = new Map<string, string>();
const friendIdentityLookups = new Map<string, Promise<string>>();

const FRIEND_IDENTITY_EVENTS = new Set([
  'friend-online',
  'friend-active',
  'friend-offline',
  'friend-location',
  'friend-update',
  'friend-add',
  'friend-delete',
]);

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

let listenersRegistered = false;
let pipelineActive = false;

/// Wire up the Tauri event listeners that the Rust pipeline task emits.
/// The actual connection (including proxy + User-Agent handling) lives in Rust;
/// the WebView's native WebSocket does not honour the app's proxy setting.
async function ensureListeners() {
  if (listenersRegistered) return;
  listenersRegistered = true;

  await listen<Record<string, unknown>>('pipeline_ws_status', (event) => {
    const s = event.payload || {};
    if (typeof s.phase === 'string') wsState.phase = s.phase as typeof wsState.phase;
    if (typeof s.connected === 'boolean') wsState.connected = s.connected;
    if (typeof s.messageCount === 'number') wsState.messageCount = s.messageCount;
    if (typeof s.reconnectAttempts === 'number') wsState.reconnectAttempts = s.reconnectAttempts;
    if (typeof s.lastError === 'string') wsState.lastError = s.lastError;
    if (s.phase === 'connected') wsState.everConnected = true;
  });

  await listen<string>('pipeline_ws_message', (event) => {
    const data = event.payload;
    if (typeof data !== 'string') return;
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
  });
}

function errorMessage(err: unknown): string {
  if (err instanceof Error) return err.message;
  return typeof err === 'string' ? err : 'Unknown pipeline error';
}

export async function initWebsocket() {
  pipelineActive = true;
  wsState.phase = 'authenticating';
  wsState.lastError = '';

  // The native pipeline bridge only exists inside the Tauri shell.
  if (!isTauri()) {
    wsState.phase = 'idle';
    return;
  }

  await ensureListeners();

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

    if (!token) {
      wsState.phase = 'failed';
      wsState.lastError = translate('status.pipeline_auth_missing');
      return;
    }

    // Read the user-configured pipeline URL (empty = use the default).
    // Reference: VRCX exposes `AppDebug.websocketDomain` so users can switch to
    // an alternate pipeline mirror when the primary host is unreachable.
    const savedUrl = await DbApi.getSetting({ key: 'pipelineUrl' });
    const pipelineUrl = (savedUrl || '').trim() || DEFAULT_PIPELINE_URL;

    await VrcApi.startPipelineWs({ authToken: token, pipelineUrl });
    // Rust now owns the connection lifecycle and reports status via events.
  } catch (err) {
    console.error('[WSS] Failed to start pipeline WebSocket:', err);
    wsState.phase = 'failed';
    wsState.lastError = errorMessage(err);
  }
}

export async function closeWebSocket() {
  pipelineActive = false;
  try {
    if (isTauri()) await VrcApi.stopPipelineWs();
  } catch { /* best effort */ }
  wsState.connected = false;
  wsState.phase = 'idle';
  wsState.lastError = '';
  wsState.reconnectAttempts = 0;
  wsState.messageCount = 0;
  wsState.bytesReceived = 0;
}

function getFriendUserId(content: any): string {
  return String(
    content?.userId
    || content?.user_id
    || content?.user?.id
    || content?.user?.userId
    || '',
  ).trim();
}

function usableDisplayName(value: unknown, userId = ''): string {
  const name = typeof value === 'string' ? value.trim() : '';
  if (!name || name === userId || name.toLowerCase() === 'unknown') return '';
  return name;
}

function displayNameFromUser(user: any, userId = ''): string {
  return usableDisplayName(
    user?.displayName || user?.display_name || user?.username || user?.name,
    userId,
  );
}

function getFriendDisplayName(content: any): string {
  const userId = getFriendUserId(content);
  return displayNameFromUser(content?.user, userId)
    || usableDisplayName(content?.displayName || content?.display_name, userId)
    || friendDisplayNameCache.get(userId)
    || userId
    || 'VRChat 好友';
}

function rememberFriendDisplayName(userId: string, displayName: string) {
  if (userId && displayName) friendDisplayNameCache.set(userId, displayName);
}

async function lookupFriendDisplayName(userId: string): Promise<string> {
  try {
    const friendsStore = useFriendsStore();
    const friend = Array.isArray(friendsStore.allFriends)
      ? friendsStore.allFriends.find((candidate: any) => candidate?.id === userId || candidate?.userId === userId)
      : undefined;
    const storeName = displayNameFromUser(friend, userId);
    if (storeName) return storeName;
  } catch {
    // Pinia may not be active during the earliest pipeline events.
  }

  try {
    const cached = await DbApi.getCachedFriends();
    const friend = Array.isArray(cached)
      ? cached.find((candidate: any) => candidate?.id === userId || candidate?.userId === userId || candidate?.user_id === userId)
      : undefined;
    const cachedName = displayNameFromUser(friend, userId);
    if (cachedName) return cachedName;
  } catch {
    // Continue to the API fallback when the local cache is unavailable.
  }

  try {
    const user = await VrcApi.getUser({ userId });
    return displayNameFromUser(user, userId);
  } catch {
    return '';
  }
}

async function enrichFriendIdentity(content: any): Promise<any> {
  const source = content && typeof content === 'object' ? content : {};
  const userId = getFriendUserId(source);
  if (!userId) return source;

  let displayName = displayNameFromUser(source.user, userId)
    || usableDisplayName(source.displayName || source.display_name, userId)
    || friendDisplayNameCache.get(userId)
    || '';

  if (!displayName) {
    let lookup = friendIdentityLookups.get(userId);
    if (!lookup) {
      lookup = lookupFriendDisplayName(userId).finally(() => friendIdentityLookups.delete(userId));
      friendIdentityLookups.set(userId, lookup);
    }
    displayName = await lookup;
  }

  rememberFriendDisplayName(userId, displayName);
  return {
    ...source,
    userId,
    ...(displayName ? { displayName } : {}),
  };
}

function getFriendLocation(content: any): string {
  return content?.location || content?.user?.location || '';
}

function friendPatchFromPipeline(type: string, content: any): { userId: string; patch: Record<string, unknown> } | null {
  const userId = getFriendUserId(content);
  if (!userId) return null;

  const user = content?.user || {};
  const displayName = getFriendDisplayName(content);
  const location = type === 'friend-offline' ? 'offline' : getFriendLocation(content) || user.location;
  return {
    userId,
    patch: {
      ...user,
      id: userId,
      ...(usableDisplayName(displayName, userId) ? { displayName } : {}),
      location,
      status: type === 'friend-offline' ? 'offline' : user.status || 'online',
    },
  };
}

function shouldEmitFriendPresenceNotification(type: string, content: any): boolean {
  const userId = getFriendUserId(content);
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

  const userId = getFriendUserId(content);
  if (!userId) return;

  const displayName = getFriendDisplayName(content);
  const location = getFriendLocation(content);

  let title: string;
  let detail: string;
  let notificationKind: 'friend_online' | 'friend_offline' | 'friend_location';

  if (type === 'friend-online') {
    title = `${displayName} 已上线`;
    detail = location && location !== 'offline' ? `位置：${location}` : '好友现在在线';
    notificationKind = 'friend_online';
  } else if (type === 'friend-offline') {
    title = `${displayName} 已下线`;
    detail = '好友现在离线';
    notificationKind = 'friend_offline';
  } else {
    // friend-location: friend moved to a new world
    if (!location || location === 'offline' || location === 'private') return;
    title = `${displayName} 切换了世界`;
    detail = `新位置：${location}`;
    notificationKind = 'friend_location';
  }

  try {
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

    const { notify } = useNotificationEngine();
    await notify('VRC 好友状态', `${title}${detail ? ` - ${detail}` : ''}`, notificationKind);
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

function getNotificationId(content: any): string | null {
  if (typeof content === 'string') return content;
  return content?.notificationId || content?.id || null;
}

function shouldToastNotification(id: string): boolean {
  const now = Date.now();
  const previous = notificationToastTimes.get(id) || 0;
  if (now - previous < 60_000) return false;
  notificationToastTimes.set(id, now);
  if (notificationToastTimes.size > 500) {
    for (const [key, timestamp] of notificationToastTimes) {
      if (now - timestamp > 5 * 60_000) notificationToastTimes.delete(key);
    }
  }
  return true;
}

async function saveAndNotifyRemoteNotification(content: any, version: 1 | 2) {
  const normalized = normalizeNotificationForDb({ ...content, version });
  if (!normalized.id) return;
  await DbApi.saveNotification({ notificationJson: JSON.stringify(normalized) });
  window.dispatchEvent(new CustomEvent('vrc-notifications-synced'));
  if (!shouldToastNotification(normalized.id)) return;
  const normalizedType = String(normalized.type);
  const kind = normalizedType === 'friendRequest'
    ? 'friend_request'
    : normalizedType.startsWith('group.')
      ? 'group'
      : normalizedType === 'invite' || normalizedType === 'requestInvite'
        ? 'invite'
        : 'other';
  const { notify } = useNotificationEngine();
  await notify(notificationTitle({ ...content, ...normalized }), notificationBody({ ...content, ...normalized }), kind);
}

async function handlePipeline(json: any) {
  wsState.lastUpdate = new Date().toLocaleTimeString();

  // Any inbound pipeline message proves the realtime channel is healthy.
  // Mark the data service fresh so the sidebar status dot turns green even
  // when the user is parked on a view that does not trigger REST refreshes.
  markDataHealthy();

  try {
    const type = json.type;
    let content = json.content;
    if (FRIEND_IDENTITY_EVENTS.has(type)) {
      content = await enrichFriendIdentity(content);
      json.content = content;
    }

    // ====== 1. Friend log recording ======
    if (type === 'friend-online') {
      const userId = getFriendUserId(content);
      const displayName = getFriendDisplayName(content);
      if (userId) {
        await DbApi.addFriendLog({ eventType: 'online', userId, displayName, detail: content.location });
      }
      await emitFriendPresenceNotification(type, content);
    } else if (type === 'friend-offline') {
      const userId = getFriendUserId(content);
      const displayName = getFriendDisplayName(content);
      if (userId) {
        await DbApi.addFriendLog({ eventType: 'offline', userId, displayName, detail: null });
      }
      await emitFriendPresenceNotification(type, content);
    } else if (type === 'friend-location') {
      const userId = getFriendUserId(content);
      const displayName = getFriendDisplayName(content);
      if (userId) {
        await DbApi.addFriendLog({ eventType: 'location_change', userId, displayName, detail: content.location });
      }
      await emitFriendPresenceNotification(type, content);
    } else if (type === 'friend-add') {
      const userId = getFriendUserId(content);
      const displayName = getFriendDisplayName(content);
      if (userId) {
        await DbApi.addFriendLog({ eventType: 'friend_add', userId, displayName, detail: null });
      }
    } else if (type === 'friend-delete') {
      const userId = getFriendUserId(content);
      if (userId) {
        await DbApi.addFriendLog({ eventType: 'friend_remove', userId, displayName: getFriendDisplayName(content), detail: null });
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
          displayName: getFriendDisplayName(content),
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
      await saveAndNotifyRemoteNotification(content, 1);
    } else if (type === 'notification-v2') {
      await saveAndNotifyRemoteNotification(content, 2);
    } else if (type === 'notification-v2-update') {
      const updated = { ...(content?.updates || {}), id: content?.id, version: 2 };
      // Avoid replacing a complete local row with a seen-only partial update.
      if (updated.id && (updated.type || updated.data || updated.details)) {
        await saveAndNotifyRemoteNotification(updated, 2);
      }
    } else if (type === 'hide-notification' || type === 'clear-notification') {
      const id = getNotificationId(content);
      if (id) {
        await DbApi.deleteNotification({ id });
        window.dispatchEvent(new CustomEvent('vrc-notifications-synced'));
      }
    } else if (type === 'notification-v2-delete') {
      const ids = Array.isArray(content?.ids) ? content.ids : [];
      await Promise.allSettled(ids.filter(Boolean).map((id: string) => DbApi.deleteNotification({ id })));
      window.dispatchEvent(new CustomEvent('vrc-notifications-synced'));
    }

    // ====== 4. Heatmap activity recording ======
    if (type === 'friend-online' || type === 'friend-location') {
      if (content.userId) {
        await DbApi.recordActivity({
          userId: content.userId,
          displayName: getFriendDisplayName(content),
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
