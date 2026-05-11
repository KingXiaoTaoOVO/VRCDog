import { request } from './request';

export const GroupApi = {
  getGroup: (params: { groupId: string, includeRoles?: boolean }) =>
    request(`/groups/${params.groupId}`, { method: 'GET', params: { includeRoles: params.includeRoles || false } }),

  getGroups: (params: { userId: string }) =>
    request(`/users/${params.userId}/groups`, { method: 'GET' }),

  getRepresentedGroup: (params: { userId: string }) =>
    request(`/users/${params.userId}/groups/represented`, { method: 'GET' }),

  joinGroup: (params: { groupId: string }) =>
    request(`/groups/${params.groupId}/join`, { method: 'POST' }),

  leaveGroup: (params: { groupId: string }) =>
    request(`/groups/${params.groupId}/leave`, { method: 'POST' }),

  getGroupMembers: (params: { groupId: string, n?: number, offset?: number }) =>
    request(`/groups/${params.groupId}/members`, { method: 'GET', params }),

  getGroupRoles: (params: { groupId: string }) =>
    request(`/groups/${params.groupId}/roles`, { method: 'GET' }),

  getGroupPosts: (params: { groupId: string, n?: number, offset?: number }) =>
    request(`/groups/${params.groupId}/posts`, { method: 'GET', params }),

  createGroupPost: (params: { groupId: string, title: string, text: string, roleIds?: string[], sendNotification?: boolean }) =>
    request(`/groups/${params.groupId}/posts`, { method: 'POST', params }),

  getGroupLogs: (params: { groupId: string, n?: number, offset?: number }) =>
    request(`/groups/${params.groupId}/auditLogs`, { method: 'GET', params }),

  sendGroupInvite: (params: { groupId: string, userId: string }) =>
    request(`/groups/${params.groupId}/invites`, { method: 'POST', params: { userId: params.userId } }),

  kickGroupMember: (params: { groupId: string, userId: string }) =>
    request(`/groups/${params.groupId}/members/${params.userId}`, { method: 'DELETE' }),

  banGroupMember: (params: { groupId: string, userId: string }) =>
    request(`/groups/${params.groupId}/bans`, { method: 'POST', params: { userId: params.userId } }),

  unbanGroupMember: (params: { groupId: string, userId: string }) =>
    request(`/groups/${params.groupId}/bans/${params.userId}`, { method: 'DELETE' }),

  getUserGroupPermissions: (params: { userId: string }) =>
    request(`/users/${params.userId}/groups/permissions`, { method: 'GET' }),

  getGroupJoinRequests: (params: { groupId: string }) =>
    request(`/groups/${params.groupId}/requests`, { method: 'GET' }),

  respondGroupJoinRequest: (params: { groupId: string, requestId: string, action: 'accept' | 'reject' }) =>
    request(`/groups/${params.groupId}/requests/${params.requestId}`, { method: 'PUT', params: { action: params.action } }),

  searchGroups: (params: { query: string, n?: number, offset?: number }) =>
    request('/groups', { method: 'GET', params: { search: params.query, n: params.n, offset: params.offset } }),
};
