import { describe, expect, it } from 'vitest';
import { buildCurrentRoomPlayers, parseGameLogIdentity } from './gameLogSession';

describe('gameLogSession', () => {
  it('extracts the exact display name and user id from VRChat logs', () => {
    expect(parseGameLogIdentity('火锅涮冰块 ffbc (usr_b9cd9d50-cac5-43c2-91f0-7637426effbc)')).toEqual({
      displayName: '火锅涮冰块 ffbc',
      userId: 'usr_b9cd9d50-cac5-43c2-91f0-7637426effbc',
      keys: [
        'id:usr_b9cd9d50-cac5-43c2-91f0-7637426effbc',
        'name:火锅涮冰块 ffbc',
      ],
    });
  });

  it('pairs joins and leaves by user id even if the displayed name changes', () => {
    const snapshot = buildCurrentRoomPlayers([
      {
        time: '2026.08.15 01:02:00',
        event_type: 'Player Left',
        content: 'New Name (usr_same)',
        display_name: 'New Name',
        user_id: 'usr_same',
      },
      {
        time: '2026.08.15 01:00:00',
        event_type: 'Player Joined',
        content: 'Old Name (usr_same)',
        display_name: 'Old Name',
        user_id: 'usr_same',
      },
      { time: '2026.08.15 00:59:00', event_type: 'Instance Joined', content: 'wrld_test:1' },
    ]);

    expect(snapshot).toEqual({ roomName: 'wrld_test:1', players: [] });
  });

  it('keeps only players still present in the latest instance', () => {
    const snapshot = buildCurrentRoomPlayers([
      { time: '2026.08.15 01:03:00', event_type: 'Player Left', content: 'Bob (usr_bob)' },
      { time: '2026.08.15 01:02:00', event_type: 'Player Joined', content: 'Alice (usr_alice)' },
      { time: '2026.08.15 01:01:00', event_type: 'Player Joined', content: 'Bob (usr_bob)' },
      { time: '2026.08.15 01:00:00', event_type: 'Instance Joined', content: 'wrld_test:2' },
      { time: '2026.08.15 00:50:00', event_type: 'Player Joined', content: 'Old Room (usr_old)' },
    ]);

    expect(snapshot.roomName).toBe('wrld_test:2');
    expect(snapshot.players).toEqual([
      { name: 'Alice', userId: 'usr_alice', joinTime: '2026.08.15 01:02:00' },
    ]);
  });

  it('clears a stale snapshot after application quit', () => {
    expect(buildCurrentRoomPlayers([
      { time: '2026.08.15 01:05:00', event_type: 'Application Quit', content: 'VRChat' },
      { time: '2026.08.15 01:00:00', event_type: 'Player Joined', content: 'Alice (usr_alice)' },
    ])).toEqual({ roomName: '', players: [] });
  });
});
