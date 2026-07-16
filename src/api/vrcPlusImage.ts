import { getStoredAuthCookie, parseExecuteResponse, request, safeInvoke } from './request';

function toCleanBase64(imageData: string) {
  return imageData.includes(',') ? imageData.split(',')[1] : imageData;
}

async function uploadImage(imageData: string, params: Record<string, unknown>) {
  const authCookie = await getStoredAuthCookie();
  const formData: any[] = [
    { name: 'data', value: JSON.stringify({ tag: 'gallery', ...params }) },
    {
      name: 'image',
      file_name: 'image.png',
      file_content_base64: toCleanBase64(imageData),
      file_mime: 'image/png',
    },
  ];

  const res = await safeInvoke('vrc_execute', {
    options: {
      url: 'https://api.vrchat.cloud/api/1/file/image',
      method: 'POST',
      auth_cookie: authCookie,
      form_data: formData,
    },
  });
  return parseExecuteResponse(res, 'https://api.vrchat.cloud/api/1/file/image');
}

async function resolveCurrentUserId(userId?: string) {
  if (userId) return userId;
  const currentUser: any = await request('/auth/user', { method: 'GET' });
  if (!currentUser?.id) throw new Error('无法获取当前用户 ID');
  return currentUser.id;
}

export const VrcPlusImageApi = {
  uploadGalleryImage: (imageData: string) =>
    uploadImage(imageData, { tag: 'gallery' }),

  uploadSticker: (imageData: string, params: Record<string, unknown>) =>
    uploadImage(imageData, params),

  uploadEmoji: (imageData: string, params: Record<string, unknown>) =>
    uploadImage(imageData, params),

  getPrints: async (params: { userId?: string; n?: number; offset?: number } = {}) => {
    const userId = await resolveCurrentUserId(params.userId);
    const { userId: _userId, ...query } = params;
    return request(`/prints/user/${userId}`, { method: 'GET', params: query });
  },

  getPrint: (params: { printId: string }) =>
    request(`/prints/${params.printId}`, { method: 'GET' }),

  deletePrint: (printId: string) =>
    request(`/prints/${printId}`, { method: 'DELETE' }),

  uploadPrint: async (imageData: string, cropWhiteBorder: boolean, params: Record<string, unknown>) => {
    const authCookie = await getStoredAuthCookie();
    const res = await safeInvoke('vrc_execute', {
      options: {
        url: `https://api.vrchat.cloud/api/1/prints${cropWhiteBorder ? '?cropWhiteBorder=true' : ''}`,
        method: 'POST',
        auth_cookie: authCookie,
        form_data: [
          ...Object.entries(params).map(([name, value]) => ({ name, value: String(value) })),
          {
            name: 'image',
            file_name: 'image',
            file_content_base64: toCleanBase64(imageData),
            file_mime: 'image/png',
          },
        ],
      },
    });
    return parseExecuteResponse(res, 'https://api.vrchat.cloud/api/1/prints');
  },

  createPrint: (imageData: string, params: Record<string, unknown> = {}) =>
    VrcPlusImageApi.uploadPrint(imageData, false, params),
};
