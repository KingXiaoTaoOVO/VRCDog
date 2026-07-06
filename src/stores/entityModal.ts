import { defineStore } from 'pinia';
import { ref } from 'vue';
import { VrcApi, DbApi } from '../api';
import type { VrcWorld, VrcAvatar, VrcGroup } from './userProfile';

export const useEntityModalStore = defineStore('entityModal', () => {
  const isWorldOpen = ref(false);
  const isGroupOpen = ref(false);
  const isAvatarOpen = ref(false);

  const selectedWorld = ref<VrcWorld | null>(null);
  const selectedGroup = ref<VrcGroup | null>(null);
  const selectedAvatar = ref<VrcAvatar | null>(null);

  const isWorldFavorited = ref(false);
  const isAvatarFavorited = ref(false);
  
  const loadingWorld = ref(false);
  const loadingGroup = ref(false);
  const loadingAvatar = ref(false);

  const errorMsg = ref('');

  const openWorld = async (worldId: string) => {
    isWorldOpen.value = true;
    loadingWorld.value = true;
    selectedWorld.value = null;
    isWorldFavorited.value = false;
    errorMsg.value = '';
    
    try {
      const world = await VrcApi.getWorld({ worldId });
      selectedWorld.value = world;
      
      const favs = await DbApi.getFavoriteWorlds() as Array<{world_id: string}>;
      isWorldFavorited.value = favs.some((f) => f.world_id === worldId);
    } catch (err: unknown) {
      errorMsg.value = (err as Error).message || String(err);
    } finally {
      loadingWorld.value = false;
    }
  };

  const closeWorld = () => { isWorldOpen.value = false; };

  const toggleFavoriteWorld = async () => {
    if (!selectedWorld.value) return;
    try {
      if (isWorldFavorited.value) {
        await DbApi.removeFavoriteWorld({ worldId: selectedWorld.value.id as string });
        isWorldFavorited.value = false;
      } else {
        await DbApi.addFavoriteWorld({
          worldId: selectedWorld.value.id as string,
          name: selectedWorld.value.name as string,
          imageUrl: (selectedWorld.value.imageUrl || selectedWorld.value.thumbnailImageUrl || null) as string | null
        });
        isWorldFavorited.value = true;
      }
    } catch (e) {
      console.error("Favorite toggle failed:", e);
    }
  };

  const openAvatar = async (avatar: VrcAvatar) => {
    isAvatarOpen.value = true;
    selectedAvatar.value = avatar;
    isAvatarFavorited.value = false;
    try {
      const favs = await DbApi.getFavoriteAvatars() as Array<{avatar_id: unknown}>;
      isAvatarFavorited.value = favs.some((f) => f.avatar_id === avatar.id);
    } catch (e) {}
  };

  const closeAvatar = () => { isAvatarOpen.value = false; };

  const toggleFavoriteAvatar = async () => {
    if (!selectedAvatar.value) return;
    try {
      if (isAvatarFavorited.value) {
        await DbApi.removeFavoriteAvatar({ avatarId: selectedAvatar.value.id as string });
        isAvatarFavorited.value = false;
      } else {
        await DbApi.addFavoriteAvatar({
          avatarId: selectedAvatar.value.id as string,
          name: selectedAvatar.value.name as string,
          imageUrl: (selectedAvatar.value.imageUrl || selectedAvatar.value.thumbnailImageUrl || null) as string | null,
          authorId: selectedAvatar.value.authorId as string,
          authorName: selectedAvatar.value.authorName as string
        });
        isAvatarFavorited.value = true;
      }
    } catch (e) {}
  };

  const groupMembers = ref<any[]>([]);
  const groupRoles = ref<any[]>([]);
  const groupJoinRequests = ref<any[]>([]);
  const groupPermissions = ref<string[]>([]);
  const loadingGroupData = ref(false);

  const openGroup = async (groupOrId: Record<string, unknown> | string) => {
    isGroupOpen.value = true;
    loadingGroup.value = true;
    errorMsg.value = '';
    
    groupMembers.value = [];
    groupRoles.value = [];
    groupJoinRequests.value = [];
    groupPermissions.value = [];

    try {
      const groupId = typeof groupOrId === 'string' ? groupOrId : groupOrId.id as string;
      if (typeof groupOrId !== 'string') {
        selectedGroup.value = groupOrId as VrcGroup; // Optimistic update
      }

      const fullGroup = await VrcApi.getGroup({ groupId });
      selectedGroup.value = fullGroup as VrcGroup;

      fetchGroupAdminData(groupId as string);
    } catch (err: unknown) {
      errorMsg.value = (err as Error).message || String(err);
    } finally {
      loadingGroup.value = false;
    }
  };

  const fetchGroupAdminData = async (groupId: string) => {
    loadingGroupData.value = true;
    try {
      const permsRes = await VrcApi.getUserGroupPermissions({ userId: 'me' }) as any;
      if (Array.isArray(permsRes)) {
        const currentGroupPerms = permsRes.find((p) => p.groupId === groupId || p.id === groupId);
        groupPermissions.value = currentGroupPerms?.permissions || [];
      } else if (permsRes && Array.isArray(permsRes[groupId])) {
        groupPermissions.value = permsRes[groupId];
      } else if (permsRes?.[groupId]?.permissions) {
        groupPermissions.value = permsRes[groupId].permissions || [];
      }

      const [membersRes, rolesRes] = await Promise.allSettled([
        VrcApi.getGroupMembers({ groupId }),
        VrcApi.getGroupRoles({ groupId })
      ]);

      if (membersRes.status === 'fulfilled') groupMembers.value = membersRes.value;
      if (rolesRes.status === 'fulfilled') groupRoles.value = rolesRes.value;

      if (groupPermissions.value.includes('group-join-requests-manage')) {
        const requestsRes = await VrcApi.getGroupJoinRequests({ groupId });
        groupJoinRequests.value = requestsRes;
      }

    } catch (err) {
      console.warn("Failed to fetch group admin data", err);
    } finally {
      loadingGroupData.value = false;
    }
  };

  const respondGroupJoinRequest = async (params: { groupId: string, requestId: string, action: 'accept' | 'reject' }) => {
    try {
      await VrcApi.respondGroupJoinRequest(params);
      // Refresh join requests after action
      const requestsRes = await VrcApi.getGroupJoinRequests({ groupId: params.groupId });
      groupJoinRequests.value = requestsRes;
    } catch (err) {
      console.error("Failed to respond to group join request", err);
      throw err;
    }
  };

  const closeGroup = () => { isGroupOpen.value = false; };

  return {
    isWorldOpen, isGroupOpen, isAvatarOpen,
    selectedWorld, selectedGroup, selectedAvatar,
    isWorldFavorited, isAvatarFavorited,
    loadingWorld, loadingGroup, loadingAvatar, errorMsg,
    groupMembers, groupRoles, groupJoinRequests, groupPermissions, loadingGroupData,
    openWorld, closeWorld, toggleFavoriteWorld,
    openAvatar, closeAvatar, toggleFavoriteAvatar,
    openGroup, closeGroup, fetchGroupAdminData, respondGroupJoinRequest
  };
});
