import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import { VrcApi, DbApi } from '../api';
import { isTauri } from '@tauri-apps/api/core';
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

  async function doFetch(): Promise<VrcUser[]> {
    loading.value = true;
    error.value = '';

    try {
      // 1. Try DB cache first (instant, no network)
      let cached: VrcUser[] = (await DbApi.getCachedFriends()) || [];

      // 2. If cache empty, fetch from API
      if (!cached || cached.length === 0) {
        try {
          const live: VrcUser[] = await VrcApi.getAllFriends({ n: 100, offset: 0 });
          if (live && live.length > 0) {
            cached = live;
            if (isTauri()) {
              await DbApi.batchSaveFriends({ friendsJson: JSON.stringify(live) });
            }
          }
        } catch (err: any) {
          error.value = err.message || String(err);
        }
      }

      const normalized = (cached || []).filter((f: any) => f?.id || f?.displayName);
      allFriends.value = normalized;
      lastFetchTime.value = Date.now();
      if (error.value && normalized.length === 0) {
        throw new Error(error.value);
      }
      return normalized;
    } finally {
      loading.value = false;
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
    beginSync,
    setFriends,
    setError,
    updateFriend,
    removeFriend,
    addFriend,
    clear,
  };
});
