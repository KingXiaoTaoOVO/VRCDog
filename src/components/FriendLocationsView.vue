<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue';
import { VrcApi, DbApi } from '../api';
import VrcAvatar from './VrcAvatar.vue';
import { MapPin, Users, Globe2, RefreshCcw, Lock } from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';
import { useUserProfileStore } from '../stores/userProfile';
import { useFriendsStore } from '../stores/friendsStore';

const { t } = useI18n();
const profileStore = useUserProfileStore();
const friendsStore = useFriendsStore();

interface FriendLocation {
  worldId: string;
  instanceId: string;
  fullLocation: string;
  friends: any[];
  worldName?: string;
  worldImageUrl?: string;
}

const locations = ref<FriendLocation[]>([]);
const loading = ref(true);
const privateFriends = ref<any[]>([]);
const offlineFriends = ref<any[]>([]);
let timer: number | null = null;

// ── 持久化世界名缓存（DB + 内存两级）──────────────────────────────
const memWorldCache = new Map<string, string>();

async function getCachedWorldName(worldId: string): Promise<string | null> {
  if (memWorldCache.has(worldId)) return memWorldCache.get(worldId)!;
  try {
    const cached = await DbApi.getApiCache({ key: `world_name:${worldId}` });
    if (cached) {
      memWorldCache.set(worldId, cached);
      return cached;
    }
  } catch { /* ignore */ }
  return null;
}

async function setCachedWorldName(worldId: string, name: string) {
  memWorldCache.set(worldId, name);
  try {
    await DbApi.saveApiCache({ key: `world_name:${worldId}`, data: name });
  } catch { /* ignore */ }
}

// ── 并发拉取世界名（VrcDog 风格：立即渲染，后台补全名称）────────────
async function fetchWorldNamesConcurrent(locs: FriendLocation[]) {
  const CONCURRENCY = 5; // 同 VrcDog bulkRefreshFriends 并发数
  const queue = locs.filter(l => !l.worldName || l.worldName === l.worldId);

  // 先用缓存填充，避免任何网络请求
  for (const loc of queue) {
    const cached = await getCachedWorldName(loc.worldId);
    if (cached) loc.worldName = cached;
  }

  // 只请求真正没有缓存的
  const needFetch = queue.filter(l => !l.worldName || l.worldName === l.worldId);
  if (needFetch.length === 0) return;

  let idx = 0;
  async function worker() {
    while (idx < needFetch.length) {
      const loc = needFetch[idx++];
      try {
        const w = await VrcApi.getWorld({ worldId: loc.worldId });
        if (w?.name) {
          loc.worldName = w.name;
          if (w.imageUrl) loc.worldImageUrl = w.imageUrl;
          await setCachedWorldName(loc.worldId, w.name);
        } else {
          loc.worldName = loc.worldId;
        }
      } catch {
        loc.worldName = loc.worldId;
      }
      // 触发响应式更新
      locations.value = [...locations.value];
    }
  }

  await Promise.all(Array.from({ length: Math.min(CONCURRENCY, needFetch.length) }, worker));
}

const fetchLocations = async () => {
  loading.value = true;
  try {
    // 使用共享好友数据，避免重复API调用
    await friendsStore.fetchFriends();
    const friends = friendsStore.allFriends;
    const locMap = new Map<string, FriendLocation>();
    const privates: any[] = [];
    const offlines: any[] = [];

    for (const f of friends) {
      if (!f.location || f.location === 'offline') {
        offlines.push(f);
      } else if (f.location === 'private') {
        privates.push(f);
      } else {
        const loc = f.location;
        const parts = loc.split(':');
        const worldId = parts[0] || loc;
        const instanceId = parts.slice(1).join(':') || '';

        if (!locMap.has(loc)) {
          // 先用缓存填充世界名，立即可见
          const cachedName = memWorldCache.get(worldId) || undefined;
          locMap.set(loc, {
            worldId,
            instanceId,
            fullLocation: loc,
            friends: [],
            worldName: cachedName,
          });
        }
        locMap.get(loc)!.friends.push(f);
      }
    }

    locations.value = Array.from(locMap.values()).sort((a, b) => b.friends.length - a.friends.length);
    privateFriends.value = privates;
    offlineFriends.value = offlines;

    // 立即结束 loading，让好友列表先显示出来
    loading.value = false;

    // 后台并发补全世界名（不阻塞 UI）
    fetchWorldNamesConcurrent(locations.value);
  } catch (err) {
    console.warn('fetchLocations error', err);
    loading.value = false;
  }
};

onMounted(() => {
  fetchLocations();
  timer = setInterval(fetchLocations, 60000) as unknown as number;
});
onUnmounted(() => { if (timer) clearInterval(timer); });

const totalOnline = computed(() =>
  locations.value.reduce((s, l) => s + l.friends.length, 0) + privateFriends.value.length
);

const getStatusDot = (status: string) => {
  const s = status?.toLowerCase() || '';
  if (s === 'active' || s === 'online') return 'bg-green-500';
  if (s === 'join me') return 'bg-blue-500';
  if (s === 'ask me' || s === 'busy') return 'bg-orange-500';
  if (s === 'do not disturb' || s === 'dnd') return 'bg-red-500';
  if (s === 'offline') return 'bg-slate-400';
  return 'bg-green-500';
};

// Trust rank color — aligned with VrcDog
const getTrustColor = (tags: string[]) => {
  if (!tags || !tags.length) return undefined;
  if (tags.includes('system_trust_legend')) return '#ff69b4';
  if (tags.includes('system_trust_veteran')) return '#8b5cf6';
  if (tags.includes('system_trust_trusted')) return '#ff7b42';
  if (tags.includes('system_trust_known')) return '#2bcf5c';
  if (tags.includes('system_trust_basic')) return '#1778ff';
  return undefined;
};

// Country flags from language tags (like VrcDog)
const LANGUAGE_FLAGS: Record<string, string> = {
  language_eng: '🇺🇸', language_kor: '🇰🇷', language_rus: '🇷🇺',
  language_spa: '🇪🇸', language_por: '🇧🇷', language_zho: '🇨🇳',
  language_deu: '🇩🇪', language_jpn: '🇯🇵', language_fra: '🇫🇷',
  language_swe: '🇸🇪', language_nld: '🇳🇱', language_pol: '🇵🇱',
  language_tha: '🇹🇭', language_ita: '🇮🇹', language_tur: '🇹🇷',
  language_ara: '🇸🇦', language_vie: '🇻🇳', language_ukr: '🇺🇦',
  language_ind: '🇮🇩', language_msa: '🇲🇾',
};
const getFlags = (tags: string[]) =>
  (tags || []).filter(tag => tag.startsWith('language_')).map(tag => LANGUAGE_FLAGS[tag]).filter(Boolean).slice(0, 2);

import { SysApi } from '../api';

const launchInstance = async (fullLocation: string) => {
  try {
    await SysApi.launchVrc({ launchArgs: `vrchat://launch?id=${fullLocation}` });
  } catch (err) {
    console.warn('Failed to launch instance', err);
  }
};

const inviteMyself = async (worldId: string, instanceId: string) => {
  try {
    await VrcApi.inviteMyself({ worldId, instanceId });
  } catch (err) {
    console.warn('Failed to invite myself', err);
  }
};
</script>

<template>
  <div class="h-full flex flex-col">
    <header class="mb-5 flex justify-between items-end">
      <div>
        <h1 class="text-2xl font-extrabold text-text tracking-tight flex items-center gap-2">
          <Globe2
            class="text-primary"
            :size="24"
          /> {{ t('locations.title') }}
        </h1>
        <p class="text-text-muted/70 text-sm mt-1">
          {{ t('locations.subtitle') }}
        </p>
      </div>
      <div class="flex items-center gap-2">
        <span class="text-xs font-bold px-3 py-1 rounded-full bg-green-500/10 text-green-400">{{ t('locations.online_count', { count: totalOnline }) }}</span>
        <span class="text-xs font-bold px-3 py-1 rounded-full bg-surface text-text-muted">{{ t('locations.offline_count', { count: offlineFriends.length }) }}</span>
        <button
          class="p-2 rounded-full bg-surface hover:bg-primary/10 text-primary shadow-sm border-primary transition-colors"
          @click="fetchLocations"
        >
          <RefreshCcw
            class="w-4 h-4"
            :class="{'animate-spin': loading}"
          />
        </button>
      </div>
    </header>

    <div class="flex-1 overflow-y-auto pr-1 custom-scrollbar space-y-4">
      <div
        v-if="loading && locations.length === 0"
        class="flex items-center justify-center py-16 text-primary font-bold animate-pulse"
      >
        <Globe2
          class="animate-spin mr-3"
          :size="24"
        /> {{ t('locations.scanning') }}
      </div>

      <!-- 公开实例 -->
      <div
        v-for="loc in locations"
        :key="loc.fullLocation"
        class="bg-surface backdrop-blur rounded-2xl border-primary hover:border-primary transition-all overflow-hidden"
      >
        <div class="px-4 py-3 bg-primary/10 flex items-center justify-between border-primary">
          <div class="flex items-center gap-2 min-w-0">
            <MapPin
              class="text-primary flex-shrink-0"
              :size="16"
            />
          <span class="font-bold text-primary text-sm truncate">
            {{ loc.worldName || loc.worldId }}
            <span v-if="!loc.worldName" class="text-primary/40 text-xs font-normal animate-pulse">...</span>
          </span>
          </div>
          <div class="flex items-center gap-2">
            <span class="text-xs font-bold px-2 py-0.5 rounded-full bg-primary/10 text-primary flex-shrink-0 flex items-center gap-1 mr-2">
              <Users :size="10" /> {{ loc.friends.length }}
            </span>
            <button
              class="px-3 py-1 bg-surface hover:bg-primary/10 border-border-soft hover:border-primary text-primary rounded-lg text-xs font-bold shadow-sm transition-all"
              title="Launch VRChat"
              @click="launchInstance(loc.fullLocation)"
            >
              Join
            </button>
            <button
              class="px-3 py-1 bg-primary text-white hover:brightness-110 rounded-lg text-xs font-bold shadow-sm transition-all"
              title="Drop Portal (Invite Myself)"
              @click="inviteMyself(loc.worldId, loc.instanceId)"
            >
              Drop Portal
            </button>
          </div>
        </div>
        <div class="p-3 grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-6 gap-2">
          <div
            v-for="friend in loc.friends"
            :key="friend.id"
            class="flex items-center gap-2 p-2 rounded-xl bg-primary/5 hover:bg-primary/10 transition-colors cursor-pointer"
            @click="profileStore.openProfile(friend.id, friend)"
          >
            <div class="relative flex-shrink-0">
              <div class="w-8 h-8 rounded-full overflow-hidden border-border-soft shadow-sm">
                <VrcAvatar
                  :user="friend"
                  custom-class="w-full h-full object-cover"
                />
              </div>
              <div
                class="absolute -bottom-0.5 -right-0.5 w-3 h-3 rounded-full border-border-soft"
                :class="getStatusDot(friend.status)"
              />
            </div>
            <div class="flex items-center gap-0.5 min-w-0">
              <span v-for="flag in getFlags(friend.tags || [])" :key="flag" class="text-[11px] leading-none shrink-0">{{ flag }}</span>
              <span class="text-xs font-bold truncate" :style="{ color: getTrustColor(friend.tags || []) || 'var(--theme-primary)' }">{{ friend.displayName }}</span>
            </div>
          </div>
        </div>
      </div>

      <div
        v-if="privateFriends.length > 0"
        class="bg-surface backdrop-blur rounded-2xl border-border-soft overflow-hidden"
      >
        <div class="px-4 py-3 bg-surface-hover flex items-center justify-between border-border-soft">
          <div class="flex items-center gap-2">
            <Lock
              class="text-border-strong"
              :size="16"
            />
            <span class="font-bold text-text-muted text-sm">{{ t('locations.private') }}</span>
          </div>
          <span class="text-xs font-bold px-2 py-0.5 rounded-full bg-surface text-text-muted flex items-center gap-1">
            <Users :size="10" /> {{ privateFriends.length }}
          </span>
        </div>
        <div class="p-3 grid grid-cols-3 sm:grid-cols-4 md:grid-cols-6 lg:grid-cols-8 gap-2">
          <div
            v-for="friend in privateFriends"
            :key="friend.id"
            class="flex flex-col items-center gap-1 p-2 rounded-xl bg-surface-hover hover:bg-surface transition-colors cursor-pointer"
            @click="profileStore.openProfile(friend.id, friend)"
          >
            <div class="relative">
              <div class="w-8 h-8 rounded-full overflow-hidden border-border-soft shadow-sm">
                <VrcAvatar
                  :user="friend"
                  custom-class="w-full h-full object-cover"
                />
              </div>
              <div
                class="absolute -bottom-0.5 -right-0.5 w-3 h-3 rounded-full border-border-soft"
                :class="getStatusDot(friend.status)"
              />
            </div>
            <div class="flex items-center gap-0.5 max-w-full">
              <span v-for="flag in getFlags(friend.tags || [])" :key="flag" class="text-[9px] leading-none shrink-0">{{ flag }}</span>
              <span class="text-[10px] font-bold truncate" :style="{ color: getTrustColor(friend.tags || []) || 'var(--theme-text-muted)' }">{{ friend.displayName }}</span>
            </div>
          </div>
        </div>
      </div>

      <!-- 无数据 -->
      <div
        v-if="!loading && locations.length === 0 && privateFriends.length === 0"
        class="flex flex-col items-center justify-center py-16 text-primary"
      >
        <Globe2
          :size="48"
          class="mb-4 opacity-40"
        />
        <p class="font-bold text-lg">
          {{ t('locations.no_online') }}
        </p>
        <p class="text-sm mt-1">
          {{ t('locations.no_online_desc') }}
        </p>
      </div>
    </div>
  </div>
</template>


