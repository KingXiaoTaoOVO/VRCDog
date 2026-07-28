import { request } from './request';
import { resolveCurrentUserId } from './utils';

export const MiscApi = {
  getFile: (params: { fileId: string }) =>
    request(`/file/${params.fileId}`, { method: 'GET' }),

  saveNote: (params: { targetUserId?: string; userId?: string; note: string }) =>
    request('/userNotes', { method: 'POST', params }),

  reportUser: (params: { userId: string; contentType: string; reason: string; type: string }) =>
    request(`/feedback/${params.userId}/user`, {
      method: 'POST',
      params: {
        contentType: params.contentType,
        reason: params.reason,
        type: params.type,
      },
    }),

  getVRChatCredits: async (params: { userId?: string } = {}) => {
    const userId = await resolveCurrentUserId(params.userId);
    return request(`/user/${userId}/balance`, { method: 'GET' });
  },

  closeInstance: (params: { location: string; hardClose?: boolean }) =>
    request(`/instances/${params.location}`, {
      method: 'DELETE',
      params: { hardClose: params.hardClose ?? false },
    }),

  deleteWorldPersistData: async (params: { userId?: string; worldId: string }) => {
    const userId = await resolveCurrentUserId(params.userId);
    return request(`/users/${userId}/${params.worldId}/persist`, { method: 'DELETE' });
  },

  hasWorldPersistData: async (params: { userId?: string; worldId: string }) => {
    const userId = await resolveCurrentUserId(params.userId);
    return request(`/users/${userId}/${params.worldId}/persist/exists`, { method: 'GET' });
  },

  updateBadge: async (params: { userId?: string; badgeId: string; hidden: boolean; showcased: boolean }) => {
    const userId = await resolveCurrentUserId(params.userId);
    return request(`/users/${userId}/badges/${params.badgeId}`, {
      method: 'PUT',
      params: {
        userId,
        badgeId: params.badgeId,
        hidden: params.hidden,
        showcased: params.showcased,
      },
    });
  },

  getVisits: () =>
    request('/visits', { method: 'GET' }),

  getFileAnalysis: (params: { fileId: string; version: number | string; variant: string }) =>
    request(`/analysis/${params.fileId}/${params.version}/${params.variant}`, { method: 'GET' }),

  deleteFile: (fileId: string) =>
    request(`/file/${fileId}`, { method: 'DELETE' }),

  sendBoop: (params: { userId: string; emojiId: string; inventoryItemId?: string }) =>
    request(`/users/${params.userId}/boop`, {
      method: 'POST',
      params: {
        emojiId: params.emojiId,
        inventoryItemId: params.inventoryItemId,
      },
    }),
};
