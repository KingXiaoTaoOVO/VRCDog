import { request } from './request';

export const FriendApi = {
  getFriends: (params: { n?: number, offset?: number, offline?: boolean }) =>
    request('/auth/user/friends', { method: 'GET', params }),

  sendFriendRequest: (params: { userId: string }) =>
    request(`/user/${params.userId}/friendRequest`, { method: 'POST' }),

  cancelFriendRequest: (params: { userId: string }) =>
    request(`/user/${params.userId}/friendRequest`, { method: 'DELETE' }),

  deleteFriend: (params: { userId: string }) =>
    request(`/auth/user/friends/${params.userId}`, { method: 'DELETE' }),

  getFriendStatus: (params: { userId: string }) =>
    request(`/user/${params.userId}/friendStatus`, { method: 'GET' }),

  deleteHiddenFriendRequest: (params: { userId: string }) =>
    request(`/user/${params.userId}/friendRequest`, { method: 'DELETE' }),
};
