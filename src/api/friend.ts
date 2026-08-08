import { request } from './request';

export interface FriendQuery {
  n?: number;
  offset?: number;
  offline?: boolean;
}

const FRIENDS_PAGE_SIZE = 100;
const MAX_FRIENDS_PAGES = 100;

export const FriendApi = {
  getFriends: (params: FriendQuery = {}) =>
    request('/auth/user/friends', { method: 'GET', params }),

  /**
   * Fetch every friend page without asking each UI view to duplicate pagination.
   * VRChat splits the list into online (`offline=false`) and offline
   * (`offline=true`) partitions, as VRCX does. A request without an explicit
   * partition therefore combines both lists.
   */
  getAllFriends: async (params: FriendQuery = {}) => {
    const pageSize = Math.min(Math.max(Math.floor(params.n ?? FRIENDS_PAGE_SIZE), 1), FRIENDS_PAGE_SIZE);
    const byId = new Map<string, any>();
    const partitions = typeof params.offline === 'boolean' ? [params.offline] : [false, true];
    let completedPartitions = 0;
    let firstError: unknown = null;

    for (const offline of partitions) {
      let offset = Math.max(Math.floor(params.offset ?? 0), 0);
      try {
        for (let page = 0; page < MAX_FRIENDS_PAGES; page += 1) {
          const batch: any = await request('/auth/user/friends', {
            method: 'GET',
            params: { ...params, offline, n: pageSize, offset },
          });
          if (!Array.isArray(batch) || batch.length === 0) break;

          for (const friend of batch) {
            const key = friend?.id || `${friend?.displayName || 'unknown'}:${offset}`;
            byId.set(key, friend);
          }
          if (batch.length < pageSize) break;
          offset += batch.length;
        }
        completedPartitions += 1;
      } catch (error) {
        firstError ||= error;
      }
    }

    // VRCX treats a partially-fetched friend list as a failure — it never
    // displays a silently truncated roster. If any partition errored mid-pagination
    // the combined set is incomplete, so surface the error and let the caller fall
    // back to its cached snapshot instead of showing fewer friends than reality.
    if (firstError) throw firstError;
    return Array.from(byId.values());
  },

  sendFriendRequest: (params: { userId: string }) =>
    request(`/user/${params.userId}/friendRequest`, { method: 'POST' }),

  cancelFriendRequest: (params: { userId: string }) =>
    request(`/user/${params.userId}/friendRequest`, { method: 'DELETE' }),

  deleteFriend: (params: { userId: string }) =>
    request(`/auth/user/friends/${params.userId}`, { method: 'DELETE' }),

  getFriendStatus: (params: { userId: string }) =>
    request(`/user/${params.userId}/friendStatus`, { method: 'GET' }),

  deleteHiddenFriendRequest: (params: { userId: string }) =>
    request(`/user/${params.userId}/friendRequest`, { method: 'DELETE' }),
};
