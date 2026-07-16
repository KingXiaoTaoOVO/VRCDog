import { request } from './request';

type FileVersionParams = {
  fileId: string;
  fileVersion: string | number;
};

const finishParams = {
  maxParts: 0,
  nextPartNumber: 0,
};

export const ImageApi = {
  uploadAvatarFailCleanup: async (fileId: string) => {
    const file: any = await request(`/file/${fileId}`, { method: 'GET' });
    const latest = Array.isArray(file?.versions) ? file.versions[file.versions.length - 1] : null;
    if (!latest?.version) return file;
    await Promise.allSettled([
      request(`/file/${fileId}/${latest.version}/signature/finish`, { method: 'PUT' }),
      request(`/file/${fileId}/${latest.version}/file/finish`, { method: 'PUT' }),
    ]);
    return file;
  },

  uploadWorldFailCleanup: async (fileId: string) => {
    const file: any = await request(`/file/${fileId}`, { method: 'GET' });
    const latest = Array.isArray(file?.versions) ? file.versions[file.versions.length - 1] : null;
    if (!latest?.version) return file;
    await Promise.allSettled([
      request(`/file/${fileId}/${latest.version}/signature/finish`, { method: 'PUT' }),
      request(`/file/${fileId}/${latest.version}/file/finish`, { method: 'PUT' }),
    ]);
    return file;
  },

  uploadAvatarImage: (params: Record<string, unknown>, fileId: string) =>
    request(`/file/${fileId}`, { method: 'POST', params }),

  uploadAvatarImageFileStart: (params: FileVersionParams) =>
    request(`/file/${params.fileId}/${params.fileVersion}/file/start`, { method: 'PUT' }),

  uploadAvatarImageFileFinish: (params: FileVersionParams) =>
    request(`/file/${params.fileId}/${params.fileVersion}/file/finish`, {
      method: 'PUT',
      params: finishParams,
    }),

  uploadAvatarImageSigStart: (params: FileVersionParams) =>
    request(`/file/${params.fileId}/${params.fileVersion}/signature/start`, { method: 'PUT' }),

  uploadAvatarImageSigFinish: (params: FileVersionParams) =>
    request(`/file/${params.fileId}/${params.fileVersion}/signature/finish`, {
      method: 'PUT',
      params: finishParams,
    }),

  setAvatarImage: (params: { id: string; [key: string]: unknown }) =>
    request(`/avatars/${params.id}`, { method: 'PUT', params }),

  uploadWorldImage: (params: Record<string, unknown>, fileId: string) =>
    request(`/file/${fileId}`, { method: 'POST', params }),

  uploadWorldImageFileStart: (params: FileVersionParams) =>
    request(`/file/${params.fileId}/${params.fileVersion}/file/start`, { method: 'PUT' }),

  uploadWorldImageFileFinish: (params: FileVersionParams) =>
    request(`/file/${params.fileId}/${params.fileVersion}/file/finish`, {
      method: 'PUT',
      params: finishParams,
    }),

  uploadWorldImageSigStart: (params: FileVersionParams) =>
    request(`/file/${params.fileId}/${params.fileVersion}/signature/start`, { method: 'PUT' }),

  uploadWorldImageSigFinish: (params: FileVersionParams) =>
    request(`/file/${params.fileId}/${params.fileVersion}/signature/finish`, {
      method: 'PUT',
      params: finishParams,
    }),

  setWorldImage: (params: { id: string; [key: string]: unknown }) =>
    request(`/worlds/${params.id}`, { method: 'PUT', params }),

  getAvatarImages: (params: { fileId: string }) =>
    request(`/file/${params.fileId}`, { method: 'GET' }),

  getWorldImages: (params: { fileId: string; [key: string]: unknown }) =>
    request(`/file/${params.fileId}`, { method: 'GET', params }),
};
