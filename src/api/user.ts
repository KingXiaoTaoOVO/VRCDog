import { request } from './request';

async function resolveCurrentUserId(userId?: string) {
  if (userId) return userId;
  const currentUser: any = await request('/auth/user', { method: 'GET' });
  if (!currentUser?.id) {
    throw new Error('无法获取当前用户 ID');
  }
  return currentUser.id;
}

export const UserApi = {
  getUser: (params: { userId: string } | string) => {
    const userId = typeof params === 'string' ? params : params.userId;
    return request(`/users/${userId}`);
  },

  getUsers: (params: { search?: string, n?: number, offset?: number }) =>
    request('/users', { method: 'GET', params }),

  addUserTags: async (params: { userId?: string, tags: string[] }) => {
    const userId = await resolveCurrentUserId(params.userId);
    return request(`/users/${userId}/addTags`, { method: 'POST', params: { tags: params.tags } });
  },

  removeUserTags: async (params: { userId?: string, tags: string[] }) => {
    const userId = await resolveCurrentUserId(params.userId);
    return request(`/users/${userId}/removeTags`, { method: 'POST', params: { tags: params.tags } });
  },

  getUserFeedback: (params: { userId: string }) =>
    request(`/users/${params.userId}/feedback`, { method: 'GET', params: { n: 100 } }),

  saveCurrentUser: async (params: any) => {
    const userId = await resolveCurrentUserId(params?.userId);
    const { userId: _userId, ...body } = params || {};
    return request(`/users/${userId}`, { method: 'PUT', params: body });
  },

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
