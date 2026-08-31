import { describe, expect, it } from 'vitest';
import {
  getDisplayNotificationDetails,
  getReadableNotificationText,
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

  it('does not expose internal metadata as notification text', () => {
    const stored = normalizeNotificationForDb({
      id: 'frq_2',
      type: 'friendRequest',
      senderUsername: 'Bob',
      message: '{"__vrcdog":{"version":1}}',
    });

    expect(stored.message).toBe('');
    expect(getDisplayNotificationDetails(stored.details)).toEqual({});
  });

  it('returns readable text without exposing internal metadata', () => {
    const stored = normalizeNotificationForDb({
      id: 'n_3',
      type: 'friendRequest',
      senderUsername: 'Alice',
      details: { message: 'Please add me', __vrcdog: { version: 1 } },
    });
    expect(getReadableNotificationText(stored.details)).toBe('Please add me');
  });

  it('returns an empty body for metadata-only details', () => {
    expect(getReadableNotificationText('{"__vrcdog":{"version":1}}')).toBe('');
  });
});
