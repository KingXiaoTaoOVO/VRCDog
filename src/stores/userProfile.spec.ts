import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';

// 用户详情面板是单例复用的（Store-driven Modal）：所有 Tab 的数据写进同一批 ref。
// 这组测试锁定的正是「切换目标用户后，上一个用户的迟到响应不许再落地」这个行为。
const mocks = vi.hoisted(() => ({
  request: vi.fn(),
  getApiCache: vi.fn(),
  saveApiCache: vi.fn(),
  getNote: vi.fn(),
  getFriendLogs: vi.fn(),
  getMutualFriends: vi.fn(),
  getMutualGroups: vi.fn(),
  getFavorites: vi.fn(),
}));

vi.mock('../api', () => ({
  VrcApi: {
    request: (...args: any[]) => mocks.request(...args),
    getMutualFriends: (...args: any[]) => mocks.getMutualFriends(...args),
    getMutualGroups: (...args: any[]) => mocks.getMutualGroups(...args),
    getFavorites: (...args: any[]) => mocks.getFavorites(...args),
  },
  DbApi: {
    getApiCache: (...args: any[]) => mocks.getApiCache(...args),
    saveApiCache: (...args: any[]) => mocks.saveApiCache(...args),
    getNote: (...args: any[]) => mocks.getNote(...args),
    getFriendLogs: (...args: any[]) => mocks.getFriendLogs(...args),
  },
}));

type Deferred = { promise: Promise<any>; resolve: (value: any) => void };

function deferred(): Deferred {
  let resolve!: (value: any) => void;
  const promise = new Promise<any>((r) => { resolve = r; });
  return { promise, resolve };
}

const flush = () => new Promise((r) => setTimeout(r, 0));

async function openTwoProfiles() {
  const { useUserProfileStore } = await import('./userProfile');
  const store = useUserProfileStore();

  const pendingA = deferred();
  const pendingB = deferred();
  const mutualA = deferred();
  const mutualB = deferred();

  mocks.request.mockImplementation((path: string) => {
    if (path === '/auth/user') return Promise.resolve({ id: 'me' });
    if (path === '/users/user-a') return pendingA.promise;
    if (path === '/users/user-b') return pendingB.promise;
    return Promise.resolve([]);
  });
  mocks.getMutualFriends.mockImplementation(({ userId }: { userId: string }) =>
    userId === 'user-a' ? mutualA.promise : mutualB.promise
  );
  mocks.getMutualGroups.mockResolvedValue([]);
  mocks.getFavorites.mockResolvedValue([]);
  mocks.getApiCache.mockResolvedValue(null);
  mocks.saveApiCache.mockResolvedValue(undefined);
  mocks.getNote.mockResolvedValue(null);
  mocks.getFriendLogs.mockResolvedValue([]);

  await store.openProfile('user-a', null as any);
  await store.openProfile('user-b', null as any);
  return { store, pendingA, pendingB, mutualA, mutualB };
}

describe('userProfile 单例面板的响应世代守卫', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.resetModules();
    Object.values(mocks).forEach((fn) => fn.mockReset());
  });

  it('丢弃上一个用户迟到的基础资料，不串号', async () => {
    const { store, pendingA, pendingB, mutualA, mutualB } = await openTwoProfiles();

    pendingB.resolve({ id: 'user-b', displayName: 'B' });
    mutualB.resolve([]);
    await flush();
    expect(store.baseInfo?.id).toBe('user-b');

    // A 的请求现在才回来：绝不能覆盖 B 的资料
    pendingA.resolve({ id: 'user-a', displayName: 'A' });
    mutualA.resolve([{ id: 'x', displayName: 'X' }]);
    await flush();

    expect(store.baseInfo?.id).toBe('user-b');
    expect(store.baseInfo?.displayName).toBe('B');
    expect(store.mutualFriends).toEqual([]);
  });

  it('迟到的响应不会把新用户的 loading 灯提前熄灭', async () => {
    const { store, pendingA, pendingB, mutualA, mutualB } = await openTwoProfiles();

    // B 的共同好友仍在途中，isLoadingMutual 必须是 true
    expect(store.isLoadingMutual).toBe(true);

    // A 的响应后到 —— 不该关掉 B 的加载态
    mutualA.resolve([]);
    await flush();
    expect(store.isLoadingMutual).toBe(true);

    mutualB.resolve([{ id: 'y', displayName: 'Y' }]);
    pendingB.resolve({ id: 'user-b', displayName: 'B' });
    await flush();
    expect(store.isLoadingMutual).toBe(false);
    expect(store.mutualFriends).toHaveLength(1);
  });

  it('关闭面板后所有在途响应都被丢弃', async () => {
    const { store, pendingA, pendingB, mutualA, mutualB } = await openTwoProfiles();
    store.closeProfile();

    pendingA.resolve({ id: 'user-a', displayName: 'A' });
    pendingB.resolve({ id: 'user-b', displayName: 'B' });
    mutualA.resolve([{ id: 'x', displayName: 'X' }]);
    mutualB.resolve([{ id: 'x', displayName: 'X' }]);
    await flush();

    expect(store.baseInfo).toBeNull();
    expect(store.mutualFriends).toEqual([]);
  });
});
