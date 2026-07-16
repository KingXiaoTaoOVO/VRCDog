import { getStoredAuthCookie, parseExecuteResponse, request, safeInvoke } from './request';

function toCleanBase64(imageData: string) {
  return imageData.includes(',') ? imageData.split(',')[1] : imageData;
}

export const VrcPlusIconApi = {
  getFileList: (params: { tag?: string; userId?: string; n?: number; offset?: number } = {}) =>
    request('/files', { method: 'GET', params }),

  deleteFile: (fileId: string) =>
    request(`/file/${fileId}`, { method: 'DELETE' }),

  deleteFileVersion: (params: { fileId: string; version: string | number }) =>
    request(`/file/${params.fileId}/${params.version}`, { method: 'DELETE' }),

  uploadVrcPlusIcon: async (imageData: string) => {
    const authCookie = await getStoredAuthCookie();
    const res = await safeInvoke('vrc_execute', {
      options: {
        url: 'https://api.vrchat.cloud/api/1/file/image',
        method: 'POST',
        auth_cookie: authCookie,
        form_data: [
          { name: 'data', value: JSON.stringify({ tag: 'icon' }) },
          {
            name: 'image',
            file_name: 'icon.png',
            file_content_base64: toCleanBase64(imageData),
            file_mime: 'image/png',
          },
        ],
      },
    });
    return parseExecuteResponse(res, 'https://api.vrchat.cloud/api/1/file/image');
  },

  uploadVRCPlusIcon: (imageData: string) =>
    VrcPlusIconApi.uploadVrcPlusIcon(imageData),
};
