<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useStorage } from '@vueuse/core';
import { useI18n } from 'vue-i18n';
import { VrctApi } from '../api';

const { t } = useI18n();

const startDrag = async (e: MouseEvent) => {
  const target = e.target as HTMLElement | null;
  // 防止点击气泡里的文字时意外拖动
  if (target && !target.closest('p, [data-no-drag]')) {
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
const collapsed = useStorage('vrc_translation_overlay_collapsed', false);
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
      const nearBottom = window.innerHeight + window.scrollY >= document.body.scrollHeight - 100;
      if (nearBottom) {
        window.scrollTo({ top: 0, behavior: 'smooth' });
      }
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
    class="overlay-shell h-screen w-screen overflow-hidden rounded-2xl flex flex-col select-none font-mono"
    :style="panelStyle"
    @mousedown="startDrag"
  >
    <header class="overlay-header flex items-center justify-between gap-3 px-3 py-2 shrink-0" data-no-drag>
      <div class="flex items-center gap-2 min-w-0">
        <span class="status-dot" :class="{ active: logs.length > 0 }" />
        <span class="text-[11px] font-black tracking-[0.16em] uppercase text-text truncate">VRCDOG / LIVE TRANSLATION</span>
      </div>
      <div class="flex items-center gap-1">
        <span class="text-[10px] font-bold text-text-muted mr-1">{{ logs.length }}/10</span>
        <button class="overlay-action" :title="collapsed ? '展开' : '收起'" @click.stop="collapsed = !collapsed">{{ collapsed ? '+' : '−' }}</button>
        <button class="overlay-action" title="关闭" @click.stop="getCurrentWindow().destroy()">×</button>
      </div>
    </header>

    <!-- 没有任何记录时的提示语 -->
    <div
      v-if="collapsed"
      class="flex-1 flex items-center justify-center text-[11px] text-text-muted font-bold"
    >
      {{ logs.length ? 'Translation overlay paused' : 'Translation overlay ready' }}
    </div>
    <div
      v-else-if="logs.length === 0"
      class="flex flex-col items-center justify-center flex-1 w-full opacity-60 pointer-events-none"
    >
      <div
        class="px-4 py-2 text-[var(--theme-primary)] font-bold text-sm text-center"
        v-html="t('overlay.ready_desc')"
      />
    </div>

    <!-- 聊天气泡区域 -->
    <div
      v-else
      class="space-y-2 flex-1 min-h-0 overflow-y-auto p-3 pb-5 custom-scrollbar"
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
.overlay-shell {
  border: 1px solid color-mix(in srgb, var(--theme-primary) 58%, transparent);
  background-image: linear-gradient(145deg, rgba(255,255,255,.08), transparent 42%);
  box-shadow: 0 14px 40px rgba(0,0,0,.28), 0 0 24px color-mix(in srgb, var(--theme-primary) 26%, transparent);
  backdrop-filter: blur(18px) saturate(1.2);
}
.overlay-header {
  border-bottom: 1px solid rgba(255,255,255,.12);
  background: rgba(255,255,255,.06);
}
.status-dot { width: 7px; height: 7px; border-radius: 999px; background: #94a3b8; }
.status-dot.active { background: #34d399; box-shadow: 0 0 0 4px rgba(52,211,153,.16), 0 0 10px rgba(52,211,153,.6); }
.overlay-action { width: 22px; height: 22px; border-radius: 7px; color: var(--theme-text-muted); font-size: 16px; line-height: 1; }
.overlay-action:hover { color: var(--theme-text); background: rgba(255,255,255,.12); }
.animate-fade-in {
  animation: fadeIn 0.3s ease-out forwards;
}

@keyframes fadeIn {
  from { opacity: 0; transform: translateY(10px); }
  to { opacity: 1; transform: translateY(0); }
}



</style>
