<script setup lang="ts">
import { ref, onMounted, computed, onUnmounted, watch } from 'vue';
import { VrcApi, DbApi } from "../api";
import { isTauri } from '@tauri-apps/api/core';
import { Search, RefreshCcw, Settings, ChevronDown, ChevronRight, UsersRound, X } from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';
import { useAuthStore } from '../stores/authStore';
import { useUserProfileStore } from '../stores/userProfile';
import { useEntityModalStore } from '../stores/entityModal';
import { useFriendsStore } from '../stores/friendsStore';
import FriendItem from './FriendItem.vue';

const { t } = useI18n();
const profileStore = useUserProfileStore();
const authStore = useAuthStore();
const entityStore = useEntityModalStore();
const friendsStore = useFriendsStore();

const onlineFriends = ref<any[]>([]);
const activeFriends = ref<any[]>([]); // 活跃中 (仅登录网页端)
const offlineFriends = ref<any[]>([]);
const groups = ref<any[]>([]);
const loading = ref(true);
const groupsLoading = ref(false);
const groupsLoaded = ref(false);
const errorMsg = ref('');
const groupsErrorMsg = ref('');
const searchQuery = ref('');
const groupSearchQuery = ref('');

const activeTab = ref<'friends' | 'groups'>('friends');

// Accordion states
const collapsedSections = ref<Set<string>>(new Set(['active', 'offline']));

const toggleSection = (sectionName: string) => {
  if (collapsedSections.value.has(sectionName)) {
    collapsedSections.value.delete(sectionName);
  } else {
    collapsedSections.value.add(sectionName);
  }
};

const currentUser = computed(() => authStore.currentUser);
let fetchTimeout: any = null;

const fetchFriends = async () => {
  if (fetchTimeout) {
    clearTimeout(fetchTimeout);
  }
  
  fetchTimeout = setTimeout(async () => {
    loading.value = true;
    errorMsg.value = '';
    try {
      // Use shared store — avoids duplicate API calls from DashboardView, ChartsView, etc.
      const cached = await friendsStore.fetchFriends();

      const normalized = cached.filter((f: any) => f?.id || f?.displayName);
      const isOnlineLocation = (location: unknown) => {
        const loc = String(location || '').trim().toLowerCase();
        return Boolean(loc) && loc !== 'offline';
      };
      const isActiveStatus = (status: unknown) => {
        const value = String(status || '').trim().toLowerCase();
        return Boolean(value) && value !== 'offline';
      };

      onlineFriends.value = normalized.filter((f: any) => isOnlineLocation(f.location));
      activeFriends.value = normalized.filter((f: any) => !isOnlineLocation(f.location) && isActiveStatus(f.status));
      offlineFriends.value = normalized.filter((f: any) => !isOnlineLocation(f.location) && !isActiveStatus(f.status));
    } catch (err: any) {
      errorMsg.value = err.message || err;
    } finally {
      loading.value = false;
      // 后台解析世界名称
      resolveWorldNames();
    }
  }, 500); // Reduced debounce: 500ms (was 1s)
};

const normalizeGroup = (entry: any) => {
  const group = entry?.group || entry;
  if (!group?.id && !group?.groupId) return null;
  return {
    ...group,
    id: group.id || group.groupId,
    name: group.name || group.displayName || group.shortCode || group.id || group.groupId,
    shortCode: group.shortCode || group.discriminator || '',
    iconUrl: group.iconUrl || group.thumbnailUrl || group.bannerUrl || '',
    memberCount: group.memberCount ?? group.members ?? group.member_count ?? 0,
    privacy: group.privacy || group.joinState || group.visibility || '',
  };
};

const fetchGroups = async (force = false) => {
  if (groupsLoading.value) return;
  if (groupsLoaded.value && !force) return;

  groupsLoading.value = true;
  groupsErrorMsg.value = '';
  try {
    const res: any = await VrcApi.getGroups({ userId: currentUser.value?.id });
    const list = Array.isArray(res) ? res : [];
    groups.value = list
      .map(normalizeGroup)
      .filter((group): group is any => Boolean(group))
      .sort((a, b) => (a.name || '').localeCompare(b.name || ''));
    groupsLoaded.value = true;
  } catch (err: any) {
    groupsErrorMsg.value = err.message || String(err);
  } finally {
    groupsLoading.value = false;
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
  if (fetchTimeout) clearTimeout(fetchTimeout);
});

const onlineCount = computed(() => onlineFriends.value.length);
const totalCount = computed(() => onlineFriends.value.length + activeFriends.value.length + offlineFriends.value.length);
const groupCount = computed(() => groups.value.length);
const currentSearchQuery = computed({
  get: () => activeTab.value === 'groups' ? groupSearchQuery.value : searchQuery.value,
  set: (value: string) => {
    if (activeTab.value === 'groups') {
      groupSearchQuery.value = value;
      return;
    }

    searchQuery.value = value;
  },
});

const searchPlaceholder = computed(() =>
  activeTab.value === 'groups' ? t('groups.search_placeholder') : t('friends.search_placeholder')
);

// Extract Trust Color — aligned with VrcDog color scheme
const getTrustColor = (tags: string[]) => {
  if (!tags || !tags.length) return '#9e9e9e'; // Visitor - gray
  if (tags.includes('system_trust_legend')) return '#ff69b4';   // Legend - pink (VrcDog)
  if (tags.includes('system_trust_veteran')) return '#8b5cf6';  // Trusted User - purple (VrcDog)
  if (tags.includes('system_trust_trusted')) return '#ff7b42';  // Known User - orange
  if (tags.includes('system_trust_known')) return '#2bcf5c';    // User - green
  if (tags.includes('system_trust_basic')) return '#1778ff';    // New User - blue
  return '#9e9e9e'; // Visitor - gray
};

// Grouping Logic matching VrcDog
const groupedFriends = computed(() => {
  let q = searchQuery.value.toLowerCase();
  let list = onlineFriends.value;
  if (q) list = list.filter(f => f.displayName?.toLowerCase().includes(q));

  // Same Room (Location sharing logic: Group by location if > 1 friend in it)
  const locationMap = new Map<string, any[]>();
  list.forEach(f => {
    if (f.location && f.location !== 'private') {
      if (!locationMap.has(f.location)) locationMap.set(f.location, []);
      locationMap.get(f.location)!.push(f);
    }
  });

  const sameRoomGroups: { location: string, locationName: string, flag: string, friends: any[] }[] = [];
  const justOnline: any[] = [];

  locationMap.forEach((friendsInLoc, loc) => {
    if (friendsInLoc.length > 1) { // 2 or more friends in the same instance
      let locName = friendsInLoc[0]?.location || t('auto_65717df5');
      let flag = '🌐';
      if (locName.includes('JP')) flag = '🇯🇵';
      else if (locName.includes('US')) flag = '🇺🇸';
      else if (locName.includes('CN')) flag = '🇨🇳';
      else if (locName.includes('EU')) flag = '🇪🇺';
      else if (locName.includes('KR')) flag = '🇰🇷';
      
      // Clean up location name for display - resolve world name from cache
      let displayLoc = locName.split('~')[0];
      const worldId = displayLoc.split(':')[0];
      if (worldId.startsWith('wrld_') && worldNameCache.value.has(worldId)) {
        displayLoc = worldNameCache.value.get(worldId)!;
      } else if (worldId.startsWith('wrld_')) {
        displayLoc = worldId; // 暂时显示 ID，后台会异步更新
      }
      
      sameRoomGroups.push({ location: loc, locationName: displayLoc, flag, friends: friendsInLoc });
    } else {
      justOnline.push(friendsInLoc[0]);
    }
  });

  sameRoomGroups.sort((a, b) => b.friends.length - a.friends.length);
  list.filter(f => f.location === 'private').forEach(f => justOnline.push(f));
  justOnline.sort((a, b) => (a.displayName || '').localeCompare(b.displayName || ''));

  return {
    sameRoom: sameRoomGroups,
    online: justOnline
  };
});

const filteredGroups = computed(() => {
  const q = groupSearchQuery.value.trim().toLowerCase();
  if (!q) return groups.value;

  return groups.value.filter((group) => {
    const fields = [
      group.name,
      group.shortCode,
      group.description,
      group.privacy,
    ].filter(Boolean);

    return fields.some((field) => String(field).toLowerCase().includes(q));
  });
});

const openDetail = (friend: any) => {
  profileStore.openProfile(friend.id, friend);
};

const openGroupDetail = async (group: any) => {
  await entityStore.openGroup(group);
};

const setActiveTab = (tab: 'friends' | 'groups') => {
  activeTab.value = tab;
};

const refreshCurrentTab = () => {
  if (activeTab.value === 'groups') {
    fetchGroups(true);
  } else {
    fetchFriends();
  }
};

const clearCurrentSearch = () => {
  currentSearchQuery.value = '';
};

watch(activeTab, (tab) => {
  if (tab === 'groups') {
    fetchGroups();
  }
});

// Status dot color
const getStatusColor = (status: string) => {
  const s = status?.toLowerCase() || '';
  if (s === 'active' || s === 'online') return '#22c55e'; // green
  if (s === 'join me') return '#3b82f6'; // blue
  if (s === 'ask me' || s === 'busy') return '#f97316'; // orange (busy is orange)
  if (s === 'do not disturb' || s === 'dnd') return '#ef4444'; // red (dnd is red)
  if (s === 'offline') return '#94a3b8'; // gray
  return '#22c55e'; // Default to green
};

const getFlag = (locName: string) => {
  if (!locName) return '🌐';
  if (locName.includes('JP')) return '🇯🇵';
  if (locName.includes('US')) return '🇺🇸';
  if (locName.includes('CN')) return '🇨🇳';
  if (locName.includes('EU')) return '🇪🇺';
  if (locName.includes('KR')) return '🇰🇷';
  return '🌐';
};

const cleanLocName = (loc: string) => {
  if (!loc) return t('auto_0fcd7253');
  if (loc === 'private') return t('auto_4ad3ada9');
  // 如果是 wrld_ 开头，尝试从缓存获取世界名
  if (loc.startsWith('wrld_')) {
    const worldId = loc.split(':')[0];
    return worldNameCache.value.get(worldId) || loc.split('~')[0];
  }
  return loc.split('~')[0] || t('auto_0fcd7253');
};

// 世界名缓存 (响应式)
const worldNameCache = ref(new Map<string, string>());

// 解析世界名称 (后台并发，最多5个同时)
const resolveWorldNames = async () => {
  const worldIds = new Set<string>();
  onlineFriends.value.forEach(f => {
    if (f.location && f.location.startsWith('wrld_')) {
      worldIds.add(f.location.split(':')[0]);
    }
  });

  // 先从缓存批量读取
  const needFetch: string[] = [];
  for (const worldId of worldIds) {
    if (worldNameCache.value.has(worldId)) continue;
    try {
      const cached = await DbApi.getApiCache({ key: `world_name:${worldId}` });
      if (cached) {
        worldNameCache.value.set(worldId, cached);
        worldNameCache.value = new Map(worldNameCache.value);
        continue;
      }
    } catch { /* ignore */ }
    needFetch.push(worldId);
  }

  if (needFetch.length === 0) return;

  // 并发限制5个，避免大量API请求
  const CONCURRENCY = 5;
  let idx = 0;
  async function worker() {
    while (idx < needFetch.length) {
      const worldId = needFetch[idx++];
      try {
        const w: any = await VrcApi.getWorld({ worldId });
        if (w?.name) {
          worldNameCache.value.set(worldId, w.name);
          worldNameCache.value = new Map(worldNameCache.value);
          await DbApi.saveApiCache({ key: `world_name:${worldId}`, data: w.name });
        }
      } catch { /* ignore */ }
    }
  }
  await Promise.all(Array.from({ length: Math.min(CONCURRENCY, needFetch.length) }, worker));
};
</script>

<template>
  <div class="h-full w-full bg-[var(--theme-surface)]/20 text-[var(--theme-text-muted)] overflow-hidden rounded-xl border border-border-soft flex">
    <div class="w-full h-full flex flex-col bg-[var(--theme-surface)]/60 backdrop-blur-md">
      
      <!-- Top Search & Title Bar -->
      <div class="p-3 border-b-2 border-border-soft shrink-0">
        <div class="relative group">
          <Search class="absolute left-4 top-1/2 -translate-y-1/2 w-4 h-4 text-[var(--theme-text-muted)] group-focus-within:text-primary transition-colors" />
          <input 
            v-model="currentSearchQuery"
            type="text" 
            :placeholder="searchPlaceholder" 
            class="w-full bg-[var(--theme-surface)]-hover text-[var(--theme-text)] pl-10 pr-10 py-2.5 rounded-full text-sm outline-none placeholder-[var(--theme-text-muted)] focus:ring-4 focus:ring-primary/20 transition-all border-2 border-border-soft focus:border-primary/50 font-bold"
          />
          <button v-if="currentSearchQuery" @click="clearCurrentSearch" class="absolute right-4 top-1/2 -translate-y-1/2 text-[var(--theme-text-muted)] hover:text-[var(--theme-text)] transition-transform hover:scale-110">
            <X class="w-4 h-4" />
          </button>
        </div>
      </div>

      <!-- Tabs -->
      <div class="flex border-b border-border-soft shrink-0 bg-[var(--theme-surface)]/40">
        <button 
          class="flex-1 py-2.5 text-[13px] font-bold transition-colors relative"
          :class="activeTab === 'friends' ? 'text-[var(--theme-text)]' : 'text-[var(--theme-text-muted)] hover:text-[var(--theme-text)]'"
          @click="setActiveTab('friends')"
        >
          {{ t('friends.title') }} ({{ totalCount }})
          <div v-if="activeTab === 'friends'" class="absolute bottom-0 left-0 right-0 h-[2px] bg-primary"></div>
        </button>
        <button 
          class="flex-1 py-2.5 text-[13px] font-bold transition-colors relative"
          :class="activeTab === 'groups' ? 'text-[var(--theme-text)]' : 'text-[var(--theme-text-muted)] hover:text-[var(--theme-text)]'"
          @click="setActiveTab('groups')"
        >
          {{ t('groups.title') }} ({{ groupCount }})
          <div v-if="activeTab === 'groups'" class="absolute bottom-0 left-0 right-0 h-[2px] bg-primary"></div>
        </button>
      </div>

      <!-- Content Area -->
      <div class="flex-1 overflow-y-auto custom-scrollbar">
        
        <div v-if="activeTab === 'friends' && loading && onlineCount === 0" class="flex justify-center items-center h-20 text-[var(--theme-text-muted)]">
          <RefreshCcw class="w-5 h-5 animate-spin" />
        </div>

        <template v-else-if="activeTab === 'friends'">
          
          <!-- 我 (Me) -->
          <div class="mb-2 mt-2">
            <div 
              class="flex items-center gap-2 py-2 px-4 cursor-pointer bg-[var(--theme-surface)] border-2 border-transparent hover:border-border-soft rounded-2xl transition-all text-[14px] font-black text-[var(--theme-text)] select-none mx-2 shadow-sm hover:shadow-md hover:-translate-y-0.5"
              @click="toggleSection('me')"
            >
              <component :is="collapsedSections.has('me') ? ChevronRight : ChevronDown" class="w-4 h-4 text-primary" />
              {{ t('friends.me') }}
            </div>
            <div v-show="!collapsedSections.has('me')" class="mt-1 pl-1">
              <FriendItem 
                v-if="currentUser"
                :friend="currentUser" 
                :statusColor="getStatusColor(currentUser.status || 'offline')" 
                :trustColor="getTrustColor(currentUser.tags || [])" 
                @click="openDetail(currentUser)"
              >
                <template #subtitle>
                  <span class="text-[12px] font-medium text-text-muted mt-0.5 truncate">{{ currentUser.statusDescription || t('status.online') }}</span>
                </template>
              </FriendItem>
            </div>
          </div>

          <!-- 同一房间 (Same Room) -->
          <div v-if="groupedFriends.sameRoom.length > 0" class="mb-2">
            <div 
              class="flex items-center gap-2 py-2 px-4 cursor-pointer bg-[var(--theme-surface)] border-2 border-transparent hover:border-border-soft rounded-2xl transition-all text-[14px] font-black text-[var(--theme-text)] select-none mx-2 shadow-sm hover:shadow-md hover:-translate-y-0.5"
              @click="toggleSection('sameRoom')"
            >
              <component :is="collapsedSections.has('sameRoom') ? ChevronRight : ChevronDown" class="w-4 h-4 text-primary" />
              {{ t('friends.same_room') }}
              <span class="ml-auto bg-primary text-white px-2.5 py-0.5 rounded-full text-[11px] border-2 border-primary/20">{{ groupedFriends.sameRoom.length }}</span>
            </div>
            
            <div v-show="!collapsedSections.has('sameRoom')" class="mt-1 pl-1">
              <template v-for="group in groupedFriends.sameRoom" :key="group.location">
                <!-- Location Header -->
                <div class="flex items-center gap-2 py-1.5 px-4 text-[13px] font-bold text-[var(--theme-text-muted)] bg-[var(--theme-surface)]-hover rounded-[16px] mx-4 mb-2 mt-2 border-2 border-border-soft shadow-sm hover:scale-[1.01] transition-transform">
                  <span class="text-[16px] drop-shadow-sm">{{ group.flag }}</span>
                  <span class="truncate max-w-[200px]">{{ group.locationName }}</span>
                  <span class="ml-auto shrink-0 bg-[var(--theme-surface)] border-2 border-border-soft px-2.5 py-0.5 rounded-full text-[11px] text-[var(--theme-text)]">{{ group.friends.length }}</span>
                </div>
                <!-- Friends in Location -->
                <FriendItem 
                  v-for="friend in group.friends" 
                  :key="friend.id"
                  :friend="friend" 
                  :statusColor="getStatusColor(friend.status)" 
                  :trustColor="getTrustColor(friend.tags)" 
                  @click="openDetail(friend)"
                >
                  <template #subtitle>
                    <span class="text-[12px] font-medium text-text-muted mt-0.5">{{ t('auto_3bdab2c6') }}</span>
                  </template>
                </FriendItem>
              </template>
            </div>
          </div>

          <!-- 在线 (Online) -->
          <div v-if="groupedFriends.online.length > 0" class="mb-2">
            <div 
              class="flex items-center gap-2 py-2 px-4 cursor-pointer bg-[var(--theme-surface)] border-2 border-transparent hover:border-border-soft rounded-2xl transition-all text-[14px] font-black text-[var(--theme-text)] select-none mx-2 shadow-sm hover:shadow-md hover:-translate-y-0.5"
              @click="toggleSection('online')"
            >
              <component :is="collapsedSections.has('online') ? ChevronRight : ChevronDown" class="w-4 h-4 text-primary" />
              {{ t('friends.online') }}
              <span class="ml-auto bg-primary text-white px-2.5 py-0.5 rounded-full text-[11px] border-2 border-primary/20">{{ groupedFriends.online.length }}</span>
            </div>
            
            <div v-show="!collapsedSections.has('online')" class="mt-1 pl-1 pr-1">
              <FriendItem 
                v-for="friend in groupedFriends.online" 
                :key="friend.id"
                :friend="friend" 
                :statusColor="getStatusColor(friend.status)" 
                :trustColor="getTrustColor(friend.tags)" 
                :location-name="cleanLocName(friend.location)"
                @click="openDetail(friend)"
              />
            </div>
          </div>

          <!-- 活跃中 (Active on website) -->
          <div v-if="activeFriends.length > 0 && !searchQuery" class="mb-2">
            <div 
              class="flex items-center gap-2 py-2 px-4 cursor-pointer bg-[var(--theme-surface)] border-2 border-transparent hover:border-border-soft rounded-2xl transition-all text-[14px] font-black text-[var(--theme-text)] select-none mx-2 shadow-sm hover:shadow-md hover:-translate-y-0.5"
              @click="toggleSection('active')"
            >
              <component :is="collapsedSections.has('active') ? ChevronRight : ChevronDown" class="w-4 h-4 text-primary" />
              {{ t('friends.active_web') }}
              <span class="ml-auto bg-primary text-white px-2.5 py-0.5 rounded-full text-[11px] border-2 border-primary/20">{{ activeFriends.length }}</span>
            </div>
            <div v-show="!collapsedSections.has('active')" class="mt-1 pl-1 pr-1">
              <FriendItem 
                v-for="friend in activeFriends" 
                :key="friend.id"
                :friend="friend" 
                :statusColor="getStatusColor(friend.status)" 
                :trustColor="getTrustColor(friend.tags)" 
                :location-name="cleanLocName(friend.location)"
                @click="openDetail(friend)"
              />
            </div>
          </div>

          <!-- 离线 (Offline) -->
          <div v-if="offlineFriends.length > 0 && !searchQuery" class="mb-4">
            <div 
              class="flex items-center gap-2 py-2 px-4 cursor-pointer bg-[var(--theme-surface)] border-2 border-transparent hover:border-border-soft rounded-2xl transition-all text-[14px] font-black text-[var(--theme-text-muted)] select-none mx-2 shadow-sm hover:shadow-md hover:-translate-y-0.5 opacity-80"
              @click="toggleSection('offline')"
            >
              <component :is="collapsedSections.has('offline') ? ChevronRight : ChevronDown" class="w-4 h-4" />
              {{ t('friends.offline') }}
              <span class="ml-auto bg-[var(--theme-surface)]-hover px-2.5 py-0.5 rounded-full text-[11px] border-2 border-border-soft">{{ offlineFriends.length }}</span>
            </div>
            <div v-show="!collapsedSections.has('offline')" class="mt-1 pl-1 pr-1 opacity-80 hover:opacity-100 transition-opacity">
              <FriendItem 
                v-for="friend in offlineFriends" 
                :key="friend.id"
                :friend="friend" 
                :statusColor="''" 
                :trustColor="''" 
                :isOffline="true"
                :location-name="cleanLocName(friend.location)"
                @click="openDetail(friend)"
              />
            </div>
          </div>

        </template>

        <template v-else-if="activeTab === 'groups'">
          <div v-if="groupsLoading" class="flex justify-center items-center h-24 text-primary">
            <RefreshCcw class="w-5 h-5 animate-spin" />
          </div>

          <div v-else-if="groupsErrorMsg" class="p-4 text-center text-sm text-red-500">
            <p class="font-bold mb-3">{{ groupsErrorMsg }}</p>
            <button
              class="px-4 py-2 rounded-xl bg-primary text-white font-bold text-xs hover:bg-primary-hover transition-colors"
              @click="fetchGroups(true)"
            >
              {{ t('global.retry') }}
            </button>
          </div>

          <div v-else-if="filteredGroups.length === 0" class="p-6 text-center text-[var(--theme-text-muted)] text-sm mt-8">
            <UsersRound class="w-10 h-10 mx-auto mb-3 opacity-40" />
            <p class="font-bold">
              {{ groupSearchQuery ? t('search.no_results') : t('groups.no_groups') }}
            </p>
          </div>

          <div v-else class="p-2 space-y-2">
            <button
              v-for="group in filteredGroups"
              :key="group.id"
              class="w-full flex items-center gap-3 p-3 rounded-2xl bg-[var(--theme-surface)] border-2 border-transparent hover:border-primary/30 hover:bg-[var(--theme-surface-hover)] transition-all text-left shadow-sm"
              @click="openGroupDetail(group)"
            >
              <div class="w-11 h-11 rounded-xl overflow-hidden bg-primary/10 border border-border-soft shrink-0 flex items-center justify-center">
                <img
                  v-if="group.iconUrl"
                  :src="group.iconUrl"
                  class="w-full h-full object-cover"
                  :alt="group.name"
                >
                <UsersRound v-else class="w-5 h-5 text-primary" />
              </div>
              <div class="min-w-0 flex-1">
                <div class="flex items-center gap-2 min-w-0">
                  <p class="text-sm font-black text-[var(--theme-text)] truncate">
                    {{ group.name }}
                  </p>
                  <span v-if="group.shortCode" class="text-[10px] font-black uppercase px-1.5 py-0.5 rounded bg-primary/10 text-primary shrink-0">
                    {{ group.shortCode }}
                  </span>
                </div>
                <p class="text-xs text-[var(--theme-text-muted)] truncate mt-0.5">
                  {{ group.description || t('global.groups.no_desc') }}
                </p>
              </div>
              <div class="shrink-0 text-right">
                <div class="text-xs font-black text-[var(--theme-text)]">
                  {{ group.memberCount || 0 }}
                </div>
                <div class="text-[10px] text-[var(--theme-text-muted)] font-bold">
                  {{ t('global.groups.members') }}
                </div>
              </div>
            </button>
          </div>
        </template>
      </div>
      
      <!-- Footer Settings / Refresh -->
      <div class="h-10 border-t border-border-soft shrink-0 flex items-center justify-between px-3 bg-[var(--theme-surface)]/50">
        <button class="w-7 h-7 flex items-center justify-center rounded hover:bg-[var(--theme-surface)]-hover text-[var(--theme-text-muted)] hover:text-[var(--theme-text)] transition-colors">
          <Settings class="w-4 h-4" />
        </button>
        <button class="w-7 h-7 flex items-center justify-center rounded hover:bg-[var(--theme-surface)]-hover text-[var(--theme-text-muted)] hover:text-[var(--theme-text)] transition-colors" @click="refreshCurrentTab" :disabled="loading || groupsLoading">
          <RefreshCcw class="w-4 h-4" :class="{ 'animate-spin': loading || groupsLoading }" />
        </button>
      </div>

    </div>
  </div>
</template>


