<script setup lang="ts">
import { ref, onMounted, computed, nextTick, watch, onUnmounted } from 'vue';
import { VrcApi, DbApi } from '../api';
import { BarChart3, TrendingUp, Users, Clock, Globe2, Activity, Network, Trophy, LayoutDashboard } from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';
import type { VrcUser, FriendLog } from '../types/vrc';
import * as echarts from 'echarts';

const { t } = useI18n();

const currentTab = ref<'overview' | 'network' | 'worlds'>('overview');
const loading = ref(true);

// 概览数据
const friendStats = ref({ total: 0, online: 0, joinMe: 0, busy: 0, askMe: 0, offline: 0 });
const weeklyActivity = ref<number[]>([0,0,0,0,0,0,0]);
const recentLogs = ref<FriendLog[]>([]);
const allFriends = ref<VrcUser[]>([]);

// 热门世界数据
const topWorlds = ref<Array<{id: string, count: number, name: string, thumbnail?: string}>>([]);

const dayLabels = computed(() => {
  const days = t('charts.days');
  if (Array.isArray(days)) return days;
  return [t('charts.day_1'), t('charts.day_2'), t('charts.day_3'), t('charts.day_4'), t('charts.day_5'), t('charts.day_6'), t('charts.day_7')];
});

const calculateTopWorlds = async (friends: VrcUser[]) => {
  const worldCounts = new Map<string, number>();
  for (const f of friends) {
    if (f.location && f.location !== 'offline' && f.location !== 'private' && f.location.startsWith('wrld_')) {
      const worldId = f.location.split(':')[0];
      worldCounts.set(worldId, (worldCounts.get(worldId) || 0) + 1);
    }
  }
  
  const sorted = Array.from(worldCounts.entries()).sort((a, b) => b[1] - a[1]).slice(0, 15);
  
  const results = sorted.map(([id, count]) => {
    let initialName = id;
    if (initialName.startsWith('wrld_')) {
      initialName = t('charts.unknown_world') + ' (' + initialName.substring(0, 8) + '...)';
    }
    return { id, count, name: initialName, thumbnail: '' };
  });
  topWorlds.value = results;
  
  // 异步获取世界详细信息
  results.forEach(async (item) => {
    try {
      const w: any = await VrcApi.getWorld({ worldId: item.id });
      item.name = w.name;
      item.thumbnail = w.thumbnailImageUrl || w.imageUrl;
    } catch (e) {
      // 忽略单个世界的加载错误
    }
  });
};

const fetchAll = async () => {
  loading.value = true;
  try {
    // 1. 好友统计
    const friends = await VrcApi.getFriends({ n: 100, offset: 0 }); // 暂时取100
    allFriends.value = friends;
    const total = friends.length;
    let online = 0, joinMe = 0, busy = 0, askMe = 0, offline = 0;

    for (const f of friends) {
      if (!f.location || f.location === 'offline') { offline++; }
      else {
        online++;
        if (f.status === 'join me') joinMe++;
        else if (f.status === 'busy') busy++;
        else if (f.status === 'ask me') askMe++;
      }
    }
    friendStats.value = { total, online, joinMe, busy, askMe, offline };

    // 2. 热门世界
    calculateTopWorlds(friends);

    // 3. 周活跃数据
    const heatmap = await DbApi.getHeatmap();
    const days = [0,0,0,0,0,0,0];
    if (Array.isArray(heatmap)) {
      heatmap.forEach((cell: { day: number; count: number }) => {
        if (cell.day >= 0 && cell.day < 7) days[cell.day] += cell.count;
      });
    }
    weeklyActivity.value = days;

    // 4. 最近好友日志
    try {
      const logs = await DbApi.getFriendLogs({ limit: 20 });
      recentLogs.value = logs;
    } catch { recentLogs.value = []; }

  } catch (err) {
    console.warn('统计数据加载失败:', err);
  } finally {
    loading.value = false;
  }
};

onMounted(() => fetchAll());

const maxWeekly = computed(() => Math.max(1, ...weeklyActivity.value));

const onlinePercent = computed(() => {
  if (friendStats.value.total === 0) return 0;
  return Math.round((friendStats.value.online / friendStats.value.total) * 100);
});

// --- 共同好友拓扑图逻辑 (参考 VRCX) ---
const networkChartRef = ref<HTMLElement | null>(null);
let chartInstance: echarts.ECharts | null = null;

const isFetchingMutuals = ref(false);
const mutualFetchProgress = ref({ current: 0, total: 0 });
const mutualGraphReady = ref(false);
const mutualGraphNodes = ref<any[]>([]);
const mutualGraphLinks = ref<any[]>([]);

const fetchMutualFriends = async () => {
  if (isFetchingMutuals.value) return;
  isFetchingMutuals.value = true;
  mutualGraphReady.value = false;
  
  const friends = allFriends.value;
  mutualFetchProgress.value = { current: 0, total: friends.length };
  
  const nodesMap = new Map();
  const linksMap = new Map();
  const nodeDegree = new Map();
  
  const ensureNode = (id: string, name: string) => {
    if (!nodesMap.has(id)) {
      nodesMap.set(id, { id, name });
      nodeDegree.set(id, 0);
    }
  };
  const addEdge = (source: string, target: string) => {
    if (source === target) return;
    const [a, b] = [source, target].sort();
    const key = `${a}__${b}`;
    if (!linksMap.has(key)) {
      linksMap.set(key, { source: a, target: b });
      nodeDegree.set(a, (nodeDegree.get(a) || 0) + 1);
      nodeDegree.set(b, (nodeDegree.get(b) || 0) + 1);
    }
  };

  const PALETTE = ['#5470c6', '#91cc75', '#fac858', '#ee6666', '#73c0de', '#3ba272', '#fc8452', '#9a60b4', '#ea7ccc'];

  for (let i = 0; i < friends.length; i++) {
    const friend = friends[i];
    ensureNode(friend.id, friend.displayName);
    
    try {
      let offset = 0;
      while (true) {
        const res = await VrcApi.getMutualFriends({ userId: friend.id, n: 100, offset });
        const mutuals = Array.isArray(res) ? res : (res.data || []);
        if (!mutuals || mutuals.length === 0) break;
        
        for (const m of mutuals) {
          ensureNode(m.id, m.displayName || m.id);
          addEdge(friend.id, m.id);
        }
        
        if (mutuals.length < 100) break;
        offset += mutuals.length;
        await new Promise(r => setTimeout(r, 100)); // 小间隔避免触发限制
      }
    } catch(e) {
      console.warn("获取共同好友失败:", friend.displayName);
    }
    mutualFetchProgress.value.current = i + 1;
    // 速率限制，VRCX 设置的是每秒5次，我们保守一点，每次请求间隔 250ms
    await new Promise(r => setTimeout(r, 250));
  }
  
  // 简单分配颜色 (用节点 ID hash 或者 连通度分类)
  mutualGraphNodes.value = Array.from(nodesMap.values()).map((n, idx) => {
    const degree = nodeDegree.get(n.id) || 0;
    const color = PALETTE[degree % PALETTE.length];
    return {
      id: n.id,
      name: n.name,
      symbolSize: Math.max(8, Math.min(40, 8 + degree * 0.8)),
      itemStyle: { color: color },
      category: degree % PALETTE.length
    };
  });
  mutualGraphLinks.value = Array.from(linksMap.values());
  mutualGraphReady.value = true;
  isFetchingMutuals.value = false;
  
  await nextTick();
  renderNetworkChart();
};

const renderNetworkChart = () => {
  if (!networkChartRef.value) return;
  if (!chartInstance) {
    chartInstance = echarts.init(networkChartRef.value);
  }

  // 渲染 VRCX 风格的相互图
  const option = {
    tooltip: {
      formatter: '{b}'
    },
    series: [
      {
        type: 'graph',
        layout: 'force',
        data: mutualGraphNodes.value,
        links: mutualGraphLinks.value,
        roam: true,
        label: {
          show: true,
          position: 'right',
          formatter: '{b}',
          fontSize: 10,
          color: 'auto'
        },
        force: {
          repulsion: 150,
          edgeLength: [30, 80],
          friction: 0.1,
          gravity: 0.1
        },
        lineStyle: {
          color: 'source',
          curveness: 0.1,
          opacity: 0.3
        }
      }
    ]
  };

  chartInstance.setOption(option);
};

watch(currentTab, async (newVal) => {
  if (newVal === 'network') {
    await nextTick();
    if (mutualGraphReady.value) {
      renderNetworkChart();
    }
  }
});

const handleResize = () => {
  if (chartInstance) {
    chartInstance.resize();
  }
};

onMounted(() => {
  window.addEventListener('resize', handleResize);
});
onUnmounted(() => {
  window.removeEventListener('resize', handleResize);
  if (chartInstance) chartInstance.dispose();
});
</script>

<template>
  <div class="h-full flex flex-col p-6 bg-slate-50/50 rounded-3xl relative overflow-hidden">
    <!-- Subtle Background Glow -->
    <div class="absolute top-0 right-0 w-96 h-96 bg-indigo-500/10 rounded-full blur-[100px] pointer-events-none -z-10" />
    <div class="absolute bottom-0 left-0 w-[500px] h-[500px] bg-blue-500/5 rounded-full blur-[120px] pointer-events-none -z-10" />

    <!-- 顶部导航 Tab -->
    <div class="flex items-center gap-6 border-b border-slate-200/60 mb-6 shrink-0 z-10">
      <button
        :class="currentTab === 'overview' ? 'border-b-2 border-indigo-600 text-indigo-700 font-bold' : 'border-b-2 border-transparent text-slate-500 hover:text-slate-800 hover:border-slate-300 font-medium'"
        class="py-3 px-2 transition-all text-sm flex items-center gap-2"
        @click="currentTab = 'overview'"
      >
        <LayoutDashboard :size="16" /> {{ t('charts.overview') }}
      </button>
      <button
        :class="currentTab === 'network' ? 'border-b-2 border-indigo-600 text-indigo-700 font-bold' : 'border-b-2 border-transparent text-slate-500 hover:text-slate-800 hover:border-slate-300 font-medium'"
        class="py-3 px-2 transition-all text-sm flex items-center gap-2"
        @click="currentTab = 'network'"
      >
        <Network :size="16" /> {{ t('charts.network') }}
      </button>
      <button
        :class="currentTab === 'worlds' ? 'border-b-2 border-indigo-600 text-indigo-700 font-bold' : 'border-b-2 border-transparent text-slate-500 hover:text-slate-800 hover:border-slate-300 font-medium'"
        class="py-3 px-2 transition-all text-sm flex items-center gap-2"
        @click="currentTab = 'worlds'"
      >
        <Trophy :size="16" /> {{ t('charts.top_worlds') }}
      </button>
    </div>

    <div class="flex-1 overflow-y-auto custom-scrollbar z-10 relative pr-2">
      <div
        v-if="loading"
        class="flex flex-col items-center justify-center py-20 text-indigo-500/80 h-full"
      >
        <div class="w-10 h-10 border-4 border-indigo-200 border-t-indigo-600 rounded-full animate-spin mb-4" />
        <span class="font-extrabold text-lg tracking-wide">{{ t('charts.analyzing') }}</span>
      </div>

      <template v-else>
        <!-- 概览面板 -->
        <div
          v-show="currentTab === 'overview'"
          class="space-y-6"
        >
          <!-- 统计卡片 -->
          <div class="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-6 gap-4">
            <div class="bg-white/80 backdrop-blur-md rounded-2xl p-5 border border-slate-200 shadow-sm text-center transform transition-all hover:-translate-y-1 hover:shadow-lg hover:border-indigo-300">
              <Users
                class="mx-auto mb-3 text-indigo-500"
                :size="24"
              />
              <div class="text-3xl font-black text-slate-800 mb-1 tracking-tight">
                {{ friendStats.total }}
              </div>
              <div class="text-xs text-slate-500 font-bold">
                {{ t('charts.total_friends') }}
              </div>
            </div>
            <div class="bg-white/80 backdrop-blur-md rounded-2xl p-5 border border-slate-200 shadow-sm text-center transform transition-all hover:-translate-y-1 hover:shadow-lg hover:border-emerald-300">
              <Activity
                class="mx-auto mb-3 text-emerald-500"
                :size="24"
              />
              <div class="text-3xl font-black text-slate-800 mb-1 tracking-tight">
                {{ friendStats.online }}
              </div>
              <div class="text-xs text-slate-500 font-bold">
                {{ t('charts.online') }} <span class="text-emerald-600">({{ onlinePercent }}%)</span>
              </div>
            </div>
            <div class="bg-white/80 backdrop-blur-md rounded-2xl p-5 border border-slate-200 shadow-sm text-center transform transition-all hover:-translate-y-1 hover:shadow-lg hover:border-blue-300">
              <div class="w-6 h-6 rounded-full bg-blue-500 shadow-md shadow-blue-500/20 mx-auto mb-3" />
              <div class="text-3xl font-black text-slate-800 mb-1 tracking-tight">
                {{ friendStats.joinMe }}
              </div>
              <div class="text-xs text-slate-500 font-bold">
                {{ t('charts.join_me') }}
              </div>
            </div>
            <div class="bg-white/80 backdrop-blur-md rounded-2xl p-5 border border-slate-200 shadow-sm text-center transform transition-all hover:-translate-y-1 hover:shadow-lg hover:border-orange-300">
              <div class="w-6 h-6 rounded-full bg-orange-500 shadow-md shadow-orange-500/20 mx-auto mb-3" />
              <div class="text-3xl font-black text-slate-800 mb-1 tracking-tight">
                {{ friendStats.askMe }}
              </div>
              <div class="text-xs text-slate-500 font-bold">
                {{ t('charts.ask_me') }}
              </div>
            </div>
            <div class="bg-white/80 backdrop-blur-md rounded-2xl p-5 border border-slate-200 shadow-sm text-center transform transition-all hover:-translate-y-1 hover:shadow-lg hover:border-red-300">
              <div class="w-6 h-6 rounded-full bg-red-500 shadow-md shadow-red-500/20 mx-auto mb-3" />
              <div class="text-3xl font-black text-slate-800 mb-1 tracking-tight">
                {{ friendStats.busy }}
              </div>
              <div class="text-xs text-slate-500 font-bold">
                {{ t('charts.busy') }}
              </div>
            </div>
            <div class="bg-white/80 backdrop-blur-md rounded-2xl p-5 border border-slate-200 shadow-sm text-center transform transition-all hover:-translate-y-1 hover:shadow-lg hover:border-slate-300">
              <div class="w-6 h-6 rounded-full bg-slate-400 shadow-md shadow-slate-400/20 mx-auto mb-3" />
              <div class="text-3xl font-black text-slate-800 mb-1 tracking-tight">
                {{ friendStats.offline }}
              </div>
              <div class="text-xs text-slate-500 font-bold">
                {{ t('charts.offline') }}
              </div>
            </div>
          </div>

          <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
            <!-- 周活跃图表 -->
            <div class="bg-white/70 backdrop-blur-xl rounded-3xl p-6 border border-white shadow-lg shadow-slate-200/40">
              <h2 class="font-extrabold text-slate-900 mb-6 flex items-center gap-3 text-lg">
                <span class="p-1.5 bg-sky-50 rounded-lg text-sky-500"><TrendingUp :size="20" /></span>
                {{ t('charts.weekly_trend') }}
              </h2>
              <div class="flex items-end justify-between gap-3 h-48 px-2">
                <div
                  v-for="(count, idx) in weeklyActivity"
                  :key="idx"
                  class="flex-1 flex flex-col items-center gap-2 group"
                >
                  <span class="text-xs font-bold text-sky-600 opacity-0 group-hover:opacity-100 transition-opacity translate-y-2 group-hover:translate-y-0">{{ count }}</span>
                  <div
                    class="w-full max-w-[48px] rounded-t-xl transition-all duration-700 bg-gradient-to-t from-sky-500 to-sky-300 group-hover:from-sky-400 group-hover:to-sky-200 shadow-sm"
                    :style="{ height: (count / maxWeekly * 100) + '%', minHeight: count > 0 ? '12px' : '4px' }"
                  />
                  <span class="text-xs text-slate-500 font-bold mt-2">{{ dayLabels[idx] }}</span>
                </div>
              </div>
            </div>

            <!-- 最近事件摘要 -->
            <div class="bg-white/70 backdrop-blur-xl rounded-3xl p-6 border border-white shadow-lg shadow-slate-200/40 flex flex-col">
              <h3 class="font-extrabold text-slate-900 mb-4 flex items-center gap-3 text-lg">
                <span class="p-1.5 bg-indigo-50 rounded-lg text-indigo-600"><Clock :size="20" /></span>
                {{ t('charts.recent_events') }}
              </h3>
              <div
                v-if="recentLogs.length === 0"
                class="flex-1 flex items-center justify-center text-sm text-slate-400 font-bold"
              >
                {{ t('charts.no_events') }}
              </div>
              <div
                v-else
                class="flex-1 overflow-y-auto custom-scrollbar pr-2 space-y-2"
              >
                <div
                  v-for="log in recentLogs.slice(0, 10)"
                  :key="log.id"
                  class="flex items-center gap-3 p-3 bg-white hover:bg-slate-50 rounded-xl border border-slate-100 transition-colors shadow-sm"
                >
                  <span
                    class="w-3 h-3 rounded-full flex-shrink-0 shadow-sm"
                    :class="log.event_type === 'online' ? 'bg-emerald-500' : log.event_type === 'offline' ? 'bg-slate-400' : 'bg-indigo-500'"
                  />
                  <span class="text-slate-800 font-bold truncate flex-1 text-sm">{{ log.display_name || t('charts.system') }}</span>
                  <span class="text-slate-500 text-xs font-bold bg-slate-100 px-2.5 py-1 rounded-lg">{{ log.created_at?.slice(11, 16) }}</span>
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- 关系网拓扑图面板 (共同好友) -->
        <div
          v-show="currentTab === 'network'"
          class="h-full min-h-[500px] bg-white/70 backdrop-blur-xl rounded-3xl border border-white shadow-lg shadow-slate-200/40 p-6 flex flex-col"
        >
          <div class="mb-4 flex justify-between items-start">
            <div>
              <h2 class="font-extrabold text-slate-900 flex items-center gap-3 text-lg">
                <span class="p-1.5 bg-indigo-50 rounded-lg text-indigo-600"><Network :size="20" /></span>
                {{ t('charts.mutual_topology') }}
              </h2>
              <p class="text-xs text-slate-500 mt-2 font-medium ml-1">
                {{ t('charts.mutual_desc') }}
              </p>
            </div>
            
            <div class="flex items-center gap-4">
              <div
                v-if="isFetchingMutuals"
                class="flex flex-col items-end"
              >
                <span class="text-xs font-bold text-indigo-600 mb-1.5">
                  {{ t('charts.scanning') }} {{ mutualFetchProgress.current }} / {{ mutualFetchProgress.total }}
                </span>
                <div class="w-40 h-2.5 bg-slate-100 rounded-full overflow-hidden shadow-inner border border-slate-200/50">
                  <div
                    class="h-full bg-indigo-500 transition-all duration-300"
                    :style="{ width: `${mutualFetchProgress.total ? (mutualFetchProgress.current / mutualFetchProgress.total) * 100 : 0}%` }"
                  />
                </div>
              </div>
              <button 
                v-else-if="!mutualGraphReady"
                class="px-5 py-2.5 bg-indigo-600 hover:bg-indigo-700 text-white text-sm font-bold rounded-xl shadow-sm shadow-indigo-500/30 transition-all active:scale-95 flex items-center gap-2"
                @click="fetchMutualFriends"
              >
                <Network :size="18" />
                {{ t('charts.generate_topology') }}
              </button>
              <button 
                v-else
                class="px-5 py-2.5 bg-white border border-slate-200 text-slate-600 hover:text-indigo-600 hover:border-indigo-300 text-sm font-bold rounded-xl shadow-sm transition-all flex items-center gap-2"
                @click="fetchMutualFriends"
              >
                {{ t('charts.regenerate') }}
              </button>
            </div>
          </div>

          <div
            v-if="!mutualGraphReady && !isFetchingMutuals"
            class="flex-1 flex flex-col items-center justify-center text-slate-400 mt-2 border-2 border-dashed border-slate-200 rounded-2xl bg-white/50"
          >
            <Network
              :size="64"
              class="mb-4 opacity-30 text-slate-400"
            />
            <p class="font-bold text-xl text-slate-500">
              {{ t('charts.no_topology_data') }}
            </p>
            <p class="text-sm mt-2 text-center max-w-md font-medium">
              {{ t('charts.topology_help') }}
            </p>
          </div>
          
          <div
            v-else-if="isFetchingMutuals && mutualGraphNodes.length === 0"
            class="flex-1 flex flex-col items-center justify-center text-indigo-600 mt-2 border-2 border-dashed border-indigo-200 rounded-2xl bg-indigo-50/50"
          >
            <div class="w-12 h-12 border-4 border-indigo-200 border-t-indigo-600 rounded-full animate-spin mb-6" />
            <p class="font-extrabold text-xl tracking-wide animate-pulse">
              {{ t('charts.traversing_network') }}
            </p>
            <p class="text-sm text-indigo-400 mt-2 font-bold">
              {{ t('charts.pulling_data') }}
            </p>
          </div>

          <div
            v-show="mutualGraphReady"
            ref="networkChartRef"
            class="flex-1 w-full rounded-2xl bg-white mt-2 border border-slate-100 shadow-inner overflow-hidden relative"
          >
            <div
              v-if="isFetchingMutuals"
              class="absolute top-4 right-4 z-10 bg-white/90 backdrop-blur px-4 py-2 rounded-xl text-xs font-bold text-indigo-600 shadow-sm border border-indigo-100 flex items-center gap-2"
            >
              <div class="w-2.5 h-2.5 bg-indigo-500 rounded-full animate-ping" />
              {{ t('charts.updating_realtime') }}
            </div>
          </div>
        </div>

        <!-- 热门世界排行榜面板 -->
        <div
          v-show="currentTab === 'worlds'"
          class="space-y-4"
        >
          <div class="bg-white/70 backdrop-blur-xl rounded-3xl border border-white shadow-lg shadow-slate-200/40 p-6">
            <h2 class="font-extrabold text-slate-900 mb-6 flex items-center gap-3 text-lg">
              <span class="p-1.5 bg-indigo-50 rounded-lg text-indigo-600"><Globe2 :size="20" /></span>
              {{ t('charts.top_worlds_title') }}
            </h2>
            
            <div
              v-if="topWorlds.length === 0"
              class="flex flex-col items-center justify-center py-20 text-slate-400"
            >
              <Trophy
                :size="64"
                class="mb-4 opacity-30"
              />
              <p class="font-bold text-xl">
                {{ t('charts.no_data') }}
              </p>
              <p class="text-sm mt-2 font-medium">
                {{ t('charts.no_data_desc') }}
              </p>
            </div>

            <div
              v-else
              class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-5"
            >
              <div
                v-for="(world, index) in topWorlds"
                :key="world.id"
                class="flex items-center gap-4 p-4 rounded-2xl border border-slate-200 hover:border-indigo-300 bg-white hover:shadow-md transition-all group"
              >
                <div class="w-8 flex justify-center">
                  <span
                    class="text-2xl font-black drop-shadow-sm"
                    :class="index === 0 ? 'text-indigo-400' : index === 1 ? 'text-slate-300' : index === 2 ? 'text-orange-400' : 'text-slate-200'"
                  >
                    #{{ index + 1 }}
                  </span>
                </div>
                <div class="w-16 h-16 rounded-xl bg-slate-100 overflow-hidden flex-shrink-0 relative shadow-inner">
                  <img
                    v-if="world.thumbnail"
                    :src="world.thumbnail"
                    class="w-full h-full object-cover"
                    referrerpolicy="no-referrer"
                  >
                  <Globe2
                    v-else
                    class="w-full h-full p-4 text-slate-300"
                  />
                </div>
                <div class="flex-1 min-w-0">
                  <h3 class="font-bold text-slate-900 truncate group-hover:text-indigo-600 transition-colors text-base">
                    {{ world.name }}
                  </h3>
                  <div class="flex items-center gap-1.5 mt-1.5 text-xs text-slate-500 font-bold">
                    <Users
                      :size="14"
                      class="text-indigo-400"
                    /> {{ world.count }} {{ t('charts.friends_in_world') }}
                  </div>
                </div>
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
.custom-scrollbar::-webkit-scrollbar-thumb { background: #cbd5e1; border-radius: 10px; }
.custom-scrollbar::-webkit-scrollbar-thumb:hover { background: #94a3b8; }
</style>
