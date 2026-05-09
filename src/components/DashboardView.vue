<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { LayoutDashboard, Users, Activity, Clock, Zap, Map } from 'lucide-vue-next';
import { VrcApi, DbApi } from '../api';
import VrcAvatar from './VrcAvatar.vue';
import { useI18n } from 'vue-i18n';
import type { VrcUser } from '../types/vrc';

const { t } = useI18n();

const loading = ref(false);
const onlineFriendsCount = ref(0);
const activeInstancesCount = ref(0);
const serverStatus = ref('ok');
const recentFriends = ref<VrcUser[]>([]);
const heatmapData = ref<number[]>([0, 0, 0, 0, 0, 0, 0]); // 7天数据

const fetchData = async () => {
  loading.value = true;
  try {
    // 1. 获取在线好友与实例
    const friends = await VrcApi.getFriends({ n: 100, offset: 0 });
    const online = friends.filter((f: VrcUser) => f.location && f.location !== 'offline');
    onlineFriendsCount.value = online.length;
    
    // 计算不重复的实例数量
    const instances = new Set(online.map((f: VrcUser) => f.location).filter((loc: string | undefined) => loc && loc !== 'private'));
    activeInstancesCount.value = instances.size;
    
    // 获取最近5个在线好友
    recentFriends.value = online.slice(0, 5);

    // 2. 获取服务器状态
    const statusRes = await VrcApi.getServerStatus();
    serverStatus.value = statusRes?.status?.indicator === 'none' ? 'ok' : 'error';

    // 3. 获取热力图数据 (按天统计)
    const heatmap = await DbApi.getHeatmap();
    const days = [0, 0, 0, 0, 0, 0, 0];
    if (Array.isArray(heatmap)) {
      heatmap.forEach((cell: any) => {
        if (cell.day >= 0 && cell.day < 7) {
          days[cell.day] += cell.count;
        }
      });
    }
    heatmapData.value = days;

  } catch (err) {
    console.error('Failed to fetch dashboard data:', err);
  } finally {
    loading.value = false;
  }
};

onMounted(() => {
  fetchData();
});
</script>

<template>
  <div class="h-full flex flex-col">
    <header class="mb-6 flex justify-between items-end">
      <div>
        <h1 class="text-3xl font-extrabold text-[#451a03] tracking-tight flex items-center gap-3">
          {{ t('dashboard.title') }}
          <span class="inline-flex items-center justify-center p-1.5 bg-amber-100 rounded-xl">
            <LayoutDashboard class="w-6 h-6 text-amber-600" />
          </span>
        </h1>
        <p class="text-amber-700/80 font-medium mt-1">
          {{ t('dashboard.subtitle') }}
        </p>
      </div>
      <button
        :disabled="loading"
        class="px-4 py-2 bg-white rounded-full text-amber-700 font-bold border border-amber-200 shadow-sm hover:shadow-md transition-all flex items-center gap-2 disabled:opacity-50"
        @click="fetchData"
      >
        <Zap
          class="w-4 h-4"
          :class="{'animate-pulse text-amber-500': loading}"
        /> {{ t('dashboard.refresh') }}
      </button>
    </header>

    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6 mb-8">
      <div class="bg-white/80 backdrop-blur-xl border-2 border-white rounded-3xl p-6 shadow-xl shadow-amber-900/5 hover:-translate-y-1 transition-transform cursor-default">
        <div class="flex items-center justify-between mb-4">
          <div class="w-12 h-12 rounded-2xl flex items-center justify-center shadow-inner bg-green-100">
            <Users class="w-6 h-6 text-green-500" />
          </div>
        </div>
        <div>
          <p class="text-sm font-bold text-amber-900/60 mb-1">
            {{ t('dashboard.online_friends') }}
          </p>
          <h3 class="text-3xl font-black text-[#451a03] flex items-baseline gap-1">
            {{ onlineFriendsCount }} <span class="text-sm font-bold text-amber-700/50">{{ t('dashboard.unit_people') }}</span>
          </h3>
        </div>
      </div>
      <div class="bg-white/80 backdrop-blur-xl border-2 border-white rounded-3xl p-6 shadow-xl shadow-amber-900/5 hover:-translate-y-1 transition-transform cursor-default">
        <div class="flex items-center justify-between mb-4">
          <div class="w-12 h-12 rounded-2xl flex items-center justify-center shadow-inner bg-blue-100">
            <Map class="w-6 h-6 text-blue-500" />
          </div>
        </div>
        <div>
          <p class="text-sm font-bold text-amber-900/60 mb-1">
            {{ t('dashboard.active_instances') }}
          </p>
          <h3 class="text-3xl font-black text-[#451a03] flex items-baseline gap-1">
            {{ activeInstancesCount }} <span class="text-sm font-bold text-amber-700/50">{{ t('dashboard.unit_count') }}</span>
          </h3>
        </div>
      </div>
      <div class="bg-white/80 backdrop-blur-xl border-2 border-white rounded-3xl p-6 shadow-xl shadow-amber-900/5 hover:-translate-y-1 transition-transform cursor-default">
        <div class="flex items-center justify-between mb-4">
          <div class="w-12 h-12 rounded-2xl flex items-center justify-center shadow-inner bg-amber-100">
            <Clock class="w-6 h-6 text-amber-500" />
          </div>
        </div>
        <div>
          <p class="text-sm font-bold text-amber-900/60 mb-1">
            {{ t('dashboard.estimated_records') }}
          </p>
          <h3 class="text-3xl font-black text-[#451a03] flex items-baseline gap-1">
            {{ t('dashboard.latest') }} <span class="text-sm font-bold text-amber-700/50">{{ t('dashboard.status') }}</span>
          </h3>
        </div>
      </div>
      <div class="bg-white/80 backdrop-blur-xl border-2 border-white rounded-3xl p-6 shadow-xl shadow-amber-900/5 hover:-translate-y-1 transition-transform cursor-default">
        <div class="flex items-center justify-between mb-4">
          <div
            class="w-12 h-12 rounded-2xl flex items-center justify-center shadow-inner"
            :class="serverStatus === 'ok' ? 'bg-emerald-100' : 'bg-red-100'"
          >
            <Activity
              class="w-6 h-6"
              :class="serverStatus === 'ok' ? 'text-emerald-500' : 'text-red-500'"
            />
          </div>
        </div>
        <div>
          <p class="text-sm font-bold text-amber-900/60 mb-1">
            {{ t('dashboard.server_status') }}
          </p>
          <h3 class="text-3xl font-black text-[#451a03] flex items-baseline gap-1">
            {{ serverStatus === 'ok' ? t('dashboard.normal') : t('dashboard.error') }} <span class="text-sm font-bold text-amber-700/50">API</span>
          </h3>
        </div>
      </div>
    </div>

    <!-- 真实数据展示区 -->
    <div class="flex-1 grid grid-cols-1 lg:grid-cols-3 gap-6 min-h-0">
      <div class="lg:col-span-2 bg-white/60 backdrop-blur-md border-2 border-white rounded-3xl p-6 shadow-lg shadow-amber-900/5 flex flex-col">
        <h3 class="text-lg font-extrabold text-[#451a03] mb-4">
          {{ t('dashboard.weekly_trend') }}
        </h3>
        <div class="flex-1 bg-amber-50/50 rounded-2xl border border-amber-100 flex items-end justify-around pt-8 pb-4 px-4 overflow-hidden relative">
          <div
            v-for="(val, idx) in heatmapData"
            :key="idx"
            class="flex flex-col items-center gap-2 group z-10 w-1/8"
          >
            <div
              class="w-full max-w-[40px] bg-amber-200 rounded-t-lg transition-all duration-500 relative group-hover:bg-amber-400" 
              :style="{ height: `${Math.max(val * 5, 5)}%`, minHeight: '10px' }"
            >
              <div class="absolute -top-8 left-1/2 -translate-x-1/2 opacity-0 group-hover:opacity-100 bg-amber-900 text-white text-[10px] font-bold py-1 px-2 rounded transition-opacity whitespace-nowrap shadow-lg">
                {{ t('dashboard.recorded_times', { count: val }) }}
              </div>
            </div>
            <span class="text-xs font-bold text-amber-800">{{ t(`dashboard.days.${idx}`) }}</span>
          </div>
        </div>
      </div>
      <div class="bg-white/60 backdrop-blur-md border-2 border-white rounded-3xl p-6 shadow-lg shadow-amber-900/5 flex flex-col">
        <h3 class="text-lg font-extrabold text-[#451a03] mb-4">
          {{ t('dashboard.active_friends') }}
        </h3>
        <div class="flex-1 bg-amber-50/50 rounded-2xl border border-amber-100 p-2 overflow-y-auto custom-scrollbar space-y-2">
          <div
            v-if="recentFriends.length === 0"
            class="h-full flex items-center justify-center text-sm font-bold text-amber-600/50"
          >
            {{ t('dashboard.no_online_friends') }}
          </div>
          <div
            v-for="friend in recentFriends"
            :key="friend.id"
            class="flex items-center gap-3 p-2 hover:bg-white rounded-xl transition-colors cursor-pointer group"
          >
            <VrcAvatar
              :user="friend"
              custom-class="w-10 h-10 rounded-full object-cover shadow-sm bg-orange-100 border border-amber-200"
            />
            <div class="flex-1 min-w-0">
              <p class="text-sm font-bold text-amber-950 truncate">
                {{ friend.displayName }}
              </p>
              <p class="text-[10px] font-medium text-amber-700 truncate">
                {{ friend.statusDescription || t('dashboard.online') }}
              </p>
            </div>
            <div class="w-2 h-2 rounded-full bg-green-500 group-hover:animate-ping shadow shadow-green-500" />
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.custom-scrollbar::-webkit-scrollbar { width: 4px; }
.custom-scrollbar::-webkit-scrollbar-track { background: transparent; }
.custom-scrollbar::-webkit-scrollbar-thumb { background: rgba(245, 158, 11, 0.2); border-radius: 10px; }
.custom-scrollbar::-webkit-scrollbar-thumb:hover { background: rgba(245, 158, 11, 0.4); }
</style>
