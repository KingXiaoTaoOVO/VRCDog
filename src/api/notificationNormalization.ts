export interface StoredNotificationRecord {
  id: string;
  type: string;
  senderUserId: string | null;
  senderUsername: string;
  receiverUserId: string | null;
  message: string;
  details: string;
  created_at: string;
}

export interface StoredNotificationMeta {
  version: 1 | 2;
  category?: string;
  seen?: boolean;
  canDelete?: boolean;
  expiresAt?: string;
  link?: string;
  responses?: Array<{ type?: string; data?: string; text?: string }>;
}

const META_KEY = '__vrcdog';

function objectValue(value: unknown): Record<string, any> {
  if (value && typeof value === 'object' && !Array.isArray(value)) {
    return value as Record<string, any>;
  }
  if (typeof value === 'string' && value.trim()) {
    try {
      const parsed = JSON.parse(value);
      return parsed && typeof parsed === 'object' && !Array.isArray(parsed) ? parsed : {};
    } catch {
      return { message: value };
    }
  }
  return {};
}

export function isNotificationV2(notification: Record<string, any>): boolean {
  return notification.version === 2
    || 'createdAt' in notification
    || 'category' in notification
    || 'responses' in notification
    || 'canDelete' in notification;
}

export function normalizeNotificationForDb(notification: Record<string, any>): StoredNotificationRecord {
  const version: 1 | 2 = isNotificationV2(notification) ? 2 : 1;
  const data = objectValue(notification.data);
  const originalDetails = objectValue(notification.details);
  const meta: StoredNotificationMeta = {
    version,
    category: notification.category,
    seen: notification.seen,
    canDelete: notification.canDelete,
    expiresAt: notification.expiresAt,
    link: notification.link,
    responses: Array.isArray(notification.responses) ? notification.responses : undefined,
  };
  const details = {
    ...data,
    ...originalDetails,
    [META_KEY]: meta,
  };
  const createdAt = notification.created_at
    || notification.createdAt
    || (notification.createdAtMs ? new Date(Number(notification.createdAtMs)).toISOString() : '');

  return {
    id: String(notification.id || ''),
    type: String(notification.type || 'notification'),
    senderUserId: notification.senderUserId || null,
    senderUsername: String(
      notification.senderUsername
      || notification.senderDisplayName
      || data.senderUsername
      || data.senderDisplayName
      || '',
    ),
    receiverUserId: notification.receiverUserId || null,
    message: String(
      notification.message
      || notification.title
      || data.message
      || data.title
      || data.announcementTitle
      || '',
    ),
    details: JSON.stringify(details),
    created_at: createdAt || new Date().toISOString(),
  };
}

export function parseStoredNotificationDetails(details: unknown): Record<string, any> {
  return objectValue(details);
}

export function getStoredNotificationMeta(details: unknown): StoredNotificationMeta {
  const parsed = parseStoredNotificationDetails(details);
  const meta = parsed[META_KEY];
  if (!meta || typeof meta !== 'object') return { version: 1 };
  return {
    ...meta,
    version: meta.version === 2 ? 2 : 1,
    responses: Array.isArray(meta.responses) ? meta.responses : undefined,
  };
}

export function getDisplayNotificationDetails(details: unknown): Record<string, any> | string {
  const parsed = parseStoredNotificationDetails(details);
  const { [META_KEY]: _meta, ...display } = parsed;
  if (Object.keys(display).length === 1 && typeof display.message === 'string') return display.message;
  return display;
}
