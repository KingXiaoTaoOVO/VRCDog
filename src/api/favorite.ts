import { request } from './request';

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

  removeFavorite: (favoriteId: string) =>
    request(`/favorites/${favoriteId}`, { method: 'DELETE' }),

  getFavoriteLimits: () =>
    request('/auth/user/favoritelimits', { method: 'GET' }),
};
