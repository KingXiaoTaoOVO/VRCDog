<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue';
import { VrcApi, DbApi, GamelogApi, SysApi } from "../api";
import { Users, Search, MapPin, Bone, StickyNote, RefreshCcw } from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';
import VrcResourceCard from './VrcResourceCard.vue';
import { useUserProfileStore } from '../stores/userProfile';
import { useAuthStore } from '../stores/authStore';
import { useFriendsStore } from '../stores/friendsStore';
import { buildCurrentRoomPlayers, type GameLogEvent } from '../utils/gameLogSession';
import { currentInstanceState, refreshCurrentInstance } from '../stores/currentInstance';

const { t } = useI18n();
const profileStore = useUserProfileStore();
const authStore = useAuthStore();
const friendsStore = useFriendsStore();

interface Player {
  name: string;
  joinTime: string;
  userId?: string;
  userData?: any;
  loadingData?: boolean;
  note?: string;
}

interface LogRoomSnapshot {
  roomName: string;
  players: Player[];
}

const currentRoom = ref(t('player_list.unknown_instance'));
const currentLocation = ref('');
const instancePlayerCount = ref<number | null>(null);
const players = ref<Player[]>([]);
const loading = ref(true);
const searchQuery = ref('');
const resolvedNames = new Set<string>();
let refreshTimer: number | null = null;
let fetchInFlight = false;
let fetchPending = false;
let componentMounted = false;

function applySharedInstanceSnapshot() {
  if (!currentInstanceState.updatedAt) return;
  currentLocation.value = currentInstanceState.location;
  if (currentInstanceState.roomName) currentRoom.value = currentInstanceState.roomName;
  instancePlayerCount.value = currentInstanceState.playerCount;

  const sharedPlayers: Player[] = currentInstanceState.players.map(player => ({
    name: player.name,
    userId: player.userId,
    joinTime: player.joinTime,
    userData: player.userData,
    loadingData: false,
  }));
  const merged = new Map<string, Player>();
  mergePlayers(merged, players.value);
  mergePlayers(merged, sharedPlayers);
  players.value = Array.from(merged.values()).sort((a, b) => b.joinTime.localeCompare(a.joinTime));
  void resolveMissingPlayers();
}

function parseLocation(location: string): { worldId: string; instanceId: string } | null {
  if (!location || !location.startsWith('wrld_') || !location.includes(':')) return null;
  const splitAt = location.indexOf(':');
  const worldId = location.slice(0, splitAt);
  const instanceId = location.slice(splitAt + 1);
  if (!worldId || !instanceId) return null;
  return { worldId, instanceId };
}

function readableLocationState(location: string): string {
  if (location === 'private') return t('player_list.private_location');
  if (location === 'offline') return t('player_list.offline_location');
  if (location === 'traveling') return t('player_list.traveling_location');
  return t('player_list.unknown_instance');
}

function mapInstanceUser(raw: any): Player | null {
  const id = raw?.id || raw?.userId || raw?.user_id;
  const displayName = raw?.displayName || raw?.display_name || raw?.username || raw?.name || id;
  if (!displayName) return null;
  return {
    name: displayName,
    joinTime: raw?.joinedAt || raw?.joined_at || raw?.last_activity || new Date().toISOString(),
    userId: id,
    userData: {
      ...raw,
      id,
      displayName,
      currentAvatarThumbnailImageUrl: raw?.currentAvatarThumbnailImageUrl || raw?.profilePicOverride || raw?.userIcon || raw?.avatarUrl,
    },
    loadingData: false,
  };
}

function mergePlayers(target: Map<string, Player>, list: Player[]) {
  for (const player of list) {
    const key = (player.userData?.id || player.userId || player.name).trim().toLowerCase();
    const existing = target.get(key);
    if (!existing) {
      target.set(key, { ...player });
    } else {
      target.set(key, {
        ...existing,
        ...player,
        userId: player.userId || existing.userId,
        userData: player.userData || existing.userData,
        note: player.note || existing.note,
        loadingData: existing.loadingData || player.loadingData,
      });
    }
  }
}

async function buildLogRoomSnapshot(): Promise<LogRoomSnapshot> {
  try {
    const logs: GameLogEvent[] = await GamelogApi.getSnapshot({ maxLines: 20000 });
    if (!Array.isArray(logs) || logs.length === 0) return { roomName: '', players: [] };
    const snapshot = buildCurrentRoomPlayers(logs);
    return {
      roomName: snapshot.roomName,
      players: snapshot.players.map(player => ({
        name: player.name,
        userId: player.userId || undefined,
        joinTime: player.joinTime || new Date().toISOString(),
        loadingData: false,
      })),
    };
  } catch (e) {
    console.warn('Failed to load room players from game log', e);
    return { roomName: '', players: [] };
  }
}

async function buildApiFallbackPlayers(freshUser: any, location: string): Promise<Player[]> {
  const sameRoom = new Map<string, Player>();

  if (freshUser?.id || freshUser?.displayName) {
    sameRoom.set(freshUser.id || freshUser.displayName, {
      name: freshUser.displayName || freshUser.username || t('charts.me'),
      joinTime: freshUser.last_login || new Date().toISOString(),
      userId: freshUser.id,
      userData: freshUser,
      loadingData: false,
    });
  }

  try {
    const pageSize = 100;
    for (let offset = 0; offset < 500; offset += pageSize) {
      const friends: any[] = await VrcApi.getFriends({ n: pageSize, offset, offline: false });
      if (!Array.isArray(friends) || friends.length === 0) break;

      for (const friend of friends) {
        if (friend?.location === location) {
          const player = mapInstanceUser(friend);
          if (player) sameRoom.set(friend.id || player.name, player);
        }
      }

      if (friends.length < pageSize) break;
    }
  } catch (e) {
    console.warn('Failed to load same-location friends', e);
  }

  return Array.from(sameRoom.values());
}

const fetchPlayerList = async () => {
  if (fetchInFlight) {
    fetchPending = true;
    return;
  }
  fetchInFlight = true;
  loading.value = true;
  try {
    const [freshUser, vrcRunning]: [any, boolean] = await Promise.all([
      VrcApi.getCurrentUser().catch(() => authStore.currentUser || {}),
      SysApi.isVrcRunning().catch(() => true),
    ]);
    if (freshUser?.id) {
      authStore.currentUser = { ...(authStore.currentUser || {}), ...freshUser } as any;
    }

    if (!vrcRunning) {
      currentLocation.value = 'offline';
      currentRoom.value = readableLocationState('offline');
      instancePlayerCount.value = null;
      players.value = [];
      return;
    }

    const location = String(freshUser?.location || authStore.currentUser?.location || '');
    currentLocation.value = location;
    const parsed = parseLocation(location);
    const logSnapshot = await buildLogRoomSnapshot();

    if (!parsed) {
      currentRoom.value = logSnapshot.roomName || readableLocationState(location);
      instancePlayerCount.value = null;
      const fallbackPlayers = new Map<string, Player>();
      mergePlayers(fallbackPlayers, players.value);
      mergePlayers(fallbackPlayers, logSnapshot.players);
      if (freshUser?.displayName) {
        mergePlayers(fallbackPlayers, [{
          name: freshUser.displayName,
          joinTime: freshUser.last_login || new Date().toISOString(),
          userId: freshUser.id,
          userData: freshUser,
          loadingData: false,
        }]);
      }
      players.value = Array.from(fallbackPlayers.values()).sort((a, b) => b.joinTime.localeCompare(a.joinTime));
      void resolveMissingPlayers();
      return;
    }

    const [world, instance]: any[] = await Promise.all([
      VrcApi.getWorld({ worldId: parsed.worldId }).catch(() => null),
      VrcApi.getInstance({ worldId: parsed.worldId, instanceId: parsed.instanceId }).catch(() => null),
    ]);

    currentRoom.value = world?.name
      ? `${world.name} · ${parsed.instanceId}`
      : `${parsed.worldId}:${parsed.instanceId}`;

    instancePlayerCount.value = typeof instance?.n_users === 'number'
      ? instance.n_users
      : typeof instance?.userCount === 'number'
        ? instance.userCount
        : null;

    const instanceUsers = Array.isArray(instance?.users)
      ? instance.users
      : Array.isArray(instance?.players)
        ? instance.players
        : [];

    let nextPlayers: Player[] = instanceUsers
      .map(mapInstanceUser)
      .filter((player: Player | null): player is Player => Boolean(player));

    const mergedPlayers = new Map<string, Player>();
    mergePlayers(mergedPlayers, nextPlayers);
    mergePlayers(mergedPlayers, logSnapshot.players);

    if (mergedPlayers.size <= 1) {
      mergePlayers(mergedPlayers, await buildApiFallbackPlayers(freshUser, location));
    }

    const updatedPlayers: Player[] = Array.from(mergedPlayers.values()).map((player: Player) => {
      const playerId = player.userData?.id || player.userId;
      const existing = players.value.find(p => (playerId && (p.userData?.id || p.userId) === playerId) || p.name === player.name);
      if (existing) {
        return {
          ...existing,
          ...player,
          userId: player.userId || existing.userId,
          userData: player.userData || existing.userData,
          note: player.note || existing.note,
          loadingData: existing.loadingData ?? false,
        };
      }
      return { ...player };
    });

    if (freshUser?.displayName && !updatedPlayers.some(p => (p.userData?.id || p.userId) === freshUser.id || p.name === freshUser.displayName)) {
      updatedPlayers.unshift({
        name: freshUser.displayName,
        joinTime: freshUser.last_login || new Date().toISOString(),
        userId: freshUser.id,
        userData: freshUser,
        loadingData: false,
      });
    }

    players.value = updatedPlayers.sort((a, b) => b.joinTime.localeCompare(a.joinTime));
    void resolveMissingPlayers();
  } catch (err) {
    console.error(err);
    players.value = [];
    instancePlayerCount.value = null;
    currentRoom.value = currentLocation.value ? readableLocationState(currentLocation.value) : t('player_list.unknown_instance');
  } finally {
    loading.value = false;
    fetchInFlight = false;
    if (fetchPending && componentMounted) {
      fetchPending = false;
      void fetchPlayerList();
    }
  }
};

const formatJoinTime = (time: string) => {
  const date = new Date(time);
  if (!Number.isNaN(date.getTime())) {
    return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  }
  return time?.slice(11, 16) || time || '--:--';
};

const userProfileCache = new Map<string, any>();
const resolvingKeys = new Set<string>();

function applyResolvedData(playerName: string, userData: any) {
  const target = players.value.find(p => p.name === playerName || (userData.id && (p.userData?.id || p.userId) === userData.id));
  if (!target) return;

  target.userData = {
    ...userData,
    id: userData.id,
    displayName: userData.displayName || playerName,
    currentAvatarThumbnailImageUrl: userData.currentAvatarThumbnailImageUrl || userData.profilePicOverride || userData.currentAvatarImageUrl || userData.userIcon,
  };
  target.userId = userData.id;
  target.loadingData = false;

  const sharedTarget = currentInstanceState.players.find(p => p.name === playerName || (userData.id && p.userId === userData.id));
  if (sharedTarget) {
    sharedTarget.userData = target.userData;
    sharedTarget.userId = target.userId;
  }

  if (userData.id && !target.note) {
    DbApi.getNote({ userId: userData.id }).then(noteRes => {
      if (noteRes?.note) {
        target.note = noteRes.note;
      }
    }).catch(() => {});
  }

  players.value = [...players.value];
}

const resolveMissingPlayers = async () => {
  const unresolved = players.value.filter(p => !p.userData);
  if (unresolved.length === 0) return;

  for (const player of unresolved) {
    const cleanName = player.name.trim();
    if (!cleanName) continue;
    const cacheKey = (player.userId || cleanName).toLowerCase();

    // 1. 本地缓存直接命中
    if (userProfileCache.has(cacheKey)) {
      applyResolvedData(player.name, userProfileCache.get(cacheKey));
      continue;
    }

    // 2. 好友列表命中（0 网络开销）
    const friend = (friendsStore.allFriends as any[]).find((f: any) =>
      (player.userId && f?.id === player.userId) ||
      String(f?.displayName || '').trim().toLowerCase() === cleanName.toLowerCase()
    );
    if (friend) {
      userProfileCache.set(cacheKey, friend);
      applyResolvedData(player.name, friend);
      continue;
    }

    // 3. API 异步解析
    void resolvePlayerData(player);
  }
};

const resolvePlayerData = async (player: Player) => {
  const cleanName = player.name.trim();
  const lookupKey = (player.userId || cleanName).toLowerCase();
  if (resolvingKeys.has(lookupKey)) return;
  resolvingKeys.add(lookupKey);

  const target = players.value.find(p => p.name === player.name || (player.userId && p.userId === player.userId));
  if (target) target.loadingData = true;

  try {
    let resolved: any = null;

    if (player.userId) {
      resolved = await VrcApi.getUser({ userId: player.userId }).catch(() => null);
    }

    if (!resolved?.id) {
      const searchRes: any = await VrcApi.searchUsers({ query: cleanName, n: 10, offset: 0 }).catch(() => null);
      if (Array.isArray(searchRes) && searchRes.length > 0) {
        const lower = cleanName.toLowerCase();
        resolved = searchRes.find(c => String(c.displayName || '').trim().toLowerCase() === lower);
        if (!resolved) {
          resolved = searchRes.find(c => String(c.displayName || '').trim().toLowerCase().replace(/\s+/g, '') === lower.replace(/\s+/g, ''));
        }
        if (!resolved) {
          resolved = searchRes[0];
        }
      }
    }

    if (resolved?.id) {
      userProfileCache.set(lookupKey, resolved);
      userProfileCache.set(resolved.id.toLowerCase(), resolved);
      if (resolved.displayName) {
        userProfileCache.set(resolved.displayName.trim().toLowerCase(), resolved);
      }
      applyResolvedData(player.name, resolved);
    }
  } catch (e) {
    console.warn(`Failed to resolve player data for ${player.name}`, e);
  } finally {
    resolvingKeys.delete(lookupKey);
    const p = players.value.find(x => x.name === player.name || (player.userId && x.userId === player.userId));
    if (p) p.loadingData = false;
  }
};

const handleRefresh = () => {
  resolvingKeys.clear();
  void fetchPlayerList();
};

onMounted(() => {
  componentMounted = true;
  applySharedInstanceSnapshot();
  void refreshCurrentInstance({ force: true });
  fetchPlayerList();
  refreshTimer = window.setInterval(fetchPlayerList, 30000);
  window.addEventListener('vrc-gamelog-updated', fetchPlayerList);
  window.addEventListener('vrc-instance-updated', applySharedInstanceSnapshot);
});

onUnmounted(() => {
  componentMounted = false;
  fetchPending = false;
  if (refreshTimer) window.clearInterval(refreshTimer);
  window.removeEventListener('vrc-gamelog-updated', fetchPlayerList);
  window.removeEventListener('vrc-instance-updated', applySharedInstanceSnapshot);
});

const filteredPlayers = computed(() => {
  if (!searchQuery.value) return players.value;
  const q = searchQuery.value.toLowerCase();
  return players.value.filter(p => p.name.toLowerCase().includes(q));
});

const openPlayerProfile = async (player: Player) => {
  if (player.userData?.id) {
    profileStore.openProfile(player.userData.id, player.userData);
    return;
  }
  if (player.userId) {
    profileStore.openProfile(player.userId);
    return;
  }
  try {
    const searchRes = await VrcApi.searchUsers({ query: player.name, n: 10, offset: 0 });
    const resolved = searchRes?.find((c: any) => String(c.displayName || '').trim().toLowerCase() === player.name.trim().toLowerCase()) || searchRes?.[0];
    if (resolved?.id) {
      applyResolvedData(player.name, resolved);
      profileStore.openProfile(resolved.id, resolved);
    }
  } catch (e) {
    console.warn('Failed to resolve player by name', player.name, e);
  }
};
</script>

<template>
  <div class="h-full flex flex-col bg-surface-hover p-6 rounded-xl relative overflow-hidden">
    <div class="absolute inset-0 pointer-events-none opacity-70" style="background: radial-gradient(circle at 20% 0%, color-mix(in srgb, var(--theme-primary) 13%, transparent), transparent 34%), radial-gradient(circle at 95% 8%, color-mix(in srgb, var(--theme-primary-hover) 10%, transparent), transparent 30%);" />
    <header class="mb-6 flex justify-between items-end shrink-0 relative z-10">
      <div>
        <h1 class="text-2xl font-extrabold text-text tracking-tight flex items-center gap-3">
          {{ t('player_list.title') }}
          <span class="inline-flex items-center justify-center p-1.5 rounded-xl shadow-sm border border-primary/20 bg-primary/10">
            <Users class="w-5 h-5 text-primary" />
          </span>
        </h1>
        <p class="text-text-muted font-medium mt-2 flex items-center gap-1.5 text-sm">
          <MapPin
            :size="14"
            class="text-border-strong"
          /> {{ t('player_list.current_location') }}
          <span class="font-bold text-text bg-surface/80 backdrop-blur px-2 py-0.5 rounded-md border border-border-soft shadow-sm max-w-[560px] truncate">{{ currentRoom }}</span>
        </p>
      </div>
      <div class="flex items-center gap-3">
        <span class="text-xs font-bold text-primary bg-primary/10 px-3 py-1.5 rounded-md border border-primary/20 shadow-sm backdrop-blur">
          {{ t('player_list.total_players', { count: instancePlayerCount ?? players.length }) }}
        </span>
        <button
          class="inline-flex items-center gap-1.5 text-xs font-bold px-3 py-1.5 rounded-md bg-surface/80 border border-border-soft text-text-muted hover:text-primary hover:border-primary/40 transition-all shadow-sm"
          :disabled="loading"
          @click="handleRefresh"
        >
          <RefreshCcw :size="13" :class="{ 'animate-spin': loading }" />
          {{ t('player_list.refresh') }}
        </button>
      </div>
    </header>

    <div class="flex-1 flex flex-col overflow-hidden relative z-10">
      <!-- Search -->
      <div class="relative mb-4 shrink-0">
        <Search class="absolute left-3.5 top-1/2 -translate-y-1/2 text-border-strong w-4 h-4" />
        <input
          v-model="searchQuery"
          type="text"
          :placeholder="t('player_list.search_placeholder')"
          class="w-full pl-10 pr-4 py-2.5 bg-surface border border-border-soft focus:border-primary focus:ring-4 focus:ring-primary/10 rounded-lg outline-none transition-all text-text text-sm shadow-sm"
        >
      </div>

      <!-- Player List Grid -->
      <div class="flex-1 overflow-y-auto custom-scrollbar pr-2 pb-4">
        <div
          v-if="loading && players.length === 0"
          class="h-full flex flex-col items-center justify-center text-blue-500 opacity-70"
        >
          <Bone
            class="mb-4"
            :size="48"
          />
          <p class="font-bold text-sm tracking-wide">
            {{ t('player_list.loading') }}
          </p>
        </div>

        <div
          v-else-if="players.length === 0"
          class="h-full flex flex-col items-center justify-center text-border-strong"
        >
          <Users
            class="mb-4 opacity-50"
            :size="48"
          />
          <p class="font-bold text-base">
            {{ t('player_list.no_data') }}
          </p>
          <p class="text-xs mt-1 text-text-muted">
            {{ t('player_list.no_players') }}
          </p>
        </div>

        <div
          v-else
          class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4"
        >
          <div
            v-for="player in filteredPlayers"
            :key="player.userId || player.name"
            class="relative group"
          >
            <VrcResourceCard
              v-if="player.userData"
              type="user"
              :data="player.userData"
              :is-user="true"
              @click="openPlayerProfile(player)"
            />
            
            <div
              v-else
              class="h-36 bg-surface/85 backdrop-blur rounded-xl overflow-hidden border border-border-soft shadow-sm flex items-center justify-center relative transition-all hover:-translate-y-0.5 hover:shadow-md hover:border-primary/30 cursor-pointer"
              @click="openPlayerProfile(player)"
            >
              <div
                v-if="player.loadingData"
                class="absolute inset-0 bg-surface-hover/80 flex items-center justify-center"
              >
                <div class="animate-pulse w-10 h-10 rounded-full bg-primary/20" />
              </div>
              <div
                v-else
                class="text-center p-4"
              >
                <div class="w-14 h-14 rounded-full bg-primary/10 flex items-center justify-center text-primary font-black text-xl uppercase mx-auto mb-3 border border-primary/20 shadow-sm">
                  {{ player.name.charAt(0) }}
                </div>
                <h3 class="font-bold text-text text-sm truncate max-w-[180px] mx-auto">
                  {{ player.name }}
                </h3>
                <p class="text-[11px] text-text-muted font-mono mt-1">
                  {{ formatJoinTime(player.joinTime) }}
                </p>
              </div>
            </div>

            <!-- Note Badge -->
            <div
              v-if="player.note"
              class="absolute top-2 left-2 bg-yellow-100/90 backdrop-blur-md text-yellow-800 text-[10px] px-2 py-0.5 rounded flex items-center gap-1 font-bold border-yellow-300/50 shadow-sm z-10 max-w-[120px]"
            >
              <StickyNote :size="10" /> <span class="truncate">{{ player.note }}</span>
            </div>

            <!-- Join Time Badge -->
            <div class="absolute top-2 right-2 bg-surface/90 backdrop-blur-md text-text text-[10px] px-2 py-0.5 rounded-md uppercase font-mono font-bold border border-border-soft pointer-events-none z-10 shadow-sm">
              {{ formatJoinTime(player.joinTime) }}
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>


