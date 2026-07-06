import { request } from './request';

export const MiscApi = {
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

  getVRChatCredits: (params: { userId: string }) =>
    request(`/user/${params.userId}/balance`, { method: 'GET' }),

  closeInstance: (params: { location: string; hardClose?: boolean }) =>
    request(`/instances/${params.location}`, {
      method: 'DELETE',
      params: { hardClose: params.hardClose ?? false },
    }),

  deleteWorldPersistData: (params: { userId: string; worldId: string }) =>
    request(`/users/${params.userId}/${params.worldId}/persist`, { method: 'DELETE' }),

  hasWorldPersistData: (params: { userId: string; worldId: string }) =>
    request(`/users/${params.userId}/${params.worldId}/persist/exists`, { method: 'GET' }),

  updateBadge: (params: { userId: string; badgeId: string; hidden: boolean; showcased: boolean }) =>
    request(`/users/${params.userId}/badges/${params.badgeId}`, {
      method: 'PUT',
      params: {
        userId: params.userId,
        badgeId: params.badgeId,
        hidden: params.hidden,
        showcased: params.showcased,
      },
    }),

  getVisits: () =>
    request('/visits', { method: 'GET' }),

  sendBoop: (params: { userId: string; emojiId: string; inventoryItemId?: string }) =>
    request(`/users/${params.userId}/boop`, {
      method: 'POST',
      params: {
        emojiId: params.emojiId,
        inventoryItemId: params.inventoryItemId,
      },
    }),
};
