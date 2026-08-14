import { describe, expect, it } from 'vitest';
import { isNotificationKindEnabled } from './notificationEngine';

describe('notification category settings', () => {
  it('keeps friend presence and world changes independently configurable', () => {
    expect(isNotificationKindEnabled({ notifyFriendsOnline: false, notifyStatusChange: true }, 'friend_online')).toBe(false);
    expect(isNotificationKindEnabled({ notifyFriendsOnline: false, notifyStatusChange: true }, 'friend_location')).toBe(true);
  });

  it('uses the invite switch for friend requests and group notifications', () => {
    expect(isNotificationKindEnabled({ notifyInvite: false }, 'friend_request')).toBe(false);
    expect(isNotificationKindEnabled({ notifyInvite: false }, 'group')).toBe(false);
  });
});
