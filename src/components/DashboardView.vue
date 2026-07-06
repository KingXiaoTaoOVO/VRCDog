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
  profileStore.openProfile(friend.id, friend as any);
};
</script>

<template>
  <div class="h-full flex flex-col p-6 bg-[var(--theme-surface-hover)] rounded-3xl relative overflow-hidden">
    <!-- Subtle Background Glow -->
    <div class="absolute top-0 right-0 w-96 h-96 bg-[var(--theme-primary)]/10 rounded-full blur-[100px] pointer-events-none -z-10" />
    <div class="absolute bottom-0 left-0 w-[500px] h-[500px] bg-[var(--theme-primary)]/5 rounded-full blur-[120px] pointer-events-none -z-10" />

    <header class="mb-8 flex justify-between items-end shrink-0 z-10">
      <div>
        <h1 class="text-3xl font-extrabold text-[var(--theme-text-strong)] tracking-tight flex items-center gap-3">
          {{ t('dashboard.title') }}
          <span class="inline-flex items-center justify-center p-2 bg-[var(--theme-primary)]/10 rounded-2xl shadow-sm border-[var(--theme-border-soft)]">
            <LayoutDashboard class="w-6 h-6 text-[var(--theme-primary)]" />
          </span>
        </h1>
        <p class="text-[var(--theme-text-soft)] font-medium mt-1">
          {{ t('dashboard.subtitle') }}
        </p>
      </div>
      <button
        :disabled="loading"
        class="px-5 py-2.5 bg-[var(--theme-surface)] backdrop-blur rounded-full text-[var(--theme-text-soft)] font-bold border-[var(--theme-border-soft)] shadow-sm hover:shadow-md hover:bg-[var(--theme-surface-hover)] hover:text-[var(--theme-primary)] transition-all flex items-center gap-2 disabled:opacity-50"
        @click="fetchData"
      >
        <Zap
          class="w-4 h-4"
          :class="{'animate-pulse text-[var(--theme-primary)]': loading}"
        /> {{ t('dashboard.refresh') }}
      </button>
    </header>

    <!-- Top Stats Grid -->
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6 mb-8 shrink-0 z-10">
      <!-- Online Friends Card -->
      <div class="bg-[var(--theme-surface)] backdrop-blur-xl border border-[var(--theme-border-soft)] rounded-3xl p-6 shadow-lg shadow-black/5 transition-all group cursor-default relative overflow-hidden">
        <div class="absolute top-0 right-0 p-4 opacity-10 group-hover:opacity-20 transition-opacity">
          <Users class="w-24 h-24 text-[var(--theme-primary)] transform translate-x-4 -translate-y-4" />
        </div>
        <div class="flex items-center justify-between mb-4 relative z-10">
          <div class="w-12 h-12 rounded-2xl flex items-center justify-center bg-gradient-to-br from-[var(--theme-primary)]/10 to-[var(--theme-primary)]/5 text-[var(--theme-primary)] shadow-sm border border-[var(--theme-border-soft)]">
            <Users class="w-6 h-6" />
          </div>
        </div>
        <div class="relative z-10">
          <p class="text-sm font-bold text-[var(--theme-text-soft)] mb-1">
            {{ t('dashboard.online_friends') }}
          </p>
          <h3 class="text-3xl font-black text-[var(--theme-text-strong)] flex items-baseline gap-1">
            {{ onlineFriendsCount }} <span class="text-sm font-bold text-[var(--theme-text-muted)]">{{ t('dashboard.unit_people') }}</span>
          </h3>
        </div>
      </div>
      
      <!-- Active Instances Card -->
      <div class="bg-[var(--theme-surface)] backdrop-blur-xl border border-[var(--theme-border-soft)] rounded-3xl p-6 shadow-lg shadow-black/5 transition-all group cursor-default relative overflow-hidden">
        <div class="absolute top-0 right-0 p-4 opacity-10 group-hover:opacity-20 transition-opacity">
          <Map class="w-24 h-24 text-[var(--theme-primary)] transform translate-x-4 -translate-y-4" />
        </div>
        <div class="flex items-center justify-between mb-4 relative z-10">
          <div class="w-12 h-12 rounded-2xl flex items-center justify-center bg-gradient-to-br from-[var(--theme-primary)]/10 to-[var(--theme-primary)]/5 text-[var(--theme-primary)] shadow-sm border border-[var(--theme-border-soft)]">
            <Map class="w-6 h-6" />
          </div>
        </div>
        <div class="relative z-10">
          <p class="text-sm font-bold text-[var(--theme-text-soft)] mb-1">
            {{ t('dashboard.active_instances') }}
          </p>
          <h3 class="text-3xl font-black text-[var(--theme-text-strong)] flex items-baseline gap-1">
            {{ activeInstancesCount }} <span class="text-sm font-bold text-[var(--theme-text-muted)]">{{ t('dashboard.unit_count') }}</span>
          </h3>
        </div>
      </div>
      
      <!-- Records / DB Stats Card -->
      <div class="bg-[var(--theme-surface)] backdrop-blur-xl border border-[var(--theme-border-soft)] rounded-3xl p-6 shadow-lg shadow-black/5 transition-all group cursor-default relative overflow-hidden">
        <div class="absolute top-0 right-0 p-4 opacity-10 group-hover:opacity-20 transition-opacity">
          <Clock class="w-24 h-24 text-[var(--theme-primary)] transform translate-x-4 -translate-y-4" />
        </div>
        <div class="flex items-center justify-between mb-4 relative z-10">
          <div class="w-12 h-12 rounded-2xl flex items-center justify-center bg-gradient-to-br from-[var(--theme-primary)]/10 to-[var(--theme-primary)]/5 text-[var(--theme-primary)] shadow-sm border border-[var(--theme-border-soft)]">
            <Clock class="w-6 h-6" />
          </div>
        </div>
        <div class="relative z-10">
          <p class="text-sm font-bold text-[var(--theme-text-soft)] mb-1">
            {{ t('dashboard.estimated_records') }}
          </p>
          <h3 class="text-3xl font-black text-[var(--theme-text-strong)] flex items-baseline gap-1">
            {{ t('dashboard.latest') }} <span class="text-sm font-bold text-[var(--theme-text-muted)]">{{ t('dashboard.status') }}</span>
          </h3>
        </div>
      </div>
      
      <!-- Server Status Card -->
      <div class="bg-[var(--theme-surface)] backdrop-blur-xl border border-[var(--theme-border-soft)] rounded-3xl p-6 shadow-lg shadow-black/5 transition-all group cursor-default relative overflow-hidden">
        <div class="absolute top-0 right-0 p-4 opacity-10 group-hover:opacity-20 transition-opacity">
          <Activity
            class="w-24 h-24 transform translate-x-4 -translate-y-4"
            :class="serverStatus === 'ok' ? 'text-emerald-500' : 'text-red-500'"
          />
        </div>
        <div class="flex items-center justify-between mb-4 relative z-10">
          <div
            class="w-12 h-12 rounded-2xl flex items-center justify-center shadow-sm border border-[var(--theme-border-soft)]"
            :class="serverStatus === 'ok' ? 'bg-emerald-500/10' : 'bg-red-500/10'"
          >
            <Activity
              class="w-6 h-6"
              :class="serverStatus === 'ok' ? 'text-emerald-500' : 'text-red-500'"
            />
          </div>
        </div>
        <div class="relative z-10">
          <p class="text-sm font-bold text-[var(--theme-text-soft)] mb-1">
            {{ t('dashboard.server_status') }}
          </p>
          <h3 class="text-3xl font-black text-[var(--theme-text-strong)] flex items-baseline gap-1">
            {{ serverStatus === 'ok' ? t('dashboard.normal') : t('dashboard.error') }} <span class="text-sm font-bold text-[var(--theme-text-muted)]">API</span>
          </h3>
        </div>
      </div>
    </div>

    <!-- Main Content Area -->
    <div class="flex-1 grid grid-cols-1 lg:grid-cols-3 gap-6 min-h-0 z-10">
      <!-- Activity Heatmap -->
      <div class="lg:col-span-2 bg-[var(--theme-surface)] backdrop-blur-xl border border-[var(--theme-border-soft)] rounded-3xl p-6 shadow-lg shadow-black/5 flex flex-col relative min-h-0">
        <div class="flex items-center justify-between mb-6">
          <h3 class="text-xl font-extrabold text-[var(--theme-text-strong)] flex items-center gap-2">
            <TrendingUp class="w-5 h-5 text-[var(--theme-primary)]" />
            {{ t('dashboard.weekly_trend') }}
          </h3>
          <div class="px-3 py-1 rounded-lg text-xs font-bold border border-[var(--theme-border-soft)] text-[var(--theme-text-soft)] bg-[var(--theme-surface-hover)]">
            7 Days Activity
          </div>
        </div>
        
        <div class="flex-1 rounded-2xl flex items-end justify-around pt-8 pb-4 px-6 relative shadow-inner"
             :style="{ 
               background: `linear-gradient(to bottom, var(--theme-bg-main), var(--theme-surface))`,
               borderColor: 'var(--theme-border-strong)' 
             }">
             
          <!-- Horizontal Grid Lines -->
          <div class="absolute inset-0 z-0 flex flex-col justify-between pt-12 pb-10 px-6 pointer-events-none">
            <div class="w-full border-dashed" :style="{ borderColor: 'var(--theme-border-soft)', opacity: 0.4 }"></div>
            <div class="w-full border-dashed" :style="{ borderColor: 'var(--theme-border-soft)', opacity: 0.4 }"></div>
            <div class="w-full border-dashed" :style="{ borderColor: 'var(--theme-border-soft)', opacity: 0.4 }"></div>
            <div class="w-full border-dashed" :style="{ borderColor: 'var(--theme-border-soft)', opacity: 0.4 }"></div>
          </div>

          <div
            v-for="(val, idx) in heatmapData"
            :key="idx"
            class="flex flex-col items-center group z-10 w-[10%] h-full justify-end relative"
          >
            <!-- Background Track -->
            <div class="absolute bottom-[28px] top-0 w-full max-w-[40px] rounded-xl transition-colors"
                 :style="{ backgroundColor: 'var(--theme-border-soft)', opacity: 0.15 }">
            </div>

            <!-- Chart Bar Wrapper -->
            <div class="w-full max-w-[40px] flex flex-col justify-end relative mb-3"
                 :style="{ height: 'calc(100% - 28px)' }">
                 
              <!-- The Active Bar -->
              <div
                class="w-full rounded-xl transition-all duration-700 ease-out relative shadow-md cursor-pointer hover:brightness-110" 
                :style="{ 
                  height: `${Math.max((val / Math.max(...heatmapData, 1)) * 100, 2)}%`,
                  background: `linear-gradient(to top, var(--theme-primary-hover), var(--theme-primary))`,
                  opacity: val === 0 ? 0.3 : 1
                }"
              >
                <!-- Tooltip -->
                <div class="absolute -top-10 left-1/2 -translate-x-1/2 opacity-0 group-hover:opacity-100 bg-[var(--theme-bg-main)]/90 backdrop-blur-md border border-[var(--theme-border-strong)] text-[var(--theme-text-strong)] text-xs font-bold py-1.5 px-3 rounded-lg transition-all transform group- whitespace-nowrap shadow-xl pointer-events-none z-50">
                  {{ val }} <span class="text-[var(--theme-text-soft)] font-normal">Events</span>
                  <!-- little arrow -->
                  <div class="absolute -bottom-1.5 left-1/2 -translate-x-1/2 w-2.5 h-2.5 bg-[var(--theme-bg-main)]/90 border-b border-r border-[var(--theme-border-strong)] rotate-45" />
                </div>
              </div>
            </div>
            
            <!-- X-Axis Label -->
            <span class="text-xs font-bold transition-colors h-[16px] flex items-center text-[var(--theme-text-soft)]">
              {{ t(`dashboard.days.${idx}`) }}
            </span>
          </div>
        </div>
      </div>
      
      <!-- Active Friends List -->
      <div class="bg-[var(--theme-surface)] backdrop-blur-xl border border-[var(--theme-border-soft)] rounded-3xl p-6 shadow-lg shadow-black/5 flex flex-col min-h-0">
        <div class="flex items-center justify-between mb-6 shrink-0">
          <h3 class="text-xl font-extrabold text-[var(--theme-text-strong)] flex items-center gap-2">
            <div class="w-2 h-2 rounded-full bg-green-500 animate-pulse" />
            {{ t('dashboard.active_friends') }}
          </h3>
          <span class="text-xs font-bold text-[var(--theme-text-muted)]">{{ onlineFriendsCount }} {{ t('dashboard.online') }}</span>
        </div>
        
        <div class="flex-1 overflow-y-auto custom-scrollbar space-y-1.5 pr-2 -mr-2">
          <div
            v-if="recentFriends.length === 0"
            class="h-full flex flex-col items-center justify-center text-[var(--theme-text-muted)]"
          >
            <Users class="w-12 h-12 mb-3 opacity-20" />
            <p class="text-sm font-bold">
              {{ t('dashboard.no_online_friends') }}
            </p>
          </div>
          
          <div
            v-for="friend in recentFriends"
            :key="friend.id"
            class="flex items-center gap-3 p-2.5 bg-[var(--theme-surface)] border border-transparent hover:border-[var(--theme-border-soft)] hover:shadow-sm rounded-2xl transition-all cursor-pointer group"
            @click="openPlayerProfile(friend)"
          >
            <div class="relative">
              <VrcAvatar
                :user="friend"
                custom-class="w-11 h-11 rounded-full object-cover shadow-sm bg-[var(--theme-surface)] border border-[var(--theme-border-soft)]"
              />
              <div class="absolute -bottom-0.5 -right-0.5 w-3.5 h-3.5 rounded-full bg-green-500 border border-[var(--theme-border-soft)]" />
            </div>
            
            <div class="flex-1 min-w-0">
              <p class="text-sm font-extrabold text-[var(--theme-text-strong)] truncate group-hover:text-[var(--theme-primary)] transition-colors">
                {{ friend.displayName }}
              </p>
              <p class="text-[10px] font-medium text-[var(--theme-text-soft)] truncate flex items-center gap-1">
                <Map
                  v-if="friend.location && friend.location !== 'private'"
                  class="w-3 h-3 opacity-70"
                />
                {{ friend.location === 'private' ? t('dashboard.private_instance') : (friend.statusDescription || t('dashboard.online')) }}
              </p>
            </div>
            <div class="w-6 h-6 rounded-full bg-[var(--theme-surface-hover)] flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity">
              <div class="w-1.5 h-1.5 rounded-full bg-[var(--theme-surface)]" />
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>


