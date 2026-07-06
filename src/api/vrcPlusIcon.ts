import { request, safeInvoke } from './request';

function toCleanBase64(imageData: string) {
  return imageData.includes(',') ? imageData.split(',')[1] : imageData;
}

export const VrcPlusIconApi = {
  getFileList: (params: { tag?: string; userId?: string; n?: number; offset?: number } = {}) =>
    request('/files', { method: 'GET', params }),

  deleteFile: (fileId: string) =>
    request(`/file/${fileId}`, { method: 'DELETE' }),

  uploadVrcPlusIcon: (imageData: string) =>
    safeInvoke('vrc_execute', {
      options: {
        url: 'https://api.vrchat.cloud/api/1/file/image',
        method: 'POST',
        form_data: [
          { name: 'tag', value: 'icon' },
          {
            name: 'file',
            file_name: 'icon.png',
            file_content_base64: toCleanBase64(imageData),
            file_mime: 'image/png',
          },
        ],
      },
    }),
};
