import { request } from './request';

export const AvatarModerationApi = {
  getAvatarModerations: () =>
    request('/auth/user/avatarmoderations', { method: 'GET' }),

  sendAvatarModeration: (params: { avatarModerationType: string; targetAvatarId: string }) =>
    request('/auth/user/avatarmoderations', { method: 'POST', params }),

  deleteAvatarModeration: (params: { avatarModerationType: string; targetAvatarId: string }) =>
    request(
      `/auth/user/avatarmoderations?targetAvatarId=${encodeURIComponent(params.targetAvatarId)}&avatarModerationType=${encodeURIComponent(params.avatarModerationType)}`,
      { method: 'DELETE' },
    ),
};
