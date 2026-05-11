<script setup lang="ts">
import { useI18n } from 'vue-i18n';
const { t } = useI18n();
import { ref, onMounted, onUnmounted, nextTick } from 'vue';
import { DbApi } from '../api';
import { Terminal, X, Trash2, Maximize2, Minimize2, CheckCircle2, AlertCircle, Download } from 'lucide-vue-next';

interface DebugLogEntry {
  timestamp: string;
  type: string;
  cmd: string;
  duration: number;
  args?: any;
  error?: string;
  [key: string]: any;
}

const isVisible = ref(false);
const isExpanded = ref(false);
const isMasterEnabled = ref(false); // 默认对普通用户关闭
const logs = ref<DebugLogEntry[]>([]);
const logContainer = ref<HTMLElement | null>(null);

const handleNewLog = (event: Event) => {
  if (!isMasterEnabled.value) return; // 关闭时不记录
  const e = event as CustomEvent;
  if (!e.detail) return;
  
  // 只保留最近 200 条日志，防止内存泄漏
  if (logs.value.length > 200) {
    logs.value.pop();
  }
  
  logs.value.unshift(e.detail);
  
  // 自动滚动到顶部
  nextTick(() => {
    if (logContainer.value) {
      logContainer.value.scrollTop = 0;
    }
  });
};

const handleSettingsUpdate = (event: Event) => {
  const e = event as CustomEvent;
  if (e.detail && typeof e.detail.enableDebugConsole === 'boolean') {
    isMasterEnabled.value = e.detail.enableDebugConsole;
    if (!isMasterEnabled.value) {
      isVisible.value = false;
      logs.value = [];
    }
  }
};

const loadConfig = async () => {
  try {
    const all = await DbApi.getAllSettings();
    if (all && all.enableDebugConsole !== undefined) {
      isMasterEnabled.value = all.enableDebugConsole === 'true' || all.enableDebugConsole === true;
    }
  } catch { /* ignore */ }
};

onMounted(() => {
  loadConfig();
  window.addEventListener('app-debug-log', handleNewLog);
  window.addEventListener('settings-updated', handleSettingsUpdate);
});

onUnmounted(() => {
  window.removeEventListener('app-debug-log', handleNewLog);
  window.removeEventListener('settings-updated', handleSettingsUpdate);
});

const clearLogs = () => {
  logs.value = [];
};

const exportLogs = async () => {
  if (!logs.value.length) return;
  
  let txtContent = "=== VrcDog API Debug Log ===\n";
  txtContent += `Exported At: ${new Date().toLocaleString()}\n\n`;
  
  for (const log of logs.value) {
    txtContent += `[${log.timestamp}] [${log.type.toUpperCase()}] ${log.cmd} (${log.duration}ms)\n`;
    if (log.args && Object.keys(log.args).length > 0) {
      txtContent += `Args: ${JSON.stringify(log.args)}\n`;
    }
    if (log.error) {
      txtContent += `Error: ${log.error}\n`;
    }
    txtContent += `----------------------------------------\n`;
  }

  try {
    const { save } = await import('@tauri-apps/plugin-dialog');
    const { SysApi } = await import('../api');
    
    const filePath = await save({
      filters: [{ name: 'Text', extensions: ['txt'] }],
      defaultPath: `vrcdog-debug-log-${new Date().toISOString().replace(/[:.]/g, '-')}.txt`
    });
    
    if (filePath) {
      await SysApi.saveTextFile({ path: filePath, content: txtContent });
    }
  } catch (err) {
    console.error('Failed to export log:', err);
    // Fallback if not in Tauri
    const dataUri = 'data:text/plain;charset=utf-8,' + encodeURIComponent(txtContent);
    const linkElement = document.createElement('a');
    linkElement.setAttribute('href', dataUri);
    linkElement.setAttribute('download', `vrcdog-debug-log-${new Date().toISOString().replace(/[:.]/g, '-')}.txt`);
    linkElement.click();
  }
};

const formatArgs = (args: any) => {
  if (!args) return '';
  try {
    return JSON.stringify(args, null, 2);
  } catch {
    return String(args);
  }
};
</script>

<template>
  <!-- 悬浮触发按钮 -->
  <button
    v-if="!isVisible && isMasterEnabled"
    class="fixed bottom-4 right-4 z-[999] bg-surface-hover hover:bg-surface-active text-green-400 p-3 rounded-full shadow-2xl transition-all hover:scale-110 border-border-strong flex items-center justify-center group"
    @click="isVisible = true"
  >
    <Terminal :size="20" />
    <span class="absolute right-full mr-3 whitespace-nowrap bg-surface-hover text-xs text-text px-2 py-1 rounded opacity-0 group-hover:opacity-100 transition-opacity pointer-events-none">{{ t('debug.open_debugger') }}</span>
  </button>

  <!-- 调试控制台面板 -->
  <div
    v-if="isVisible && isMasterEnabled" 
    class="fixed bottom-0 right-0 z-[1000] terminal-black shadow-2xl border-l border-t border-white/10 flex flex-col transition-all duration-300 ease-in-out"
    :class="isExpanded ? 'w-full h-[60vh]' : 'w-[450px] h-[400px] rounded-tl-2xl'"
  >
    <!-- 头部工具栏 -->
    <div
      class="h-10 bg-surface-hover flex items-center justify-between px-3 border-border-soft shrink-0"
      :class="{ 'rounded-tl-xl': !isExpanded }"
    >
      <div class="flex items-center gap-2 text-text font-mono text-xs select-none">
        <Terminal
          :size="14"
          class="text-green-500"
        /> API Debug Console
        <span class="bg-surface-active px-1.5 py-0.5 rounded text-[10px]">{{ logs.length }}</span>
      </div>
      <div class="flex items-center gap-1.5">
        <button
          class="p-1 hover:bg-surface-active text-text hover:text-green-400 rounded transition-colors"
          :title="t('debug.export_logs')"
          @click="exportLogs"
        >
          <Download :size="14" />
        </button>
        <button
          class="p-1 hover:bg-surface-active text-text hover:text-red-400 rounded transition-colors"
          :title="t('debug.clear_logs')"
          @click="clearLogs"
        >
          <Trash2 :size="14" />
        </button>
        <button
          class="p-1 hover:bg-surface-active text-text hover:text-white rounded transition-colors"
          :title="isExpanded ? t('debug.restore') : t('debug.maximize')"
          @click="isExpanded = !isExpanded"
        >
          <component
            :is="isExpanded ? Minimize2 : Maximize2"
            :size="14"
          />
        </button>
        <div class="w-px h-4 bg-surface-hover mx-1" />
        <button
          class="p-1 hover:bg-red-500 hover:text-white text-text rounded transition-colors"
          :title="t('debug.close')"
          @click="isVisible = false"
        >
          <X :size="16" />
        </button>
      </div>
    </div>

    <!-- 日志列表 -->
    <div
      ref="logContainer"
      class="flex-1 overflow-y-auto p-2 space-y-2 bg-background font-mono text-[11px] custom-scrollbar"
    >
      <div
        v-if="logs.length === 0"
        class="flex h-full items-center justify-center text-text-muted italic select-none"
      >
        {{ t('debug.waiting_api') }}
      </div>
      
      <div
        v-for="(log, idx) in logs"
        :key="idx" 
        class="rounded p-2 relative group"
        :class="log.type === 'error' ? 'bg-red-500/10 border-red-500/20 text-red-500' : 'bg-surface border-border-soft text-text'"
      >
        <div class="flex items-center justify-between mb-1 opacity-80">
          <div class="flex items-center gap-1.5">
            <CheckCircle2
              v-if="log.type === 'success'"
              :size="12"
              class="text-green-500"
            />
            <AlertCircle
              v-else
              :size="12"
              class="text-red-500"
            />
            <span class="font-bold text-blue-400">{{ log.cmd }}</span>
            <span class="text-text-muted">{{ log.duration }}ms</span>
          </div>
          <span class="text-[10px] text-text-muted">{{ log.timestamp }}</span>
        </div>

        <div
          v-if="log.args && Object.keys(log.args).length > 0"
          class="mt-1.5 p-1.5 bg-background rounded border-border-soft/50 overflow-x-auto whitespace-pre"
        >
          <span class="text-primary">Args:</span> {{ formatArgs(log.args) }}
        </div>

        <div
          v-if="log.type === 'error'"
          class="mt-1.5 p-1.5 bg-red-500/10 rounded border-red-500/20 text-red-500 whitespace-pre-wrap font-bold"
        >
          {{ log.error }}
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.custom-scrollbar::-webkit-scrollbar { width: 6px; height: 6px; }
.custom-scrollbar::-webkit-scrollbar-track { background: transparent; }
.custom-scrollbar::-webkit-scrollbar-thumb { background: #334155; border-radius: 4px; }
.custom-scrollbar::-webkit-scrollbar-thumb:hover { background: #475569; }
</style>
