import { reactive, ref, computed } from 'vue';

/**
 * Tracks whether the app's primary VRChat data (friends, activity, logs) has
 * been successfully loaded recently. This is what the bottom-left "数据服务"
 * indicator reflects — it is intentionally decoupled from the optional VRChat
 * realtime WebSocket pipeline, which may be unreachable on some networks.
 */

// After this many ms without a successful data fetch, the service is "stale".
const HEALTHY_WINDOW_MS = 10 * 60 * 1000;

export const dataHealth = reactive({
  lastSuccessAt: null as number | null,
});

// Ticking clock so the "stale" state and "synced Xs ago" label update over time.
export const nowTs = ref(Date.now());
setInterval(() => { nowTs.value = Date.now(); }, 10_000);

export function markDataHealthy() {
  dataHealth.lastSuccessAt = Date.now();
}

export function setDataStale() {
  dataHealth.lastSuccessAt = null;
}

export type DataServiceStatus = 'online' | 'stale' | 'offline';

export const dataServiceStatus = computed<DataServiceStatus>(() => {
  const last = dataHealth.lastSuccessAt;
  if (last && nowTs.value - last < HEALTHY_WINDOW_MS) return 'online';
  if (last) return 'stale';
  return 'offline';
});
