import { defineStore } from 'pinia';
import { ref } from 'vue';
import { VrcApi, DbApi } from '../api';

export const useUserProfileStore = defineStore('userProfile', () => {
  const isOpen = ref(false);
  const targetUserId = ref<string | null>(null);
  
  // Base Data
  const baseInfo = ref<any>(null);
  const myId = ref<string | null>(null);
  const isLoadingBase = ref(false);

  // Tabs Data
  const mutualFriends = ref<any[]>([]);
  const isLoadingMutual = ref(false);
  
  const groups = ref<any[]>([]);
  const isLoadingGroups = ref(false);

  const createdWorlds = ref<any[]>([]);
  const isLoadingWorlds = ref(false);
  
  const createdAvatars = ref<any[]>([]);
  const isLoadingAvatars = ref(false);

  // Local Data
  const localNote = ref('');
  const isSavingNote = ref(false);

  // Open the profile for a specific user
  const openProfile = async (userId: string, prefillData: any = null) => {
    targetUserId.value = userId;
    isOpen.value = true;
    
    // Reset state
    if (prefillData) {
      baseInfo.value = prefillData;
    } else {
      baseInfo.value = null;
    }
    
    mutualFriends.value = [];
    groups.value = [];
    createdWorlds.value = [];
    createdAvatars.value = [];
    localNote.value = '';
    
    // Parallel Fetch core info & local data
    isLoadingBase.value = true;
    
    // Optimistic load
    const baseCacheKey = `user_${userId}`;
    DbApi.getApiCache({ key: baseCacheKey }).then(cachedStr => {
      if (cachedStr && !baseInfo.value) { // Don't override if prefillData was set and better
        try {
          const cachedData = JSON.parse(cachedStr);
          baseInfo.value = { ...cachedData, ...baseInfo.value }; // Merge
          // We can't turn off isLoadingBase yet, because we still want to show the background refresh happening for the core info.
        } catch(e) {}
      }
    }).catch(() => {});
    
    Promise.allSettled([
      VrcApi.request(`/users/${userId}`).then(res => {
        baseInfo.value = res;
        DbApi.saveApiCache({ key: baseCacheKey, data: JSON.stringify(res) }).catch(() => {});
      }),
      DbApi.getNote({ userId }).then((res: any) => {
        if (res && res.note) {
          localNote.value = res.note;
        }
      }),
      VrcApi.getCurrentUser().then((user: any) => {
        if (user && user.id) myId.value = user.id;
      })
    ]).finally(() => {
      isLoadingBase.value = false;
    });
    
    // Note: Other tabs will trigger their own fetch when selected or we can prefetch them
    fetchGroups(userId);
    fetchCreatedWorlds(userId);
    fetchCreatedAvatars(userId);
  };

  const saveLocalNote = async () => {
    if (!targetUserId.value || !baseInfo.value?.displayName) return;
    isSavingNote.value = true;
    try {
      await DbApi.saveNote({
        userId: targetUserId.value,
        displayName: baseInfo.value.displayName,
        note: localNote.value
      });
    } catch (err) {
      console.warn("Failed to save note", err);
    } finally {
      isSavingNote.value = false;
    }
  };

  const closeProfile = () => {
    isOpen.value = false;
    targetUserId.value = null;
  };

  const fetchGroups = async (userId: string) => {
    isLoadingGroups.value = true;
    const cacheKey = `groups_${userId}`;
    
    try {
      // Optimistic load from DB
      const cachedStr = await DbApi.getApiCache({ key: cacheKey });
      if (cachedStr) {
        try {
          const cachedData = JSON.parse(cachedStr);
          if (Array.isArray(cachedData) && cachedData.length > 0) {
            groups.value = cachedData;
            isLoadingGroups.value = false; // Turn off loader early!
          }
        } catch(e) {}
      }

      // Background fetch
      const res = await VrcApi.request(`/users/${userId}/groups`);
      if (Array.isArray(res)) {
        groups.value = res;
        DbApi.saveApiCache({ key: cacheKey, data: JSON.stringify(res) }).catch(() => {});
      }
    } catch (err) {
      console.warn("Failed to fetch user groups", err);
    } finally {
      isLoadingGroups.value = false;
    }
  };

  const fetchCreatedWorlds = async (userId: string) => {
    isLoadingWorlds.value = true;
    const cacheKey = `worlds_${userId}`;
    
    try {
      // Optimistic load
      const cachedStr = await DbApi.getApiCache({ key: cacheKey });
      if (cachedStr) {
        try {
          const cachedData = JSON.parse(cachedStr);
          if (Array.isArray(cachedData) && cachedData.length > 0) {
            createdWorlds.value = cachedData;
            isLoadingWorlds.value = false;
          }
        } catch(e) {}
      }

      // Background fetch
      const res = await VrcApi.request('/worlds', 'GET', { user: userId, n: 60 });
      if (Array.isArray(res)) {
        createdWorlds.value = res;
        DbApi.saveApiCache({ key: cacheKey, data: JSON.stringify(res) }).catch(() => {});
      }
    } catch (err) {
      console.warn("Failed to fetch created worlds", err);
    } finally {
      isLoadingWorlds.value = false;
    }
  };
  
  const fetchCreatedAvatars = async (userId: string) => {
    isLoadingAvatars.value = true;
    const cacheKey = `avatars_${userId}`;
    
    try {
      const cachedStr = await DbApi.getApiCache({ key: cacheKey });
      if (cachedStr) {
        try {
          const cachedData = JSON.parse(cachedStr);
          if (Array.isArray(cachedData) && cachedData.length > 0) {
            createdAvatars.value = cachedData;
            isLoadingAvatars.value = false;
          }
        } catch(e) {}
      }

      const res = await VrcApi.request('/avatars', 'GET', { user: userId, n: 60 });
      if (Array.isArray(res)) {
        createdAvatars.value = res;
        DbApi.saveApiCache({ key: cacheKey, data: JSON.stringify(res) }).catch(() => {});
      }
    } catch (err) {
      console.warn("Failed to fetch created avatars", err);
    } finally {
      isLoadingAvatars.value = false;
    }
  };

  const fetchMutualFriends = async (userId: string) => {
    isLoadingMutual.value = true;
    try {
      VrcApi.getMutualFriends({ userId }).then((res: any) => {
        if (Array.isArray(res)) mutualFriends.value = res;
        else mutualFriends.value = [];
      }).catch(() => {
        mutualFriends.value = [];
      }).finally(() => {
        isLoadingMutual.value = false;
      });
    } catch (err) {
      console.warn("Failed to fetch mutual friends", err);
      isLoadingMutual.value = false;
    }
  };

  return {
    isOpen,
    targetUserId,
    baseInfo,
    myId,
    mutualFriends,
    groups,
    createdWorlds,
    createdAvatars,
    localNote,
    isSavingNote,
    isLoadingBase,
    isLoadingMutual,
    isLoadingGroups,
    isLoadingWorlds,
    isLoadingAvatars,
    openProfile,
    closeProfile,
    fetchMutualFriends,
    fetchGroups,
    fetchCreatedWorlds,
    fetchCreatedAvatars,
    saveLocalNote
  };
});
