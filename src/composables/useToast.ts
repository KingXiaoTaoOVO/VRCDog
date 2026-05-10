import { ref } from 'vue';

export interface ToastMessage {
  id: number;
  message: string;
  type: 'success' | 'error' | 'info' | 'warning';
}

const toasts = ref<ToastMessage[]>([]);
let nextId = 0;

export function useToast() {
  const addToast = (message: string, type: ToastMessage['type'] = 'info', duration = 3000) => {
    const id = nextId++;
    toasts.value.push({ id, message, type });
    setTimeout(() => {
      removeToast(id);
    }, duration);
  };

  const removeToast = (id: number) => {
    const index = toasts.value.findIndex(t => t.id === id);
    if (index > -1) {
      toasts.value.splice(index, 1);
    }
  };

  return {
    toasts,
    addToast,
    removeToast,
    success: (msg: string, d?: number) => addToast(msg, 'success', d),
    error: (msg: string, d?: number) => addToast(msg, 'error', d),
    info: (msg: string, d?: number) => addToast(msg, 'info', d),
    warning: (msg: string, d?: number) => addToast(msg, 'warning', d)
  };
}
