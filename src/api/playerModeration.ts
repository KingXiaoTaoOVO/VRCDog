import { request } from './request';

export const PlayerModerationApi = {
  getPlayerModerations: () =>
    request('/auth/user/playermoderations', { method: 'GET' }),

  sendPlayerModeration: (params: { moderated: string; type: string }) =>
    request('/auth/user/playermoderations', { method: 'POST', params }),

  deletePlayerModeration: (params: { moderated: string; type: string }) =>
    request('/auth/user/unplayermoderate', { method: 'PUT', params }),
};
