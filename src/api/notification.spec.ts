import { beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({ request: vi.fn() }));
vi.mock('./request', () => ({
  request: mocks.request,
  getStoredAuthCookie: vi.fn(),
  parseExecuteResponse: vi.fn(),
  safeInvoke: vi.fn(),
}));

import { NotificationApi } from './notification';

describe('NotificationApi', () => {
  beforeEach(() => mocks.request.mockReset().mockResolvedValue({}));

  it('uses the documented clear endpoints for both notification versions', async () => {
    await NotificationApi.clearNotifications();
    await NotificationApi.clearNotificationsV2();

    expect(mocks.request).toHaveBeenNthCalledWith(1, '/auth/user/notifications/clear', { method: 'PUT' });
    expect(mocks.request).toHaveBeenNthCalledWith(2, '/notifications', { method: 'DELETE' });
  });

  it('does not leak notificationId into the V2 response body', async () => {
    await NotificationApi.sendNotificationResponse({
      notificationId: 'not_1',
      responseType: 'Accept',
      responseData: 'yes',
    });

    expect(mocks.request).toHaveBeenCalledWith('/notifications/not_1/respond', {
      method: 'POST',
      params: { responseType: 'Accept', responseData: 'yes' },
    });
  });
});
