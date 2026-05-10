import { defineStore } from 'pinia';
import { ref } from 'vue';
import { VrcApi, DbApi } from '../api';
import type { VrcWorld } from '../types/vrc';

export const useEntityModalStore = defineStore('entityModal', () => {
  const isWorldOpen = ref(false);
  const isGroupOpen = ref(false);
  const isAvatarOpen = ref(false);

  const selectedWorld = ref<any>(null);
  const selectedGroup = ref<any>(null);
  const selectedAvatar = ref<any>(null);

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
      
      const favs: any = await DbApi.getFavoriteWorlds();
      isWorldFavorited.value = favs.some((f: any) => f.world_id === worldId);
    } catch (err: any) {
      errorMsg.value = err.message || err;
    } finally {
      loadingWorld.value = false;
    }
  };

  const closeWorld = () => { isWorldOpen.value = false; };

  const toggleFavoriteWorld = async () => {
    if (!selectedWorld.value) return;
    try {
      if (isWorldFavorited.value) {
        await DbApi.removeFavoriteWorld({ worldId: selectedWorld.value.id });
        isWorldFavorited.value = false;
      } else {
        await DbApi.addFavoriteWorld({
          worldId: selectedWorld.value.id,
          name: selectedWorld.value.name,
          imageUrl: selectedWorld.value.imageUrl || selectedWorld.value.thumbnailImageUrl || null
        });
        isWorldFavorited.value = true;
      }
    } catch (e) {
      console.error("Favorite toggle failed:", e);
    }
  };

  const openAvatar = async (avatar: any) => {
    isAvatarOpen.value = true;
    selectedAvatar.value = avatar;
    isAvatarFavorited.value = false;
    try {
      const favs: any = await DbApi.getFavoriteAvatars();
      isAvatarFavorited.value = favs.some((f: any) => f.avatar_id === avatar.id);
    } catch (e) {}
  };

  const closeAvatar = () => { isAvatarOpen.value = false; };

  const toggleFavoriteAvatar = async () => {
    if (!selectedAvatar.value) return;
    try {
      if (isAvatarFavorited.value) {
        await DbApi.removeFavoriteAvatar({ avatarId: selectedAvatar.value.id });
        isAvatarFavorited.value = false;
      } else {
        await DbApi.addFavoriteAvatar({
          avatarId: selectedAvatar.value.id,
          name: selectedAvatar.value.name,
          imageUrl: selectedAvatar.value.imageUrl || selectedAvatar.value.thumbnailImageUrl || null,
          authorId: selectedAvatar.value.authorId,
          authorName: selectedAvatar.value.authorName
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

  const openGroup = async (groupOrId: any) => {
    isGroupOpen.value = true;
    loadingGroup.value = true;
    errorMsg.value = '';
    
    groupMembers.value = [];
    groupRoles.value = [];
    groupJoinRequests.value = [];
    groupPermissions.value = [];

    try {
      const groupId = typeof groupOrId === 'string' ? groupOrId : groupOrId.id;
      if (typeof groupOrId !== 'string') {
        selectedGroup.value = groupOrId; // Optimistic update
      }

      const fullGroup = await VrcApi.getGroup({ groupId });
      selectedGroup.value = fullGroup;

      fetchGroupAdminData(groupId);
    } catch (err: any) {
      errorMsg.value = err.message || err;
    } finally {
      loadingGroup.value = false;
    }
  };

  const fetchGroupAdminData = async (groupId: string) => {
    loadingGroupData.value = true;
    try {
      const permsRes: any = await VrcApi.getUserGroupPermissions({ userId: 'me' });
      const currentGroupPerms = permsRes.find((p: any) => p.groupId === groupId);
      if (currentGroupPerms) {
        groupPermissions.value = currentGroupPerms.permissions || [];
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

  const closeGroup = () => { isGroupOpen.value = false; };

  return {
    isWorldOpen, isGroupOpen, isAvatarOpen,
    selectedWorld, selectedGroup, selectedAvatar,
    isWorldFavorited, isAvatarFavorited,
    loadingWorld, loadingGroup, loadingAvatar, errorMsg,
    groupMembers, groupRoles, groupJoinRequests, groupPermissions, loadingGroupData,
    openWorld, closeWorld, toggleFavoriteWorld,
    openAvatar, closeAvatar, toggleFavoriteAvatar,
    openGroup, closeGroup, fetchGroupAdminData
  };
});
