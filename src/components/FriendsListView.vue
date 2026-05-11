<script setup lang="ts">
import { ref, onMounted, computed, onUnmounted } from 'vue';
import { VrcApi, DbApi } from "../api";
import { Search, RefreshCcw, Bell, Settings, ChevronDown, ChevronRight } from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';
import { useUserProfileStore } from '../stores/userProfile';

const { t } = useI18n();
const profileStore = useUserProfileStore();

const onlineFriends = ref<any[]>([]);
const activeFriends = ref<any[]>([]); // 活跃中 (仅登录网页端)
const offlineFriends = ref<any[]>([]);
const loading = ref(true);
const errorMsg = ref('');
const searchQuery = ref('');

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

const currentUser = ref<any>(null);

const fetchFriends = async () => {
  loading.value = true;
  errorMsg.value = '';
  try {
    // Current user mock or fetch
    const current = await VrcApi.getCurrentUser().catch(() => null);
    if (current) {
      currentUser.value = current;
    }

    const cached: any[] = await DbApi.getCachedFriends() || [];
    
    onlineFriends.value = cached.filter((f: any) => f.location && f.location !== 'offline');
    
    // In VRChat, active usually means location is 'offline' but status is not 'offline', or similar.
    // For now we mock 'active' users as those with location 'offline' but status != 'offline'
    activeFriends.value = cached.filter((f: any) => f.location === 'offline' && f.status && f.status !== 'offline');
    
    offlineFriends.value = cached.filter((f: any) => f.location === 'offline' && (!f.status || f.status === 'offline'));
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

const onlineCount = computed(() => onlineFriends.value.length);
const totalCount = computed(() => onlineFriends.value.length + activeFriends.value.length + offlineFriends.value.length);

// Extract Trust Color
const getTrustColor = (tags: string[]) => {
  if (!tags) return '#CCCCCC'; // Visitor
  if (tags.includes('system_trust_legend') || tags.includes('system_trust_veteran')) return 'var(--color-primary)'; // Trusted (Primary)
  if (tags.includes('system_trust_trusted')) return '#ff7b42'; // Known (Orange)
  if (tags.includes('system_trust_known')) return '#2bcf5c'; // User (Green)
  if (tags.includes('system_trust_basic')) return '#1778ff'; // New User (Blue)
  return '#CCCCCC'; // Visitor
};

// Grouping Logic matching VRCX
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
      
      // Clean up location name for display
      let displayLoc = locName.split('~')[0];
      
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

const openDetail = (friend: any) => {
  profileStore.openProfile(friend.id, friend);
};

// Status dot color
const getStatusColor = (status: string) => {
  switch (status) {
    case 'active': return '#22c55e'; // green-500
    case 'join me': return '#3b82f6'; // blue-500
    case 'ask me': return '#f97316'; // orange-500
    case 'busy': return '#ef4444'; // red-500
    default: return '#94a3b8'; // slate-400
  }
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
  if (loc === 'private') return t('auto_4ad3ada9');
  return loc?.split('~')[0] || t('auto_0fcd7253');
};
</script>

<template>
  <div class="h-full w-full flex justify-center bg-background/80 backdrop-blur-md text-text-muted overflow-hidden">
    <!-- Main container mimicking VRCX right sidebar -->
    <div class="w-full max-w-[340px] h-full flex flex-col bg-surface/80 border-border-soft">
      
      <!-- Top Bar -->
      <div class="flex items-center gap-2 p-2 border-border-soft">
        <div class="flex-1 relative flex items-center bg-surface rounded px-3 py-1.5 focus-within:ring-1 focus-within:ring-slate-500 transition-all">
          <Search class="w-4 h-4 text-border-strong mr-2 shrink-0" />
          <input
            v-model="searchQuery"
            type="text"
            class="w-full bg-transparent text-[13px] text-text-muted placeholder-slate-500 outline-none"
            :placeholder="$t('auto_e5f71fc3')"
          >
          <div class="flex items-center gap-1 shrink-0 ml-2">
            <kbd class="min-w-[20px] text-center bg-surface/80 text-border-strong border-border-soft rounded flex items-center justify-center px-1.5 py-0.5 text-[10px] font-mono shadow-[0_1px_0_rgba(0,0,0,0.5)]">Ctrl</kbd>
            <kbd class="min-w-[20px] text-center bg-surface/80 text-border-strong border-border-soft rounded flex items-center justify-center px-1.5 py-0.5 text-[10px] font-mono shadow-[0_1px_0_rgba(0,0,0,0.5)]">K</kbd>
          </div>
        </div>
        <button class="p-1.5 text-border-strong hover:text-text-muted transition-colors" @click="fetchFriends" :title="$t('auto_694fc5ef')">
          <RefreshCcw class="w-4 h-4" :class="{'animate-spin text-primary': loading}" />
        </button>
        <button class="p-1.5 text-border-strong hover:text-text-muted transition-colors" :title="$t('auto_5660bcd2')">
          <Bell class="w-4 h-4" />
        </button>
        <button class="p-1.5 text-border-strong hover:text-text-muted transition-colors" :title="$t('auto_e366ccf1')">
          <Settings class="w-4 h-4" />
        </button>
      </div>

      <!-- Tabs -->
      <div class="flex border-border-soft">
        <button 
          class="flex-1 py-2.5 text-[13px] font-bold transition-colors relative"
          :class="activeTab === 'friends' ? 'text-text' : 'text-text-muted hover:text-text'"
          @click="activeTab = 'friends'"
        >
          {{ $t('friends.title') || '好友' }} ({{ onlineCount }}/{{ totalCount }})
          <div v-if="activeTab === 'friends'" class="absolute bottom-0 left-0 right-0 h-[2px] bg-surface"></div>
        </button>
        <button 
          class="flex-1 py-2.5 text-[13px] font-bold transition-colors relative"
          :class="activeTab === 'groups' ? 'text-text' : 'text-text-muted hover:text-text'"
          @click="activeTab = 'groups'"
        >
          {{ $t('groups.title') || '群组房间' }} (0)
          <div v-if="activeTab === 'groups'" class="absolute bottom-0 left-0 right-0 h-[2px] bg-surface"></div>
        </button>
      </div>

      <!-- Content Area -->
      <div class="flex-1 overflow-y-auto custom-scrollbar">
        
        <div v-if="loading && onlineCount === 0" class="flex justify-center items-center h-20 text-text-muted">
          <RefreshCcw class="w-5 h-5 animate-spin" />
        </div>

        <template v-else-if="activeTab === 'friends'">
          
          <!-- 我 (Me) -->
          <div class="mb-2 mt-2">
            <div 
              class="flex items-center gap-2 py-1 px-3 cursor-pointer hover:bg-surface transition-colors text-[13px] font-bold text-text-muted select-none"
              @click="toggleSection('me')"
            >
              <component :is="collapsedSections.has('me') ? ChevronRight : ChevronDown" class="w-3.5 h-3.5 text-border-strong" />
              {{ $t('friends.me') || '我' }}
            </div>
            <div v-show="!collapsedSections.has('me')" class="mt-1 pl-2">
              <div class="flex items-center gap-3 py-1.5 px-4 rounded-md transition-colors">
                <div class="relative shrink-0">
                  <img :src="currentUser?.currentAvatarThumbnailImageUrl || currentUser?.profilePicOverride || 'https://via.placeholder.com/150'" class="w-8 h-8 rounded-full object-cover bg-surface-hover" />
                  <div class="absolute bottom-0 right-0 w-2.5 h-2.5 rounded-full border-border-strong bg-green-500"></div>
                </div>
                <div class="flex-1 min-w-0 flex flex-col justify-center leading-tight">
                  <span class="text-[13px] font-bold truncate" :style="{ color: getTrustColor(currentUser?.tags) }">
                    {{ currentUser?.displayName || 'King小韬' }}
                  </span>
                </div>
              </div>
            </div>
          </div>

          <!-- 同一房间 (Same Room) -->
          <div v-if="groupedFriends.sameRoom.length > 0" class="mb-2">
            <div 
              class="flex items-center gap-2 py-1 px-3 cursor-pointer hover:bg-surface transition-colors text-[13px] font-bold text-text-muted select-none"
              @click="toggleSection('sameRoom')"
            >
              <component :is="collapsedSections.has('sameRoom') ? ChevronRight : ChevronDown" class="w-3.5 h-3.5 text-border-strong" />
              {{ $t('friends.same_room') || '同一房间' }} — {{ groupedFriends.sameRoom.length }}
            </div>
            
            <div v-show="!collapsedSections.has('sameRoom')" class="mt-1">
              <template v-for="group in groupedFriends.sameRoom" :key="group.location">
                <!-- Location Header -->
                <div class="flex items-center gap-2 py-1 px-6 text-[12px] font-medium text-text-muted">
                  <span class="text-[13px]">{{ group.flag }}</span>
                  <span class="truncate max-w-[200px]">{{ group.locationName }}</span>
                  <span class="ml-1 shrink-0">({{ group.friends.length }})</span>
                </div>
                <!-- Friends in Location -->
                <div 
                  v-for="friend in group.friends" 
                  :key="friend.id"
                  class="flex items-center gap-3 py-1.5 px-6 ml-4 mr-2 cursor-pointer hover:bg-surface-hover rounded transition-colors"
                  @click="openDetail(friend)"
                >
                  <div class="relative shrink-0">
                    <img :src="friend.currentAvatarThumbnailImageUrl || friend.currentAvatarImageUrl" class="w-8 h-8 rounded-full object-cover bg-surface-hover" />
                    <div class="absolute bottom-0 right-0 w-2.5 h-2.5 rounded-full border-border-strong" :style="{ backgroundColor: getStatusColor(friend.status) }"></div>
                  </div>
                  <div class="flex-1 min-w-0 flex flex-col justify-center leading-tight">
                    <span class="text-[13px] font-bold truncate" :style="{ color: getTrustColor(friend.tags) }">
                      {{ friend.displayName }}
                    </span>
                    <span class="text-[11px] text-text-muted mt-0.5">{{ $t('auto_3bdab2c6') }}</span> <!-- Mock time to match design -->
                  </div>
                </div>
              </template>
            </div>
          </div>

          <!-- 在线 (Online) -->
          <div v-if="groupedFriends.online.length > 0" class="mb-2">
            <div 
              class="flex items-center gap-2 py-1 px-3 cursor-pointer hover:bg-surface transition-colors text-[13px] font-bold text-text-muted select-none"
              @click="toggleSection('online')"
            >
              <component :is="collapsedSections.has('online') ? ChevronRight : ChevronDown" class="w-3.5 h-3.5 text-border-strong" />
              {{ $t('friends.online') || '在线' }} — {{ groupedFriends.online.length }}
            </div>
            
            <div v-show="!collapsedSections.has('online')" class="mt-1 pl-2 pr-2">
              <div 
                v-for="friend in groupedFriends.online" 
                :key="friend.id"
                class="flex items-center gap-3 py-2 px-4 cursor-pointer hover:bg-surface-hover rounded transition-colors"
                @click="openDetail(friend)"
              >
                <div class="relative shrink-0">
                  <img :src="friend.currentAvatarThumbnailImageUrl || friend.currentAvatarImageUrl" class="w-8 h-8 rounded-full object-cover bg-surface-hover" />
                  <div class="absolute bottom-0 right-0 w-2.5 h-2.5 rounded-full border-border-strong" :style="{ backgroundColor: getStatusColor(friend.status) }"></div>
                </div>
                <div class="flex-1 min-w-0 flex flex-col justify-center leading-tight">
                  <span class="text-[13px] font-bold truncate" :style="{ color: getTrustColor(friend.tags) }">
                    {{ friend.displayName }}
                  </span>
                  <div class="flex items-center gap-1.5 mt-0.5 text-[11px] text-text-muted truncate">
                    <span v-if="friend.location === 'private'" class="shrink-0 text-orange-400 opacity-80">🔒</span>
                    <span v-else class="shrink-0 text-[12px]">{{ getFlag(friend.location) }}</span>
                    <span class="truncate">{{ cleanLocName(friend.location) }}</span>
                  </div>
                </div>
              </div>
            </div>
          </div>

          <!-- 活跃中 (Active on website) -->
          <div v-if="activeFriends.length > 0 && !searchQuery" class="mb-2">
            <div 
              class="flex items-center gap-2 py-1 px-3 cursor-pointer hover:bg-surface transition-colors text-[13px] font-bold text-text-muted select-none"
              @click="toggleSection('active')"
            >
              <component :is="collapsedSections.has('active') ? ChevronRight : ChevronDown" class="w-3.5 h-3.5 text-border-strong" />
              {{ $t('friends.active_web') || '活跃中 (网页端)' }} — {{ activeFriends.length }}
            </div>
            <div v-show="!collapsedSections.has('active')" class="mt-1 pl-2 pr-2">
              <div 
                v-for="friend in activeFriends" 
                :key="friend.id"
                class="flex items-center gap-3 py-1.5 px-4 cursor-pointer hover:bg-surface-hover rounded transition-colors"
                @click="openDetail(friend)"
              >
                <div class="relative shrink-0">
                  <img :src="friend.currentAvatarThumbnailImageUrl || friend.currentAvatarImageUrl" class="w-8 h-8 rounded-full object-cover bg-surface-hover" />
                  <div class="absolute bottom-0 right-0 w-2.5 h-2.5 rounded-full border-border-strong" :style="{ backgroundColor: getStatusColor(friend.status) }"></div>
                </div>
                <div class="flex-1 min-w-0 flex flex-col justify-center leading-tight">
                  <span class="text-[13px] font-bold truncate" :style="{ color: getTrustColor(friend.tags) }">
                    {{ friend.displayName }}
                  </span>
                  <span class="text-[11px] text-text-muted mt-0.5 truncate">{{ $t('auto_7cdc4c2a') }}</span>
                </div>
              </div>
            </div>
          </div>

          <!-- 离线 (Offline) -->
          <div v-if="offlineFriends.length > 0 && !searchQuery" class="mb-4">
            <div 
              class="flex items-center gap-2 py-1 px-3 cursor-pointer hover:bg-surface transition-colors text-[13px] font-bold text-border-strong select-none"
              @click="toggleSection('offline')"
            >
              <component :is="collapsedSections.has('offline') ? ChevronRight : ChevronDown" class="w-3.5 h-3.5 text-text-muted" />
              {{ $t('friends.offline') || '离线' }} — {{ offlineFriends.length }}
            </div>
            <div v-show="!collapsedSections.has('offline')" class="mt-1 pl-2 pr-2 opacity-60 hover:opacity-100 transition-opacity">
              <div 
                v-for="friend in offlineFriends" 
                :key="friend.id"
                class="flex items-center gap-3 py-1.5 px-4 cursor-pointer hover:bg-surface-hover rounded transition-colors"
                @click="openDetail(friend)"
              >
                <div class="relative shrink-0">
                  <img :src="friend.currentAvatarThumbnailImageUrl || friend.currentAvatarImageUrl" class="w-8 h-8 rounded-full object-cover bg-surface-hover grayscale" />
                  <div class="absolute bottom-0 right-0 w-2.5 h-2.5 rounded-full border-border-strong bg-surface"></div>
                </div>
                <div class="flex-1 min-w-0 flex flex-col justify-center leading-tight">
                  <span class="text-[13px] font-bold truncate text-text-muted">
                    {{ friend.displayName }}
                  </span>
                  <span class="text-[11px] text-text-muted/70 mt-0.5 truncate">{{ $t('auto_50d4a850') }}</span>
                </div>
              </div>
            </div>
          </div>

        </template>
        
        <div v-else class="h-full flex flex-col items-center justify-center text-text-muted text-sm font-medium">
          {{ $t('groups.no_groups') || '暂无群组房间' }}
        </div>

      </div>
    </div>
  </div>
</template>

<style scoped>
.custom-scrollbar::-webkit-scrollbar { width: 8px; }
.custom-scrollbar::-webkit-scrollbar-track { background: transparent; }
.custom-scrollbar::-webkit-scrollbar-thumb { background: var(--color-surface-hover); border-radius: 4px; border: 2px solid var(--color-border-soft); }
.custom-scrollbar::-webkit-scrollbar-thumb:hover { background: var(--color-text-muted); }
</style>
