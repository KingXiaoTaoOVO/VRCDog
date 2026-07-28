import { beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
  request: vi.fn(),
}));

vi.mock('./request', () => ({ request: mocks.request }));

import { FriendApi } from './friend';

describe('FriendApi.getAllFriends', () => {
  beforeEach(() => {
    mocks.request.mockReset();
  });

  it('fetches both presence partitions, every page, and removes duplicate users', async () => {
    const firstPage = Array.from({ length: 100 }, (_, index) => ({ id: `usr_${index}` }));
    mocks.request
      .mockResolvedValueOnce(firstPage)
      .mockResolvedValueOnce([{ id: 'usr_99' }, { id: 'usr_100' }]);

    const friends = await FriendApi.getAllFriends();

    expect(friends).toHaveLength(101);
    expect(mocks.request).toHaveBeenNthCalledWith(1, '/auth/user/friends', {
      method: 'GET',
      params: { offline: false, n: 100, offset: 0 },
    });
    expect(mocks.request).toHaveBeenNthCalledWith(2, '/auth/user/friends', {
      method: 'GET',
      params: { offline: false, n: 100, offset: 100 },
    });
    expect(mocks.request).toHaveBeenNthCalledWith(3, '/auth/user/friends', {
      method: 'GET',
      params: { offline: true, n: 100, offset: 0 },
    });
  });

  it('reports an API failure instead of treating an empty result as zero friends', async () => {
    mocks.request.mockRejectedValue(new Error('Missing Credentials'));

    await expect(FriendApi.getAllFriends()).rejects.toThrow('Missing Credentials');
    expect(mocks.request).toHaveBeenCalledTimes(2);
  });
});
