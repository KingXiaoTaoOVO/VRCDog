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
  <!-- 使用琥珀色主题的半透明背景，通过 mousedown 手动调用 startDragging() 实现完美拖拽 -->
  <div
    class="h-screen w-screen overflow-hidden bg-indigo-50/30 backdrop-blur-sm border border-indigo-300/30 rounded-lg flex flex-col justify-end p-4 pb-8 select-none"
    @mousedown="startDrag"
  >
    <!-- 没有任何记录时的提示语 -->
    <div
      v-if="logs.length === 0"
      class="flex flex-col items-center justify-center h-full w-full opacity-50 pointer-events-none"
    >
      <div
        class="px-4 py-2 rounded-xl bg-black/20 text-white font-bold text-sm backdrop-blur-md border border-white/10 text-center"
        v-html="t('overlay.ready_desc')"
      />
    </div>

    <!-- 聊天气泡区域 -->
    <div
      v-else
      class="space-y-3 max-h-[80vh] overflow-y-auto pr-2 custom-scrollbar"
      data-tauri-drag-region
    >
      <div
        v-for="log in logs"
        :key="log.id" 
        class="flex flex-col animate-fade-in"
        :class="log.type === 'self' ? 'items-end' : 'items-start'"
      >
        <div
          class="max-w-[85%] rounded-2xl p-3 shadow-lg backdrop-blur-md border border-white/20"
          :class="log.type === 'self' ? 'bg-indigo-500/80 text-white rounded-br-sm' : 'bg-white/80 text-slate-800 rounded-bl-sm'"
        >
          <p class="text-[13px] opacity-80 font-medium mb-0.5">
            {{ log.text }}
          </p>
          <p
            v-if="log.translation"
            class="text-[16px] font-bold leading-snug"
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

.custom-scrollbar::-webkit-scrollbar {
  width: 4px;
}
.custom-scrollbar::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.3);
  border-radius: 4px;
}
</style>
