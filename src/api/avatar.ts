import { getStoredAuthCookie, parseExecuteResponse, request, safeInvoke } from './request';

function toCleanBase64(imageData: string) {
  return imageData.includes(',') ? imageData.split(',')[1] : imageData;
}

async function uploadImage(imageData: string, params: Record<string, unknown>) {
  const authCookie = await getStoredAuthCookie();
  const res = await safeInvoke('vrc_execute', {
    options: {
      url: 'https://api.vrchat.cloud/api/1/file/image',
      method: 'POST',
      auth_cookie: authCookie,
      form_data: [
        { name: 'data', value: JSON.stringify(params) },
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
}

export const AvatarApi = {
  getAvatar: (params: { avatarId: string } | string) => {
    const avatarId = typeof params === 'string' ? params : params.avatarId;
    return request(`/avatars/${avatarId}`);
  },

  getAvatars: (params: { search?: string, n?: number, offset?: number, user?: string, releaseStatus?: string, sort?: string }) =>
    request('/avatars', { method: 'GET', params }),

  saveAvatar: (params: { id: string, [key: string]: any }) =>
    request(`/avatars/${params.id}`, { method: 'PUT', params }),

  selectAvatar: (params: { avatarId: string }) =>
    request(`/avatars/${params.avatarId}/select`, { method: 'PUT', params }),

  selectFallbackAvatar: (params: { avatarId: string }) =>
    request(`/avatars/${params.avatarId}/selectfallback`, { method: 'PUT', params }),

  deleteAvatar: (params: { avatarId: string }) =>
    request(`/avatars/${params.avatarId}`, { method: 'DELETE' }),

  createImposter: (params: { avatarId: string }) =>
    request(`/avatars/${params.avatarId}/impostor/enqueue`, { method: 'POST' }),

  deleteImposter: (params: { avatarId: string }) =>
    request(`/avatars/${params.avatarId}/impostor`, { method: 'DELETE' }),

  getAvailableAvatarStyles: () =>
    request('/avatarStyles', { method: 'GET' }),

  getAvatarGallery: (avatarId: string) =>
    request('/files', {
      method: 'GET',
      params: {
        tag: 'avatargallery',
        galleryId: avatarId,
        n: 100,
        offset: 0,
      },
    }),

  uploadAvatarImage: (imageData: string) =>
    uploadImage(imageData, { tag: 'avatarimage' }),

  uploadAvatarGalleryImage: (imageData: string, avatarId: string) =>
    uploadImage(imageData, { tag: 'avatargallery', galleryId: avatarId }),

  setAvatarGalleryOrder: (ids: string[]) =>
    request('/files/order', { method: 'PUT', params: { ids } }),

  getLicensedAvatars: (params: { n?: number, offset?: number }) =>
    request('/avatars/licensed', { method: 'GET', params }),
};
