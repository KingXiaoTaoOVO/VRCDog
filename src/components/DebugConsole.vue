<script setup lang="ts">
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
    class="fixed bottom-4 right-4 z-[999] bg-slate-800 hover:bg-slate-700 text-green-400 p-3 rounded-full shadow-2xl transition-all hover:scale-110 border border-slate-600 flex items-center justify-center group"
    @click="isVisible = true"
  >
    <Terminal :size="20" />
    <span class="absolute right-full mr-3 whitespace-nowrap bg-slate-800 text-xs text-slate-300 px-2 py-1 rounded opacity-0 group-hover:opacity-100 transition-opacity pointer-events-none">打开接口调试器</span>
  </button>

  <!-- 调试控制台面板 -->
  <div
    v-if="isVisible && isMasterEnabled" 
    class="fixed bottom-0 right-0 z-[1000] bg-slate-900 shadow-2xl border-t border-l border-slate-700 flex flex-col transition-all duration-300 ease-in-out"
    :class="isExpanded ? 'w-full h-[60vh]' : 'w-[450px] h-[400px] rounded-tl-xl'"
  >
    <!-- 头部工具栏 -->
    <div
      class="h-10 bg-slate-800 flex items-center justify-between px-3 border-b border-slate-700 shrink-0"
      :class="{ 'rounded-tl-xl': !isExpanded }"
    >
      <div class="flex items-center gap-2 text-slate-300 font-mono text-xs select-none">
        <Terminal
          :size="14"
          class="text-green-500"
        /> API Debug Console
        <span class="bg-slate-700 px-1.5 py-0.5 rounded text-[10px]">{{ logs.length }}</span>
      </div>
      <div class="flex items-center gap-1.5">
        <button
          class="p-1 hover:bg-slate-700 text-slate-400 hover:text-green-400 rounded transition-colors"
          title="一键导出日志"
          @click="exportLogs"
        >
          <Download :size="14" />
        </button>
        <button
          class="p-1 hover:bg-slate-700 text-slate-400 hover:text-red-400 rounded transition-colors"
          title="清空日志"
          @click="clearLogs"
        >
          <Trash2 :size="14" />
        </button>
        <button
          class="p-1 hover:bg-slate-700 text-slate-400 hover:text-white rounded transition-colors"
          :title="isExpanded ? '还原' : '最大化'"
          @click="isExpanded = !isExpanded"
        >
          <component
            :is="isExpanded ? Minimize2 : Maximize2"
            :size="14"
          />
        </button>
        <div class="w-px h-4 bg-slate-600 mx-1" />
        <button
          class="p-1 hover:bg-red-500 hover:text-white text-slate-400 rounded transition-colors"
          title="关闭"
          @click="isVisible = false"
        >
          <X :size="16" />
        </button>
      </div>
    </div>

    <!-- 日志列表 -->
    <div
      ref="logContainer"
      class="flex-1 overflow-y-auto p-2 space-y-2 bg-slate-950 font-mono text-[11px] custom-scrollbar"
    >
      <div
        v-if="logs.length === 0"
        class="flex h-full items-center justify-center text-slate-600 italic select-none"
      >
        等待 API 请求触发...
      </div>
      
      <div
        v-for="(log, idx) in logs"
        :key="idx" 
        class="rounded p-2 border relative group"
        :class="log.type === 'error' ? 'bg-red-950/30 border-red-900/50 text-red-200' : 'bg-slate-900 border-slate-800 text-slate-300'"
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
            <span class="text-slate-500">{{ log.duration }}ms</span>
          </div>
          <span class="text-[10px] text-slate-500">{{ log.timestamp }}</span>
        </div>

        <div
          v-if="log.args && Object.keys(log.args).length > 0"
          class="mt-1.5 p-1.5 bg-slate-950 rounded border border-slate-800/50 overflow-x-auto whitespace-pre"
        >
          <span class="text-purple-400">Args:</span> {{ formatArgs(log.args) }}
        </div>

        <div
          v-if="log.type === 'error'"
          class="mt-1.5 p-1.5 bg-red-950/50 rounded border border-red-900/50 text-red-400 whitespace-pre-wrap font-bold"
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
