import { ref } from 'vue';
import { isTauri } from '@tauri-apps/api/core';
import { ServerApi } from '../api';

export const webBackendChecked = ref(false);
export const webBackendOk = ref(false);
export const webBackendError = ref('');

export async function checkWebBackend(): Promise<boolean> {
  if (isTauri()) {
    webBackendChecked.value = true;
    webBackendOk.value = true;
    return true;
  }

  webBackendError.value = '';
  webBackendChecked.value = false;

  try {
    const result = await ServerApi.ping();
    if (result.status === 'ok') {
      webBackendOk.value = true;
      webBackendChecked.value = true;
      return true;
    }
    webBackendOk.value = false;
    webBackendError.value = result.message || 'Backend unreachable';
    webBackendChecked.value = true;
    return false;
  } catch (err: any) {
    webBackendOk.value = false;
    webBackendError.value = err?.message || '无法连接服务器';
    webBackendChecked.value = true;
    return false;
  }
}
