<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
import { DbApi } from "../api";
import { Globe2, Rocket, ArrowRightCircle, ArrowLeftCircle, Home, UserPlus, Image as ImageIcon } from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';

const { t } = useI18n();

interface LogEvent {
  time: string;
  event_type: string;
  content: string;
}

const events = ref<LogEvent[]>([]);
const loading = ref(true);

const fetchLogs = async () => {
  try {
    const res: any = await DbApi.getGameLogs({ limit: 500, offset: 0 });
    // 对日志进行简单的清洗和过滤，只保留我们关心的事件
    events.value = res.filter((e: LogEvent) => 
      ['Player Joined', 'Player Left', 'Instance Joined'].includes(e.event_type)
    );
  } catch (err) {
    console.error(err);
  } finally {
    loading.value = false;
  }
};

onMounted(() => {
  fetchLogs();
  window.addEventListener('vrc-gamelog-updated', fetchLogs);
});

onUnmounted(() => {
  window.removeEventListener('vrc-gamelog-updated', fetchLogs);
});

const getEventMeta = (type: string) => {
  switch(type) {
    case 'Player Joined': return { icon: ArrowRightCircle, color: 'text-green-500', bg: 'bg-green-100', verb: t('feed.verb_joined') };
    case 'Player Left': return { icon: ArrowLeftCircle, color: 'text-gray-400', bg: 'bg-gray-100', verb: t('feed.verb_left') };
    case 'Instance Joined': return { icon: Home, color: 'text-blue-500', bg: 'bg-blue-100', verb: t('feed.verb_instance') };
    default: return { icon: Rocket, color: 'text-amber-500', bg: 'bg-amber-100', verb: t('feed.verb_unknown') };
  }
};
</script>

<template>
  <div class="h-full flex flex-col">
    <header class="mb-6 flex justify-between items-end">
      <div>
        <h1 class="text-3xl font-extrabold text-[#451a03] tracking-tight flex items-center gap-3">
          {{ t('feed.title') }}
          <span class="inline-flex items-center justify-center p-1.5 bg-blue-100 rounded-xl">
            <Globe2 class="w-6 h-6 text-blue-600" />
          </span>
        </h1>
        <p class="text-amber-700/80 font-medium mt-1">
          {{ t('feed.subtitle') }}
        </p>
      </div>
      <div
        v-if="loading"
        class="text-blue-500 font-bold flex items-center gap-2 animate-pulse text-sm"
      >
        <Rocket class="w-4 h-4 animate-bounce" /> {{ t('feed.listening') }}
      </div>
    </header>

    <div class="flex-1 bg-white/60 backdrop-blur-md border-2 border-white rounded-3xl p-6 shadow-lg overflow-y-auto custom-scrollbar">
      <div
        v-if="events.length === 0 && !loading"
        class="h-full flex flex-col items-center justify-center text-blue-900/40"
      >
        <Globe2
          class="mb-4 animate-spin-slow"
          :size="48"
        />
        <p class="font-bold text-lg">
          {{ t('feed.silent') }}
        </p>
        <p class="text-sm mt-1">
          {{ t('feed.go_make_friends') }}
        </p>
      </div>

      <!-- 时间轴布局 -->
      <div
        v-else
        class="relative border-l-4 border-blue-100/50 ml-4 space-y-8 pb-8"
      >
        <div
          v-for="(evt, idx) in events"
          :key="idx"
          class="relative pl-8"
        >
          <!-- 轴上的小圆点/图标 -->
          <div class="absolute -left-[22px] top-1">
            <div
              class="w-10 h-10 rounded-full shadow-md flex items-center justify-center border-4 border-white z-10 relative"
              :class="getEventMeta(evt.event_type).bg"
            >
              <component
                :is="getEventMeta(evt.event_type).icon"
                :size="18"
                :class="getEventMeta(evt.event_type).color"
              />
            </div>
          </div>

          <!-- 内容卡片 -->
          <div class="bg-white/80 hover:bg-white backdrop-blur rounded-2xl p-4 shadow-sm hover:shadow-md transition-all border border-blue-50 relative group">
            <!-- 小三角 -->
            <div class="absolute top-4 -left-2 w-4 h-4 bg-white/80 border-l border-b border-blue-50 transform rotate-45 group-hover:bg-white transition-colors" />
            
            <div class="flex justify-between items-start mb-2">
              <div
                class="font-bold text-sm"
                :class="getEventMeta(evt.event_type).color"
              >
                {{ evt.event_type }}
              </div>
              <div class="text-xs text-amber-900/40 font-mono font-bold bg-amber-50 px-2 py-0.5 rounded-md">
                {{ evt.time }}
              </div>
            </div>
            
            <div class="text-amber-950 font-medium flex items-center gap-2">
              <span
                v-if="evt.event_type !== 'Instance Joined'"
                class="font-extrabold text-lg"
              >{{ evt.content }}</span>
              <span
                v-else
                class="text-blue-700 font-bold bg-blue-50 px-2 py-1 rounded-lg break-all"
              >📍 {{ evt.content }}</span>
              <span class="text-amber-700/60 text-sm">{{ getEventMeta(evt.event_type).verb }}</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.custom-scrollbar::-webkit-scrollbar { width: 6px; }
.custom-scrollbar::-webkit-scrollbar-track { background: transparent; }
.custom-scrollbar::-webkit-scrollbar-thumb { background: rgba(59, 130, 246, 0.2); border-radius: 10px; }
.custom-scrollbar::-webkit-scrollbar-thumb:hover { background: rgba(59, 130, 246, 0.4); }
.animate-spin-slow { animation: spin 8s linear infinite; }
</style>
