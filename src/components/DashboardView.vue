<script setup lang="ts">
import { computed, ref, onMounted } from 'vue';
import { LayoutDashboard, Users, Activity, Clock, Zap, Map, TrendingUp } from 'lucide-vue-next';
import { VrcApi, DbApi } from '../api';
import VrcAvatar from './VrcAvatar.vue';
import { useI18n } from 'vue-i18n';
import type { VrcUser } from '../types/vrc';
import { useUserProfileStore } from '../stores/userProfile';
import { useFriendsStore } from '../stores/friendsStore';
import { markDataHealthy } from '../stores/dataHealth';
import { currentTheme } from '../theme';

const { t } = useI18n();
const profileStore = useUserProfileStore();
const friendsStore = useFriendsStore();

const loading = ref(false);
const onlineFriendsCount = ref(0);
const activeInstancesCount = ref(0);
const serverStatus = ref('ok');
const recentFriends = ref<VrcUser[]>([]);
const heatmapData = ref<number[]>([0, 0, 0, 0, 0, 0, 0]); // 7天数据
const totalActivity = computed(() => heatmapData.value.reduce((sum, value) => sum + value, 0));
const maxActivity = computed(() => Math.max(...heatmapData.value, 1));

const fetchData = async () => {
  loading.value = true;
  try {
    // 1. 使用共享好友数据，避免重复API调用
    await friendsStore.fetchFriends();
    const friends = friendsStore.allFriends;
    const online = friendsStore.onlineFriends;
    onlineFriendsCount.value = online.length;
    
    // 计算不重复的实例数量
    const instances = new Set(online.map((f: any) => f.location).filter((loc: string | undefined) => loc && loc !== 'private'));
    activeInstancesCount.value = instances.size;
    
    // 获取所有的在线好友
    recentFriends.value = online as VrcUser[];

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

    // 数据（好友 + 热力图）已成功加载，标记数据服务健康
    markDataHealthy();

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
  <main class="dashboard-shell">
    <header class="dashboard-header">
      <div>
        <h1>
          <span class="heading-icon"><LayoutDashboard :size="22" /></span>
          {{ t('dashboard.title') }}
        </h1>
        <p>{{ t('dashboard.subtitle') }}</p>
      </div>
      <button class="refresh-button" :disabled="loading" @click="fetchData">
        <Zap :size="16" :class="{ 'animate-pulse': loading }" />
        {{ t('dashboard.refresh') }}
      </button>
    </header>

    <section class="metrics-strip" aria-label="Dashboard metrics">
      <article class="metric-card">
        <span class="metric-icon"><Users :size="21" /></span>
        <div class="metric-copy">
          <p>{{ t('dashboard.online_friends') }}</p>
          <strong>{{ onlineFriendsCount }} <small>{{ t('dashboard.unit_people') }}</small></strong>
        </div>
      </article>

      <article class="metric-card">
        <span class="metric-icon"><Map :size="21" /></span>
        <div class="metric-copy">
          <p>{{ t('dashboard.active_instances') }}</p>
          <strong>{{ activeInstancesCount }} <small>{{ t('dashboard.unit_count') }}</small></strong>
        </div>
      </article>

      <article class="metric-card">
        <span class="metric-icon"><Clock :size="21" /></span>
        <div class="metric-copy">
          <p>{{ t('dashboard.estimated_records') }}</p>
          <strong>{{ totalActivity }} <small>{{ t('dashboard.events') }}</small></strong>
        </div>
      </article>

      <article class="metric-card status-card" :class="serverStatus">
        <span class="metric-icon"><Activity :size="21" /></span>
        <div class="metric-copy">
          <p>{{ t('dashboard.server_status') }}</p>
          <strong class="status-value">{{ serverStatus === 'ok' ? t('dashboard.normal') : t('dashboard.error') }}</strong>
        </div>
      </article>
    </section>

    <section class="dashboard-content">
      <article class="trend-panel">
        <header class="panel-header">
          <h2><TrendingUp :size="19" /> {{ t('dashboard.weekly_trend') }}</h2>
          <span>{{ t('dashboard.weekly_activity') }}</span>
        </header>

        <div class="trend-chart">
          <div class="chart-grid" aria-hidden="true"><i /><i /><i /><i /></div>
          <div
            v-for="(value, index) in heatmapData"
            :key="index"
            class="bar-column"
          >
            <div class="bar-track">
              <div
                class="activity-bar"
                :class="{ empty: value === 0 }"
                :style="{ height: `${Math.max((value / maxActivity) * 100, value === 0 ? 4 : 10)}%` }"
              >
                <span class="bar-value">{{ value }}</span>
              </div>
            </div>
            <span class="day-label">{{ t(`dashboard.days.${index}`) }}</span>
          </div>
        </div>
      </article>

      <article class="friends-panel">
        <header class="panel-header">
          <h2><span class="online-dot" /> {{ t('dashboard.active_friends') }}</h2>
          <span>{{ onlineFriendsCount }} {{ t('dashboard.online') }}</span>
        </header>

        <div class="friends-list custom-scrollbar">
          <div v-if="recentFriends.length === 0" class="empty-friends">
            <Users :size="38" />
            <p>{{ t('dashboard.no_online_friends') }}</p>
          </div>

          <button
            v-for="friend in recentFriends"
            :key="friend.id"
            class="friend-row"
            @click="openPlayerProfile(friend)"
          >
            <span class="avatar-wrap">
              <VrcAvatar
                :user="friend"
                custom-class="w-10 h-10 rounded-lg object-cover bg-[var(--theme-surface)] border border-[var(--theme-border-soft)]"
              />
              <i />
            </span>
            <span class="friend-copy">
              <strong>{{ friend.displayName }}</strong>
              <small>
                <Map v-if="friend.location && friend.location !== 'private'" :size="12" />
                {{ friend.location === 'private' ? t('dashboard.private_instance') : (friend.statusDescription || t('dashboard.online')) }}
              </small>
            </span>
          </button>
        </div>
      </article>
    </section>
  </main>
</template>

<style scoped>
.dashboard-shell {
  height: 100%;
  min-width: 0;
  padding: 22px;
  display: flex;
  flex-direction: column;
  gap: 18px;
  overflow: hidden;
  color: var(--theme-text);
  background: color-mix(in srgb, var(--theme-bg-main) 82%, var(--theme-surface-hover));
}

.dashboard-header,
.dashboard-header h1,
.metrics-strip,
.metric-card,
.panel-header,
.panel-header h2,
.friend-row,
.friend-copy small {
  display: flex;
  align-items: center;
}

.dashboard-header {
  justify-content: space-between;
  gap: 20px;
}

.dashboard-header h1 {
  margin: 0;
  gap: 10px;
  color: var(--theme-text-strong);
  font-size: 28px;
  line-height: 1.15;
  font-weight: 850;
  letter-spacing: 0;
}

.dashboard-header p {
  margin: 5px 0 0 42px;
  color: var(--theme-text-soft);
  font-size: 13px;
  font-weight: 600;
}

.heading-icon,
.metric-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex: 0 0 auto;
  color: var(--theme-primary);
  background: color-mix(in srgb, var(--theme-primary) 13%, var(--theme-surface));
  border: 1px solid color-mix(in srgb, var(--theme-primary) 24%, var(--theme-border-soft));
  border-radius: 8px;
}

.heading-icon { width: 32px; height: 32px; }
.metric-icon { width: 42px; height: 42px; }

.refresh-button {
  min-height: 38px;
  padding: 0 14px;
  display: inline-flex;
  align-items: center;
  gap: 8px;
  border: 1px solid var(--theme-border-strong);
  border-radius: 8px;
  color: var(--theme-text-strong);
  background: var(--theme-surface);
  font-size: 13px;
  font-weight: 750;
  cursor: pointer;
  transition: transform 180ms ease, border-color 180ms ease, background 180ms ease;
}

.refresh-button:hover { border-color: var(--theme-primary); background: var(--theme-surface-hover); }
.refresh-button:active { transform: translateY(1px); }
.refresh-button:focus-visible { outline: 3px solid color-mix(in srgb, var(--theme-primary) 25%, transparent); outline-offset: 2px; }
.refresh-button:disabled { cursor: wait; opacity: .65; }

.metrics-strip {
  min-width: 0;
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 10px;
}

.metric-card {
  min-width: 0;
  min-height: 88px;
  gap: 12px;
  padding: 14px;
  overflow: hidden;
  border: 1px solid var(--theme-border-soft);
  border-radius: 8px;
  background: color-mix(in srgb, var(--theme-surface) 92%, transparent);
  box-shadow: 0 8px 22px color-mix(in srgb, var(--theme-text-strong) 7%, transparent);
}

.metric-copy { min-width: 0; }
.metric-copy p {
  margin: 0 0 4px;
  overflow: hidden;
  color: var(--theme-text-soft);
  font-size: 12px;
  line-height: 1.25;
  font-weight: 700;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.metric-copy strong {
  min-width: 0;
  display: block;
  color: var(--theme-text-strong);
  font-size: 25px;
  line-height: 1.05;
  font-weight: 850;
  font-variant-numeric: tabular-nums;
}

.metric-copy small {
  color: var(--theme-text-muted);
  font-size: 11px;
  font-weight: 700;
}

.status-card.ok .metric-icon { color: #047857; background: #d1fae5; border-color: #a7f3d0; }
.status-card.error .metric-icon { color: #b91c1c; background: #fee2e2; border-color: #fecaca; }
.metric-copy .status-value {
  max-width: 100%;
  font-size: 20px;
  line-height: 1.1;
  overflow-wrap: anywhere;
  text-wrap: balance;
}

.dashboard-content {
  flex: 1;
  min-height: 0;
  display: grid;
  grid-template-columns: minmax(0, 1.85fr) minmax(260px, .85fr);
  gap: 12px;
}

.trend-panel,
.friends-panel {
  min-width: 0;
  min-height: 0;
  padding: 18px;
  display: flex;
  flex-direction: column;
  border: 1px solid var(--theme-border-soft);
  border-radius: 8px;
  background: color-mix(in srgb, var(--theme-surface) 94%, transparent);
  box-shadow: 0 10px 24px color-mix(in srgb, var(--theme-text-strong) 6%, transparent);
}

.panel-header {
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 14px;
}

.panel-header h2 {
  min-width: 0;
  gap: 8px;
  margin: 0;
  color: var(--theme-text-strong);
  font-size: 16px;
  font-weight: 800;
}

.panel-header h2 > svg { flex: 0 0 auto; color: var(--theme-primary); }
.panel-header > span {
  flex: 0 0 auto;
  padding: 5px 8px;
  border: 1px solid var(--theme-border-soft);
  border-radius: 6px;
  color: var(--theme-text-muted);
  background: var(--theme-surface-hover);
  font-size: 10px;
  font-weight: 750;
}

.trend-chart {
  position: relative;
  flex: 1;
  min-height: 220px;
  padding: 18px 12px 8px;
  display: grid;
  grid-template-columns: repeat(7, minmax(34px, 1fr));
  align-items: stretch;
  gap: 12px;
  overflow: hidden;
  border: 1px solid var(--theme-border-soft);
  border-radius: 8px;
  background: color-mix(in srgb, var(--theme-bg-main) 60%, var(--theme-surface));
}

.chart-grid {
  position: absolute;
  inset: 18px 12px 31px;
  display: flex;
  flex-direction: column;
  justify-content: space-between;
  pointer-events: none;
}
.chart-grid i { width: 100%; border-top: 1px dashed var(--theme-border-strong); opacity: .75; }

.bar-column {
  z-index: 1;
  min-width: 0;
  display: grid;
  grid-template-rows: minmax(0, 1fr) 18px;
  gap: 6px;
}

.bar-track {
  min-height: 0;
  display: flex;
  align-items: flex-end;
  border-radius: 6px 6px 3px 3px;
  background: color-mix(in srgb, var(--theme-primary) 9%, var(--theme-surface));
  box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--theme-primary) 11%, transparent);
}

.activity-bar {
  position: relative;
  width: 100%;
  min-height: 8px;
  border-radius: 6px 6px 3px 3px;
  background: linear-gradient(to top, var(--theme-primary-hover), var(--theme-primary));
  box-shadow: 0 7px 16px color-mix(in srgb, var(--theme-primary) 28%, transparent);
  transition: height 500ms ease, filter 180ms ease;
}
.activity-bar:hover { filter: brightness(1.06); }
.activity-bar.empty { background: var(--theme-border-strong); box-shadow: none; }
.bar-value {
  position: absolute;
  top: -20px;
  left: 50%;
  translate: -50% 0;
  color: var(--theme-text-strong);
  font-size: 10px;
  font-weight: 800;
  font-variant-numeric: tabular-nums;
}
.day-label { color: var(--theme-text-strong); font-size: 11px; font-weight: 750; text-align: center; }

.online-dot { width: 8px; height: 8px; border-radius: 50%; background: #10b981; box-shadow: 0 0 0 4px #d1fae5; }
.friends-list { flex: 1; min-height: 0; overflow-y: auto; display: flex; flex-direction: column; gap: 5px; }
.empty-friends { flex: 1; display: grid; place-items: center; align-content: center; gap: 8px; color: var(--theme-text-muted); }
.empty-friends p { margin: 0; font-size: 12px; font-weight: 700; }
.friend-row {
  width: 100%;
  min-width: 0;
  gap: 10px;
  padding: 7px;
  border: 1px solid transparent;
  border-radius: 8px;
  color: inherit;
  background: transparent;
  text-align: left;
  cursor: pointer;
  transition: background 160ms ease, border-color 160ms ease;
}
.friend-row:hover { border-color: var(--theme-border-soft); background: var(--theme-surface-hover); }
.friend-row:focus-visible { outline: 3px solid color-mix(in srgb, var(--theme-primary) 25%, transparent); }
.avatar-wrap { position: relative; flex: 0 0 auto; }
.avatar-wrap i { position: absolute; right: -2px; bottom: 0; width: 10px; height: 10px; border: 2px solid var(--theme-surface); border-radius: 50%; background: #10b981; }
.friend-copy { min-width: 0; display: block; }
.friend-copy strong,
.friend-copy small { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.friend-copy strong { display: block; color: var(--theme-text-strong); font-size: 12px; font-weight: 800; }
.friend-copy small { gap: 4px; color: var(--theme-text-soft); font-size: 10px; font-weight: 600; }

@media (max-width: 980px) {
  .metrics-strip { grid-template-columns: repeat(2, minmax(0, 1fr)); }
  .dashboard-shell { overflow-y: auto; }
  .dashboard-content { flex: none; grid-template-columns: 1fr; }
  .trend-panel { min-height: 360px; }
  .friends-panel { min-height: 280px; }
}

@media (max-width: 620px) {
  .dashboard-shell { padding: 16px; }
  .dashboard-header { align-items: flex-start; }
  .dashboard-header h1 { font-size: 23px; }
  .dashboard-header p { margin-left: 0; }
  .metrics-strip { grid-template-columns: 1fr; }
  .metric-card { min-height: 76px; }
  .trend-chart { gap: 6px; padding-inline: 8px; }
  .panel-header { align-items: flex-start; }
}
</style>


