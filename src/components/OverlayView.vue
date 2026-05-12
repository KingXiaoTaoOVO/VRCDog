<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useI18n } from 'vue-i18n';

const { t } = useI18n();

const startDrag = async (e: MouseEvent) => {
  // 防止点击气泡里的文字时意外拖动
  if ((e.target as HTMLElement).tagName !== 'P') {
    try {
      await getCurrentWindow().startDragging();
    } catch (err) {
      console.warn('Drag error:', err);
    }
  }
};

interface LogMessage {
  id: number;
  type: 'self' | 'other';
  text: string;
  translation?: string;
}

const logs = ref<LogMessage[]>([]);

onMounted(() => {
  // 监听来自主窗口的翻译日志事件
  listen('translation-log', (event: any) => {
    const payload = event.payload as LogMessage;
    logs.value.unshift({
      ...payload,
      id: Date.now()
    });
    
    // 保持最多显示 10 条，并自动滚动到顶部
    if (logs.value.length > 10) {
      logs.value.pop();
    }
    
    setTimeout(() => {
      window.scrollTo({ top: 0, behavior: 'smooth' });
    }, 50);
  });

  // 监听强制关闭指令
  listen('cmd-close-overlay', async () => {
    try {
      await getCurrentWindow().destroy();
    } catch (e) {}
  });
});
</script>

<template>
  <div
    class="h-screen w-screen overflow-hidden bg-black border border-[var(--theme-primary)] rounded-xl flex flex-col justify-end p-4 pb-8 select-none shadow-[0_0_20px_rgba(var(--theme-primary),0.3)] font-mono"
    @mousedown="startDrag"
  >
    <!-- 没有任何记录时的提示语 -->
    <div
      v-if="logs.length === 0"
      class="flex flex-col items-center justify-center h-full w-full opacity-50 pointer-events-none"
    >
      <div
        class="px-4 py-2 text-[var(--theme-primary)] font-bold text-sm text-center"
        v-html="t('overlay.ready_desc')"
      />
    </div>

    <!-- 聊天气泡区域 -->
    <div
      v-else
      class="space-y-2 max-h-[80vh] overflow-y-auto pr-2 custom-scrollbar"
      data-tauri-drag-region
    >
      <div
        v-for="log in logs"
        :key="log.id" 
        class="flex flex-col animate-fade-in w-full"
      >
        <div class="w-full">
          <p class="text-[13px] opacity-70 font-medium mb-0.5 flex items-start gap-2" :class="log.type === 'self' ? 'text-[var(--theme-primary)]' : 'text-emerald-400'">
            <span class="opacity-50 mt-0.5">&gt;</span> 
            <span class="break-words flex-1">{{ log.text }}</span>
          </p>
          <p
            v-if="log.translation"
            class="text-[16px] font-bold leading-snug pl-4 break-words"
            :class="log.type === 'self' ? 'text-[var(--theme-primary)]' : 'text-emerald-400'"
          >
            {{ log.translation }}
          </p>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.animate-fade-in {
  animation: fadeIn 0.3s ease-out forwards;
}

@keyframes fadeIn {
  from { opacity: 0; transform: translateY(10px); }
  to { opacity: 1; transform: translateY(0); }
}



</style>
