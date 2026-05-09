<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
import { VrcApi, DbApi, SysApi, GamelogApi } from "../api";
import { ScrollText, Loader2, ArrowRightCircle, ArrowLeftCircle, Home, KeyRound, MonitorPlay } from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';

const { t } = useI18n();

interface LogEvent {
  time: string;
  event_type: string;
  content: string;
}

const logs = ref<LogEvent[]>([]);
const loading = ref(true);
const errorMsg = ref('');

const fetchLogs = async () => {
  errorMsg.value = '';
  try {
    const res: any = await DbApi.getGameLogs({ limit: 500, offset: 0 });
    logs.value = res;
  } catch (err: any) {
    errorMsg.value = t('gamelog.error_read') + ': ' + (err.message || err);
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

const getEventIcon = (type: string) => {
  if (type === 'Player Joined') return ArrowRightCircle;
  if (type === 'Player Left') return ArrowLeftCircle;
  if (type === 'Instance Joined') return Home;
  if (type === 'Authenticated') return KeyRound;
  if (type === 'Video Playback') return MonitorPlay;
  return ScrollText;
};

const getEventColor = (type: string) => {
  if (type === 'Player Joined') return 'text-green-500 bg-green-50 border-green-100';
  if (type === 'Player Left') return 'text-gray-500 bg-gray-50 border-gray-100';
  if (type === 'Instance Joined') return 'text-blue-500 bg-blue-50 border-blue-100';
  if (type === 'Authenticated') return 'text-purple-500 bg-purple-50 border-purple-100';
  if (type === 'Video Playback') return 'text-pink-500 bg-pink-50 border-pink-100';
  return 'text-amber-500 bg-amber-50 border-amber-100';
};
</script>

<template>
  <div class="h-full flex flex-col">
    <div class="flex items-center justify-between mb-4">
      <h1 class="text-2xl font-extrabold text-[#451a03] tracking-tight flex items-center gap-2">
        <ScrollText
          class="text-amber-500"
          :size="28"
        /> {{ t('gamelog.title') }}
      </h1>
      <div class="flex items-center gap-2">
        <span
          v-if="loading"
          class="text-xs text-amber-500 font-bold flex items-center gap-1"
        >
          <Loader2
            class="animate-spin"
            :size="12"
          /> {{ t('gamelog.sniffing') }}
        </span>
      </div>
    </div>

    <div
      v-if="errorMsg"
      class="bg-red-50 text-red-600 p-3 rounded-xl border border-red-200 text-sm font-bold mb-4"
    >
      {{ errorMsg }}
    </div>

    <div class="flex-1 overflow-y-auto pr-1 bg-white/80 backdrop-blur rounded-2xl border border-amber-100 shadow-inner p-4 font-mono text-sm">
      <div
        v-if="loading && logs.length === 0"
        class="flex items-center justify-center py-12 text-amber-500 h-full font-bold"
      >
        <Loader2
          class="animate-spin mr-2"
          :size="24"
        /> {{ t('gamelog.loading') }}
      </div>
      
      <div
        v-else-if="logs.length === 0"
        class="text-center text-amber-500 py-12 h-full flex flex-col items-center justify-center font-bold"
      >
        <ScrollText
          class="mb-3 opacity-50"
          :size="48"
        />
        <p>{{ t('gamelog.no_logs') }} 🐕</p>
        <p class="text-xs mt-2 opacity-70">
          {{ t('gamelog.make_sure_running') }}
        </p>
      </div>

      <div
        v-else
        class="space-y-2"
      >
        <div
          v-for="(log, idx) in logs"
          :key="idx" 
          class="flex items-start gap-3 p-2 rounded-lg border transition-colors hover:shadow-sm"
          :class="getEventColor(log.event_type)"
        >
          <div class="w-24 flex-shrink-0 text-[10px] opacity-70 pt-0.5 font-bold">
            {{ log.time.split(' ')[1] }}
          </div>
          
          <div class="flex items-center gap-2 w-32 flex-shrink-0 font-bold">
            <component
              :is="getEventIcon(log.event_type)"
              :size="14"
            />
            <span class="text-xs">{{ log.event_type }}</span>
          </div>
          
          <div class="flex-1 min-w-0 break-all font-bold opacity-90">
            {{ log.content }}
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
