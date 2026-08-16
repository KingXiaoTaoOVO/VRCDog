import { beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
  getSnapshot: vi.fn(),
  getCurrentUser: vi.fn(),
  getWorld: vi.fn(),
  getInstance: vi.fn(),
}));

vi.mock('../api', () => ({
  GamelogApi: { getSnapshot: mocks.getSnapshot },
  VrcApi: {
    getCurrentUser: mocks.getCurrentUser,
    getWorld: mocks.getWorld,
    getInstance: mocks.getInstance,
  },
}));

import {
  currentInstanceState,
  parseVrcLocation,
  refreshCurrentInstance,
  resetCurrentInstanceForTests,
} from './currentInstance';

describe('current instance monitor', () => {
  beforeEach(() => {
    resetCurrentInstanceForTests();
    vi.clearAllMocks();
    mocks.getSnapshot.mockResolvedValue([]);
  });

  it('parses a full VRChat location without losing instance tags', () => {
    expect(parseVrcLocation('wrld_abc:12345~region(eu)')).toEqual({
      worldId: 'wrld_abc',
      instanceId: '12345~region(eu)',
    });
    expect(parseVrcLocation('private')).toBeNull();
  });

  it('combines API instance count with players observed in the game log', async () => {
    mocks.getCurrentUser.mockResolvedValue({ id: 'usr_me', displayName: 'Me', location: 'wrld_abc:42' });
    mocks.getWorld.mockResolvedValue({ name: 'Test World' });
    mocks.getInstance.mockResolvedValue({
      n_users: 3,
      users: [{ id: 'usr_api', displayName: 'API Player' }],
    });
    mocks.getSnapshot.mockResolvedValue([
      { time: '2026-08-16T12:01:00Z', event_type: 'Player Joined', content: 'Log Player (usr_log)' },
      { time: '2026-08-16T12:00:00Z', event_type: 'Instance Joined', content: 'Test World' },
    ]);

    await refreshCurrentInstance({ vrcRunning: true, force: true });

    expect(currentInstanceState.roomName).toBe('Test World · 42');
    expect(currentInstanceState.playerCount).toBe(3);
    expect(currentInstanceState.players.map(player => player.name)).toEqual(expect.arrayContaining([
      'Me',
      'API Player',
      'Log Player',
    ]));
  });

  it('clears stale room data as soon as VRChat stops', async () => {
    currentInstanceState.roomName = 'Old Room';
    currentInstanceState.players = [{ name: 'Old Player', joinTime: '' }];
    await refreshCurrentInstance({ vrcRunning: false });
    expect(currentInstanceState.location).toBe('offline');
    expect(currentInstanceState.players).toEqual([]);
    expect(currentInstanceState.playerCount).toBeNull();
  });
});
