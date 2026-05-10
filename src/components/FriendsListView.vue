<script setup lang="ts">
import { ref, onMounted, computed, onUnmounted } from 'vue';
import { VrcApi, DbApi, SysApi, GamelogApi } from "../api";
import { RefreshCcw, Search, MapPin, Loader2, Users } from 'lucide-vue-next';
import VrcResourceCard from './VrcResourceCard.vue';
import type { VrcUser } from '../types/vrc';
import { useI18n } from 'vue-i18n';
import { useUserProfileStore } from '../stores/userProfile';

const { t } = useI18n();
const profileStore = useUserProfileStore();

const onlineFriends = ref<VrcUser[]>([]);
const offlineFriends = ref<VrcUser[]>([]);
const loading = ref(true);
const errorMsg = ref('');
const searchQuery = ref('');

const favoriteGroups = ref<any[]>([]);
const favorites = ref<any[]>([]);
const collapsedGroups = ref<Set<string>>(new Set());

const toggleGroup = (groupName: string) => {
  if (collapsedGroups.value.has(groupName)) {
    collapsedGroups.value.delete(groupName);
  } else {
    collapsedGroups.value.add(groupName);
  }
};

const fetchFriends = async () => {
  loading.value = true;
  errorMsg.value = '';
  try {
    // 1. Fetch cached friends
    const cached: any[] = await DbApi.getCachedFriends() || [];
    
    // 2. Fetch friend favorite groups
    try {
      const groupsRes: any = await VrcApi.getFavoriteGroups();
      favoriteGroups.value = Array.isArray(groupsRes) ? groupsRes.filter(g => g.type === 'friend') : [];
      
      const favsRes: any = await VrcApi.getFavorites({ type: 'friend', n: 100 });
      favorites.value = Array.isArray(favsRes) ? favsRes : [];
    } catch (err) {
      console.warn('Failed to fetch favorite groups, fallback to normal list');
    }

    onlineFriends.value = cached.filter((f: any) => f.location && f.location !== 'offline');
    offlineFriends.value = cached.filter((f: any) => !f.location || f.location === 'offline');
  } catch (err: any) {
    errorMsg.value = err.message || err;
  } finally {
    loading.value = false;
  }
};

onMounted(() => {
  fetchFriends();
  window.addEventListener('vrc-friends-synced', fetchFriends);
  window.addEventListener('vrc-pipeline-event', fetchFriends);
});

onUnmounted(() => {
  window.removeEventListener('vrc-friends-synced', fetchFriends);
  window.removeEventListener('vrc-pipeline-event', fetchFriends);
});

const allFriends = computed(() => [...onlineFriends.value, ...offlineFriends.value]);
const onlineCount = computed(() => onlineFriends.value.length);
const offlineCount = computed(() => offlineFriends.value.length);
const totalCount = computed(() => allFriends.value.length);

const groupedFriends = computed(() => {
  const result: { name: string; displayName: string; isOffline: boolean; friends: any[] }[] = [];
  
  const favMap = new Map<string, Set<string>>(); // friendId -> Set<groupName>
  favorites.value.forEach(f => {
    if (!favMap.has(f.favoriteId)) favMap.set(f.favoriteId, new Set());
    favMap.get(f.favoriteId)!.add(f.tags[0]); 
  });

  // 1. Favorite Groups
  favoriteGroups.value.forEach(g => {
    const friendsInGroup = onlineFriends.value.filter(f => favMap.get(f.id)?.has(g.name));
    if (friendsInGroup.length > 0) {
      result.push({
        name: g.name,
        displayName: g.displayName,
        isOffline: false,
        friends: friendsInGroup
      });
    }
  });

  // 2. Active (Online but not in any favorite group)
  const activeFriends = onlineFriends.value.filter(f => !favMap.has(f.id));
  if (activeFriends.length > 0) {
    result.push({
      name: 'active',
      displayName: t('status.online') || '在线',
      isOffline: false,
      friends: activeFriends
    });
  }

  // 3. Offline
  if (offlineFriends.value.length > 0) {
    result.push({
      name: 'offline',
      displayName: t('status.offline') || '离线',
      isOffline: true,
      friends: offlineFriends.value
    });
  }

  return result;
});

const filteredGroupedFriends = computed(() => {
  const q = searchQuery.value.toLowerCase();
  if (!q) return groupedFriends.value;
  
  return groupedFriends.value.map(g => ({
    ...g,
    friends: g.friends.filter(f => f.displayName?.toLowerCase().includes(q))
  })).filter(g => g.friends.length > 0);
});

const openDetail = (friend: any) => {
  profileStore.openProfile(friend.id, friend);
};
</script>

<template>
  <div class="h-full flex flex-col p-6 bg-slate-50/50 rounded-3xl relative overflow-hidden">
    <!-- Subtle Background Glow -->
    <div class="absolute top-0 right-0 w-96 h-96 bg-indigo-500/10 rounded-full blur-[100px] pointer-events-none -z-10" />
    <div class="absolute bottom-0 left-0 w-[500px] h-[500px] bg-blue-500/5 rounded-full blur-[120px] pointer-events-none -z-10" />

    <div class="flex items-center justify-between mb-8 shrink-0 z-10">
      <h1 class="text-3xl font-extrabold text-slate-900 tracking-tight flex items-center gap-3">
        <span class="inline-flex items-center justify-center p-2 bg-indigo-100 rounded-2xl shadow-sm border border-indigo-200/50">
          <Users class="w-6 h-6 text-indigo-600" />
        </span>
        {{ t('friends_list.title') }}
      </h1>
      <div class="flex items-center gap-3">
        <span class="text-xs font-bold px-3 py-1.5 rounded-xl bg-green-100 text-green-700 border border-green-200 shadow-sm">🟢 {{ onlineCount }}</span>
        <span class="text-xs font-bold px-3 py-1.5 rounded-xl bg-slate-100 text-slate-600 border border-slate-200 shadow-sm">⚫ {{ offlineCount }}</span>
        <span class="text-xs font-bold px-3 py-1.5 rounded-xl bg-indigo-100 text-indigo-700 border border-indigo-200 shadow-sm">{{ t('friends_list.total_count', { count: totalCount }) }}</span>
        <button
          class="p-2 rounded-xl bg-white hover:bg-slate-50 text-slate-700 shadow-sm border border-slate-200 transition-colors"
          @click="fetchFriends"
        >
          <RefreshCcw
            class="w-5 h-5"
            :class="{'animate-spin text-indigo-600': loading}"
          />
        </button>
      </div>
    </div>

    <!-- 搜索框 -->
    <div class="relative mb-6 shrink-0 z-10">
      <div class="absolute inset-y-0 left-0 pl-4 flex items-center pointer-events-none">
        <Search class="h-4 w-4 text-slate-400" />
      </div>
      <input
        v-model="searchQuery"
        type="text"
        class="block w-full pl-12 pr-4 py-3 bg-white border border-slate-200 shadow-sm rounded-xl text-slate-900 placeholder-slate-400 focus:outline-none focus:border-indigo-400 focus:ring-2 focus:ring-indigo-500/10 text-sm font-bold transition-all"
        :placeholder="t('friends_list.search_placeholder')"
      >
    </div>

    <div
      v-if="errorMsg"
      class="bg-red-50 text-red-600 p-3 rounded-xl border border-red-200 text-sm font-bold mb-4 z-10"
    >
      {{ errorMsg }}
    </div>

    <!-- 加载中 -->
    <div
      v-if="loading && allFriends.length === 0"
      class="flex-1 flex flex-col items-center justify-center text-indigo-500/80 z-10"
    >
      <Loader2
        class="animate-spin mb-4"
        :size="48"
      />
      <p class="font-extrabold text-lg tracking-wide">
        {{ t('friends_list.loading') }}
      </p>
    </div>

    <!-- 好友列表 -->
    <div
      v-else
      class="flex-1 overflow-y-auto pr-2 custom-scrollbar z-10 relative"
    >
      <div
        v-for="group in filteredGroupedFriends"
        :key="group.name"
        class="mb-8"
      >
        <!-- 分组标题 -->
        <div
          class="sticky top-0 bg-slate-50/90 backdrop-blur py-2 px-2 z-20 flex items-center justify-between cursor-pointer group/header rounded-lg mb-3"
          @click="toggleGroup(group.name)"
        >
          <h2 class="text-sm font-extrabold text-slate-700 uppercase tracking-widest flex items-center gap-2 group-hover/header:text-indigo-600 transition-colors">
            <div
              class="w-1.5 h-1.5 rounded-full"
              :class="group.isOffline ? 'bg-slate-400' : 'bg-green-500 shadow shadow-green-500/50'"
            />
            {{ group.displayName }}
            <span class="bg-slate-200/50 text-slate-500 px-2 py-0.5 rounded-md text-[10px]">{{ group.friends.length }}</span>
          </h2>
          <div
            class="text-slate-400 text-xs font-bold transition-transform"
            :class="collapsedGroups.has(group.name) ? '' : 'rotate-180'"
          >
            ▼
          </div>
        </div>
        
        <!-- 分组内容 -->
        <div
          v-show="!collapsedGroups.has(group.name)"
          class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4 px-2"
        >
          <VrcResourceCard
            v-for="friend in group.friends"
            :key="friend.id"
            type="user"
            :data="friend"
            :is-user="true"
            minimal
            @click="openDetail(friend)"
          />
        </div>
      </div>

      <div
        v-if="filteredGroupedFriends.length === 0"
        class="h-full flex flex-col items-center justify-center text-slate-400 pt-20"
      >
        <Users
          class="mb-4 opacity-30"
          :size="64"
        />
        <p class="font-bold text-xl text-slate-500">
          暂无数据
        </p>
      </div>
    </div>
  </div>
</template>

<style scoped>
.custom-scrollbar::-webkit-scrollbar { width: 6px; }
.custom-scrollbar::-webkit-scrollbar-track { background: transparent; }
.custom-scrollbar::-webkit-scrollbar-thumb { background: #cbd5e1; border-radius: 10px; }
.custom-scrollbar::-webkit-scrollbar-thumb:hover { background: #94a3b8; }
</style>
