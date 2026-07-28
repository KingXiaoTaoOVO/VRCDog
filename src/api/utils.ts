import { request } from './request';

/**
 * Resolve the current VRChat user ID.
 * If userId is provided and is not 'me', returns it directly.
 * Otherwise fetches from /auth/user.
 */
export async function resolveCurrentUserId(userId?: string): Promise<string> {
  if (userId && userId !== 'me') return userId;
  const currentUser: any = await request('/auth/user', { method: 'GET' });
  if (!currentUser?.id) {
    throw new Error('Cannot resolve current user ID');
  }
  return currentUser.id;
}

/**
 * Clean base64 data - strip data URL prefix if present.
 */
export function toCleanBase64(base64Data: string): string {
  return base64Data.includes(',') ? base64Data.split(',')[1] : base64Data;
}

/**
 * Build multipart form data for image upload.
 */
export function buildImageUploadFormData(params: {
  data: Record<string, unknown>;
  base64Content: string;
  fileName?: string;
  mimeType?: string;
}): any[] {
  return [
    { name: 'data', value: JSON.stringify(params.data) },
    {
      name: 'image',
      file_name: params.fileName || 'image.png',
      file_content_base64: toCleanBase64(params.base64Content),
      file_mime: params.mimeType || 'image/png',
    }
  ];
}
