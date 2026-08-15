export interface GameLogEvent {
  time: string;
  event_type: string;
  content: string;
  display_name?: string;
  user_id?: string;
}

export interface GameLogIdentity {
  displayName: string;
  userId: string;
  keys: string[];
}

function normalizeName(name: string): string {
  return String(name || '')
    .normalize('NFKC')
    .replace(/[\u0000-\u001f\u007f\u200b-\u200d\ufeff]/g, '')
    .trim()
    .toLocaleLowerCase();
}

export function parseGameLogIdentity(event: Partial<GameLogEvent> | string): GameLogIdentity {
  const record = typeof event === 'string' ? { content: event } : event;
  const content = String(record.content || '').trim();
  let displayName = String(record.display_name || '').trim();
  let userId = String(record.user_id || '').trim();

  const match = content.match(/^(.*?)\s+\((usr_[A-Za-z0-9_-]+)\)$/);
  if (match) {
    if (!displayName) displayName = match[1].trim();
    if (!userId) userId = match[2];
  } else if (!displayName) {
    displayName = content;
  }

  const keys = new Set<string>();
  if (userId) keys.add(`id:${userId}`);
  const normalizedName = normalizeName(displayName);
  if (normalizedName) keys.add(`name:${normalizedName}`);

  return { displayName: displayName || userId, userId, keys: Array.from(keys) };
}

export function eventDisplayName(event: Partial<GameLogEvent>): string {
  return parseGameLogIdentity(event).displayName;
}

export function buildCurrentRoomPlayers(events: GameLogEvent[]) {
  const departed = new Set<string>();
  const current = new Map<string, { name: string; userId: string; joinTime: string }>();
  let roomName = '';

  for (const event of events) {
    if (event?.event_type === 'Application Quit') {
      return { roomName: '', players: [] };
    }
    if (event?.event_type === 'Instance Joined') {
      roomName = String(event.content || '').trim();
      break;
    }
    if (event?.event_type !== 'Player Joined' && event?.event_type !== 'Player Left') continue;

    const identity = parseGameLogIdentity(event);
    if (!identity.displayName || identity.keys.length === 0) continue;

    if (event.event_type === 'Player Left') {
      identity.keys.forEach(key => departed.add(key));
      continue;
    }

    if (identity.keys.some(key => departed.has(key))) continue;
    const primaryKey = identity.userId ? `id:${identity.userId}` : identity.keys[0];
    current.set(primaryKey, {
      name: identity.displayName,
      userId: identity.userId,
      joinTime: event.time,
    });
  }

  return { roomName, players: Array.from(current.values()) };
}
