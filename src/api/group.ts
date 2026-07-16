import { request } from './request';

async function resolveCurrentUserId(userId?: string) {
  if (userId && userId !== 'me') return userId;
  const currentUser: any = await request('/auth/user', { method: 'GET' });
  if (!currentUser?.id) throw new Error('无法获取当前用户 ID');
  return currentUser.id;
}

export const GroupApi = {
  getGroup: (params: { groupId: string, includeRoles?: boolean }) =>
    request(`/groups/${params.groupId}`, { method: 'GET', params: { includeRoles: params.includeRoles || false } }),

  createGroup: (params: Record<string, unknown>) =>
    request('/groups', { method: 'POST', params }),

  updateGroup: (params: { id?: string, groupId?: string, [key: string]: any }) => {
    const groupId = params.id || params.groupId;
    if (!groupId) throw new Error('缺少群组 ID');
    return request(`/groups/${groupId}`, { method: 'PUT', params });
  },

  editGroup: (params: { id?: string, groupId?: string, [key: string]: any }) => {
    const groupId = params.id || params.groupId;
    if (!groupId) throw new Error('Missing group id');
    return request(`/groups/${groupId}`, { method: 'PUT', params });
  },

  getGroups: (params: { userId: string }) =>
    request(`/users/${params.userId}/groups`, { method: 'GET' }),

  getRepresentedGroup: (params: { userId: string }) =>
    request(`/users/${params.userId}/groups/represented`, { method: 'GET' }),

  getGroupAnnouncement: (params: { groupId: string }) =>
    request(`/groups/${params.groupId}/announcement`, { method: 'GET' }),

  setGroupAnnouncement: (params: { groupId: string, [key: string]: any }) =>
    request(`/groups/${params.groupId}/announcement`, { method: 'PUT', params }),

  setGroupRepresentation: (groupIdOrParams: string | { groupId?: string; id?: string; [key: string]: any }, params: Record<string, unknown> = {}) => {
    const groupId = typeof groupIdOrParams === 'string'
      ? groupIdOrParams
      : groupIdOrParams.groupId || groupIdOrParams.id;
    if (!groupId) throw new Error('缺少群组 ID');
    const body = typeof groupIdOrParams === 'string' ? params : { ...groupIdOrParams, ...params };
    return request(`/groups/${groupId}/representation`, { method: 'PUT', params: body });
  },

  joinGroup: (params: { groupId: string }) =>
    request(`/groups/${params.groupId}/join`, { method: 'POST' }),

  leaveGroup: (params: { groupId: string }) =>
    request(`/groups/${params.groupId}/leave`, { method: 'POST' }),

  cancelGroupRequest: (params: { groupId: string }) =>
    request(`/groups/${params.groupId}/requests`, { method: 'DELETE' }),

  getGroupMembers: (params: { groupId: string, n?: number, offset?: number }) =>
    request(`/groups/${params.groupId}/members`, { method: 'GET', params }),

  getGroupMembersSearch: (params: { groupId: string, query?: string, n?: number, offset?: number }) =>
    request(`/groups/${params.groupId}/members/search`, { method: 'GET', params }),

  getGroupMember: (params: { groupId: string, userId: string }) =>
    request(`/groups/${params.groupId}/members/${params.userId}`, { method: 'GET' }),

  setGroupMemberProps: (userId: string, groupId: string, params: Record<string, unknown>) =>
    request(`/groups/${groupId}/members/${userId}`, { method: 'PUT', params }),

  kickGroupMember: (params: { groupId: string, userId: string }) =>
    request(`/groups/${params.groupId}/members/${params.userId}`, { method: 'DELETE' }),

  getGroupRoles: (params: { groupId: string }) =>
    request(`/groups/${params.groupId}/roles`, { method: 'GET' }),

  getGroupRoleTemplates: () =>
    request('/groups/roleTemplates', { method: 'GET' }),

  getRoleTemplates: () =>
    request('/groups/roleTemplates', { method: 'GET' }),

  createGroupRole: (params: { groupId: string, [key: string]: any }) =>
    request(`/groups/${params.groupId}/roles`, { method: 'POST', params }),

  editGroupRole: (params: { groupId: string, roleId: string, [key: string]: any }) =>
    request(`/groups/${params.groupId}/roles/${params.roleId}`, { method: 'PUT', params }),

  deleteGroupRole: (params: { groupId: string, roleId: string }) =>
    request(`/groups/${params.groupId}/roles/${params.roleId}`, { method: 'DELETE' }),

  addGroupMemberRole: (params: { groupId: string, userId: string, roleId: string }) =>
    request(`/groups/${params.groupId}/members/${params.userId}/roles/${params.roleId}`, { method: 'PUT' }),

  removeGroupMemberRole: (params: { groupId: string, userId: string, roleId: string }) =>
    request(`/groups/${params.groupId}/members/${params.userId}/roles/${params.roleId}`, { method: 'DELETE' }),

  getGroupPosts: (params: { groupId: string, n?: number, offset?: number }) =>
    request(`/groups/${params.groupId}/posts`, { method: 'GET', params }),

  createGroupPost: (params: { groupId: string, title: string, text: string, roleIds?: string[], sendNotification?: boolean }) =>
    request(`/groups/${params.groupId}/posts`, { method: 'POST', params }),

  updateGroupPost: (params: { groupId: string, postId: string, [key: string]: any }) =>
    request(`/groups/${params.groupId}/posts/${params.postId}`, { method: 'PUT', params }),

  editGroupPost: (params: { groupId: string, postId: string, [key: string]: any }) =>
    request(`/groups/${params.groupId}/posts/${params.postId}`, { method: 'PUT', params }),

  deleteGroupPost: (params: { groupId: string, postId: string }) =>
    request(`/groups/${params.groupId}/posts/${params.postId}`, { method: 'DELETE' }),

  getGroupLogs: (params: { groupId: string, n?: number, offset?: number, eventTypes?: string[] }) =>
    request(`/groups/${params.groupId}/auditLogs`, { method: 'GET', params }),

  getGroupAuditLogTypes: (params: { groupId: string }) =>
    request(`/groups/${params.groupId}/auditLogTypes`, { method: 'GET' }),

  getGroupInvites: (params: { groupId: string, n?: number, offset?: number }) =>
    request(`/groups/${params.groupId}/invites`, { method: 'GET', params }),

  sendGroupInvite: (params: { groupId: string, userId: string }) =>
    request(`/groups/${params.groupId}/invites`, { method: 'POST', params: { userId: params.userId } }),

  deleteGroupInvite: (params: { groupId: string, userId: string }) =>
    request(`/groups/${params.groupId}/invites/${params.userId}`, { method: 'DELETE' }),

  deleteSentGroupInvite: (params: { groupId: string, userId: string }) =>
    request(`/groups/${params.groupId}/invites/${params.userId}`, { method: 'DELETE' }),

  getGroupBans: (params: { groupId: string, n?: number, offset?: number }) =>
    request(`/groups/${params.groupId}/bans`, { method: 'GET', params }),

  banGroupMember: (params: { groupId: string, userId: string }) =>
    request(`/groups/${params.groupId}/bans`, { method: 'POST', params: { userId: params.userId } }),

  unbanGroupMember: (params: { groupId: string, userId: string }) =>
    request(`/groups/${params.groupId}/bans/${params.userId}`, { method: 'DELETE' }),

  blockGroup: (params: { groupId: string }) =>
    request(`/groups/${params.groupId}/block`, { method: 'POST' }),

  unblockGroup: (params: { groupId: string }) =>
    request(`/groups/${params.groupId}/block`, { method: 'DELETE' }),

  getBlockedGroups: async (params: { userId?: string, membershipStatus: 'invited' | 'requested' | 'userblocked' }) => {
    const userId = await resolveCurrentUserId(params.userId);
    return request(`/users/${userId}/groups/${params.membershipStatus}`, { method: 'GET' });
  },

  getUserGroupPermissions: async (params: { userId?: string } = {}) => {
    const userId = await resolveCurrentUserId(params.userId);
    return request(`/users/${userId}/groups/permissions`, { method: 'GET' });
  },

  getGroupPermissions: async (params: { userId?: string } = {}) => {
    const userId = await resolveCurrentUserId(params.userId);
    return request(`/users/${userId}/groups/permissions`, { method: 'GET' });
  },

  getGroupInstances: async (params: { groupId: string, userId?: string }) => {
    const userId = await resolveCurrentUserId(params.userId);
    return request(`/users/${userId}/instances/groups/${params.groupId}`, { method: 'GET' });
  },

  getUsersGroupInstances: async (params: { userId?: string } = {}) => {
    const userId = await resolveCurrentUserId(params.userId);
    return request(`/users/${userId}/instances/groups`, { method: 'GET' });
  },

  getGroupJoinRequests: (params: { groupId: string }) =>
    request(`/groups/${params.groupId}/requests`, { method: 'GET' }),

  respondGroupJoinRequest: (params: { groupId: string, userId?: string, requestId?: string, action: 'accept' | 'reject', block?: boolean }) => {
    const userId = params.userId || params.requestId;
    if (!userId) throw new Error('缺少群组请求用户 ID');
    return request(`/groups/${params.groupId}/requests/${userId}`, {
      method: 'PUT',
      params: {
        action: params.action,
        ...(params.block ? { block: true } : {})
      }
    });
  },

  acceptGroupInviteRequest: (params: { groupId: string, userId?: string, requestId?: string }) => {
    const userId = params.userId || params.requestId;
    if (!userId) throw new Error('Missing group request user id');
    return request(`/groups/${params.groupId}/requests/${userId}`, {
      method: 'PUT',
      params: { action: 'accept' },
    });
  },

  rejectGroupInviteRequest: (params: { groupId: string, userId?: string, requestId?: string }) => {
    const userId = params.userId || params.requestId;
    if (!userId) throw new Error('Missing group request user id');
    return request(`/groups/${params.groupId}/requests/${userId}`, {
      method: 'PUT',
      params: { action: 'reject' },
    });
  },

  blockGroupInviteRequest: (params: { groupId: string, userId?: string, requestId?: string }) => {
    const userId = params.userId || params.requestId;
    if (!userId) throw new Error('Missing group request user id');
    return request(`/groups/${params.groupId}/requests/${userId}`, {
      method: 'PUT',
      params: { action: 'reject', block: true },
    });
  },

  deleteBlockedGroupRequest: (params: { groupId: string, userId: string }) =>
    request(`/groups/${params.groupId}/members/${params.userId}`, { method: 'DELETE' }),

  getGroupCalendar: (params: { groupId: string } | string) => {
    const groupId = typeof params === 'string' ? params : params.groupId;
    return request(`/calendar/${groupId}`, { method: 'GET' });
  },

  getGroupCalendarEvent: (params: { groupId: string, eventId: string }) =>
    request(`/calendar/${params.groupId}/${params.eventId}`, { method: 'GET' }),

  getGroupCalendars: (params?: { n?: number, offset?: number }) =>
    request('/calendar', { method: 'GET', params }),

  getFollowingGroupCalendars: (params?: { n?: number, offset?: number }) =>
    request('/calendar/following', { method: 'GET', params }),

  getFeaturedGroupCalendars: (params?: { n?: number, offset?: number }) =>
    request('/calendar/featured', { method: 'GET', params }),

  followGroupEvent: (params: { groupId: string, eventId: string, isFollowing: boolean }) =>
    request(`/calendar/${params.groupId}/${params.eventId}/follow`, {
      method: 'POST',
      params: { isFollowing: params.isFollowing },
    }),

  deleteGroupEvent: (params: { groupId: string, eventId: string }) =>
    request(`/calendar/${params.groupId}/${params.eventId}`, { method: 'DELETE' }),

  createGroupEvent: (params: { groupId: string, [key: string]: any }) =>
    request(`/calendar/${params.groupId}/event`, { method: 'POST', params }),

  editGroupEvent: (params: { groupId: string, eventId: string, [key: string]: any }) =>
    request(`/calendar/${params.groupId}/${params.eventId}`, { method: 'PUT', params }),

  searchGroups: (params: { query: string, n?: number, offset?: number, order?: string, sortBy?: string }) =>
    request('/groups', {
      method: 'GET',
      params: {
        query: params.query,
        n: params.n,
        offset: params.offset,
        order: params.order || 'descending',
        sortBy: params.sortBy || 'created'
      }
    }),

  groupSearch: (params: { query: string, n?: number, offset?: number, order?: string, sortBy?: string }) =>
    request('/groups', { method: 'GET', params }),

  getGroupGallery: (params: { groupId: string, galleryId: string, n?: number, offset?: number }) =>
    request(`/groups/${params.groupId}/galleries/${params.galleryId}`, {
      method: 'GET',
      params: { n: params.n, offset: params.offset },
    }),

  groupStrictsearch: (params: { query: string, n?: number, offset?: number }) =>
    request('/groups/strictsearch', { method: 'GET', params }),
};
