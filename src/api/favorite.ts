import { request } from './request';

async function resolveCurrentUserId(userId?: string) {
  if (userId && userId !== 'me') return userId;
  const currentUser: any = await request('/auth/user', { method: 'GET' });
  if (!currentUser?.id) throw new Error('Unable to resolve current VRChat user id');
  return currentUser.id;
}

export const FavoriteApi = {
  getFavorites: (params: { n?: number, offset?: number, type?: string }) =>
    request('/favorites', { method: 'GET', params }),

  getFavoriteWorlds: (params: { n?: number, offset?: number, tag?: string }) =>
    request('/worlds/favorites', { method: 'GET', params }),

  getFavoriteAvatars: (params: { n?: number, offset?: number, tag?: string }) =>
    request('/avatars/favorites', { method: 'GET', params }),

  getFavoriteGroups: (params?: { n?: number, offset?: number }) =>
    request('/favorite/groups', { method: 'GET', params }),

  addFavorite: (params: { type: 'world' | 'avatar' | 'friend', favoriteId: string, tags: string[] }) =>
    request('/favorites', { method: 'POST', params }),

  deleteFavorite: (params: string | { objectId?: string; favoriteId?: string; id?: string }) => {
    const favoriteId = typeof params === 'string'
      ? params
      : params.objectId || params.favoriteId || params.id;
    if (!favoriteId) throw new Error('Missing favorite id');
    return request(`/favorites/${favoriteId}`, { method: 'DELETE' });
  },

  removeFavorite: (favoriteId: string) =>
    FavoriteApi.deleteFavorite(favoriteId),

  saveFavoriteGroup: async (params: {
    type: string;
    group: string;
    userId?: string;
    displayName?: string;
    visibility?: string;
    [key: string]: unknown;
  }) => {
    const userId = await resolveCurrentUserId(params.userId);
    return request(`/favorite/group/${params.type}/${params.group}/${userId}`, { method: 'PUT', params });
  },

  clearFavoriteGroup: async (params: { type: string; group: string; userId?: string; [key: string]: unknown }) => {
    const userId = await resolveCurrentUserId(params.userId);
    return request(`/favorite/group/${params.type}/${params.group}/${userId}`, { method: 'DELETE', params });
  },

  getFavoriteLimits: () =>
    request('/auth/user/favoritelimits', { method: 'GET' }),
};
