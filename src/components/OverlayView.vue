<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useStorage } from '@vueuse/core';
import { useI18n } from 'vue-i18n';
import { VrctApi } from '../api';

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
  translations?: Array<{ lang: string; text: string }>;
}

const logs = ref<LogMessage[]>([]);
const backgroundOpacity = useStorage('vrc_translation_overlay_opacity', 0.82);
const panelStyle = computed(() => {
  const opacity = Math.min(1, Math.max(0, Number(backgroundOpacity.value) || 0));
  return { backgroundColor: `rgba(0, 0, 0, ${opacity})` };
});

let unlistenTranslation: UnlistenFn | null = null;
let unlistenClose: UnlistenFn | null = null;
let unlistenSettings: UnlistenFn | null = null;

const addLog = (payload: LogMessage) => {
  if (logs.value.some(log => log.id === payload.id)) return;
  logs.value.unshift(payload);
  if (logs.value.length > 10) logs.value.pop();
};

onMounted(async () => {
  // 监听来自主窗口的翻译日志事件
  unlistenTranslation = await listen('translation-log', (event: any) => {
    const payload = event.payload as LogMessage;
    addLog(payload);
    
    setTimeout(() => {
      window.scrollTo({ top: 0, behavior: 'smooth' });
    }, 50);
  });

  // 监听强制关闭指令
  unlistenClose = await listen('cmd-close-overlay', async () => {
    try {
      await getCurrentWindow().destroy();
    } catch (e) {}
  });

  unlistenSettings = await listen<{ backgroundOpacity?: number }>(
    'translation-overlay-settings',
    (event) => {
      const nextOpacity = Number(event.payload?.backgroundOpacity);
      if (Number.isFinite(nextOpacity)) {
        backgroundOpacity.value = Math.min(1, Math.max(0, nextOpacity));
      }
    },
  );

  try {
    const history = await VrctApi.getHistory();
    if (Array.isArray(history)) {
      const loaded: LogMessage[] = history.slice(-10).reverse().map((record: any) => ({
        id: record.id,
        type: record.source === 'speaker' ? 'other' as const : 'self' as const,
        text: record.original,
        translation: record.translated,
        translations: record.translations?.map((item: any) => ({
          lang: item.target_lang,
          text: item.translated,
        })),
      }));
      const live = logs.value.filter(log => !loaded.some(item => item.id === log.id));
      logs.value = [...live, ...loaded].slice(0, 10);
    }
  } catch {
    // Live events remain available when history loading is unavailable.
  }
});

onUnmounted(() => {
  unlistenTranslation?.();
  unlistenClose?.();
  unlistenSettings?.();
});
</script>

<template>
  <div
    class="h-screen w-screen overflow-hidden border border-[var(--theme-primary)] rounded-xl flex flex-col justify-end p-4 pb-8 select-none shadow-[0_0_20px_rgba(var(--theme-primary),0.3)] font-mono"
    :style="panelStyle"
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
        class="flex animate-fade-in w-full"
        :class="log.type === 'self' ? 'justify-end' : 'justify-start'"
      >
        <div
          class="max-w-[86%] rounded-lg border px-3 py-2"
          :class="log.type === 'self'
            ? 'bg-[var(--theme-primary)]/15 border-[var(--theme-primary)]/35 text-right'
            : 'bg-emerald-500/15 border-emerald-400/35 text-left'"
        >
          <div
            class="text-[10px] font-extrabold uppercase tracking-wide mb-1"
            :class="log.type === 'self' ? 'text-[var(--theme-primary)]' : 'text-emerald-400'"
          >
            {{ log.type === 'self' ? t('overlay.self') : t('overlay.other') }}
          </div>
          <p
            class="text-[12px] opacity-70 font-medium mb-1 break-words"
            :class="log.type === 'self' ? 'text-[var(--theme-primary)]' : 'text-emerald-300'"
          >
            {{ log.text }}
          </p>
          <div v-if="log.translations?.length" class="space-y-1">
            <p
              v-for="item in log.translations"
              :key="item.lang"
              class="text-[16px] font-bold leading-snug break-words"
              :class="log.type === 'self' ? 'text-[var(--theme-primary)]' : 'text-emerald-400'"
            >
              <span v-if="log.translations.length > 1" class="text-[10px] opacity-65 mr-1">{{ item.lang }}</span>
              {{ item.text }}
            </p>
          </div>
          <p
            v-else-if="log.translation"
            class="text-[16px] font-bold leading-snug break-words"
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
