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
  return ['一', '二', '三', '四', '五', '六', '日'];
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
  <div class="h-full flex flex-col bg-gray-50/50 p-2">
    <!-- 顶部导航 Tab -->
    <div class="flex items-center gap-6 border-b border-gray-200 mb-4 px-2">
      <button
        :class="currentTab === 'overview' ? 'border-b-2 border-violet-600 text-violet-800 font-bold' : 'text-gray-500 hover:text-gray-700 font-medium'"
        class="py-3 px-1 transition-colors text-sm flex items-center gap-2"
        @click="currentTab = 'overview'"
      >
        <LayoutDashboard :size="16" /> {{ t('charts.overview') }}
      </button>
      <button
        :class="currentTab === 'network' ? 'border-b-2 border-violet-600 text-violet-800 font-bold' : 'text-gray-500 hover:text-gray-700 font-medium'"
        class="py-3 px-1 transition-colors text-sm flex items-center gap-2"
        @click="currentTab = 'network'"
      >
        <Network :size="16" /> {{ t('charts.network') }}
      </button>
      <button
        :class="currentTab === 'worlds' ? 'border-b-2 border-violet-600 text-violet-800 font-bold' : 'text-gray-500 hover:text-gray-700 font-medium'"
        class="py-3 px-1 transition-colors text-sm flex items-center gap-2"
        @click="currentTab = 'worlds'"
      >
        <Trophy :size="16" /> {{ t('charts.top_worlds') }}
      </button>
    </div>

    <div class="flex-1 overflow-y-auto px-2 custom-scrollbar">
      <div
        v-if="loading"
        class="flex flex-col items-center justify-center py-20 text-violet-600/70"
      >
        <div class="w-8 h-8 border-4 border-violet-200 border-t-violet-600 rounded-full animate-spin mb-4"></div>
        <span class="font-bold text-lg">{{ t('charts.analyzing') }}</span>
      </div>

      <template v-else>
        <!-- 概览面板 -->
        <div v-show="currentTab === 'overview'" class="space-y-6">
          <!-- 统计卡片 -->
          <div class="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-6 gap-4">
            <div class="bg-white rounded-xl p-5 border border-violet-100 shadow-sm text-center transform transition-all hover:scale-105 hover:shadow-md">
              <Users class="mx-auto mb-2 text-violet-500" :size="24" />
              <div class="text-3xl font-black text-violet-800 mb-1">{{ friendStats.total }}</div>
              <div class="text-xs text-violet-600 font-bold">{{ t('charts.total_friends') }}</div>
            </div>
            <div class="bg-white rounded-xl p-5 border border-green-100 shadow-sm text-center transform transition-all hover:scale-105 hover:shadow-md">
              <Activity class="mx-auto mb-2 text-green-500" :size="24" />
              <div class="text-3xl font-black text-green-700 mb-1">{{ friendStats.online }}</div>
              <div class="text-xs text-green-600 font-bold">{{ t('charts.online') }} ({{ onlinePercent }}%)</div>
            </div>
            <div class="bg-white rounded-xl p-5 border border-blue-100 shadow-sm text-center transform transition-all hover:scale-105 hover:shadow-md">
              <div class="w-6 h-6 rounded-full bg-blue-500 mx-auto mb-2 shadow-inner" />
              <div class="text-3xl font-black text-blue-700 mb-1">{{ friendStats.joinMe }}</div>
              <div class="text-xs text-blue-600 font-bold">{{ t('charts.join_me') }}</div>
            </div>
            <div class="bg-white rounded-xl p-5 border border-orange-100 shadow-sm text-center transform transition-all hover:scale-105 hover:shadow-md">
              <div class="w-6 h-6 rounded-full bg-orange-500 mx-auto mb-2 shadow-inner" />
              <div class="text-3xl font-black text-orange-700 mb-1">{{ friendStats.askMe }}</div>
              <div class="text-xs text-orange-600 font-bold">{{ t('charts.ask_me') }}</div>
            </div>
            <div class="bg-white rounded-xl p-5 border border-red-100 shadow-sm text-center transform transition-all hover:scale-105 hover:shadow-md">
              <div class="w-6 h-6 rounded-full bg-red-500 mx-auto mb-2 shadow-inner" />
              <div class="text-3xl font-black text-red-700 mb-1">{{ friendStats.busy }}</div>
              <div class="text-xs text-red-600 font-bold">{{ t('charts.busy') }}</div>
            </div>
            <div class="bg-white rounded-xl p-5 border border-gray-200 shadow-sm text-center transform transition-all hover:scale-105 hover:shadow-md">
              <div class="w-6 h-6 rounded-full bg-gray-400 mx-auto mb-2 shadow-inner" />
              <div class="text-3xl font-black text-gray-600 mb-1">{{ friendStats.offline }}</div>
              <div class="text-xs text-gray-500 font-bold">{{ t('charts.offline') }}</div>
            </div>
          </div>

          <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
            <!-- 周活跃图表 -->
            <div class="bg-white rounded-xl p-6 border border-gray-100 shadow-sm">
              <h2 class="font-extrabold text-gray-900 mb-6 flex items-center gap-2 text-lg">
                <TrendingUp class="text-violet-500" :size="20" /> {{ t('charts.weekly_trend') }}
              </h2>
              <div class="flex items-end justify-between gap-3 h-48 px-2">
                <div v-for="(count, idx) in weeklyActivity" :key="idx" class="flex-1 flex flex-col items-center gap-2 group">
                  <span class="text-xs font-bold text-violet-700 opacity-0 group-hover:opacity-100 transition-opacity">{{ count }}</span>
                  <div
                    class="w-full max-w-[40px] rounded-t-xl transition-all duration-700 bg-gradient-to-t from-violet-600 to-violet-400 group-hover:from-violet-500 group-hover:to-violet-300 shadow-md"
                    :style="{ height: (count / maxWeekly * 100) + '%', minHeight: count > 0 ? '12px' : '4px' }"
                  />
                  <span class="text-xs text-gray-600 font-bold mt-2">{{ dayLabels[idx] }}</span>
                </div>
              </div>
            </div>

            <!-- 最近事件摘要 -->
            <div class="bg-white rounded-xl p-6 border border-gray-100 shadow-sm">
              <h3 class="font-extrabold text-gray-900 mb-4 flex items-center gap-2 text-lg">
                <Clock class="text-violet-500" :size="20" /> {{ t('charts.recent_events') }}
              </h3>
              <div v-if="recentLogs.length === 0" class="text-sm text-gray-400 text-center py-10 font-medium">
                {{ t('charts.no_events') }}
              </div>
              <div v-else class="space-y-3 max-h-56 overflow-y-auto custom-scrollbar pr-2">
                <div v-for="log in recentLogs.slice(0, 10)" :key="log.id" class="flex items-center gap-3 p-2 hover:bg-gray-50 rounded-lg transition-colors">
                  <span
                    class="w-2.5 h-2.5 rounded-full flex-shrink-0 shadow-sm"
                    :class="log.event_type === 'online' ? 'bg-green-500' : log.event_type === 'offline' ? 'bg-gray-400' : 'bg-violet-500'"
                  />
                  <span class="text-gray-800 font-bold truncate flex-1 text-sm">{{ log.display_name || t('charts.system') }}</span>
                  <span class="text-gray-400 text-xs font-medium bg-gray-100 px-2 py-1 rounded-md">{{ log.created_at?.slice(11, 16) }}</span>
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- 关系网拓扑图面板 (共同好友) -->
        <div v-show="currentTab === 'network'" class="h-full min-h-[500px] bg-white rounded-xl border border-gray-100 shadow-sm p-4 flex flex-col">
          <div class="mb-2 flex justify-between items-center">
            <div>
              <h2 class="font-extrabold text-gray-900 flex items-center gap-2 text-lg">
                <Network class="text-violet-500" :size="20" /> 共同好友拓扑图
              </h2>
              <p class="text-xs text-gray-500 mt-1">展示您与好友之间的相互联系与社区聚类</p>
            </div>
            
            <div class="flex items-center gap-4">
              <div v-if="isFetchingMutuals" class="flex flex-col items-end">
                <span class="text-xs font-bold text-violet-600 mb-1">
                  正在扫描: {{ mutualFetchProgress.current }} / {{ mutualFetchProgress.total }}
                </span>
                <div class="w-32 h-2 bg-gray-100 rounded-full overflow-hidden shadow-inner">
                  <div class="h-full bg-violet-500 transition-all duration-300" :style="{ width: `${mutualFetchProgress.total ? (mutualFetchProgress.current / mutualFetchProgress.total) * 100 : 0}%` }"></div>
                </div>
              </div>
              <button 
                v-else-if="!mutualGraphReady"
                @click="fetchMutualFriends"
                class="px-4 py-2 bg-violet-600 hover:bg-violet-700 text-white text-sm font-bold rounded-lg shadow-sm transition-colors flex items-center gap-2"
              >
                <Network :size="16" />
                开始生成拓扑图
              </button>
              <button 
                v-else
                @click="fetchMutualFriends"
                class="px-4 py-2 bg-white border border-violet-200 text-violet-600 hover:bg-violet-50 text-sm font-bold rounded-lg shadow-sm transition-colors flex items-center gap-2"
              >
                重新生成
              </button>
            </div>
          </div>

          <div v-if="!mutualGraphReady && !isFetchingMutuals" class="flex-1 flex flex-col items-center justify-center text-gray-400 mt-4 border-2 border-dashed border-gray-100 rounded-xl">
            <Network :size="48" class="mb-4 opacity-50 text-gray-300" />
            <p class="font-bold text-lg text-gray-600">无拓扑图数据</p>
            <p class="text-sm mt-2 text-center max-w-sm">点击右上角按钮开始扫描。由于需要获取所有好友的共同好友列表，这可能需要一定时间，具体取决于您的好友数量。</p>
          </div>
          
          <div v-else-if="isFetchingMutuals && mutualGraphNodes.length === 0" class="flex-1 flex flex-col items-center justify-center text-violet-500 mt-4 border-2 border-dashed border-violet-100 rounded-xl bg-violet-50/30">
            <div class="w-10 h-10 border-4 border-violet-300 border-t-violet-600 rounded-full animate-spin mb-4"></div>
            <p class="font-bold text-lg animate-pulse">正在深度遍历关系网...</p>
            <p class="text-xs text-violet-400 mt-2">请耐心等待，正在拉取 VRChat 服务器数据</p>
          </div>

          <div v-show="mutualGraphReady" ref="networkChartRef" class="flex-1 w-full rounded-lg bg-gray-50 mt-2 border border-gray-100 shadow-inner overflow-hidden relative">
            <div v-if="isFetchingMutuals" class="absolute top-2 right-2 z-10 bg-white/80 backdrop-blur px-3 py-1.5 rounded-full text-xs font-bold text-violet-600 shadow-sm border border-violet-100 flex items-center gap-2">
              <div class="w-2 h-2 bg-violet-500 rounded-full animate-ping"></div>
              图表数据实时更新中...
            </div>
          </div>
        </div>

        <!-- 热门世界排行榜面板 -->
        <div v-show="currentTab === 'worlds'" class="space-y-4">
          <div class="bg-white rounded-xl border border-gray-100 shadow-sm p-6">
            <h2 class="font-extrabold text-gray-900 mb-6 flex items-center gap-2 text-lg">
              <Globe2 class="text-violet-500" :size="20" /> {{ t('charts.top_worlds_title') }}
            </h2>
            
            <div v-if="topWorlds.length === 0" class="text-center py-20 text-gray-400">
              <Trophy :size="48" class="mx-auto mb-4 opacity-50 text-gray-300" />
              <p class="font-bold">{{ t('charts.no_data') }}</p>
              <p class="text-sm mt-1">{{ t('charts.no_data_desc') }}</p>
            </div>

            <div v-else class="grid grid-cols-1 md:grid-cols-2 gap-4">
              <div v-for="(world, index) in topWorlds" :key="world.id" class="flex items-center gap-4 p-3 rounded-xl border border-gray-50 hover:border-violet-200 bg-gray-50/50 hover:bg-violet-50/30 transition-colors group">
                <div class="w-8 flex justify-center">
                  <span class="text-xl font-black" :class="index === 0 ? 'text-yellow-500' : index === 1 ? 'text-gray-400' : index === 2 ? 'text-amber-700' : 'text-gray-300'">
                    #{{ index + 1 }}
                  </span>
                </div>
                <div class="w-14 h-14 rounded-lg bg-gray-200 overflow-hidden flex-shrink-0 relative">
                  <img v-if="world.thumbnail" :src="world.thumbnail" class="w-full h-full object-cover" referrerpolicy="no-referrer" />
                  <Globe2 v-else class="w-full h-full p-3 text-gray-400" />
                </div>
                <div class="flex-1 min-w-0">
                  <h3 class="font-bold text-gray-900 truncate group-hover:text-violet-700 transition-colors">{{ world.name }}</h3>
                  <div class="flex items-center gap-1 mt-1 text-xs text-gray-500 font-medium">
                    <Users :size="12" /> {{ world.count }} {{ t('charts.friends_in_world') }}
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
