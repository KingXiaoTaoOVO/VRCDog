import { reactive, ref, computed } from 'vue';

/**
 * Tracks whether the app's primary VRChat data (friends, activity, logs) has
 * been successfully loaded recently. This is what the bottom-left "数据服务"
 * indicator reflects — it is intentionally decoupled from the optional VRChat
 * realtime WebSocket pipeline, which may be unreachable on some networks.
 */

// 后台刷新间隔（秒），由设置页的「后台数据刷新频率」驱动。
// 区间必须和那个滑块的 min/max 保持一致，否则界面允许的值与实际生效的值会脱节。
export const DATA_REFRESH_MIN_SEC = 10;
export const DATA_REFRESH_MAX_SEC = 120;
export const DATA_REFRESH_DEFAULT_SEC = 30;

export const dataRefresh = reactive({
  intervalSec: DATA_REFRESH_DEFAULT_SEC,
});

export function setDataRefreshIntervalSec(value: unknown): number {
  // 只接受「数字」或「非空数字字符串」：
  // - `Number('')` / `Number(null)` / `Number(false)` 都是 0，只判 isFinite 会被夹成
  //   区间下限，等于让一个坏掉的配置变成最激进的轮询频率；
  // - `Number(true)` 是 1，同样会悄悄变成 10 秒而不是回落默认值。
  const numeric = typeof value === 'number'
    ? value
    : (typeof value === 'string' && value.trim() !== '' ? Number(value) : NaN);
  const valid = Number.isFinite(numeric) && numeric > 0;
  const clamped = valid
    ? Math.min(DATA_REFRESH_MAX_SEC, Math.max(DATA_REFRESH_MIN_SEC, Math.round(numeric)))
    : DATA_REFRESH_DEFAULT_SEC;
  dataRefresh.intervalSec = clamped;
  return clamped;
}

/**
 * 「多久没刷就算过期」。
 *
 * 原先是一个写死的 10 分钟窗口，而后台刷新根本不存在（只有登录和进特定页面时才会
 * 刷新一次），于是正常使用十几分钟后侧边栏就会一直挂着「数据已过期」，
 * 且把间隔调到多大都不会改变这个判定。
 *
 * 现在窗口跟随刷新间隔：允许连续 20 次刷新失败才算过期，并保留 5 分钟下限，
 * 这样把间隔调到 120s 的用户不会刚刷完就被判过期，也不会等上一小时才知道数据停了。
 */
const STALE_AFTER_MISSED_CYCLES = 20;
const MIN_HEALTHY_WINDOW_MS = 5 * 60 * 1000;

export function healthyWindowMs(): number {
  return Math.max(
    MIN_HEALTHY_WINDOW_MS,
    dataRefresh.intervalSec * STALE_AFTER_MISSED_CYCLES * 1000,
  );
}

export const dataHealth = reactive({
  lastSuccessAt: null as number | null,
});

// Ticking clock so the "stale" state and "synced Xs ago" label update over time.
// 间隔取 10s 与最小区间同量级，足够让「同步于 Xs 前」按秒推进。
export const nowTs = ref(Date.now());
setInterval(() => { nowTs.value = Date.now(); }, 10_000);

export function markDataHealthy() {
  dataHealth.lastSuccessAt = Date.now();
}

export type DataServiceStatus = 'online' | 'stale' | 'offline';

export const dataServiceStatus = computed<DataServiceStatus>(() => {
  const last = dataHealth.lastSuccessAt;
  if (last && nowTs.value - last < healthyWindowMs()) return 'online';
  if (last) return 'stale';
  return 'offline';
});
