import { request } from './request';

export const WorldApi = {
  getWorld: (params: { worldId: string } | string) => {
    const worldId = typeof params === 'string' ? params : params.worldId;
    return request(`/worlds/${worldId}`);
  },

  getWorlds: (params: { search?: string, n?: number, offset?: number, user?: string, releaseStatus?: string, sort?: string }, option?: string) => {
    let endpoint = 'worlds';
    if (option) {
      endpoint = `worlds/${option}`;
    }
    return request(endpoint, { method: 'GET', params });
  },

  deleteWorld: (params: { worldId: string }) =>
    request(`/worlds/${params.worldId}`, { method: 'DELETE' }),

  saveWorld: (params: { id: string, [key: string]: any }) =>
    request(`/worlds/${params.id}`, { method: 'PUT', params }),

  publishWorld: (params: { worldId: string }) =>
    request(`/worlds/${params.worldId}/publish`, { method: 'PUT', params }),

  unpublishWorld: (params: { worldId: string }) =>
    request(`/worlds/${params.worldId}/publish`, { method: 'DELETE' }),
};
