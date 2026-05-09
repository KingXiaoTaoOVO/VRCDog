<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
import { GamelogApi } from "../api";
import { Users, LogIn, Search, MapPin, Bone } from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';

const { t } = useI18n();

interface Player {
  name: string;
  joinTime: string;
}

const currentRoom = ref(t('player_list.unknown_instance'));
const players = ref<Player[]>([]);
const loading = ref(true);
const searchQuery = ref('');
let timer: number | null = null;

const fetchPlayerList = async () => {
  try {
    const res = await GamelogApi.getLatestGamelogs({ maxLines: 5000 });
    // 从旧到新遍历，以构建当前房间状态
    // res 是从新到旧排序的，所以我们要 reverse 一下
    const chronological = [...res].reverse();
    
    let currentPlayers = new Map<string, string>();
    let roomName = t('player_list.unknown_instance');

    for (const evt of chronological) {
      if (evt.event_type === 'Instance Joined') {
        currentPlayers.clear(); // 新房间，清空列表
        roomName = evt.content;
      } else if (evt.event_type === 'Player Joined') {
        currentPlayers.set(evt.content, evt.time);
      } else if (evt.event_type === 'Player Left') {
        currentPlayers.delete(evt.content);
      }
    }

    currentRoom.value = roomName;
    players.value = Array.from(currentPlayers.entries()).map(([name, joinTime]) => ({
      name,
      joinTime
    })).sort((a, b) => b.joinTime.localeCompare(a.joinTime));

  } catch (err) {
    console.error(err);
  } finally {
    loading.value = false;
  }
};

onMounted(() => {
  fetchPlayerList();
  timer = setInterval(fetchPlayerList, 5000) as unknown as number; // 5秒轮询
});

onUnmounted(() => {
  if (timer) clearInterval(timer);
});

const getFilteredPlayers = () => {
  if (!searchQuery.value) return players.value;
  return players.value.filter(p => p.name.toLowerCase().includes(searchQuery.value.toLowerCase()));
};
</script>

<template>
  <div class="h-full flex flex-col">
    <header class="mb-6 flex justify-between items-end">
      <div>
        <h1 class="text-3xl font-extrabold text-[#451a03] tracking-tight flex items-center gap-3">
          {{ t('player_list.title') }}
          <span class="inline-flex items-center justify-center p-1.5 bg-green-100 rounded-xl">
            <Users class="w-6 h-6 text-green-600" />
          </span>
        </h1>
        <p class="text-amber-700/80 font-medium mt-1 flex items-center gap-1">
          <MapPin :size="14" /> {{ t('player_list.current_location') }} <span class="font-bold text-amber-900 bg-amber-100 px-2 rounded-md">{{ currentRoom }}</span>
        </p>
      </div>
      <div class="flex items-center gap-3">
        <span class="text-sm font-bold text-amber-800 bg-amber-100 px-3 py-1 rounded-full border border-amber-200">
          {{ t('player_list.total_players', { count: players.length }) }}
        </span>
      </div>
    </header>

    <div class="flex-1 bg-white/60 backdrop-blur-md border-2 border-white rounded-3xl p-6 shadow-lg flex flex-col">
      <div class="relative mb-6">
        <Search class="absolute left-3 top-1/2 -translate-y-1/2 text-amber-300 w-5 h-5" />
        <input
          v-model="searchQuery"
          type="text"
          :placeholder="t('player_list.search_placeholder')"
          class="w-full pl-10 pr-4 py-2.5 bg-white border border-amber-100 focus:border-green-400 focus:ring-0 rounded-xl outline-none transition-colors text-amber-900 font-medium"
        >
      </div>

      <div class="flex-1 overflow-y-auto custom-scrollbar pr-2 space-y-3">
        <div
          v-if="loading && players.length === 0"
          class="h-full flex flex-col items-center justify-center text-green-500 opacity-70"
        >
          <Bone
            class="animate-bounce mb-4"
            :size="48"
          />
          <p class="font-bold">
            {{ t('player_list.loading') }}
          </p>
        </div>

        <div
          v-else-if="players.length === 0"
          class="h-full flex flex-col items-center justify-center text-amber-900/40"
        >
          <Users
            class="mb-4 opacity-50"
            :size="48"
          />
          <p class="font-bold text-lg">
            {{ t('player_list.no_data') }}
          </p>
          <p class="text-sm mt-1">
            {{ t('player_list.no_players') }}
          </p>
        </div>

        <div
          v-for="player in getFilteredPlayers()"
          :key="player.name"
          class="flex items-center justify-between p-4 bg-white rounded-2xl border border-amber-50 hover:border-green-200 transition-colors shadow-sm group"
        >
          <div class="flex items-center gap-4">
            <div class="w-12 h-12 rounded-full bg-green-100 flex items-center justify-center text-green-500 font-bold text-xl uppercase">
              {{ player.name.charAt(0) }}
            </div>
            <div>
              <h3 class="font-bold text-amber-950 text-lg">
                {{ player.name }}
              </h3>
              <div class="flex items-center gap-1 mt-1 text-xs font-bold text-amber-900/40">
                <LogIn
                  :size="12"
                  class="text-green-500"
                /> {{ t('player_list.joined_at') }}{{ player.joinTime }}
              </div>
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
.custom-scrollbar::-webkit-scrollbar-thumb { background: rgba(34, 197, 94, 0.2); border-radius: 10px; }
.custom-scrollbar::-webkit-scrollbar-thumb:hover { background: rgba(34, 197, 94, 0.4); }
</style>
