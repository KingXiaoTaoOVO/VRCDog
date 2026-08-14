import { getStoredAuthCookie, parseExecuteResponse, request, safeInvoke } from './request';
import { toCleanBase64 } from './utils';

async function uploadNotificationPhoto(url: string, params: Record<string, unknown>, imageData?: string) {
  if (!imageData) throw new Error('缺少图片数据');
  const authCookie = await getStoredAuthCookie();
  const formData: any[] = [
    { name: 'data', value: JSON.stringify(params || {}) },
    {
      name: 'image',
      file_name: 'photo.png',
      file_content_base64: toCleanBase64(imageData),
      file_mime: 'image/png',
    },
  ];
  const fullUrl = `https://api.vrchat.cloud/api/1/${url.replace(/^\/+/, '')}`;
  const res: any = await safeInvoke('vrc_execute', {
    options: {
      url: fullUrl,
      method: 'POST',
      auth_cookie: authCookie,
      form_data: formData,
    },
  });
  return parseExecuteResponse(res, fullUrl);
}

export const NotificationApi = {
  getNotifications: (params: { n?: number, offset?: number, type?: string, hidden?: boolean, after?: string }) =>
    request('/auth/user/notifications', { method: 'GET', params }),

  getHiddenFriendRequests: (params: { n?: number, offset?: number } = {}) =>
    request('/auth/user/notifications', {
      method: 'GET',
      params: { type: 'friendRequest', hidden: true, ...params },
    }),

  acceptNotification: (notificationId: string | { notificationId: string }) => {
    const id = typeof notificationId === 'string' ? notificationId : notificationId.notificationId;
    return request(`/auth/user/notifications/${id}/accept`, { method: 'PUT' });
  },

  hideNotification: (notificationId: string | { notificationId: string }) => {
    const id = typeof notificationId === 'string' ? notificationId : notificationId.notificationId;
    return request(`/auth/user/notifications/${id}/hide`, { method: 'PUT' });
  },

  seeNotification: (notificationId: string | { notificationId: string }) => {
    const id = typeof notificationId === 'string' ? notificationId : notificationId.notificationId;
    return request(`/auth/user/notifications/${id}/see`, { method: 'PUT' });
  },

  acceptFriendRequestNotification: (params: { notificationId: string }) =>
    request(`/auth/user/notifications/${params.notificationId}/accept`, { method: 'PUT' }),

  clearNotifications: () =>
    request('/auth/user/notifications/clear', { method: 'PUT' }),

  getNotificationsV2: (params: { n?: number, offset?: number, type?: string, limit?: number } = {}) =>
    request('/notifications', { method: 'GET', params }),

  clearNotificationsV2: () =>
    request('/notifications', { method: 'DELETE' }),

  sendInvite: (params: { receiverUserId?: string, userId?: string, [key: string]: any }, receiverUserId?: string) => {
    const id = receiverUserId || params.receiverUserId || params.userId;
    if (!id) throw new Error('缺少邀请接收用户 ID');
    const { receiverUserId: _receiverUserId, userId: _userId, ...body } = params;
    return request(`/invite/${id}`, { method: 'POST', params: body });
  },

  sendInvitePhoto: (params: { receiverUserId?: string, userId?: string, imageData?: string, [key: string]: any }, receiverUserId?: string) => {
    const id = receiverUserId || params.receiverUserId || params.userId;
    if (!id) throw new Error('缺少邀请接收用户 ID');
    const { receiverUserId: _receiverUserId, userId: _userId, imageData, ...body } = params;
    return uploadNotificationPhoto(`/invite/${id}/photo`, body, imageData);
  },

  sendRequestInvite: (params: { receiverUserId?: string, userId?: string, [key: string]: any }, receiverUserId?: string) => {
    const id = receiverUserId || params.receiverUserId || params.userId;
    if (!id) throw new Error('缺少邀请请求接收用户 ID');
    const { receiverUserId: _receiverUserId, userId: _userId, ...body } = params;
    return request(`/requestInvite/${id}`, { method: 'POST', params: body });
  },

  sendRequestInvitePhoto: (params: { receiverUserId?: string, userId?: string, imageData?: string, [key: string]: any }, receiverUserId?: string) => {
    const id = receiverUserId || params.receiverUserId || params.userId;
    if (!id) throw new Error('缺少邀请请求接收用户 ID');
    const { receiverUserId: _receiverUserId, userId: _userId, imageData, ...body } = params;
    return uploadNotificationPhoto(`/requestInvite/${id}/photo`, body, imageData);
  },

  sendInviteResponse: (params: { inviteId?: string, responseSlot?: number | string, responseMessage?: string, [key: string]: any }, inviteId?: string) => {
    const id = inviteId || params.inviteId;
    if (!id) throw new Error('缺少邀请 ID');
    const { inviteId: _inviteId, ...body } = params;
    return request(`/invite/${id}/response`, { method: 'POST', params: body });
  },

  sendInviteResponsePhoto: (params: { inviteId?: string, imageData?: string, [key: string]: any }, inviteId?: string) => {
    const id = inviteId || params.inviteId;
    if (!id) throw new Error('缺少邀请 ID');
    const { inviteId: _inviteId, imageData, ...body } = params;
    return uploadNotificationPhoto(`/invite/${id}/response/photo`, body, imageData);
  },

  seeNotificationV2: (notificationId: string | { notificationId: string }) => {
    const id = typeof notificationId === 'string' ? notificationId : notificationId.notificationId;
    return request(`/notifications/${id}/see`, { method: 'POST' });
  },

  sendNotificationResponse: (params: { notificationId: string, responseType: string, responseData?: string }) => {
    const { notificationId, ...body } = params;
    return request(`/notifications/${notificationId}/respond`, { method: 'POST', params: body });
  },

  deleteNotificationV2: (notificationId: string | { notificationId: string }) => {
    const id = typeof notificationId === 'string' ? notificationId : notificationId.notificationId;
    return request(`/notifications/${id}`, { method: 'DELETE' });
  },

  hideNotificationV2: (notificationId: string | { notificationId: string }) => {
    const id = typeof notificationId === 'string' ? notificationId : notificationId.notificationId;
    return request(`/notifications/${id}`, { method: 'DELETE' });
  },
};
