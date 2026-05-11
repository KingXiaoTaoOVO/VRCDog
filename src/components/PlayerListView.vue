<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue';
import { GamelogApi, VrcApi, DbApi } from "../api";
import { Users, Search, MapPin, Bone, StickyNote } from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';
import VrcResourceCard from './VrcResourceCard.vue';
import { useUserProfileStore } from '../stores/userProfile';

const { t } = useI18n();
const profileStore = useUserProfileStore();

interface Player {
  name: string;
  joinTime: string;
  userData?: any;
  loadingData?: boolean;
  note?: string;
}

const currentRoom = ref(t('player_list.unknown_instance'));
const players = ref<Player[]>([]);
const loading = ref(true);
const searchQuery = ref('');
const resolvedNames = new Set<string>();

const fetchPlayerList = async () => {
  try {
    const res = await DbApi.getGameLogs({ limit: 5000 });
    const chronological = [...res].reverse();
    
    let currentPlayers = new Map<string, string>();
    let roomName = t('player_list.unknown_instance');

    for (const evt of chronological) {
      if (evt.event_type === 'Instance Joined') {
        currentPlayers.clear();
        roomName = evt.content;
      } else if (evt.event_type === 'Player Joined') {
        currentPlayers.set(evt.content, evt.time);
      } else if (evt.event_type === 'Player Left') {
        currentPlayers.delete(evt.content);
      }
    }

    currentRoom.value = roomName;
    
    // Merge new players into existing list to preserve resolved user data
    const newPlayerMap = new Map(
      Array.from(currentPlayers.entries()).map(([name, joinTime]) => [name, joinTime])
    );
    
    const updatedPlayers: Player[] = [];
    
    for (const [name, joinTime] of newPlayerMap.entries()) {
      const existing = players.value.find(p => p.name === name);
      if (existing) {
        updatedPlayers.push(existing);
      } else {
        const newPlayer: Player = { name, joinTime, loadingData: false };
        updatedPlayers.push(newPlayer);
        // Async resolve user data
        resolvePlayerData(newPlayer);
      }
    }

    players.value = updatedPlayers.sort((a, b) => b.joinTime.localeCompare(a.joinTime));

  } catch (err) {
    console.error(err);
  } finally {
    loading.value = false;
  }
};

const resolvePlayerData = async (player: Player) => {
  if (resolvedNames.has(player.name) || player.loadingData) return;
  player.loadingData = true;
  try {
    const res = await VrcApi.request(`/api/1/users?search=${encodeURIComponent(player.name)}&n=1`, { method: 'GET' });
    const p = players.value.find(x => x.name === player.name) || player;
    if (res && res.length > 0 && res[0].displayName === player.name) {
      p.userData = res[0];
      
      try {
        const noteRes = await DbApi.getNote({ targetId: res[0].id });
        if (noteRes && noteRes.note) {
           p.note = noteRes.note;
        }
      } catch (e) { /* ignore */ }
      
      resolvedNames.add(player.name);
    }
  } catch (e) {
    console.warn(`Failed to resolve player data for ${player.name}`);
  } finally {
    const p = players.value.find(x => x.name === player.name) || player;
    p.loadingData = false;
  }
};

onMounted(() => {
  fetchPlayerList();
  window.addEventListener('vrc-gamelog-updated', fetchPlayerList);
});

onUnmounted(() => {
  window.removeEventListener('vrc-gamelog-updated', fetchPlayerList);
});

const filteredPlayers = computed(() => {
  if (!searchQuery.value) return players.value;
  const q = searchQuery.value.toLowerCase();
  return players.value.filter(p => p.name.toLowerCase().includes(q));
});

const openPlayerProfile = (player: Player) => {
  if (player.userData) {
    profileStore.openProfile(player.userData.id, player.userData);
  }
};
</script>

<template>
  <div class="h-full flex flex-col bg-surface-hover p-6 rounded-3xl relative">
    <header class="mb-6 flex justify-between items-end shrink-0">
      <div>
        <h1 class="text-2xl font-extrabold text-text tracking-tight flex items-center gap-3">
          {{ t('player_list.title') }}
          <span class="inline-flex items-center justify-center p-1.5 bg-blue-100 rounded-xl shadow-sm">
            <Users class="w-5 h-5 text-blue-600" />
          </span>
        </h1>
        <p class="text-text-muted font-medium mt-2 flex items-center gap-1.5 text-sm">
          <MapPin
            :size="14"
            class="text-border-strong"
          /> {{ t('player_list.current_location') }} 
          <span class="font-bold text-text bg-surface px-2 py-0.5 rounded-md border-border-soft shadow-sm">{{ currentRoom }}</span>
        </p>
      </div>
      <div class="flex items-center gap-3">
        <span class="text-xs font-bold text-blue-800 bg-blue-100/80 px-3 py-1.5 rounded-full border-blue-200 shadow-sm">
          {{ t('player_list.total_players', { count: players.length }) }}
        </span>
      </div>
    </header>

    <div class="flex-1 flex flex-col overflow-hidden">
      <!-- Search -->
      <div class="relative mb-4 shrink-0">
        <Search class="absolute left-3.5 top-1/2 -translate-y-1/2 text-border-strong w-4 h-4" />
        <input
          v-model="searchQuery"
          type="text"
          :placeholder="t('player_list.search_placeholder')"
          class="w-full pl-10 pr-4 py-2.5 bg-surface border-border-soft focus:border-blue-400 focus:ring-4 focus:ring-blue-500/10 rounded-xl outline-none transition-all text-text text-sm shadow-sm"
        >
      </div>

      <!-- Player List Grid -->
      <div class="flex-1 overflow-y-auto custom-scrollbar pr-2 pb-4">
        <div
          v-if="loading && players.length === 0"
          class="h-full flex flex-col items-center justify-center text-blue-500 opacity-70"
        >
          <Bone
            class="animate-bounce mb-4"
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
          class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4"
        >
          <div
            v-for="player in filteredPlayers"
            :key="player.name"
            class="relative group"
          >
            <VrcResourceCard
              v-if="player.userData"
              type="avatar"
              :data="player.userData"
              :is-user="true"
              @click="openPlayerProfile(player)"
            />
            
            <div
              v-else
              class="h-32 bg-surface rounded-2xl overflow-hidden border-border-soft shadow-sm flex items-center justify-center relative"
            >
              <div
                v-if="player.loadingData"
                class="absolute inset-0 bg-surface-hover flex items-center justify-center"
              >
                <div class="animate-pulse w-8 h-8 rounded-full bg-background/20" />
              </div>
              <div
                v-else
                class="text-center p-4"
              >
                <div class="w-12 h-12 rounded-full bg-surface flex items-center justify-center text-text-muted font-black text-xl uppercase mx-auto mb-2 border-border-soft shadow-sm">
                  {{ player.name.charAt(0) }}
                </div>
                <h3 class="font-bold text-text-muted text-sm truncate max-w-[150px] mx-auto">
                  {{ player.name }}
                </h3>
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
            <div class="absolute top-2 right-2 bg-background/80 backdrop-blur-md/60 backdrop-blur-md text-white text-[9px] px-1.5 py-0.5 rounded uppercase font-mono font-bold border-transparent opacity-80 pointer-events-none z-10 shadow-sm">
              {{ player.joinTime }}
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.custom-scrollbar::-webkit-scrollbar { width: 6px; }
.custom-scrollbar::-webkit-scrollbar-track { background: transparent; }
.custom-scrollbar::-webkit-scrollbar-thumb { background: rgba(148, 163, 184, 0.3); border-radius: 10px; }
.custom-scrollbar::-webkit-scrollbar-thumb:hover { background: rgba(148, 163, 184, 0.5); }
</style>
