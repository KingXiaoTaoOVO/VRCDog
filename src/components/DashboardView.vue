<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { LayoutDashboard, Users, Activity, Clock, Zap, Map, TrendingUp } from 'lucide-vue-next';
import { VrcApi, DbApi } from '../api';
import VrcAvatar from './VrcAvatar.vue';
import { useI18n } from 'vue-i18n';
import type { VrcUser } from '../types/vrc';
import { useUserProfileStore } from '../stores/userProfile';
import { currentTheme } from '../theme';

const { t } = useI18n();
const profileStore = useUserProfileStore();

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
    
    // 获取所有的在线好友
    recentFriends.value = online;

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

const openPlayerProfile = (friend: VrcUser) => {
  profileStore.openProfile(friend.id, friend);
};
</script>

<template>
  <div class="h-full flex flex-col p-6 bg-slate-50/50 rounded-3xl relative overflow-hidden">
    <!-- Subtle Background Glow -->
    <div class="absolute top-0 right-0 w-96 h-96 bg-indigo-500/10 rounded-full blur-[100px] pointer-events-none -z-10" />
    <div class="absolute bottom-0 left-0 w-[500px] h-[500px] bg-blue-500/5 rounded-full blur-[120px] pointer-events-none -z-10" />

    <header class="mb-8 flex justify-between items-end shrink-0 z-10">
      <div>
        <h1 class="text-3xl font-extrabold text-slate-900 tracking-tight flex items-center gap-3">
          {{ t('dashboard.title') }}
          <span class="inline-flex items-center justify-center p-2 bg-indigo-100 rounded-2xl shadow-sm border border-indigo-200/50">
            <LayoutDashboard class="w-6 h-6 text-indigo-600" />
          </span>
        </h1>
        <p class="text-slate-500 font-medium mt-1">
          {{ t('dashboard.subtitle') }}
        </p>
      </div>
      <button
        :disabled="loading"
        class="px-5 py-2.5 bg-white/80 backdrop-blur rounded-full text-slate-700 font-bold border border-slate-200 shadow-sm hover:shadow-md hover:bg-white hover:text-indigo-600 transition-all flex items-center gap-2 disabled:opacity-50"
        @click="fetchData"
      >
        <Zap
          class="w-4 h-4"
          :class="{'animate-pulse text-indigo-500': loading}"
        /> {{ t('dashboard.refresh') }}
      </button>
    </header>

    <!-- Top Stats Grid -->
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6 mb-8 shrink-0 z-10">
      <!-- Online Friends Card -->
      <div class="bg-white/70 backdrop-blur-xl border border-white/80 rounded-3xl p-6 shadow-lg shadow-slate-200/40 hover:-translate-y-1 transition-all group cursor-default relative overflow-hidden">
        <div class="absolute top-0 right-0 p-4 opacity-10 group-hover:opacity-20 transition-opacity">
          <Users class="w-24 h-24 text-indigo-500 transform translate-x-4 -translate-y-4" />
        </div>
        <div class="flex items-center justify-between mb-4 relative z-10">
          <div class="w-12 h-12 rounded-2xl flex items-center justify-center bg-gradient-to-br from-indigo-100 to-indigo-50 text-indigo-500 shadow-sm border border-indigo-100/50">
            <Users class="w-6 h-6" />
          </div>
        </div>
        <div class="relative z-10">
          <p class="text-sm font-bold text-slate-500 mb-1">
            {{ t('dashboard.online_friends') }}
          </p>
          <h3 class="text-3xl font-black text-slate-800 flex items-baseline gap-1">
            {{ onlineFriendsCount }} <span class="text-sm font-bold text-slate-400">{{ t('dashboard.unit_people') }}</span>
          </h3>
        </div>
      </div>
      
      <!-- Active Instances Card -->
      <div class="bg-white/70 backdrop-blur-xl border border-white/80 rounded-3xl p-6 shadow-lg shadow-slate-200/40 hover:-translate-y-1 transition-all group cursor-default relative overflow-hidden">
        <div class="absolute top-0 right-0 p-4 opacity-10 group-hover:opacity-20 transition-opacity">
          <Map class="w-24 h-24 text-blue-500 transform translate-x-4 -translate-y-4" />
        </div>
        <div class="flex items-center justify-between mb-4 relative z-10">
          <div class="w-12 h-12 rounded-2xl flex items-center justify-center bg-gradient-to-br from-blue-100 to-blue-50 text-blue-500 shadow-sm border border-blue-100/50">
            <Map class="w-6 h-6" />
          </div>
        </div>
        <div class="relative z-10">
          <p class="text-sm font-bold text-slate-500 mb-1">
            {{ t('dashboard.active_instances') }}
          </p>
          <h3 class="text-3xl font-black text-slate-800 flex items-baseline gap-1">
            {{ activeInstancesCount }} <span class="text-sm font-bold text-slate-400">{{ t('dashboard.unit_count') }}</span>
          </h3>
        </div>
      </div>
      
      <!-- Records / DB Stats Card -->
      <div class="bg-white/70 backdrop-blur-xl border border-white/80 rounded-3xl p-6 shadow-lg shadow-slate-200/40 hover:-translate-y-1 transition-all group cursor-default relative overflow-hidden">
        <div class="absolute top-0 right-0 p-4 opacity-10 group-hover:opacity-20 transition-opacity">
          <Clock class="w-24 h-24 text-indigo-500 transform translate-x-4 -translate-y-4" />
        </div>
        <div class="flex items-center justify-between mb-4 relative z-10">
          <div class="w-12 h-12 rounded-2xl flex items-center justify-center bg-gradient-to-br from-indigo-100 to-indigo-50 text-indigo-500 shadow-sm border border-slate-100">
            <Clock class="w-6 h-6" />
          </div>
        </div>
        <div class="relative z-10">
          <p class="text-sm font-bold text-slate-500 mb-1">
            {{ t('dashboard.estimated_records') }}
          </p>
          <h3 class="text-3xl font-black text-slate-800 flex items-baseline gap-1">
            {{ t('dashboard.latest') }} <span class="text-sm font-bold text-slate-400">{{ t('dashboard.status') }}</span>
          </h3>
        </div>
      </div>
      
      <!-- Server Status Card -->
      <div class="bg-white/70 backdrop-blur-xl border border-white/80 rounded-3xl p-6 shadow-lg shadow-slate-200/40 hover:-translate-y-1 transition-all group cursor-default relative overflow-hidden">
        <div class="absolute top-0 right-0 p-4 opacity-10 group-hover:opacity-20 transition-opacity">
          <Activity
            class="w-24 h-24 transform translate-x-4 -translate-y-4"
            :class="serverStatus === 'ok' ? 'text-emerald-500' : 'text-red-500'"
          />
        </div>
        <div class="flex items-center justify-between mb-4 relative z-10">
          <div
            class="w-12 h-12 rounded-2xl flex items-center justify-center shadow-sm border"
            :class="serverStatus === 'ok' ? 'bg-gradient-to-br from-emerald-100 to-emerald-50 border-emerald-100/50' : 'bg-gradient-to-br from-red-100 to-red-50 border-red-100/50'"
          >
            <Activity
              class="w-6 h-6"
              :class="serverStatus === 'ok' ? 'text-emerald-500' : 'text-red-500'"
            />
          </div>
        </div>
        <div class="relative z-10">
          <p class="text-sm font-bold text-slate-500 mb-1">
            {{ t('dashboard.server_status') }}
          </p>
          <h3 class="text-3xl font-black text-slate-800 flex items-baseline gap-1">
            {{ serverStatus === 'ok' ? t('dashboard.normal') : t('dashboard.error') }} <span class="text-sm font-bold text-slate-400">API</span>
          </h3>
        </div>
      </div>
    </div>

    <!-- Main Content Area -->
    <div class="flex-1 grid grid-cols-1 lg:grid-cols-3 gap-6 min-h-0 z-10">
      <!-- Activity Heatmap -->
      <div class="lg:col-span-2 bg-white/70 backdrop-blur-xl border border-white rounded-3xl p-6 shadow-lg shadow-slate-200/40 flex flex-col relative overflow-hidden min-h-0">
        <div class="flex items-center justify-between mb-6">
          <h3 class="text-xl font-extrabold text-slate-800 flex items-center gap-2">
            <TrendingUp class="w-5 h-5" :style="{ color: currentTheme.colors.primaryBtnBg }" />
            {{ t('dashboard.weekly_trend') }}
          </h3>
          <div class="px-3 py-1 rounded-lg text-xs font-bold border"
               :style="{ backgroundColor: currentTheme.colors.activeBg, color: currentTheme.colors.textStrong, borderColor: currentTheme.colors.borderStrong }">
            7 Days Activity
          </div>
        </div>
        
        <div class="flex-1 rounded-2xl border flex items-end justify-around pt-8 pb-4 px-6 overflow-hidden relative shadow-inner"
             :style="{ 
               background: `linear-gradient(to bottom, ${currentTheme.colors.bgMain}, ${currentTheme.colors.activeBg})`,
               borderColor: currentTheme.colors.borderStrong 
             }">
             
          <!-- Horizontal Grid Lines -->
          <div class="absolute inset-0 z-0 flex flex-col justify-between pt-12 pb-10 px-6 pointer-events-none">
            <div class="w-full border-t border-dashed" :style="{ borderColor: currentTheme.colors.borderStrong, opacity: 0.4 }"></div>
            <div class="w-full border-t border-dashed" :style="{ borderColor: currentTheme.colors.borderStrong, opacity: 0.4 }"></div>
            <div class="w-full border-t border-dashed" :style="{ borderColor: currentTheme.colors.borderStrong, opacity: 0.4 }"></div>
            <div class="w-full border-t border-dashed" :style="{ borderColor: currentTheme.colors.borderStrong, opacity: 0.4 }"></div>
          </div>

          <div
            v-for="(val, idx) in heatmapData"
            :key="idx"
            class="flex flex-col items-center group z-10 w-[10%] h-full justify-end relative"
          >
            <!-- Background Track (Full Height of Chart Area) -->
            <div class="absolute bottom-[28px] top-0 w-full max-w-[40px] rounded-xl transition-colors"
                 :style="{ backgroundColor: currentTheme.colors.borderStrong, opacity: 0.15 }">
            </div>

            <!-- Chart Bar Wrapper for positioning -->
            <div class="w-full max-w-[40px] flex flex-col justify-end relative mb-3"
                 :style="{ height: 'calc(100% - 28px)' }">
                 
              <!-- The Active Bar -->
              <div
                class="w-full rounded-xl transition-all duration-700 ease-out relative shadow-md cursor-pointer hover:brightness-110" 
                :style="{ 
                  height: `${Math.max((val / Math.max(...heatmapData, 1)) * 100, 2)}%`,
                  background: `linear-gradient(to top, ${currentTheme.colors.primaryBtnHover}, ${currentTheme.colors.primaryBtnBg})`,
                  opacity: val === 0 ? 0.3 : 1
                }"
              >
                <!-- Tooltip -->
                <div class="absolute -top-10 left-1/2 -translate-x-1/2 opacity-0 group-hover:opacity-100 bg-slate-800 text-white text-xs font-bold py-1.5 px-3 rounded-lg transition-all transform group-hover:-translate-y-1 whitespace-nowrap shadow-xl pointer-events-none z-20">
                  {{ val }} <span class="text-slate-400 font-normal">Events</span>
                  <!-- little arrow -->
                  <div class="absolute -bottom-1 left-1/2 -translate-x-1/2 w-2 h-2 bg-slate-800 rotate-45" />
                </div>
              </div>
            </div>
            
            <!-- X-Axis Label -->
            <span class="text-xs font-bold transition-colors h-[16px] flex items-center" :style="{ color: currentTheme.colors.textSoft }">
              {{ t(`dashboard.days.${idx}`) }}
            </span>
          </div>
        </div>
      </div>
      
      <!-- Active Friends List -->
      <div class="bg-white/70 backdrop-blur-xl border border-white rounded-3xl p-6 shadow-lg shadow-slate-200/40 flex flex-col min-h-0">
        <div class="flex items-center justify-between mb-6 shrink-0">
          <h3 class="text-xl font-extrabold text-slate-800 flex items-center gap-2">
            <div class="w-2 h-2 rounded-full bg-green-500 animate-pulse" />
            {{ t('dashboard.active_friends') }}
          </h3>
          <span class="text-xs font-bold text-slate-400">{{ onlineFriendsCount }} Online</span>
        </div>
        
        <div class="flex-1 overflow-y-auto custom-scrollbar space-y-1.5 pr-2 -mr-2">
          <div
            v-if="recentFriends.length === 0"
            class="h-full flex flex-col items-center justify-center text-slate-400"
          >
            <Users class="w-12 h-12 mb-3 opacity-20" />
            <p class="text-sm font-bold">
              {{ t('dashboard.no_online_friends') }}
            </p>
          </div>
          
          <div
            v-for="friend in recentFriends"
            :key="friend.id"
            class="flex items-center gap-3 p-2.5 bg-white border border-transparent hover:border-slate-200 hover:shadow-sm rounded-2xl transition-all cursor-pointer group"
            @click="openPlayerProfile(friend)"
          >
            <div class="relative">
              <VrcAvatar
                :user="friend"
                custom-class="w-11 h-11 rounded-full object-cover shadow-sm bg-slate-100 border border-slate-200"
              />
              <div class="absolute -bottom-0.5 -right-0.5 w-3.5 h-3.5 rounded-full bg-green-500 border border-slate-200" />
            </div>
            
            <div class="flex-1 min-w-0">
              <p class="text-sm font-extrabold text-slate-800 truncate group-hover:text-indigo-600 transition-colors">
                {{ friend.displayName }}
              </p>
              <p class="text-[10px] font-medium text-slate-500 truncate flex items-center gap-1">
                <Map
                  v-if="friend.location && friend.location !== 'private'"
                  class="w-3 h-3 opacity-70"
                />
                {{ friend.location === 'private' ? 'Private Instance' : (friend.statusDescription || 'Online') }}
              </p>
            </div>
            <div class="w-6 h-6 rounded-full bg-slate-50 flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity">
              <div class="w-1.5 h-1.5 rounded-full bg-slate-300" />
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.custom-scrollbar::-webkit-scrollbar { width: 4px; }
.custom-scrollbar::-webkit-scrollbar-track { background: transparent; }
.custom-scrollbar::-webkit-scrollbar-thumb { background: rgba(148, 163, 184, 0.3); border-radius: 10px; }
.custom-scrollbar::-webkit-scrollbar-thumb:hover { background: rgba(148, 163, 184, 0.5); }
</style>
