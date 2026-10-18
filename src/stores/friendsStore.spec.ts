import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';

const mocks = vi.hoisted(() => ({
  isTauri: true,
  getAllFriends: vi.fn(),
  getCachedFriends: vi.fn(),
  batchSaveFriends: vi.fn(),
}));

vi.mock('../api', () => ({
  VrcApi: { getAllFriends: (...a: any[]) => mocks.getAllFriends(...a) },
  DbApi: {
    getCachedFriends: (...a: any[]) => mocks.getCachedFriends(...a),
    batchSaveFriends: (...a: any[]) => mocks.batchSaveFriends(...a),
  },
}));

vi.mock('@tauri-apps/api/core', () => ({
  isTauri: () => mocks.isTauri,
}));

async function loadStore() {
  const { useFriendsStore } = await import('./friendsStore');
  const { dataHealth } = await import('./dataHealth');
  return { store: useFriendsStore(), dataHealth };
}

// 后台刷新是「数据已过期」指示器的数据来源：它必须如实区分
// 「真的从 VRChat 拿到了数据」和「只是退回本地缓存」。
describe('friendsStore.backgroundRefresh', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.resetModules();
    mocks.isTauri = true;
    mocks.getAllFriends.mockReset();
    mocks.getCachedFriends.mockReset();
    mocks.batchSaveFriends.mockReset();
    mocks.batchSaveFriends.mockResolvedValue(undefined);
  });

  it('线上成功时更新列表、写缓存并标记数据健康', async () => {
    const { store, dataHealth } = await loadStore();
    dataHealth.lastSuccessAt = null;
    mocks.getCachedFriends.mockResolvedValue([]);
    mocks.getAllFriends.mockResolvedValue([
      { id: 'u1', displayName: 'A', location: 'public' },
      { id: '', displayName: '' },
    ]);

    await expect(store.backgroundRefresh()).resolves.toBe(true);
    expect(store.allFriends).toHaveLength(1);
    expect(dataHealth.lastSuccessAt).not.toBeNull();
    expect(mocks.batchSaveFriends).toHaveBeenCalled();
    // 后台轮询绝不能闪 spinner
    expect(store.loading).toBe(false);
  });

  it('线上失败、退回缓存时不算数据健康（否则永远显示绿色）', async () => {
    const { store, dataHealth } = await loadStore();
    dataHealth.lastSuccessAt = null;
    mocks.getCachedFriends.mockResolvedValue([{ id: 'u9', displayName: 'Cached' }]);
    mocks.getAllFriends.mockRejectedValue(new Error('429 too many requests'));

    await expect(store.backgroundRefresh()).resolves.toBe(false);
    expect(store.allFriends).toHaveLength(1); // 仍然回退到有得看的缓存
    expect(dataHealth.lastSuccessAt).toBeNull(); // 但不标记健康
    expect(store.error).toBe(''); // 后台失败不弹错误横幅
  });

  it('线上失败且没有缓存时也不抛到调用方', async () => {
    const { store } = await loadStore();
    mocks.getCachedFriends.mockResolvedValue([]);
    mocks.getAllFriends.mockRejectedValue(new Error('network down'));

    await expect(store.backgroundRefresh()).resolves.toBe(false);
    expect(store.error).toBe('');
  });

  it('等登录时的首次同步结束后再发请求，不叠加', async () => {
    const { store } = await loadStore();
    mocks.getCachedFriends.mockResolvedValue([]);
    mocks.getAllFriends.mockResolvedValue([{ id: 'u2', displayName: 'B' }]);

    let resolveFirst: (v: any) => void = () => {};
    const first = new Promise<any>((r) => { resolveFirst = r; });

    store.beginSync(first);
    const background = store.backgroundRefresh();
    expect(mocks.getAllFriends).not.toHaveBeenCalled(); // 还在等首次同步，不该另发一个请求

    resolveFirst([{ id: 'u1', displayName: 'A' }]);
    await expect(background).resolves.toBe(true);
    expect(mocks.getAllFriends).toHaveBeenCalledTimes(1);
  });
});
