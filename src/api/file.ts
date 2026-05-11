import { request } from './request';

export const FileApi = {
  getFile: (fileId: string) =>
    request(`/file/${fileId}`),

  deleteFile: (fileId: string) =>
    request(`/file/${fileId}`, { method: 'DELETE' }),

  deleteFileVersion: (fileId: string, version: number) =>
    request(`/file/${fileId}/${version}`, { method: 'DELETE' }),

  getFileAnalysis: (fileId: string, version: number, variant: string) =>
    request(`/analysis/${fileId}/${version}/${variant}`),
};
