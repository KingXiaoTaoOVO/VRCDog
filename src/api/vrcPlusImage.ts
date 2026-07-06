import { request, safeInvoke } from './request';

function toCleanBase64(imageData: string) {
  return imageData.includes(',') ? imageData.split(',')[1] : imageData;
}

function uploadImage(imageData: string, params: Record<string, unknown>) {
  const formData: any[] = [
    { name: 'tag', value: String(params.tag || 'gallery') },
    {
      name: 'file',
      file_name: 'image.png',
      file_content_base64: toCleanBase64(imageData),
      file_mime: 'image/png',
    },
  ];

  for (const [key, value] of Object.entries(params)) {
    if (key !== 'tag' && value !== undefined && value !== null) {
      formData.push({ name: key, value: String(value) });
    }
  }

  return safeInvoke('vrc_execute', {
    options: {
      url: 'https://api.vrchat.cloud/api/1/file/image',
      method: 'POST',
      form_data: formData,
    },
  });
}

export const VrcPlusImageApi = {
  uploadGalleryImage: (imageData: string) =>
    uploadImage(imageData, { tag: 'gallery' }),

  uploadSticker: (imageData: string, params: Record<string, unknown>) =>
    uploadImage(imageData, params),

  uploadEmoji: (imageData: string, params: Record<string, unknown>) =>
    uploadImage(imageData, params),

  getPrints: (params: { userId: string; n?: number; offset?: number }) =>
    request(`/prints/user/${params.userId}`, { method: 'GET', params }),

  getPrint: (params: { printId: string }) =>
    request(`/prints/${params.printId}`, { method: 'GET' }),

  deletePrint: (printId: string) =>
    request(`/prints/${printId}`, { method: 'DELETE' }),

  uploadPrint: (imageData: string, cropWhiteBorder: boolean, params: Record<string, unknown>) =>
    safeInvoke('vrc_execute', {
      options: {
        url: `https://api.vrchat.cloud/api/1/prints${cropWhiteBorder ? '?cropWhiteBorder=true' : ''}`,
        method: 'POST',
        form_data: [
          ...Object.entries(params).map(([name, value]) => ({ name, value: String(value) })),
          {
            name: 'file',
            file_name: 'print.png',
            file_content_base64: toCleanBase64(imageData),
            file_mime: 'image/png',
          },
        ],
      },
    }),
};
