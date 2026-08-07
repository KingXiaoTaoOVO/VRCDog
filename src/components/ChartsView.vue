<script setup lang="ts">
import { ref, onMounted, computed, nextTick, watch, onUnmounted } from 'vue';
import { VrcApi, DbApi } from '../api';
import { TrendingUp, Users, Clock, Globe2, Network, Trophy, LayoutDashboard, ChevronRight, ArrowUpRight, Activity } from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';
import type { VrcUser, FriendLog } from '../types/vrc';
import * as echarts from 'echarts';
import { useUserProfileStore } from '../stores/userProfile';
import { useFriendsStore } from '../stores/friendsStore';
import { useEntityModalStore } from '../stores/entityModal';
import { markDataHealthy } from '../stores/dataHealth';

const { t, locale } = useI18n();
const profileStore = useUserProfileStore();
const friendsStore = useFriendsStore();
const entityStore = useEntityModalStore();

const currentTab = ref<'overview' | 'network' | 'worlds'>('overview');
const loading = ref(true);

// 概览数据
const friendStats = ref({ total: 0, online: 0, joinMe: 0, busy: 0, askMe: 0, offline: 0 });
const weeklyActivity = ref<number[]>([0,0,0,0,0,0,0]);
const hourlyActivity = ref<number[]>([0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]);
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
    // 1. 好友统计 — 使用共享Store
    await friendsStore.fetchFriends();
    const friends = friendsStore.allFriends as VrcUser[];
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

    // 3. 周活跃 + 24小时活跃数据
    const heatmap = await DbApi.getHeatmap();
    const days = [0,0,0,0,0,0,0];
    const hours = [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0];
    if (Array.isArray(heatmap)) {
      heatmap.forEach((cell: { day: number; hour: number; count: number }) => {
        if (cell.day >= 0 && cell.day < 7) days[cell.day] += cell.count;
        if (cell.hour >= 0 && cell.hour < 24) hours[cell.hour] += cell.count;
      });
    }
    weeklyActivity.value = days;
    hourlyActivity.value = hours;

    // 4. 最近好友日志
    try {
      const logs = await DbApi.getFriendLogs({ limit: 20 });
      recentLogs.value = logs;
    } catch { recentLogs.value = []; }

    // 数据已成功加载（好友 + 热力图 + 日志），标记数据服务健康
    markDataHealthy();

  } catch (err) {
    console.warn(t('auto_6900bfa2'), err);
  } finally {
    loading.value = false;
  }
};

onMounted(() => fetchAll());

const peakWeekly = computed(() => Math.max(0, ...weeklyActivity.value));
const maxWeekly = computed(() => Math.max(1, peakWeekly.value));
const weeklyTotal = computed(() => weeklyActivity.value.reduce((sum, count) => sum + count, 0));
const peakDayIndex = computed(() => weeklyActivity.value.indexOf(peakWeekly.value));
const peakDayLabel = computed(() => dayLabels.value[peakDayIndex.value] || dayLabels.value[0]);

const peakHourly = computed(() => Math.max(0, ...hourlyActivity.value));
const maxHourly = computed(() => Math.max(1, peakHourly.value));
const hourlyTotal = computed(() => hourlyActivity.value.reduce((sum, count) => sum + count, 0));
const peakHourIndex = computed(() => hourlyActivity.value.indexOf(peakHourly.value));
const peakHourLabel = computed(() => {
  const h = peakHourIndex.value;
  if (h < 0) return '--';
  return `${String(h).padStart(2, '0')}:00`;
});

// 好友状态分段（用于分布条形图），按数量降序
const statusBreakdown = computed(() => {
  const s = friendStats.value;
  const total = Math.max(1, s.total);
  const order: Array<{ key: string; label: string; count: number; cls: string }> = [
    { key: 'online', label: t('charts.online'), count: s.online, cls: 'st-online' },
    { key: 'join_me', label: t('charts.join_me'), count: s.joinMe, cls: 'st-join' },
    { key: 'busy', label: t('charts.busy'), count: s.busy, cls: 'st-busy' },
    { key: 'ask_me', label: t('charts.ask_me'), count: s.askMe, cls: 'st-ask' },
    { key: 'offline', label: t('charts.offline'), count: s.offline, cls: 'st-offline' },
  ];
  return order
    .map((item) => ({ ...item, percent: Math.round((item.count / total) * 100) }))
    .sort((a, b) => b.count - a.count);
});

const openWorldDetail = (worldId: string) => {
  void entityStore.openWorld(worldId);
};

const openRecentUser = (log: FriendLog) => {
  if (!log.user_id) return;
  const friend = allFriends.value.find((item) => item.id === log.user_id);
  profileStore.openProfile(log.user_id, friend || {
    id: log.user_id,
    displayName: log.display_name,
    status: 'offline',
    isFriend: true,
  } as any);
};

const eventLabel = (eventType: string) => {
  if (eventType === 'online') return t('status.online');
  if (eventType === 'offline') return t('status.offline');
  return t('charts.events');
};

const formatEventTime = (value: string) => {
  if (!value) return '--:--';
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value.slice(11, 16);
  return date.toLocaleTimeString(locale.value, { hour: '2-digit', minute: '2-digit', hour12: false });
};

// --- 共同好友拓扑图逻辑 (参考 VrcDog) ---
const cssVar = (name: string, fallback: string) => {
  if (typeof window === 'undefined') return fallback;
  return getComputedStyle(document.documentElement).getPropertyValue(name).trim() || fallback;
};

const toRgba = (color: string, alpha: number) => {
  if (color.startsWith('#')) {
    const hex = color.slice(1);
    const full = hex.length === 3 ? hex.split('').map((c) => c + c).join('') : hex;
    const num = Number.parseInt(full, 16);
    if (!Number.isNaN(num)) {
      return `rgba(${(num >> 16) & 255}, ${(num >> 8) & 255}, ${num & 255}, ${alpha})`;
    }
  }

  const rgbMatch = color.match(/rgba?\(([^)]+)\)/);
  if (rgbMatch) {
    const [r, g, b] = rgbMatch[1].split(',').map((part) => part.trim());
    return `rgba(${r}, ${g}, ${b}, ${alpha})`;
  }

  return color;
};

const themeColors = () => {
  const primary = cssVar('--theme-primary', '#d97706');
  const primaryHover = cssVar('--theme-primary-hover', '#b45309');
  const text = cssVar('--theme-text-strong', '#451a03');
  const muted = cssVar('--theme-text-muted', '#a1887f');
  const surface = cssVar('--theme-surface', 'rgba(255,255,255,0.72)');
  const surfaceHover = cssVar('--theme-surface-hover', 'rgba(255,255,255,0.82)');
  const border = cssVar('--theme-border-soft', 'rgba(120,53,15,0.12)');

  return {
    primary,
    primaryHover,
    text,
    muted,
    surface,
    surfaceHover,
    border,
    primarySoft: toRgba(primary, 0.18),
    primaryFaint: toRgba(primary, 0.08),
  };
};

const graphPalette = () => {
  const c = themeColors();
  return [
    c.primary,
    c.primaryHover,
    '#10b981',
    '#f59e0b',
    '#ef4444',
    '#14b8a6',
    '#8b5cf6',
    '#64748b',
  ];
};

const eventDotClass = (type: string) => {
  if (type === 'online') return 'bg-emerald-500 shadow-emerald-500/30';
  if (type === 'offline') return 'bg-border-strong shadow-slate-400/20';
  return 'bg-primary shadow-primary/30';
};

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
      console.warn(t('auto_b0a0215c'), friend.displayName);
    }
    mutualFetchProgress.value.current = i + 1;
    // 速率限制，VrcDog 设置的是每秒5次，我们保守一点，每次请求间隔 250ms
    await new Promise(r => setTimeout(r, 250));
  }
  
  // 简单分配颜色 (用节点 ID hash 或者 连通度分类)
  mutualGraphNodes.value = Array.from(nodesMap.values()).map((n, idx) => {
    const degree = nodeDegree.get(n.id) || 0;
    return {
      id: n.id,
      name: n.name,
      symbolSize: Math.max(8, Math.min(40, 8 + degree * 0.8)),
      category: degree % graphPalette().length
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
    chartInstance = echarts.init(networkChartRef.value, undefined, { renderer: 'canvas' });
    chartInstance.on('click', (params: any) => {
      if (params?.dataType !== 'node' || !params?.data?.id) return;
      const prefill = allFriends.value.find((friend) => friend.id === params.data.id);
      profileStore.openProfile(params.data.id, prefill || {
        id: params.data.id,
        displayName: params.data.name,
        status: 'offline',
        isFriend: true,
      } as any);
    });
  }

  // 渲染 VrcDog 风格的相互图
  const c = themeColors();
  const palette = graphPalette();
  const graphNodes = mutualGraphNodes.value.map((node) => ({
    ...node,
    itemStyle: {
      color: palette[node.category % palette.length],
      borderColor: c.surfaceHover,
      borderWidth: 1,
      shadowBlur: 12,
      shadowColor: toRgba(palette[node.category % palette.length], 0.28),
    },
  }));

  const option = {
    backgroundColor: 'transparent',
    color: palette,
    tooltip: {
      formatter: '{b}',
      backgroundColor: c.surfaceHover,
      borderColor: c.border,
      textStyle: { color: c.text, fontWeight: 700 },
      extraCssText: 'backdrop-filter: blur(16px); border-radius: 10px; box-shadow: 0 12px 30px rgba(15,23,42,0.10);'
    },
    series: [
      {
        type: 'graph',
        layout: 'force',
        data: graphNodes,
        links: mutualGraphLinks.value,
        roam: true,
        draggable: true,
        focusNodeAdjacency: true,
        label: {
          show: true,
          position: 'right',
          formatter: '{b}',
          fontSize: 10,
          color: c.text,
          fontWeight: 700,
          backgroundColor: toRgba(c.surfaceHover, 0.75),
          borderColor: c.border,
          borderWidth: 1,
          borderRadius: 5,
          padding: [2, 5],
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
          opacity: 0.28,
          width: 1.2
        },
        emphasis: {
          focus: 'adjacency',
          lineStyle: {
            opacity: 0.7,
            width: 2,
          }
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
  <section class="charts-view">
    <nav class="charts-tabs" :aria-label="t('sidebar.charts')">
      <button :class="{ active: currentTab === 'overview' }" @click="currentTab = 'overview'">
        <LayoutDashboard :size="17" />
        <span>{{ t('charts.overview') }}</span>
      </button>
      <button :class="{ active: currentTab === 'network' }" @click="currentTab = 'network'">
        <Network :size="17" />
        <span>{{ t('charts.network') }}</span>
      </button>
      <button :class="{ active: currentTab === 'worlds' }" @click="currentTab = 'worlds'">
        <Trophy :size="17" />
        <span>{{ t('charts.top_worlds') }}</span>
      </button>
    </nav>

    <div class="charts-scroll custom-scrollbar">
      <div v-if="loading" class="loading-layout" aria-live="polite">
        <div class="loading-bars" aria-hidden="true">
          <span v-for="height in [38, 68, 48, 82, 56, 73, 44]" :key="height" :style="{ height: `${height}%` }" />
        </div>
        <p>{{ t('charts.analyzing') }}</p>
      </div>

      <template v-else>
        <div v-show="currentTab === 'overview'" class="overview-stack">
          <div class="overview-layout">
          <article class="analytics-panel trend-panel">
            <header class="panel-heading">
              <div class="heading-copy">
                <span class="heading-icon"><TrendingUp :size="19" /></span>
                <div>
                  <h2>{{ t('charts.weekly_trend') }}</h2>
                  <p>{{ weeklyTotal }} {{ t('charts.events') }}</p>
                </div>
              </div>
              <div class="peak-summary">
                <Activity :size="15" />
                <span>{{ peakDayLabel }}</span>
                <strong>{{ peakWeekly }}</strong>
              </div>
            </header>

            <div class="chart-stage">
              <div class="chart-grid" aria-hidden="true">
                <span v-for="line in 4" :key="line" />
              </div>
              <div class="bar-grid">
                <div v-for="(count, idx) in weeklyActivity" :key="idx" class="bar-column">
                  <strong class="bar-value">{{ count }}</strong>
                  <div class="bar-track">
                    <div
                      class="bar-fill"
                      :class="{ peak: count === maxWeekly && count > 0, empty: count === 0 }"
                      :style="{ height: count === 0 ? '4px' : `${Math.max((count / maxWeekly) * 100, 10)}%` }"
                    />
                  </div>
                  <span class="day-label">{{ dayLabels[idx] }}</span>
                </div>
              </div>
            </div>
          </article>

          <aside class="analytics-panel events-panel">
            <header class="panel-heading compact">
              <div class="heading-copy">
                <span class="heading-icon"><Clock :size="19" /></span>
                <div>
                  <h2>{{ t('charts.recent_events') }}</h2>
                  <p>{{ recentLogs.length }} {{ t('charts.events') }}</p>
                </div>
              </div>
            </header>

            <div v-if="recentLogs.length === 0" class="panel-empty">
              <Clock :size="34" />
              <span>{{ t('charts.no_events') }}</span>
            </div>
            <div v-else class="event-list custom-scrollbar">
              <button
                v-for="log in recentLogs.slice(0, 12)"
                :key="log.id"
                class="event-row"
                :disabled="!log.user_id"
                @click="openRecentUser(log)"
              >
                <span class="event-avatar">{{ (log.display_name || t('charts.system')).trim().charAt(0).toUpperCase() }}</span>
                <span class="event-copy">
                  <strong>{{ log.display_name || t('charts.system') }}</strong>
                  <small><i :class="eventDotClass(log.event_type)" />{{ eventLabel(log.event_type) }}</small>
                </span>
                <time>{{ formatEventTime(log.created_at) }}</time>
                <ChevronRight :size="16" aria-hidden="true" />
              </button>
            </div>
          </aside>
          </div>

          <div class="insights-layout">
            <article class="analytics-panel insights-panel">
              <header class="panel-heading">
                <div class="heading-copy">
                  <span class="heading-icon"><Users :size="19" /></span>
                  <div>
                    <h2>{{ t('charts.friend_status_dist') }}</h2>
                    <p>{{ friendStats.total }} {{ t('charts.friends') }}</p>
                  </div>
                </div>
              </header>
              <div class="status-bars">
                <div v-for="item in statusBreakdown" :key="item.key" class="status-row">
                  <span class="status-dot" :class="item.cls" />
                  <span class="status-name">{{ item.label }}</span>
                  <div class="status-track">
                    <div class="status-fill" :class="item.cls" :style="{ width: `${Math.max(item.percent, item.count > 0 ? 4 : 0)}%` }" />
                  </div>
                  <strong class="status-count">{{ item.count }}</strong>
                  <span class="status-pct">{{ item.percent }}%</span>
                </div>
              </div>
            </article>

            <article class="analytics-panel insights-panel">
              <header class="panel-heading">
                <div class="heading-copy">
                  <span class="heading-icon"><Activity :size="19" /></span>
                  <div>
                    <h2>{{ t('charts.hourly_activity') }}</h2>
                    <p>{{ hourlyTotal }} {{ t('charts.events') }}</p>
                  </div>
                </div>
                <div v-if="peakHourly > 0" class="peak-summary">
                  <Clock :size="15" />
                  <span>{{ t('charts.peak_hour') }}</span>
                  <strong>{{ peakHourLabel }}</strong>
                </div>
              </header>
              <div v-if="hourlyTotal === 0" class="panel-empty">
                <Clock :size="34" />
                <span>{{ t('charts.no_hourly_data') }}</span>
              </div>
              <div v-else class="hourly-chart">
                <div
                  v-for="(count, hour) in hourlyActivity"
                  :key="hour"
                  class="hour-bar"
                  :class="{ peak: count === peakHourly && count > 0 }"
                  :title="`${String(hour).padStart(2, '0')}:00 · ${count}`"
                >
                  <div
                    class="hour-fill"
                    :style="{ height: count === 0 ? '3px' : `${Math.max((count / maxHourly) * 100, 8)}%` }"
                  />
                </div>
              </div>
              <div v-if="hourlyTotal > 0" class="hour-axis">
                <span v-for="(label, idx) in t('charts.hours_short')" :key="idx">{{ label }}</span>
              </div>
            </article>
          </div>
        </div>

        <article v-show="currentTab === 'network'" class="analytics-panel network-panel">
          <header class="panel-heading network-heading">
            <div class="heading-copy">
              <span class="heading-icon"><Network :size="19" /></span>
              <div>
                <h2>{{ t('charts.mutual_topology') }}</h2>
                <p>{{ t('charts.mutual_desc') }}</p>
              </div>
            </div>
            <div class="network-actions">
              <div v-if="isFetchingMutuals" class="scan-progress">
                <span>{{ t('charts.scanning') }} {{ mutualFetchProgress.current }} / {{ mutualFetchProgress.total }}</span>
                <div><i :style="{ width: `${mutualFetchProgress.total ? (mutualFetchProgress.current / mutualFetchProgress.total) * 100 : 0}%` }" /></div>
              </div>
              <button v-else class="network-button" @click="fetchMutualFriends">
                <Network :size="17" />
                {{ mutualGraphReady ? t('charts.regenerate') : t('charts.generate_topology') }}
              </button>
            </div>
          </header>
          <div v-if="!mutualGraphReady && !isFetchingMutuals" class="network-empty">
            <Network :size="52" />
            <strong>{{ t('charts.no_topology_data') }}</strong>
            <p>{{ t('charts.topology_help') }}</p>
          </div>
          <div v-else-if="isFetchingMutuals && mutualGraphNodes.length === 0" class="network-empty active">
            <span class="network-loader" />
            <strong>{{ t('charts.traversing_network') }}</strong>
            <p>{{ t('charts.pulling_data') }}</p>
          </div>
          <div v-show="mutualGraphReady" ref="networkChartRef" class="network-canvas">
            <span v-if="isFetchingMutuals" class="live-update">{{ t('charts.updating_realtime') }}</span>
          </div>
        </article>

        <article v-show="currentTab === 'worlds'" class="analytics-panel worlds-panel">
          <header class="panel-heading worlds-heading">
            <div class="heading-copy">
              <span class="heading-icon"><Globe2 :size="19" /></span>
              <div>
                <h2>{{ t('charts.top_worlds_title') }}</h2>
                <p>{{ topWorlds.length }} {{ t('charts.events') }}</p>
              </div>
            </div>
          </header>

          <div v-if="topWorlds.length === 0" class="panel-empty worlds-empty">
            <Trophy :size="44" />
            <strong>{{ t('charts.no_data') }}</strong>
            <span>{{ t('charts.no_data_desc') }}</span>
          </div>

          <div v-else class="worlds-layout">
            <button class="featured-world" @click="openWorldDetail(topWorlds[0].id)">
              <div class="featured-media">
                <img v-if="topWorlds[0].thumbnail" :src="topWorlds[0].thumbnail" :alt="topWorlds[0].name" referrerpolicy="no-referrer">
                <Globe2 v-else :size="54" />
                <span>#1</span>
              </div>
              <div class="featured-copy">
                <h3>{{ topWorlds[0].name }}</h3>
                <p><Users :size="15" /> {{ topWorlds[0].count }} {{ t('charts.friends_in_world') }}</p>
                <span class="detail-link"><ArrowUpRight :size="15" /> {{ t('charts.top_worlds') }}</span>
              </div>
            </button>

            <div class="world-list">
              <button
                v-for="(world, index) in topWorlds.slice(1)"
                :key="world.id"
                class="world-row"
                @click="openWorldDetail(world.id)"
              >
                <span class="world-rank">#{{ index + 2 }}</span>
                <span class="world-thumb">
                  <img v-if="world.thumbnail" :src="world.thumbnail" :alt="world.name" referrerpolicy="no-referrer">
                  <Globe2 v-else :size="22" />
                </span>
                <span class="world-copy">
                  <strong>{{ world.name }}</strong>
                  <small><Users :size="13" /> {{ world.count }} {{ t('charts.friends_in_world') }}</small>
                </span>
                <ChevronRight :size="17" aria-hidden="true" />
              </button>
            </div>
          </div>
        </article>
      </template>
    </div>
  </section>
</template>

<style scoped>
.charts-view {
  height: 100%;
  min-height: 0;
  display: flex;
  flex-direction: column;
  padding: 18px;
  gap: 14px;
  overflow: hidden;
  color: var(--theme-text-strong);
  background: color-mix(in srgb, var(--theme-bg-main) 76%, var(--theme-surface-hover));
}

.charts-tabs {
  display: inline-grid;
  grid-auto-flow: column;
  align-self: flex-start;
  gap: 3px;
  padding: 4px;
  margin-bottom: 4px;
  border-radius: 8px;
  background: color-mix(in srgb, var(--theme-surface) 90%, transparent);
  box-shadow: 0 7px 20px color-mix(in srgb, var(--theme-text-strong) 8%, transparent);
}

.charts-tabs button {
  min-height: 42px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 0 16px;
  border: 0;
  border-radius: 6px;
  color: var(--theme-text-muted);
  background: transparent;
  font: inherit;
  font-size: 14px;
  font-weight: 750;
  cursor: pointer;
  transition: color 180ms ease, background 180ms ease, transform 180ms ease;
}

.charts-tabs button:hover { color: var(--theme-text-strong); background: var(--theme-surface-hover); }
.charts-tabs button:active { transform: translateY(1px); }
.charts-tabs button:focus-visible { outline: 2px solid var(--theme-primary); outline-offset: 2px; }
.charts-tabs button.active { color: white; background: var(--theme-primary); box-shadow: 0 5px 12px color-mix(in srgb, var(--theme-primary) 24%, transparent); }

.charts-scroll { flex: 1; min-height: 0; overflow-y: auto; padding: 0 4px 4px 0; }
.overview-layout { display: grid; grid-template-columns: minmax(0, 1.4fr) minmax(310px, .9fr); gap: 14px; align-items: stretch; }
.analytics-panel { border-radius: 8px; background: color-mix(in srgb, var(--theme-surface) 88%, transparent); box-shadow: 0 9px 28px color-mix(in srgb, var(--theme-text-strong) 7%, transparent); }
.trend-panel, .events-panel, .network-panel, .worlds-panel { padding: 16px; }
.trend-panel, .events-panel { display: flex; flex-direction: column; min-height: 340px; max-height: 420px; }

.panel-heading { min-height: 42px; display: flex; align-items: flex-start; justify-content: space-between; gap: 16px; margin-bottom: 12px; }
.panel-heading.compact { margin-bottom: 8px; }
.heading-copy { min-width: 0; display: flex; align-items: center; gap: 11px; }
.heading-icon { width: 36px; height: 36px; flex: 0 0 36px; display: grid; place-items: center; border-radius: 7px; color: var(--theme-primary); background: color-mix(in srgb, var(--theme-primary) 11%, var(--theme-surface)); }
.heading-copy h2 { margin: 0; color: var(--theme-text-strong); font-size: 17px; line-height: 1.2; font-weight: 800; letter-spacing: 0; }
.heading-copy p { margin: 4px 0 0; color: var(--theme-text-muted); font-size: 12px; line-height: 1.35; font-weight: 650; }
.peak-summary { display: inline-flex; align-items: center; gap: 7px; padding: 7px 9px; border-radius: 6px; color: var(--theme-text-muted); background: var(--theme-surface-hover); font-size: 12px; font-weight: 700; font-variant-numeric: tabular-nums; }
.peak-summary svg, .peak-summary strong { color: var(--theme-primary); }

.chart-stage { position: relative; flex: 1; min-height: 200px; padding: 12px 10px 8px; overflow: hidden; border-radius: 7px; background: color-mix(in srgb, var(--theme-surface-hover) 50%, transparent); }
.chart-grid { position: absolute; inset: 40px 18px 36px; display: flex; flex-direction: column; justify-content: space-between; pointer-events: none; }
.chart-grid span { height: 1px; background: color-mix(in srgb, var(--theme-border-soft) 72%, transparent); }
.bar-grid { position: relative; z-index: 1; height: 100%; display: grid; grid-template-columns: repeat(7, minmax(40px, 1fr)); align-items: stretch; gap: clamp(8px, 2vw, 18px); }
.bar-column { min-width: 0; display: grid; grid-template-rows: 26px minmax(0, 1fr) 22px; justify-items: center; align-items: end; }
.bar-value { align-self: center; color: var(--theme-text-strong); font-size: 12px; font-weight: 800; font-variant-numeric: tabular-nums; }
.bar-track { width: min(50px, 72%); height: 100%; display: flex; align-items: flex-end; border-radius: 6px 6px 3px 3px; background: color-mix(in srgb, var(--theme-border-soft) 26%, transparent); overflow: hidden; }
.bar-fill { width: 100%; min-height: 4px; border-radius: 6px 6px 3px 3px; background: color-mix(in srgb, var(--theme-primary) 88%, white); box-shadow: 0 -5px 15px color-mix(in srgb, var(--theme-primary) 18%, transparent); transition: height 500ms ease, filter 180ms ease; }
.bar-column:hover .bar-fill { filter: brightness(1.06); }
.bar-fill.peak { background: var(--theme-primary); }
.bar-fill.empty { opacity: .35; box-shadow: none; }
.day-label { align-self: center; color: var(--theme-text-muted); font-size: 12px; font-weight: 750; }

/* 概览第二排：状态分布 + 24小时活跃 */
.overview-stack { display: flex; flex-direction: column; gap: 14px; min-height: 100%; }
.insights-layout { display: grid; grid-template-columns: minmax(0, 1fr) minmax(0, 1.15fr); gap: 14px; align-items: stretch; }
.insights-panel { display: flex; flex-direction: column; padding: 16px; min-height: 300px; }

.status-bars { flex: 1; min-height: 0; display: flex; flex-direction: column; justify-content: center; gap: 12px; }
.status-row { display: grid; grid-template-columns: 12px minmax(64px, auto) minmax(0, 1fr) 28px 34px; align-items: center; gap: 10px; }
.status-dot { width: 10px; height: 10px; border-radius: 50%; flex: 0 0 auto; }
.status-name { color: var(--theme-text-strong); font-size: 13px; font-weight: 750; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.status-track { height: 8px; border-radius: 4px; background: color-mix(in srgb, var(--theme-surface-hover) 70%, transparent); overflow: hidden; }
.status-fill { height: 100%; border-radius: 4px; transition: width 500ms ease; }
.status-count { color: var(--theme-text-strong); font-size: 13px; font-weight: 800; font-variant-numeric: tabular-nums; text-align: right; }
.status-pct { color: var(--theme-text-muted); font-size: 11px; font-weight: 700; text-align: right; font-variant-numeric: tabular-nums; }

.st-online { background: #10b981; }
.st-join { background: #3b82f6; }
.st-busy { background: #ef4444; }
.st-ask { background: #f59e0b; }
.st-offline { background: #94a3b8; }

.hourly-chart { position: relative; flex: 1; min-height: 0; display: flex; align-items: flex-end; gap: clamp(2px, 0.6vw, 5px); padding-top: 8px; }
.hour-bar { flex: 1 1 0; min-width: 0; height: 100%; display: flex; align-items: flex-end; border-radius: 3px 3px 0 0; background: color-mix(in srgb, var(--theme-surface-hover) 55%, transparent); overflow: hidden; transition: filter 160ms ease; }
.hour-fill { width: 100%; min-height: 3px; border-radius: 3px 3px 0 0; background: linear-gradient(to top, color-mix(in srgb, var(--theme-primary-hover) 80%, var(--theme-surface)), var(--theme-primary)); transition: height 500ms ease; }
.hour-bar:hover { filter: brightness(1.08); }
.hour-bar.peak .hour-fill { background: linear-gradient(to top, var(--theme-primary-hover), var(--theme-primary)); box-shadow: 0 0 12px color-mix(in srgb, var(--theme-primary) 45%, transparent); }
.hour-axis { display: flex; justify-content: space-between; margin-top: 8px; color: var(--theme-text-muted); font-size: 10px; font-weight: 700; letter-spacing: .02em; }
.hour-axis span { flex: 1; text-align: center; }
.hour-axis span:first-child { text-align: left; }
.hour-axis span:last-child { text-align: right; }

.event-list { flex: 1; min-height: 0; overflow-y: auto; margin: 0 -4px; padding: 0 4px; }
.event-row { width: 100%; min-height: 50px; display: grid; grid-template-columns: 32px minmax(0, 1fr) auto 18px; align-items: center; gap: 10px; padding: 6px 6px; border: 0; border-top: 1px solid color-mix(in srgb, var(--theme-border-soft) 74%, transparent); color: inherit; background: transparent; text-align: left; cursor: pointer; transition: background 160ms ease, transform 160ms ease; }
.event-row:first-child { border-top: 0; }
.event-row:hover { background: color-mix(in srgb, var(--theme-primary) 7%, transparent); }
.event-row:active { transform: translateY(1px); }
.event-row:focus-visible { outline: 2px solid var(--theme-primary); outline-offset: -2px; }
.event-row:disabled { cursor: default; }
.event-avatar { width: 32px; height: 32px; display: grid; place-items: center; border-radius: 7px; color: var(--theme-primary); background: color-mix(in srgb, var(--theme-primary) 10%, var(--theme-surface)); font-size: 12px; font-weight: 850; }
.event-copy { min-width: 0; display: grid; gap: 3px; }
.event-copy strong { overflow: hidden; color: var(--theme-text-strong); font-size: 13px; font-weight: 780; text-overflow: ellipsis; white-space: nowrap; }
.event-copy small { display: flex; align-items: center; gap: 6px; color: var(--theme-text-muted); font-size: 11px; font-weight: 650; }
.event-copy i { width: 7px; height: 7px; border-radius: 50%; }
.event-row time { color: var(--theme-text-muted); font-size: 12px; font-weight: 750; font-variant-numeric: tabular-nums; }
.event-row > svg { color: var(--theme-text-muted); }

.panel-empty { flex: 1; min-height: 200px; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 10px; color: var(--theme-text-muted); text-align: center; }
.panel-empty svg { opacity: .45; }
.panel-empty strong { color: var(--theme-text-strong); font-size: 17px; }
.network-panel { min-height: 560px; display: flex; flex-direction: column; }
.network-heading { align-items: center; }
.network-actions { display: flex; align-items: center; }
.network-button { min-height: 40px; display: inline-flex; align-items: center; gap: 8px; padding: 0 15px; border: 0; border-radius: 7px; color: white; background: var(--theme-primary); font: inherit; font-size: 13px; font-weight: 750; cursor: pointer; }
.scan-progress { width: 180px; display: grid; gap: 7px; color: var(--theme-primary); font-size: 11px; font-weight: 750; text-align: right; }
.scan-progress > div { height: 6px; overflow: hidden; border-radius: 3px; background: var(--theme-surface-hover); }
.scan-progress i { display: block; height: 100%; background: var(--theme-primary); transition: width 250ms ease; }
.network-empty { flex: 1; min-height: 400px; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 12px; border-radius: 7px; color: var(--theme-text-muted); background: color-mix(in srgb, var(--theme-surface-hover) 52%, transparent); text-align: center; }
.network-empty strong { color: var(--theme-text-strong); font-size: 18px; }
.network-empty p { max-width: 520px; margin: 0; font-size: 13px; line-height: 1.55; }
.network-loader { width: 38px; height: 38px; border: 4px solid color-mix(in srgb, var(--theme-primary) 20%, transparent); border-top-color: var(--theme-primary); border-radius: 50%; animation: spin 800ms linear infinite; }
.network-canvas { position: relative; flex: 1; min-height: 440px; overflow: hidden; border-radius: 7px; background: color-mix(in srgb, var(--theme-surface-hover) 50%, transparent); }
.live-update { position: absolute; top: 12px; right: 12px; z-index: 2; padding: 7px 10px; border-radius: 6px; color: var(--theme-primary); background: var(--theme-surface); font-size: 11px; font-weight: 750; }

.worlds-panel { min-height: 560px; }
.worlds-layout { display: grid; grid-template-columns: minmax(250px, .72fr) minmax(0, 1.6fr); gap: 16px; }
.featured-world, .world-row { border: 0; color: inherit; background: transparent; font: inherit; text-align: left; cursor: pointer; }
.featured-world { min-height: 430px; display: grid; grid-template-rows: minmax(260px, 1fr) auto; padding: 0; overflow: hidden; border-radius: 8px; background: var(--theme-surface-hover); box-shadow: 0 10px 28px color-mix(in srgb, var(--theme-text-strong) 8%, transparent); transition: transform 180ms ease, box-shadow 180ms ease; }
.featured-world:hover { transform: translateY(-2px); box-shadow: 0 15px 34px color-mix(in srgb, var(--theme-text-strong) 12%, transparent); }
.featured-media { position: relative; min-height: 260px; display: grid; place-items: center; overflow: hidden; color: var(--theme-text-muted); background: color-mix(in srgb, var(--theme-primary) 8%, var(--theme-surface)); }
.featured-media img { width: 100%; height: 100%; object-fit: cover; }
.featured-media > span { position: absolute; top: 14px; left: 14px; min-width: 42px; height: 32px; display: grid; place-items: center; border-radius: 6px; color: white; background: var(--theme-primary); font-size: 15px; font-weight: 850; font-variant-numeric: tabular-nums; }
.featured-copy { padding: 17px 18px 19px; }
.featured-copy h3 { margin: 0; overflow: hidden; color: var(--theme-text-strong); font-size: 19px; line-height: 1.25; font-weight: 820; text-overflow: ellipsis; white-space: nowrap; }
.featured-copy p, .world-copy small { display: flex; align-items: center; gap: 6px; color: var(--theme-text-muted); font-size: 12px; font-weight: 700; }
.featured-copy p { margin: 8px 0 15px; }
.detail-link { display: inline-flex; align-items: center; gap: 6px; color: var(--theme-primary); font-size: 12px; font-weight: 780; }
.world-list { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); align-content: start; gap: 10px; }
.world-row { min-height: 86px; display: grid; grid-template-columns: 34px 54px minmax(0, 1fr) 18px; align-items: center; gap: 10px; padding: 9px 10px; border-radius: 8px; background: color-mix(in srgb, var(--theme-surface-hover) 68%, transparent); transition: transform 170ms ease, background 170ms ease, box-shadow 170ms ease; }
.world-row:hover { transform: translateY(-1px); background: var(--theme-surface-hover); box-shadow: 0 8px 20px color-mix(in srgb, var(--theme-text-strong) 7%, transparent); }
.world-row:focus-visible, .featured-world:focus-visible { outline: 2px solid var(--theme-primary); outline-offset: 2px; }
.world-rank { color: var(--theme-primary); font-size: 14px; font-weight: 850; font-variant-numeric: tabular-nums; }
.world-thumb { width: 54px; height: 54px; display: grid; place-items: center; overflow: hidden; border-radius: 7px; color: var(--theme-text-muted); background: var(--theme-surface); }
.world-thumb img { width: 100%; height: 100%; object-fit: cover; }
.world-copy { min-width: 0; display: grid; gap: 7px; }
.world-copy strong { overflow: hidden; color: var(--theme-text-strong); font-size: 13px; font-weight: 780; text-overflow: ellipsis; white-space: nowrap; }
.world-copy small { margin: 0; }
.world-row > svg { color: var(--theme-text-muted); }
.worlds-empty { min-height: 430px; }

.loading-layout { min-height: 520px; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 18px; color: var(--theme-text-muted); font-size: 14px; font-weight: 750; }
.loading-bars { width: 260px; height: 120px; display: flex; align-items: end; justify-content: center; gap: 10px; }
.loading-bars span { width: 22px; border-radius: 5px 5px 2px 2px; background: color-mix(in srgb, var(--theme-primary) 46%, var(--theme-surface)); animation: pulse 1.2s ease-in-out infinite alternate; }
@keyframes spin { to { transform: rotate(360deg); } }
@keyframes pulse { to { opacity: .42; } }

@media (max-width: 1100px) {
  .overview-layout { grid-template-columns: 1fr; }
  .insights-layout { grid-template-columns: 1fr; }
  .trend-panel, .events-panel { max-height: none; min-height: 360px; }
  .events-panel { min-height: 320px; }
  .insights-panel { min-height: 300px; }
  .worlds-layout { grid-template-columns: 1fr; }
  .featured-world { min-height: 360px; grid-template-rows: 240px auto; }
}

@media (max-width: 760px) {
  .charts-view { padding: 14px; }
  .charts-tabs { width: 100%; grid-auto-flow: row; grid-template-columns: repeat(3, minmax(0, 1fr)); }
  .charts-tabs button { min-width: 0; padding: 0 8px; font-size: 12px; }
  .charts-tabs button span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .trend-panel, .events-panel, .network-panel, .worlds-panel { padding: 14px; }
  .bar-grid { gap: 6px; }
  .bar-track { width: 70%; }
  .peak-summary { display: none; }
  .world-list { grid-template-columns: 1fr; }
  .network-heading { align-items: flex-start; flex-direction: column; }
}
</style>


