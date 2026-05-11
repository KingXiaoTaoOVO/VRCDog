import { request } from './request';

export const AvatarApi = {
  getAvatar: (params: { avatarId: string } | string) => {
    const avatarId = typeof params === 'string' ? params : params.avatarId;
    return request(`/avatars/${avatarId}`);
  },

  getAvatars: (params: { search?: string, n?: number, offset?: number, user?: string, releaseStatus?: string, sort?: string }) =>
    request('/avatars', { method: 'GET', params }),

  saveAvatar: (params: { id: string, [key: string]: any }) =>
    request(`/avatars/${params.id}`, { method: 'PUT', params }),

  selectAvatar: (params: { avatarId: string }) =>
    request(`/avatars/${params.avatarId}/select`, { method: 'PUT', params }),

  selectFallbackAvatar: (params: { avatarId: string }) =>
    request(`/avatars/${params.avatarId}/selectfallback`, { method: 'PUT', params }),

  deleteAvatar: (params: { avatarId: string }) =>
    request(`/avatars/${params.avatarId}`, { method: 'DELETE' }),

  createImposter: (params: { avatarId: string }) =>
    request(`/avatars/${params.avatarId}/impostor/enqueue`, { method: 'POST' }),

  deleteImposter: (params: { avatarId: string }) =>
    request(`/avatars/${params.avatarId}/impostor`, { method: 'DELETE' }),

  getAvailableAvatarStyles: () =>
    request('/avatarStyles', { method: 'GET' }),

  getLicensedAvatars: (params: { n?: number, offset?: number }) =>
    request('/avatars/licensed', { method: 'GET', params }),
};
