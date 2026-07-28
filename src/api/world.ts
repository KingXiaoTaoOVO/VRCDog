import { getStoredAuthCookie, parseExecuteResponse, request, safeInvoke } from './request';
import { toCleanBase64 } from './utils';

export interface WorldQuery {
  n?: number;
  offset?: number;
  search?: string;
  tag?: string;
  user?: string;
  sort?: string;
  order?: 'ascending' | 'descending';
  releaseStatus?: string;
  publicationStatus?: string;
  featured?: boolean;
  platform?: string;
  [key: string]: unknown;
}

export const WorldApi = {
  getWorld: (params: { worldId: string } | string) => {
    const worldId = typeof params === 'string' ? params : params.worldId;
    return request(`/worlds/${worldId}`);
  },

  getWorlds: (params: WorldQuery = {}, option?: string) => {
    let endpoint = 'worlds';
    if (option) {
      endpoint = `worlds/${option}`;
    }
    return request(endpoint, { method: 'GET', params });
  },

  getWorldsByUser: (params: WorldQuery & { userId: string }) => {
    const { userId, ...query } = params;
    return request('worlds', { method: 'GET', params: { ...query, user: userId } });
  },

  searchWorlds: (params: WorldQuery & { query: string }) => {
    const { query, ...rest } = params;
    return request('worlds', { method: 'GET', params: { ...rest, search: query } });
  },

  deleteWorld: (params: { worldId: string }) =>
    request(`/worlds/${params.worldId}`, { method: 'DELETE' }),

  saveWorld: (params: { id: string, [key: string]: any }) =>
    request(`/worlds/${params.id}`, { method: 'PUT', params }),

  publishWorld: (params: { worldId: string }) =>
    request(`/worlds/${params.worldId}/publish`, { method: 'PUT', params }),

  unpublishWorld: (params: { worldId: string }) =>
    request(`/worlds/${params.worldId}/publish`, { method: 'DELETE' }),

  uploadWorldImage: async (imageData: string) => {
    const authCookie = await getStoredAuthCookie();
    const res = await safeInvoke('vrc_execute', {
      options: {
        url: 'https://api.vrchat.cloud/api/1/file/image',
        method: 'POST',
        auth_cookie: authCookie,
        form_data: [
          { name: 'data', value: JSON.stringify({ tag: 'worldimage' }) },
          {
            name: 'image',
            file_name: 'image.png',
            file_content_base64: toCleanBase64(imageData),
            file_mime: 'image/png',
          },
        ],
      },
    });
    return parseExecuteResponse(res, 'https://api.vrchat.cloud/api/1/file/image');
  },
};
