import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import { VrcApi, DbApi } from '../api';

export interface VrcUserBadge {
  badgeId: string;
  badgeName: string;
  badgeDescription: string;
  badgeImageUrl: string;
  showBadge: boolean;
}

export interface VrcUser {
  id: string;
  displayName: string;
  username?: string;
  userIcon?: string;
  profilePicOverride?: string;
  bio?: string;
  bioLinks?: string[];
  currentAvatarImageUrl?: string;
  currentAvatarThumbnailImageUrl?: string;
  status: string;
  statusDescription?: string;
  last_login?: string;
  last_platform?: string;
  date_joined?: string;
  isFriend: boolean;
  friendKey?: string;
  location?: string;
  instanceId?: string;
  worldId?: string;
  tags?: string[];
  languages?: string[];
  pronouns?: string;
  badges?: VrcUserBadge[];
  fallbackAvatar?: string;
  state?: string;
  active_instance_id?: string;
  [key: string]: any;
}

export interface VrcGroup {
  id: string;
  name: string;
  iconUrl?: string;
  bannerUrl?: string;
  memberCount?: number;
  shortCode?: string;
  description?: string;
  privacy?: string;
  joinState?: string;
  [key: string]: any;
}

export interface VrcWorld {
  id: string;
  name: string;
  imageUrl?: string;
  thumbnailImageUrl?: string;
  occupants?: number;
  favorites?: number;
  visits?: number;
  updated_at?: string;
  releaseStatus?: string;
  supportedPlatforms?: string[];
  tags?: string[];
  instances?: any[];
  description?: string;
  capacity?: number;
  authorName?: string;
  authorId?: string;
  [key: string]: any;
}

export interface VrcAvatar {
  id: string;
  name: string;
  thumbnailImageUrl?: string;
  imageUrl?: string;
  updated_at?: string;
  releaseStatus?: string;
  supportedPlatforms?: string[];
  description?: string;
  tags?: string[];
  authorId?: string;
  authorName?: string;
  [key: string]: any;
}

export interface ActivityLog {
  id?: string;
  type?: string;
  created_at?: string;
  [key: string]: string | number | boolean | undefined | null;
}
export const useUserProfileStore = defineStore('userProfile', () => {
  const isOpen = ref(false);
  const targetUserId = ref<string | null>(null);
  
  // Base Data
  const baseInfo = ref<VrcUser | null>(null);
  const myId = ref<string | null>(null);
  const isLoadingBase = ref(false);

  // Tabs Data
  const mutualFriends = ref<VrcUser[]>([]);
  const isLoadingMutual = ref(false);
  
  const groups = ref<VrcGroup[]>([]);
  const isLoadingGroups = ref(false);

  const mutualGroups = ref<VrcGroup[]>([]);
  const isLoadingMutualGroups = ref(false);

  const createdWorlds = ref<VrcWorld[]>([]);
  const isLoadingWorlds = ref(false);
  
  const favoriteWorlds = ref<VrcWorld[]>([]);
  const isLoadingFavWorlds = ref(false);

  const createdAvatars = ref<VrcAvatar[]>([]);
  const isLoadingAvatars = ref(false);

  // Local Data
  const activityLogs = ref<ActivityLog[]>([]);
  const isLoadingActivity = ref(false);

  const favoriteId = ref<string | null>(null);
  const isFavorite = computed(() => !!favoriteId.value);

  const localNote = ref('');
  const isSavingNote = ref(false);

  const resolveMyId = async () => {
    if (myId.value) return myId.value;
    try {
      const user = await VrcApi.request('/auth/user', { method: 'GET', suppressAuthExpired: true }) as { id?: string } | null | undefined;
      myId.value = user?.id || null;
    } catch {
      myId.value = null;
    }
    return myId.value;
  };

  // ── Navigation History (breadcrumb) ──────────────────────────────
  interface NavEntry { userId: string; displayName: string; }
  const navHistory = ref<NavEntry[]>([]);

  const goBack = () => {
    if (navHistory.value.length > 1) {
      navHistory.value.pop(); // Remove current
      const prev = navHistory.value[navHistory.value.length - 1];
      if (prev) {
        // Re-open previous profile without adding to history
        targetUserId.value = prev.userId;
        _loadProfile(prev.userId, null, false);
      }
    } else {
      closeProfile();
    }
  };

  // Open the profile for a specific user
  const openProfile = async (userId: string, prefillData: VrcUser | null = null) => {    targetUserId.value = userId;
    isOpen.value = true;

    // Add to navigation history
    const displayName = prefillData?.displayName || userId;
    navHistory.value.push({ userId, displayName });

    _loadProfile(userId, prefillData, true);
  };

  /// 用户详情面板是**单例复用**的（Store-driven Modal）：所有 ref 由同一个 store 持有。
  /// 因此每个 await 之后都必须确认「当前目标仍然是发起请求时的那个人」，
  /// 否则从 A 快速跳到 B 时，A 的慢响应会陆续落到 B 的面板上——
  /// 分组、世界、头像、共同好友、收藏状态全部串号， loading 旗标也会被提前灭掉。
  const isStillTarget = (userId: string) => targetUserId.value === userId;

  // Internal load function (shared by openProfile and goBack)
  const _loadProfile = async (userId: string, prefillData: VrcUser | null, resetState: boolean) => {
    targetUserId.value = userId;
    isOpen.value = true;
    
    // Reset state
    if (prefillData) {
      baseInfo.value = prefillData;
    } else {
      baseInfo.value = null;
    }
    
    mutualFriends.value = [];
    mutualGroups.value = [];
    groups.value = [];
    createdWorlds.value = [];
    favoriteWorlds.value = [];
    createdAvatars.value = [];
    activityLogs.value = [];
    localNote.value = '';
    favoriteId.value = null;
    
    // Parallel Fetch core info & local data
    isLoadingBase.value = true;
    
    // Optimistic load
    const baseCacheKey = `user_${userId}`;
    DbApi.getApiCache({ key: baseCacheKey }).then(cachedStr => {
      if (!isStillTarget(userId)) return;
      if (cachedStr && !baseInfo.value) { // Don't override if prefillData was set and better
        try {
          const cachedData = JSON.parse(cachedStr);
          baseInfo.value = { ...cachedData, ...(baseInfo.value || {}) }; // Merge
          // We can't turn off isLoadingBase yet, because we still want to show the background refresh happening for the core info.
        } catch(e) {}
      }
    }).catch(() => {});
    
    Promise.allSettled([
      VrcApi.request(`/users/${userId}`).then(res => {
        if (!isStillTarget(userId)) return;
        baseInfo.value = res;
        DbApi.saveApiCache({ key: baseCacheKey, data: JSON.stringify(res) }).catch(() => {});
      }),
      DbApi.getNote({ userId }).then((res: { note?: string } | null | undefined) => {
        if (!isStillTarget(userId)) return;
        if (res && res.note) {
          localNote.value = res.note;
        }
      }),
      resolveMyId()
    ]).finally(() => {
      // 目标已经换成别人时不能灭灯，否则新用户的骨架屏会提前变成空数据
      if (isStillTarget(userId)) isLoadingBase.value = false;
    });
    
    // Note: Other tabs will trigger their own fetch when selected or we can prefetch them
    fetchGroups(userId);
    fetchCreatedWorlds(userId);
    fetchCreatedAvatars(userId);
    fetchActivityLogs(userId);
    fetchMutualFriends(userId);
    fetchMutualGroups(userId);
    checkIsFavorite(userId);
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
    navHistory.value = [];
  };

  const fetchGroups = async (userId: string) => {
    isLoadingGroups.value = true;
    const cacheKey = `groups_${userId}`;
    
    try {
      // Optimistic load from DB
      const cachedStr = await DbApi.getApiCache({ key: cacheKey });
      if (!isStillTarget(userId)) return;
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
      if (!isStillTarget(userId)) return;
      if (Array.isArray(res)) {
        groups.value = res;
        DbApi.saveApiCache({ key: cacheKey, data: JSON.stringify(res) }).catch(() => {});
      }
    } catch (err) {
      console.warn("Failed to fetch user groups", err);
    } finally {
      if (isStillTarget(userId)) isLoadingGroups.value = false;
    }
  };

  const fetchCreatedWorlds = async (userId: string) => {
    isLoadingWorlds.value = true;
    const cacheKey = `worlds_${userId}`;
    
    try {
      // Optimistic load
      const cachedStr = await DbApi.getApiCache({ key: cacheKey });
      if (!isStillTarget(userId)) return;
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
      const res = await VrcApi.request('/worlds', { method: 'GET', params: { userId: userId, n: 60 } });
      if (!isStillTarget(userId)) return;
      if (Array.isArray(res)) {
        createdWorlds.value = res;
        DbApi.saveApiCache({ key: cacheKey, data: JSON.stringify(res) }).catch(() => {});
      }
    } catch (err) {
      console.warn("Failed to fetch created worlds", err);
    } finally {
      if (isStillTarget(userId)) isLoadingWorlds.value = false;
    }
  };

  const fetchFavoriteWorlds = async (userId: string) => {
    isLoadingFavWorlds.value = true;
    const cacheKey = `fav_worlds_${userId}`;
    try {
      const currentUserId = await resolveMyId();
      if (!isStillTarget(userId)) return;

      if (!currentUserId || userId !== currentUserId) {
        favoriteWorlds.value = [];
        return;
      }

      // Optimistic load 缓存
      const cachedStr = await DbApi.getApiCache({ key: cacheKey });
      if (!isStillTarget(userId)) return;
      if (cachedStr) {
        try {
          const cachedData = JSON.parse(cachedStr);
          if (Array.isArray(cachedData) && cachedData.length > 0) {
            favoriteWorlds.value = cachedData;
            isLoadingFavWorlds.value = false;
          }
        } catch(e) {}
      }
      // 真实拉取（VRChat 收藏世界 API 只能查自己的，看他人时会 404，这里 catch 掉）
      try {
        const res = await VrcApi.getFavoriteWorlds({ n: 100, offset: 0 });
        if (!isStillTarget(userId)) return;
        if (Array.isArray(res)) {
          favoriteWorlds.value = res;
          DbApi.saveApiCache({ key: cacheKey, data: JSON.stringify(res) }).catch(() => {});
        }
      } catch (e) {
        // 看他人收藏夹通常 403/404，保持本地缓存即可
      }
    } finally {
      if (isStillTarget(userId)) isLoadingFavWorlds.value = false;
    }
  };
  
  const fetchCreatedAvatars = async (userId: string) => {
    isLoadingAvatars.value = true;
    const cacheKey = `avatars_${userId}`;
    
    try {
      const currentUserId = await resolveMyId();
      if (!isStillTarget(userId)) return;
      if (!currentUserId || userId !== currentUserId) {
        createdAvatars.value = [];
        return;
      }

      const cachedStr = await DbApi.getApiCache({ key: cacheKey });
      if (!isStillTarget(userId)) return;
      if (cachedStr) {
        try {
          const cachedData = JSON.parse(cachedStr);
          if (Array.isArray(cachedData) && cachedData.length > 0) {
            createdAvatars.value = cachedData;
            isLoadingAvatars.value = false;
          }
        } catch(e) {}
      }

      const res = await VrcApi.request('/avatars', { method: 'GET', params: { userId: userId, n: 60 } });
      if (!isStillTarget(userId)) return;
      if (Array.isArray(res)) {
        createdAvatars.value = res;
        DbApi.saveApiCache({ key: cacheKey, data: JSON.stringify(res) }).catch(() => {});
      }
    } catch (err) {
      if ((err as any)?.code !== 'VRCHAT_PERMISSION_DENIED') {
        console.warn("Failed to fetch created avatars", err);
      }
    } finally {
      if (isStillTarget(userId)) isLoadingAvatars.value = false;
    }
  };

    const fetchActivityLogs = async (userId: string) => {
    isLoadingActivity.value = true;
    try {
      const logs = await DbApi.getFriendLogs({ userId, limit: 100 });
      if (!isStillTarget(userId)) return;
      if (Array.isArray(logs)) {
        activityLogs.value = logs;
      }
    } catch (err) {
      console.warn("Failed to fetch activity logs", err);
    } finally {
      if (isStillTarget(userId)) isLoadingActivity.value = false;
    }
  };

  const fetchMutualFriends = async (userId: string) => {
    isLoadingMutual.value = true;
    try {
      const res = await VrcApi.getMutualFriends({ userId });
      if (!isStillTarget(userId)) return;
      if (Array.isArray(res)) {
        mutualFriends.value = res;
      } else {
        mutualFriends.value = [];
      }
    } catch (err) {
      console.warn("Failed to fetch mutual friends", err);
      if (isStillTarget(userId)) mutualFriends.value = [];
    } finally {
      if (isStillTarget(userId)) isLoadingMutual.value = false;
    }
  };

  const fetchMutualGroups = async (userId: string) => {
    isLoadingMutualGroups.value = true;
    try {
      const res = await VrcApi.getMutualGroups({ userId });
      if (!isStillTarget(userId)) return;
      if (Array.isArray(res)) mutualGroups.value = res;
      else mutualGroups.value = [];
    } catch (err) {
      console.warn("Failed to fetch mutual groups", err);
      if (isStillTarget(userId)) mutualGroups.value = [];
    } finally {
      if (isStillTarget(userId)) isLoadingMutualGroups.value = false;
    }
  };

  const checkIsFavorite = async (userId: string) => {
    try {
      const favorites = await VrcApi.getFavorites({ type: 'friend', n: 100 });
      if (!isStillTarget(userId)) return;
      if (Array.isArray(favorites)) {
        const fav = favorites.find((f: any) => f.favoriteId === userId);
        favoriteId.value = fav ? fav.id : null;
      }
    } catch (err) {
      console.warn("Failed to check if user is favorited", err);
    }
  };

  const toggleFavorite = async () => {
    if (!targetUserId.value) return;
    const userId = targetUserId.value;
    try {
      if (favoriteId.value) {
        await VrcApi.deleteFavorite(favoriteId.value);
        if (!isStillTarget(userId)) return;
        favoriteId.value = null;
      } else {
        const res = await VrcApi.addFavorite({
          type: 'friend',
          favoriteId: targetUserId.value,
          tags: ['group_0'] // Default group
        });
        if (!isStillTarget(userId)) return;
        if (res && res.id) {
          favoriteId.value = res.id;
        }
      }
    } catch (err) {
      console.warn("Failed to toggle favorite", err);
    }
  };

  const socialLinks = computed(() => {
    if (!baseInfo.value?.bio) return [];
    const bio = baseInfo.value.bio;
    const links: { type: string, url: string }[] = [];
    
    const patterns = [
      { type: 'twitter', regex: /(?:https?:\/\/)?(?:www\.)?twitter\.com\/([a-zA-Z0-9_]+)/gi },
      { type: 'x', regex: /(?:https?:\/\/)?(?:www\.)?x\.com\/([a-zA-Z0-9_]+)/gi },
      { type: 'youtube', regex: /(?:https?:\/\/)?(?:www\.)?youtube\.com\/(?:@|c\/|channel\/)?([a-zA-Z0-9_-]+)/gi },
      { type: 'twitch', regex: /(?:https?:\/\/)?(?:www\.)?twitch\.tv\/([a-zA-Z0-9_]+)/gi },
      { type: 'discord', regex: /(?:https?:\/\/)?(?:www\.)?discord(?:app)?\.(?:com\/invite|gg)\/([a-zA-Z0-9_-]+)/gi },
      { type: 'github', regex: /(?:https?:\/\/)?(?:www\.)?github\.com\/([a-zA-Z0-9_-]+)/gi },
      { type: 'patreon', regex: /(?:https?:\/\/)?(?:www\.)?patreon\.com\/([a-zA-Z0-9_-]+)/gi }
    ];

    patterns.forEach(p => {
      let match;
      while ((match = p.regex.exec(bio)) !== null) {
        links.push({ type: p.type, url: match[0] });
      }
    });

    return links;
  });

  return {
    isOpen,
    targetUserId,
    baseInfo,
    myId,
    mutualFriends,
    mutualGroups,
    groups,
    createdWorlds,
    favoriteWorlds,
    createdAvatars,
    localNote,
    isSavingNote,
    isLoadingBase,
    isLoadingMutual,
    isLoadingGroups,
    isLoadingMutualGroups,
    isLoadingWorlds,
    isLoadingFavWorlds,
    isLoadingAvatars,
    isLoadingActivity,
    activityLogs,
    isFavorite,
    socialLinks,
    openProfile,
    closeProfile,
    fetchMutualFriends,
    fetchMutualGroups,
    fetchGroups,
    fetchCreatedWorlds,
    fetchFavoriteWorlds,
    fetchCreatedAvatars,
    fetchActivityLogs,
    saveLocalNote,
    toggleFavorite
  };
});
