<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue';
import { VrcApi, SysApi } from '../api';
import VrcAvatar from './VrcAvatar.vue';
import { MapPin, Users, Globe2, RefreshCcw, Lock, Eye } from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';

const { t } = useI18n();

interface FriendLocation {
  worldId: string;
  instanceId: string;
  fullLocation: string;
  friends: any[];
  worldName?: string;
}

const locations = ref<FriendLocation[]>([]);
const loading = ref(true);
const privateFriends = ref<any[]>([]);
const offlineFriends = ref<any[]>([]);
let timer: number | null = null;

const fetchLocations = async () => {
  try {
    const friends = await VrcApi.getFriends({ n: 100, offset: 0 });
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
          locMap.set(loc, {
            worldId,
            instanceId,
            fullLocation: loc,
            friends: [],
            worldName: undefined,
          });
        }
        locMap.get(loc)!.friends.push(f);
      }
    }

    // 按好友数降序排列
    locations.value = Array.from(locMap.values()).sort((a, b) => b.friends.length - a.friends.length);
    privateFriends.value = privates;
    offlineFriends.value = offlines;

    // 异步获取世界名（不阻塞渲染，加入延迟防封禁）
    const fetchWorldNames = async () => {
      for (const loc of locations.value) {
        if (!loc.worldName) {
          // 检查本地全局缓存
          if ((window as any).__WORLD_NAME_CACHE__ && (window as any).__WORLD_NAME_CACHE__.has(loc.worldId)) {
            loc.worldName = (window as any).__WORLD_NAME_CACHE__.get(loc.worldId);
            continue;
          }
          try {
            const w = await VrcApi.getWorld({ worldId: loc.worldId });
            if (w && w.name) {
              loc.worldName = w.name;
              if (!(window as any).__WORLD_NAME_CACHE__) (window as any).__WORLD_NAME_CACHE__ = new Map();
              (window as any).__WORLD_NAME_CACHE__.set(loc.worldId, w.name);
            } else {
              loc.worldName = loc.worldId; 
            }
          } catch (e) {
            console.warn(t('auto_00ef9b81'), e);
          }
          // 延迟 500ms，防止触发 VRChat API 429 限制
          await new Promise(r => setTimeout(r, 500));
        }
      }
    };
    
    // 不 await，让它在后台慢慢跑
    fetchWorldNames();
  } catch (err) {
    console.warn(t('auto_79d6305c'), err);
  } finally {
    loading.value = false;
  }
};

onMounted(() => {
  fetchLocations();
  timer = setInterval(fetchLocations, 60000) as unknown as number;
});
onUnmounted(() => { if (timer) clearInterval(timer); });

const totalOnline = computed(() => {
  return locations.value.reduce((s, l) => s + l.friends.length, 0) + privateFriends.value.length;
});

const getStatusDot = (status: string) => {
  const s = status?.toLowerCase() || '';
  if (s === 'active' || s === 'online') return 'bg-green-500';
  if (s === 'join me') return 'bg-blue-500';
  if (s === 'ask me' || s === 'busy') return 'bg-orange-500';
  if (s === 'do not disturb' || s === 'dnd') return 'bg-red-500';
  if (s === 'offline') return 'bg-slate-400';
  return 'bg-green-500';
};

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
            <span class="font-bold text-primary text-sm truncate">{{ loc.worldName || loc.worldId }}</span>
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
            class="flex items-center gap-2 p-2 rounded-xl bg-primary/10 hover:bg-primary/10 transition-colors"
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
            <span class="text-xs font-bold text-primary truncate">{{ friend.displayName }}</span>
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
            class="flex flex-col items-center gap-1 p-2 rounded-xl bg-surface-hover hover:bg-surface transition-colors"
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
            <span class="text-[10px] font-bold text-text-muted truncate max-w-full">{{ friend.displayName }}</span>
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


