<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
import { DbApi, VrcApi } from "../api";
import { Globe2, Rocket, ArrowRightCircle, ArrowLeftCircle, Home, UserPlus, Image as ImageIcon, MapPin, Search, Trash2 } from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';
import VrcResourceCard from './VrcResourceCard.vue';
import { useUserProfileStore } from '../stores/userProfile';

const { t } = useI18n();
const profileStore = useUserProfileStore();

interface LogEvent {
  time: string;
  event_type: string;
  content: string;
  userData?: any;
  loadingData?: boolean;
  user_id?: string;
}

const activeTab = ref<'game' | 'friend'>('game');
const events = ref<LogEvent[]>([]);
const friendEvents = ref<any[]>([]);
const loading = ref(true);
const resolvedNames = new Map<string, any>();
const searchQuery = ref('');

import { computed } from 'vue';

const filteredEvents = computed(() => {
  if (!searchQuery.value) return events.value;
  const q = searchQuery.value.toLowerCase();
  return events.value.filter(e => e.content.toLowerCase().includes(q) || e.event_type.toLowerCase().includes(q));
});

const filteredFriendEvents = computed(() => {
  if (!searchQuery.value) return friendEvents.value;
  const q = searchQuery.value.toLowerCase();
  return friendEvents.value.filter(e => 
    (e.friend_id && e.friend_id.toLowerCase().includes(q)) || 
    (e.friend_name && e.friend_name.toLowerCase().includes(q)) || 
    (e.event_type && e.event_type.toLowerCase().includes(q))
  );
});

const fetchLogs = async () => {
  try {
    loading.value = true;
    if (activeTab.value === 'game') {
      const res: any = await DbApi.getGameLogs({ limit: 500, offset: 0 });
      const newEvents = res.filter((e: LogEvent) => 
        ['Player Joined', 'Player Left', 'Instance Joined'].includes(e.event_type)
      );
      
      events.value = newEvents;
      
      for (const evt of events.value) {
         if (evt.event_type === 'Player Joined' || evt.event_type === 'Player Left') {
            if (resolvedNames.has(evt.content)) {
               evt.userData = resolvedNames.get(evt.content);
            } else {
               resolvePlayerData(evt);
            }
         }
      }
    } else {
      const res: any = await DbApi.getFriendLogs({ limit: 500, offset: 0 });
      friendEvents.value = res;
    }
  } catch (err) {
    console.error(err);
  } finally {
    loading.value = false;
  }
};

const clearLogs = async () => {
  if (!confirm(t('feed.confirm_clear'))) return;
  try {
    if (activeTab.value === 'game') {
      await DbApi.clearGameLogs();
      events.value = [];
    } else {
      await DbApi.clearFriendLogs();
      friendEvents.value = [];
    }
  } catch (err) {
    console.error('Failed to clear logs', err);
  }
};

const resolvePlayerData = async (evt: LogEvent) => {
  if (evt.loadingData) return;
  evt.loadingData = true;
  try {
    let searchName = evt.content;
    let userId = null;
    const match = evt.content.match(/^(.*?)\s+\((usr_[A-Za-z0-9-]+)\)$/);
    if (match) {
      searchName = match[1];
      userId = match[2];
      evt.user_id = userId;
    }

    if (userId) {
       const res = await VrcApi.request(`/api/1/users/${userId}`, 'GET');
       if (res && res.id === userId) {
          evt.userData = res;
          resolvedNames.set(evt.content, res);
       }
    } else {
       const res = await VrcApi.request(`/api/1/users?search=${encodeURIComponent(searchName)}&n=1`, 'GET');
       if (res && res.length > 0) {
          evt.userData = res[0];
          resolvedNames.set(evt.content, res[0]);
       }
    }
  } catch (e) {
    console.warn(`Failed to resolve player data for ${evt.content}`);
  } finally {
    evt.loadingData = false;
  }
};

const openPlayerProfile = (evt: LogEvent | any) => {
  if (evt.userData) {
    profileStore.openProfile(evt.userData.id, evt.userData);
  } else if (evt.user_id) {
    profileStore.openProfile(evt.user_id);
  }
};

onMounted(() => {
  fetchLogs();
  window.addEventListener('vrc-gamelog-updated', fetchLogs);
});

onUnmounted(() => {
  window.removeEventListener('vrc-gamelog-updated', fetchLogs);
});

const getEventMeta = (type: string) => {
  switch(type) {
    case 'Player Joined': return { icon: ArrowRightCircle, color: 'text-green-500', bg: 'bg-green-100', verb: t('feed.verb_joined') };
    case 'Player Left': return { icon: ArrowLeftCircle, color: 'text-border-strong', bg: 'bg-background/10', verb: t('feed.verb_left') };
    case 'Instance Joined': return { icon: Home, color: 'text-blue-500', bg: 'bg-blue-100', verb: t('feed.verb_instance') };
    default: return { icon: Rocket, color: 'text-indigo-500', bg: 'bg-indigo-50', verb: t('feed.verb_unknown') };
  }
};
</script>

<template>
  <div class="h-full flex flex-col p-6 bg-surface-hover rounded-3xl relative overflow-hidden">
    <!-- Subtle Background Glow -->
    <div class="absolute top-0 right-0 w-96 h-96 bg-indigo-500/10 rounded-full blur-[100px] pointer-events-none -z-10" />
    <div class="absolute bottom-0 left-0 w-[500px] h-[500px] bg-blue-500/5 rounded-full blur-[120px] pointer-events-none -z-10" />
    <header class="mb-6 flex justify-between items-end">
      <div>
        <h1 class="text-2xl font-extrabold text-text tracking-tight flex items-center gap-3">
          {{ t('feed.title') }}
          <span class="inline-flex items-center justify-center p-1.5 bg-blue-100 rounded-xl shadow-sm">
            <Globe2 class="w-5 h-5 text-blue-600" />
          </span>
        </h1>
        <p class="text-text-muted font-medium mt-1 text-sm">
          {{ t('feed.subtitle') }}
        </p>
      </div>
      <div class="flex items-center gap-4">
        <div class="flex items-center gap-2">
          <div class="relative group">
            <div class="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none">
              <Search class="h-4 w-4 text-border-strong group-focus-within:text-indigo-500 transition-colors" />
            </div>
            <input 
              v-model="searchQuery" 
              type="text" 
              :placeholder="t('feed.search_logs')" 
              class="w-48 pl-9 pr-3 py-2 bg-surface border border-border-soft rounded-xl text-sm font-bold text-text-muted focus:outline-none focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500 transition-all shadow-sm placeholder:font-medium"
            >
          </div>
          <div class="bg-surface backdrop-blur-md p-1.5 rounded-xl flex shadow-sm border border-border-soft">
            <button 
              class="px-4 py-2 rounded-lg text-sm font-bold transition-all"
              :class="activeTab === 'game' ? 'bg-surface shadow-sm text-indigo-600' : 'text-text-muted hover:text-text-muted'"
              @click="activeTab = 'game'; fetchLogs()"
            >
              {{ t('feed.game_logs') }}
            </button>
            <button 
              class="px-4 py-2 rounded-lg text-sm font-bold transition-all"
              :class="activeTab === 'friend' ? 'bg-surface shadow-sm text-indigo-600' : 'text-text-muted hover:text-text-muted'"
              @click="activeTab = 'friend'; fetchLogs()"
            >
              {{ t('feed.friend_logs') }}
            </button>
          </div>
          <button
            class="px-3 py-2 bg-red-50 hover:bg-red-100 text-red-500 rounded-xl border border-red-200 transition-all shadow-sm flex items-center gap-1.5 font-bold text-sm ml-2"
            @click="clearLogs"
          >
            <Trash2 class="w-4 h-4" /> {{ t('feed.clear') }}
          </button>
        </div>
        <div
          v-if="loading"
          class="text-blue-500 font-bold flex items-center gap-2 animate-pulse text-sm bg-blue-50 px-3 py-1.5 rounded-lg border border-blue-100"
        >
          <Rocket class="w-4 h-4 animate-bounce" /> {{ t('feed.listening') }}
        </div>
      </div>
    </header>

    <div class="flex-1 bg-surface-hover border border-border-soft rounded-3xl p-6 shadow-sm overflow-y-auto custom-scrollbar">
      <!-- Game Log Tab -->
      <template v-if="activeTab === 'game'">
        <div
          v-if="events.length === 0 && !loading"
          class="h-full flex flex-col items-center justify-center text-border-strong"
        >
          <Globe2
            class="mb-4 animate-spin-slow opacity-50"
            :size="48"
          />
          <p class="font-bold text-base">
            {{ t('feed.silent') }}
          </p>
          <p class="text-xs mt-1">
            {{ t('feed.go_make_friends') }}
          </p>
        </div>

        <!-- Timeline Layout -->
        <div
          v-else
          class="relative border-l-2 border-border-soft ml-4 space-y-6 pb-8"
        >
          <div
            v-for="(evt, idx) in filteredEvents"
            :key="idx"
            class="relative pl-8"
          >
            <!-- Timeline Node -->
            <div class="absolute -left-[17px] top-1">
              <div
                class="w-8 h-8 rounded-full shadow-sm flex items-center justify-center border-[3px] border-border-strong z-10 relative"
                :class="getEventMeta(evt.event_type).bg"
              >
                <component
                  :is="getEventMeta(evt.event_type).icon"
                  :size="14"
                  :class="getEventMeta(evt.event_type).color"
                />
              </div>
            </div>

            <!-- Content Card -->
            <div class="bg-surface rounded-2xl p-4 shadow-sm hover:shadow-md transition-all border border-border-soft relative group max-w-2xl">
              <!-- Triangle indicator -->
              <div class="absolute top-4 -left-1.5 w-3 h-3 bg-surface border-l border-b border-border-soft transform rotate-45 transition-colors" />
            
              <div class="flex items-center gap-2 mb-3 px-2">
                <span
                  class="text-xs font-black uppercase tracking-wider"
                  :class="getEventMeta(evt.event_type).color"
                >
                  {{ evt.event_type === 'Player Joined' ? t('feed.type_player_joined') : evt.event_type === 'Player Left' ? t('feed.type_player_left') : evt.event_type === 'Instance Joined' ? t('feed.type_instance_joined') : evt.event_type }}
                </span>
                <span class="text-[10px] text-border-strong font-bold ml-auto">{{ evt.time }}</span>
              </div>
            
              <div
                v-if="evt.event_type === 'Instance Joined'"
                class="text-text font-medium flex flex-wrap items-center gap-2"
              >
                <span class="text-blue-700 font-bold bg-blue-50 px-3 py-2 rounded-xl border border-blue-100 shadow-sm break-all flex-1">
                  <MapPin class="w-4 h-4 inline-block mr-1 text-blue-500" />
                  {{ evt.content }}
                </span>
                <span class="text-text-muted text-xs shrink-0">{{ getEventMeta(evt.event_type).verb }}</span>
              </div>
            
              <div
                v-else
                class="text-text font-medium flex items-center gap-4"
              >
                <div
                  class="flex-1 w-64 cursor-pointer"
                  @click="openPlayerProfile(evt)"
                >
                  <VrcResourceCard
                    v-if="evt.userData"
                    type="avatar"
                    :data="evt.userData"
                    :is-user="true"
                    :minimal="true"
                  />
                  <div
                    v-else
                    class="flex items-center gap-3 p-2 rounded-xl border border-border-soft bg-surface-hover"
                  >
                    <div class="w-10 h-10 rounded-full bg-background/20 flex items-center justify-center text-text-muted font-bold uppercase relative">
                      <span v-if="!evt.loadingData">{{ evt.content.replace(/\s+\(usr_.*\)$/, '').charAt(0) }}</span>
                      <div
                        v-else
                        class="animate-spin w-4 h-4 border-2 border-border-soft border-t-transparent rounded-full"
                      />
                    </div>
                    <span class="font-extrabold text-sm truncate max-w-[150px]">{{ evt.content.replace(/\s+\(usr_.*\)$/, '') }}</span>
                  </div>
                </div>
              
                <span class="text-text-muted text-xs shrink-0 font-bold bg-background/10 px-2 py-1 rounded-md">{{ getEventMeta(evt.event_type).verb }}</span>
              </div>
            </div>
          </div>
        </div>
      </template>

      <!-- Friend Log Tab -->
      <template v-else-if="activeTab === 'friend'">
        <div
          v-if="friendEvents.length === 0 && !loading"
          class="h-full flex flex-col items-center justify-center text-border-strong"
        >
          <UserPlus
            class="mb-4 opacity-50"
            :size="48"
          />
          <p class="font-bold text-base">
            {{ t('feed.no_friend_logs') }}
          </p>
        </div>

        <div
          v-else
          class="relative border-l-2 border-border-soft ml-4 space-y-6 pb-8"
        >
          <div
            v-for="(evt, idx) in filteredFriendEvents"
            :key="idx"
            class="relative pl-8"
          >
            <div class="absolute -left-[17px] top-1">
              <div
                class="w-8 h-8 rounded-full shadow-sm flex items-center justify-center border-[3px] border-border-strong z-10 relative"
                :class="evt.event_type === 'online' ? 'bg-green-100 text-green-500' : evt.event_type === 'offline' ? 'bg-background/10 text-text-muted' : 'bg-blue-100 text-blue-500'"
              >
                <Globe2
                  v-if="evt.event_type === 'location_change'"
                  :size="14"
                />
                <ArrowRightCircle
                  v-else-if="evt.event_type === 'online'"
                  :size="14"
                />
                <ArrowLeftCircle
                  v-else-if="evt.event_type === 'offline'"
                  :size="14"
                />
                <UserPlus
                  v-else
                  :size="14"
                />
              </div>
            </div>

            <div
              class="bg-surface rounded-2xl p-4 shadow-sm hover:shadow-md transition-all border border-border-soft relative group max-w-2xl cursor-pointer"
              @click="openPlayerProfile(evt)"
            >
              <div class="absolute top-4 -left-1.5 w-3 h-3 bg-surface border-l border-b border-border-soft transform rotate-45 transition-colors" />
              
              <div class="flex justify-between items-start mb-3">
                <div
                  class="font-bold text-xs uppercase tracking-wider"
                  :class="evt.event_type === 'online' ? 'text-green-500' : evt.event_type === 'offline' ? 'text-text-muted' : 'text-blue-500'"
                >
                  {{ evt.event_type }}
                </div>
                <div class="text-[10px] text-text-muted font-mono font-bold bg-background/10 px-2 py-0.5 rounded-md">
                  {{ new Date(evt.created_at).toLocaleString() }}
                </div>
              </div>
              
              <div class="text-text font-medium flex items-center gap-4">
                <div class="flex items-center gap-3">
                  <div class="w-10 h-10 rounded-full bg-background/20 flex items-center justify-center text-text-muted font-bold uppercase relative">
                    <span>{{ (evt.display_name || 'U').charAt(0) }}</span>
                  </div>
                  <span class="font-extrabold text-sm max-w-[150px] truncate">{{ evt.display_name }}</span>
                </div>
                
                <span class="text-text-muted text-xs shrink-0 font-bold bg-background/10 px-2 py-1 rounded-md">
                  {{ evt.event_type === 'online' ? t('feed.status_online') : evt.event_type === 'offline' ? t('feed.status_offline') : evt.event_type === 'location_change' ? t('feed.status_location') : evt.event_type }}
                </span>

                <span
                  v-if="evt.detail && evt.detail !== 'private'"
                  class="text-blue-700 font-bold bg-blue-50 px-3 py-1 rounded-lg border border-blue-100 shadow-sm text-xs truncate max-w-[200px]"
                >
                  <MapPin class="w-3 h-3 inline-block mr-1 text-blue-500" />
                  {{ evt.detail }}
                </span>
                <span
                  v-else-if="evt.detail === 'private'"
                  class="text-text-muted font-bold bg-background/10 px-3 py-1 rounded-lg border border-border-soft shadow-sm text-xs truncate max-w-[200px]"
                >
                  🔒 Private
                </span>
              </div>
            </div>
          </div>
        </div>
      </template>
    </div>
  </div>
</template>

<style scoped>
.custom-scrollbar::-webkit-scrollbar { width: 6px; }
.custom-scrollbar::-webkit-scrollbar-track { background: transparent; }
.custom-scrollbar::-webkit-scrollbar-thumb { background: rgba(148, 163, 184, 0.3); border-radius: 10px; }
.custom-scrollbar::-webkit-scrollbar-thumb:hover { background: rgba(148, 163, 184, 0.5); }
.animate-spin-slow { animation: spin 8s linear infinite; }
</style>
