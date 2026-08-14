import { describe, expect, it } from 'vitest';
import {
  getDisplayNotificationDetails,
  getStoredNotificationMeta,
  normalizeNotificationForDb,
} from './notificationNormalization';

describe('notification normalization', () => {
  it('preserves Notification V2 response metadata and display data', () => {
    const stored = normalizeNotificationForDb({
      id: 'not_1',
      type: 'group.invite',
      category: 'Group',
      createdAt: '2026-08-14T00:00:00.000Z',
      data: { title: 'Group invitation', groupName: 'Example' },
      responses: [{ type: 'Accept', data: 'yes', text: 'Accept' }],
    });

    expect(stored.message).toBe('Group invitation');
    expect(stored.created_at).toBe('2026-08-14T00:00:00.000Z');
    expect(getStoredNotificationMeta(stored.details)).toMatchObject({
      version: 2,
      category: 'Group',
      responses: [{ type: 'Accept', data: 'yes', text: 'Accept' }],
    });
    expect(getDisplayNotificationDetails(stored.details)).toMatchObject({ groupName: 'Example' });
  });

  it('normalizes legacy notifications without inventing V2 semantics', () => {
    const stored = normalizeNotificationForDb({
      id: 'frq_1',
      type: 'friendRequest',
      senderUsername: 'Alice',
      details: '{"message":"Hello"}',
      created_at: '2026-08-14T00:00:00.000Z',
    });

    expect(stored.senderUsername).toBe('Alice');
    expect(getStoredNotificationMeta(stored.details).version).toBe(1);
    expect(getDisplayNotificationDetails(stored.details)).toBe('Hello');
  });
});
