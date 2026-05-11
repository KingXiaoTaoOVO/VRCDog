import { request } from './request';

export const NotificationApi = {
  getNotifications: (params: { n?: number, offset?: number, type?: string, hidden?: boolean, after?: string }) =>
    request('/auth/user/notifications', { method: 'GET', params }),

  acceptNotification: (notificationId: string) =>
    request(`/auth/user/notifications/${notificationId}/accept`, { method: 'PUT' }),

  hideNotification: (notificationId: string) =>
    request(`/auth/user/notifications/${notificationId}/hide`, { method: 'PUT' }),

  getNotificationsV2: (params: { n?: number, offset?: number, type?: string }) =>
    request('/notifications', { method: 'GET', params }),

  seeNotificationV2: (notificationId: string) =>
    request(`/notifications/${notificationId}/see`, { method: 'POST' }),

  deleteNotificationV2: (notificationId: string) =>
    request(`/notifications/${notificationId}`, { method: 'DELETE' }),
};
