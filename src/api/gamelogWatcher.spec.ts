import { beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
  getLatestGamelogs: vi.fn(),
  saveGameLogs: vi.fn(),
  getAllSettings: vi.fn(),
  isVrcRunning: vi.fn(),
  setDiscordRpc: vi.fn(),
}));

vi.mock('./index', () => ({
  GamelogApi: { getLatestGamelogs: mocks.getLatestGamelogs },
  DbApi: {
    saveGameLogs: mocks.saveGameLogs,
    getAllSettings: mocks.getAllSettings,
  },
  SysApi: {
    isVrcRunning: mocks.isVrcRunning,
    setDiscordRpc: mocks.setDiscordRpc,
  },
}));

import { pollGamelogOnce } from './gamelogWatcher';

describe('gamelogWatcher', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.getLatestGamelogs.mockResolvedValue([]);
    mocks.saveGameLogs.mockResolvedValue(0);
  });

  it('drains and finalizes the session after VRChat exits', async () => {
    mocks.isVrcRunning.mockResolvedValue(false);
    mocks.getLatestGamelogs.mockResolvedValue([
      { time: '2026.08.15 01:02:00', event_type: 'Player Left', content: 'Alice (usr_alice)' },
    ]);
    mocks.saveGameLogs.mockResolvedValue(1);
    const dispatch = vi.spyOn(window, 'dispatchEvent');

    await pollGamelogOnce();

    expect(mocks.getLatestGamelogs).toHaveBeenCalledWith({
      maxLines: 100000,
      finalizeSession: true,
    });
    expect(mocks.saveGameLogs).toHaveBeenCalledOnce();
    expect(dispatch).toHaveBeenCalledWith(expect.objectContaining({ type: 'vrc-gamelog-updated' }));
    dispatch.mockRestore();
  });

  it('does not finalize when process-state detection fails', async () => {
    mocks.isVrcRunning.mockRejectedValue(new Error('process query failed'));

    await pollGamelogOnce();

    expect(mocks.getLatestGamelogs).toHaveBeenCalledWith({
      maxLines: 100000,
      finalizeSession: false,
    });
  });
});
