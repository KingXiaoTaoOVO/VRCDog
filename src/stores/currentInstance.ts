import { reactive } from 'vue';
import { GamelogApi, VrcApi } from '../api';
import { buildCurrentRoomPlayers, type GameLogEvent } from '../utils/gameLogSession';

export interface CurrentInstancePlayer {
  name: string;
  userId?: string;
  joinTime: string;
  userData?: any;
}

export interface CurrentInstanceSnapshot {
  vrcRunning: boolean | null;
  location: string;
  roomName: string;
  playerCount: number | null;
  players: CurrentInstancePlayer[];
  updatedAt: number;
  error: string;
}

export const currentInstanceState = reactive<CurrentInstanceSnapshot>({
  vrcRunning: null,
  location: '',
  roomName: '',
  playerCount: null,
  players: [],
  updatedAt: 0,
  error: '',
});

let refreshInFlight: Promise<void> | null = null;
let lastRefreshAt = 0;

export function parseVrcLocation(location: string): { worldId: string; instanceId: string } | null {
  if (!location.startsWith('wrld_')) return null;
  const splitAt = location.indexOf(':');
  if (splitAt <= 0 || splitAt === location.length - 1) return null;
  return {
    worldId: location.slice(0, splitAt),
    instanceId: location.slice(splitAt + 1),
  };
}

function mapInstanceUser(raw: any): CurrentInstancePlayer | null {
  const userId = String(raw?.id || raw?.userId || raw?.user_id || '').trim();
  const name = String(raw?.displayName || raw?.display_name || raw?.username || raw?.name || userId).trim();
  if (!name) return null;
  return {
    name,
    userId: userId || undefined,
    joinTime: raw?.joinedAt || raw?.joined_at || raw?.last_activity || new Date().toISOString(),
    userData: { ...raw, id: userId || undefined, displayName: name },
  };
}

function mergePlayers(...lists: CurrentInstancePlayer[][]): CurrentInstancePlayer[] {
  const merged = new Map<string, CurrentInstancePlayer>();
  for (const list of lists) {
    for (const player of list) {
      const key = player.userId ? `id:${player.userId}` : `name:${player.name.normalize('NFKC').toLocaleLowerCase()}`;
      const existing = merged.get(key);
      merged.set(key, existing ? { ...existing, ...player, userData: player.userData || existing.userData } : player);
    }
  }
  return Array.from(merged.values()).sort((a, b) => b.joinTime.localeCompare(a.joinTime));
}

function publish() {
  currentInstanceState.updatedAt = Date.now();
  if (typeof window !== 'undefined') {
    window.dispatchEvent(new CustomEvent('vrc-instance-updated'));
  }
}

function clearForStoppedGame() {
  currentInstanceState.vrcRunning = false;
  currentInstanceState.location = 'offline';
  currentInstanceState.roomName = '';
  currentInstanceState.playerCount = null;
  currentInstanceState.players = [];
  currentInstanceState.error = '';
  publish();
}

export async function refreshCurrentInstance(options: { vrcRunning?: boolean | null; force?: boolean } = {}) {
  if (options.vrcRunning === false) {
    clearForStoppedGame();
    return;
  }

  const now = Date.now();
  if (!options.force && now - lastRefreshAt < 10_000) return;
  if (refreshInFlight) return refreshInFlight;

  refreshInFlight = (async () => {
    lastRefreshAt = now;
    currentInstanceState.vrcRunning = options.vrcRunning ?? currentInstanceState.vrcRunning;
    try {
      const [logs, currentUser] = await Promise.all([
        GamelogApi.getSnapshot({ maxLines: 20_000 }).catch(() => []),
        VrcApi.getCurrentUser().catch(() => null),
      ]);
      const logSnapshot = buildCurrentRoomPlayers(Array.isArray(logs) ? logs as GameLogEvent[] : []);
      const logPlayers: CurrentInstancePlayer[] = logSnapshot.players.map(player => ({
        name: player.name,
        userId: player.userId || undefined,
        joinTime: player.joinTime || new Date().toISOString(),
      }));
      const location = String(currentUser?.location || currentInstanceState.location || '');
      const parsed = parseVrcLocation(location);

      currentInstanceState.location = location;
      currentInstanceState.error = '';

      if (!parsed) {
        const me = mapInstanceUser(currentUser);
        currentInstanceState.roomName = logSnapshot.roomName;
        currentInstanceState.players = mergePlayers(logPlayers, me ? [me] : []);
        currentInstanceState.playerCount = currentInstanceState.players.length || null;
        publish();
        return;
      }

      const [world, instance] = await Promise.all([
        VrcApi.getWorld({ worldId: parsed.worldId }).catch(() => null),
        VrcApi.getInstance({ worldId: parsed.worldId, instanceId: parsed.instanceId }).catch(() => null),
      ]);
      const rawUsers = Array.isArray(instance?.users)
        ? instance.users
        : Array.isArray(instance?.players)
          ? instance.players
          : [];
      const apiPlayers = rawUsers.map(mapInstanceUser).filter(Boolean) as CurrentInstancePlayer[];
      const me = mapInstanceUser(currentUser);
      const players = mergePlayers(apiPlayers, logPlayers, me ? [me] : []);
      const apiCount = typeof instance?.n_users === 'number'
        ? instance.n_users
        : typeof instance?.userCount === 'number'
          ? instance.userCount
          : null;

      currentInstanceState.roomName = world?.name
        ? `${world.name} · ${parsed.instanceId}`
        : logSnapshot.roomName || `${parsed.worldId}:${parsed.instanceId}`;
      currentInstanceState.players = players;
      currentInstanceState.playerCount = apiCount ?? (players.length || null);
      publish();
    } catch (error) {
      currentInstanceState.error = error instanceof Error ? error.message : String(error);
      publish();
    }
  })().finally(() => {
    refreshInFlight = null;
  });

  return refreshInFlight;
}

export function resetCurrentInstanceForTests() {
  lastRefreshAt = 0;
  currentInstanceState.vrcRunning = null;
  currentInstanceState.location = '';
  currentInstanceState.roomName = '';
  currentInstanceState.playerCount = null;
  currentInstanceState.players = [];
  currentInstanceState.updatedAt = 0;
  currentInstanceState.error = '';
}
