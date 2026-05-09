<script setup lang="ts">
import { ref, onMounted, computed } from 'vue';
import { DbApi } from '../api';
import { History, UserPlus, UserMinus, MapPin, Edit3, RefreshCcw, Filter } from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';
import type { FriendLog } from '../types/vrc';

const { t } = useI18n();

const logs = ref<FriendLog[]>([]);
const loading = ref(true);
const filterType = ref('');

const eventTypes = computed(() => [
  { value: '', label: t('friend_log.all'), icon: History },
  { value: 'online', label: t('friend_log.online'), icon: UserPlus },
  { value: 'offline', label: t('friend_log.offline'), icon: UserMinus },
  { value: 'location_change', label: t('friend_log.location_change'), icon: MapPin },
  { value: 'name_change', label: t('friend_log.name_change'), icon: Edit3 },
  { value: 'friend_add', label: t('friend_log.friend_add'), icon: UserPlus },
  { value: 'friend_remove', label: t('friend_log.friend_remove'), icon: UserMinus },
]);

const fetchLogs = async () => {
  loading.value = true;
  try {
    const params: any = { limit: 500 };
    if (filterType.value) params.event_type = filterType.value;
    logs.value = await DbApi.getFriendLogs(params);
  } catch (err) {
    console.warn('获取好友日志失败:', err);
  } finally {
    loading.value = false;
  }
};

onMounted(() => fetchLogs());

const getEventMeta = (type: string) => {
  switch (type) {
    case 'online': return { color: 'text-green-500', bg: 'bg-green-100', label: t('friend_log.online') };
    case 'offline': return { color: 'text-gray-400', bg: 'bg-gray-100', label: t('friend_log.offline') };
    case 'location_change': return { color: 'text-blue-500', bg: 'bg-blue-100', label: t('friend_log.location_change') };
    case 'name_change': return { color: 'text-purple-500', bg: 'bg-purple-100', label: t('friend_log.name_change') };
    case 'friend_add': return { color: 'text-emerald-500', bg: 'bg-emerald-100', label: t('friend_log.friend_add') };
    case 'friend_remove': return { color: 'text-red-500', bg: 'bg-red-100', label: t('friend_log.friend_remove') };
    case 'status_change': return { color: 'text-orange-500', bg: 'bg-orange-100', label: t('friend_log.status_change') };
    default: return { color: 'text-amber-500', bg: 'bg-amber-100', label: type };
  }
};

const todayCount = computed(() => {
  const today = new Date().toISOString().slice(0, 10);
  return logs.value.filter(l => l.created_at?.startsWith(today)).length;
});
</script>

<template>
  <div class="h-full flex flex-col">
    <header class="mb-5 flex justify-between items-end">
      <div>
        <h1 class="text-2xl font-extrabold text-[#451a03] tracking-tight flex items-center gap-2">
          <History
            class="text-indigo-500"
            :size="24"
          /> {{ t('friend_log.title') }}
        </h1>
        <p class="text-amber-700/70 text-sm mt-1">
          {{ t('friend_log.subtitle') }}
        </p>
      </div>
      <div class="flex items-center gap-2">
        <span class="text-xs font-bold px-3 py-1 rounded-full bg-indigo-100 text-indigo-700">{{ t('friend_log.today_events', { count: todayCount }) }}</span>
        <button
          class="p-2 rounded-full bg-white hover:bg-indigo-50 text-indigo-600 shadow-sm border border-indigo-100 transition-colors"
          @click="fetchLogs"
        >
          <RefreshCcw
            class="w-4 h-4"
            :class="{'animate-spin': loading}"
          />
        </button>
      </div>
    </header>

    <!-- 过滤器 -->
    <div class="flex gap-1 mb-4 flex-wrap">
      <button
        v-for="et in eventTypes"
        :key="et.value"
        :class="filterType === et.value ? 'bg-indigo-500 text-white' : 'bg-white text-amber-700 hover:bg-indigo-50'"
        class="px-3 py-1.5 rounded-lg text-xs font-bold transition-colors border border-indigo-100 flex items-center gap-1"
        @click="filterType = et.value; fetchLogs()"
      >
        <component
          :is="et.icon"
          :size="12"
        /> {{ et.label }}
      </button>
    </div>

    <!-- 日志列表 -->
    <div class="flex-1 overflow-y-auto pr-1 custom-scrollbar">
      <div
        v-if="loading && logs.length === 0"
        class="flex items-center justify-center py-12 text-indigo-500 font-bold animate-pulse"
      >
        {{ t('friend_log.loading') }}
      </div>

      <div
        v-else-if="logs.length === 0"
        class="flex flex-col items-center justify-center py-16 text-amber-500/60"
      >
        <History
          :size="48"
          class="mb-4 opacity-40"
        />
        <p class="font-bold text-lg">
          {{ t('friend_log.empty') }}
        </p>
        <p class="text-sm mt-1">
          {{ t('friend_log.empty_desc') }}
        </p>
      </div>

      <div
        v-else
        class="space-y-2"
      >
        <div
          v-for="log in logs"
          :key="log.id"
          class="bg-white/80 backdrop-blur rounded-xl p-3 border border-amber-50 hover:border-indigo-200 transition-all flex items-center gap-3"
        >
          <div
            class="w-9 h-9 rounded-lg flex items-center justify-center flex-shrink-0"
            :class="getEventMeta(log.event_type).bg"
          >
            <span
              class="text-sm font-black"
              :class="getEventMeta(log.event_type).color"
            >
              {{ getEventMeta(log.event_type).label.charAt(0) }}
            </span>
          </div>
          <div class="flex-1 min-w-0">
            <div class="flex items-center gap-2">
              <span class="font-bold text-amber-900 text-sm truncate">{{ log.display_name === 'Unknown' ? (log.user_id || t('friend_log.system')) : (log.display_name || t('friend_log.system')) }}</span>
              <span
                class="text-[10px] font-bold px-1.5 py-0.5 rounded-md"
                :class="[getEventMeta(log.event_type).bg, getEventMeta(log.event_type).color]"
              >
                {{ getEventMeta(log.event_type).label }}
              </span>
            </div>
            <p
              v-if="log.detail"
              class="text-xs text-amber-600/70 truncate mt-0.5"
            >
              {{ log.detail }}
            </p>
          </div>
          <span class="text-[10px] text-amber-500 font-mono flex-shrink-0">{{ log.created_at?.slice(5, 16) }}</span>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.custom-scrollbar::-webkit-scrollbar { width: 5px; }
.custom-scrollbar::-webkit-scrollbar-track { background: rgba(224,231,255,0.3); border-radius: 10px; }
.custom-scrollbar::-webkit-scrollbar-thumb { background: rgba(129,140,248,0.4); border-radius: 10px; }
</style>
