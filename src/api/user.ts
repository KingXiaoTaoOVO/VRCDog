import { request } from './request';

export const UserApi = {
  getUser: (params: { userId: string } | string) => {
    const userId = typeof params === 'string' ? params : params.userId;
    return request(`/users/${userId}`);
  },

  getUsers: (params: { search?: string, n?: number, offset?: number }) =>
    request('/users', { method: 'GET', params }),

  addUserTags: (params: { tags: string[] }) =>
    request('/auth/user/addTags', { method: 'POST', params }),

  removeUserTags: (params: { tags: string[] }) =>
    request('/auth/user/removeTags', { method: 'POST', params }),

  getUserFeedback: (params: { userId: string }) =>
    request(`/users/${params.userId}/feedback`, { method: 'GET', params: { n: 100 } }),

  saveCurrentUser: (params: any) =>
    request('/auth/user', { method: 'PUT', params }),

  getUserNotes: (params: { offset?: number, n?: number }) =>
    request('/userNotes', { method: 'GET', params }),

  getMutualCounts: (params: { userId: string }) =>
    request(`/users/${params.userId}/mutuals`),

  getMutualFriends: (params: { userId: string, n?: number, offset?: number }) =>
    request(`/users/${params.userId}/mutuals/friends`, { method: 'GET', params }),

  getMutualGroups: (params: { userId: string, n?: number, offset?: number }) =>
    request(`/users/${params.userId}/mutuals/groups`, { method: 'GET', params }),

  updateStatus: (params: { userId: string, status: string, statusDescription: string }) =>
    request(`/users/${params.userId}`, { method: 'PUT', params: { status: params.status, statusDescription: params.statusDescription } }),
};
