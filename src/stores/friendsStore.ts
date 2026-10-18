import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import { VrcApi, DbApi } from '../api';
import { isTauri } from '@tauri-apps/api/core';
import { markDataHealthy } from './dataHealth';
import type { VrcUser } from '../types/vrc';

/**
 * Shared friends data store.
 * Single source of truth — prevents DashboardView, ChartsView, FriendLocationsView,
 * FriendsListView from each firing their own getFriends() API call.
 */
export const useFriendsStore = defineStore('friends', () => {
  const allFriends = ref<VrcUser[]>([]);
  const loading = ref(false);
  const lastFetchTime = ref(0);
  const error = ref('');

  const CACHE_TTL_MS = 30_000;

  const onlineFriends = computed(() =>
    allFriends.value.filter((f: any) => f.location && f.location !== 'offline' && f.location !== 'private')
  );

  const offlineFriends = computed(() =>
    allFriends.value.filter((f: any) => !f.location || f.location === 'offline')
  );

  const privateFriends = computed(() =>
    allFriends.value.filter((f: any) => f.location === 'private')
  );

  const totalCount = computed(() => allFriends.value.length);
  const onlineCount = computed(() => onlineFriends.value.length);

  /**
   * Tracks an in-flight initial sync from authStore.
   * Views that mount before sync completes wait on this promise
   * instead of firing redundant API calls.
   */
  let syncPromise: Promise<VrcUser[]> | null = null;

  /**
   * Called by authStore.startFriendsSync to register the in-flight sync.
   * Views will wait on this promise if they mount before it resolves.
   */
  function beginSync(promise: Promise<VrcUser[]>) {
    syncPromise = promise;
    promise.finally(() => { syncPromise = null; });
  }

  /**
   * Fetch friends — if an initial sync is in progress, wait for it.
   * Otherwise, use cache if fresh, or fetch from API.
   */
  async function fetchFriends(force = false): Promise<VrcUser[]> {
    const now = Date.now();

    // If an initial sync is in progress, wait for it instead of making a new call
    if (syncPromise && !force) {
      return syncPromise;
    }

    if (!force && allFriends.value.length > 0 && (now - lastFetchTime.value) < CACHE_TTL_MS) {
      return allFriends.value;
    }

    return doFetch();
  }

  /**
   * @param silent 后台轮询模式：不切 loading、失败不写 error、
   *               并且**退回缓存时不标记数据健康**（拿不到线上数据就不该显示绿色）。
   * @param live 本次是否真的从 VRChat API 取到了数据。
   */
  async function runFetch(silent: boolean): Promise<{ friends: VrcUser[]; live: boolean }> {
    // 后台轮询不切 loading：否则每 N 秒会闪一次 spinner。
    if (!silent) loading.value = true;
    if (!silent) error.value = '';

    // Load the local cache up-front so we can fall back to it if the live
    // request fails (offline / rate-limited / transient error).
    let cached: VrcUser[] = [];
    try {
      cached = (await DbApi.getCachedFriends()) || [];
    } catch {
      cached = [];
    }

    try {
      // Always prefer the live API as the single source of truth (VRCX does the
      // same). A stale or partial SQLite cache must never be shown as the final
      // result — that is what previously made the roster look "too few". The cache
      // is only a fallback for when the network/API is unavailable.
      const live: VrcUser[] = await VrcApi.getAllFriends({ n: 100, offset: 0 });
      const normalized = (live || []).filter((f: any) => f?.id || f?.displayName);
      allFriends.value = normalized;
      lastFetchTime.value = Date.now();
      if (normalized.length > 0) {
        if (isTauri()) {
          await DbApi.batchSaveFriends({ friendsJson: JSON.stringify(normalized) });
        }
        markDataHealthy();
      }
      return { friends: normalized, live: true };
    } catch (err: any) {
      // 后台轮询的偶发失败（限流、网络抖动）不该在界面上弹错误横幅；
      // 侧边栏的「数据已过期」才是给这类情况用的通道。
      if (!silent) error.value = err?.message || String(err);
      // Live fetch failed — fall back to the cached snapshot if we have one.
      if (cached && cached.length > 0) {
        const normalized = cached.filter((f: any) => f?.id || f?.displayName);
        allFriends.value = normalized;
        lastFetchTime.value = Date.now();
        // 手动刷新时沿用「有得看总比空白好」的旧行为；
        // 后台轮询则不标记健康，否则网络一直不通也永远不会显示「数据已过期」。
        if (normalized.length > 0 && !silent) markDataHealthy();
        return { friends: normalized, live: false };
      }
      throw err;
    } finally {
      if (!silent) loading.value = false;
    }
  }

  async function doFetch(silent = false): Promise<VrcUser[]> {
    const { friends } = await runFetch(silent);
    return friends;
  }

  /**
   * 后台定时刷新入口（设置页「后台数据刷新频率」驱动）。
   *
   * 与手动刷新的区别：不闪 spinner、失败不弹错误、并且会等登录时的首次同步结束，
   * 避免两个 getAllFriends 请求叠在一起互相覆盖。
   * 返回值表示这次是否真的取到了线上数据；退回本地缓存算失败（false），
   * 因为侧边栏要反映的正是「还能不能从 VRChat 拿到数据」。
   */
  async function backgroundRefresh(): Promise<boolean> {
    if (syncPromise) {
      await syncPromise.catch(() => [] as VrcUser[]);
    }
    try {
      const { live } = await runFetch(true);
      return live;
    } catch {
      return false;
    }
  }

  function updateFriend(userId: string, patch: Partial<VrcUser>) {
    const idx = allFriends.value.findIndex((f: any) => f.id === userId);
    if (idx >= 0) {
      allFriends.value[idx] = { ...allFriends.value[idx], ...patch };
      allFriends.value = [...allFriends.value];
    }
  }

  function removeFriend(userId: string) {
    allFriends.value = allFriends.value.filter((f: any) => f.id !== userId);
  }

  function addFriend(friend: VrcUser) {
    if (!allFriends.value.find((f: any) => f.id === friend.id)) {
      allFriends.value = [...allFriends.value, friend];
    }
  }

  function clear() {
    allFriends.value = [];
    lastFetchTime.value = 0;
    error.value = '';
  }

  function setFriends(friends: VrcUser[]) {
    allFriends.value = friends;
    lastFetchTime.value = Date.now();
  }

  function setError(message: string) {
    error.value = message;
  }

  return {
    allFriends,
    loading,
    error,
    onlineFriends,
    offlineFriends,
    privateFriends,
    totalCount,
    onlineCount,
    fetchFriends,
    backgroundRefresh,
    beginSync,
    setFriends,
    setError,
    updateFriend,
    removeFriend,
    addFriend,
    clear,
  };
});
