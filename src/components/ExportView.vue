<script setup lang="ts">
import { ref } from 'vue';
import { SysApi } from "../api";
import { Download, Upload, CheckCircle, DatabaseBackup, Loader2, AlertTriangle } from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';
import { save, open } from '@tauri-apps/plugin-dialog';
import { invoke } from '@tauri-apps/api/core';

const { t } = useI18n();

const exporting = ref(false);
const exportResult = ref('');
const importing = ref(false);
const importResult = ref('');

const handleExport = async () => {
  exporting.value = true;
  exportResult.value = '';
  try {
    const filePath = await save({
      filters: [{ name: 'VrcDog Backup', extensions: ['db', 'sqlite'] }],
      defaultPath: `vrcdog_backup_${new Date().toISOString().slice(0, 10).replace(/-/g, '')}.db`,
    });
    
    if (filePath) {
      await SysApi.backupDatabase({ destPath: filePath });
      exportResult.value = t('export.success') || '备份成功 (Backup successful)';
    } else {
      exportResult.value = '已取消 (Cancelled)';
    }
  } catch (err: any) {
    exportResult.value = t('export.fail') + `: ${err}`;
  } finally {
    exporting.value = false;
  }
};

const handleImport = async () => {
  if (!confirm('警告：还原数据将覆盖您当前的所有本地记录（包括好友日志、设置、笔记等）。\n是否继续？\n\nWarning: Restoring data will overwrite all your current local records. Continue?')) {
    return;
  }
  
  importing.value = true;
  importResult.value = '';
  try {
    const filePath = await open({
      filters: [{ name: 'VrcDog Backup', extensions: ['db', 'sqlite'] }],
      multiple: false,
    });
    
    if (filePath && typeof filePath === 'string') {
      await SysApi.restoreDatabase({ srcPath: filePath });
      importResult.value = '还原成功，正在重启... (Restore successful, restarting...)';
      setTimeout(() => {
        invoke('process::restart');
      }, 2000);
    } else {
      importResult.value = '已取消 (Cancelled)';
    }
  } catch (err: any) {
    importResult.value = '还原失败 (Restore failed): ' + String(err);
  } finally {
    importing.value = false;
  }
};
</script>

<template>
  <div class="h-full flex flex-col p-6 bg-slate-50/50 rounded-3xl relative overflow-hidden">
    <!-- Subtle Background Glow -->
    <div class="absolute top-0 right-0 w-96 h-96 bg-indigo-500/10 rounded-full blur-[100px] pointer-events-none -z-10" />
    <div class="absolute bottom-0 left-0 w-[500px] h-[500px] bg-blue-500/5 rounded-full blur-[120px] pointer-events-none -z-10" />

    <h2 class="text-3xl font-extrabold text-slate-900 tracking-tight flex items-center gap-3 mb-8">
      <span class="inline-flex items-center justify-center p-2 bg-indigo-100 rounded-2xl shadow-sm border border-indigo-200/50">
        <DatabaseBackup class="w-6 h-6 text-indigo-600" />
      </span>
      {{ t('export.title') || '数据备份与还原' }}
    </h2>

    <div class="grid grid-cols-1 md:grid-cols-2 gap-6 max-w-5xl">
      <!-- Backup Section -->
      <div class="bg-white/80 backdrop-blur rounded-2xl p-6 border-2 border-slate-200 shadow-sm flex flex-col items-center justify-center text-center space-y-4">
        <div class="w-16 h-16 bg-indigo-50 rounded-full flex items-center justify-center mb-2">
          <Download
            class="text-indigo-500"
            :size="32"
          />
        </div>
        <h3 class="text-xl font-bold text-slate-900">
          创建完整备份
        </h3>
        <p class="text-sm text-slate-500 px-4">
          将您所有的本地数据（好友活动日志、游戏日志、笔记、设置、自定义收藏等）完整导出为一个独立的数据库文件。
        </p>

        <button
          :disabled="exporting"
          class="bg-slate-900 hover:bg-black text-white font-bold px-8 py-3 rounded-xl shadow-lg transition-all flex items-center gap-2 mt-4 disabled:opacity-50"
          @click="handleExport"
        >
          <Loader2
            v-if="exporting"
            class="animate-spin"
            :size="18"
          />
          <Download
            v-else
            :size="18"
          />
          {{ exporting ? t('export.packing') : '备份到文件 (Backup)' }}
        </button>

        <p
          v-if="exportResult"
          class="mt-2 text-sm font-bold flex items-center justify-center gap-1"
          :class="exportResult.includes('成功') ? 'text-green-600' : 'text-slate-600'"
        >
          <CheckCircle
            v-if="exportResult.includes('成功')"
            :size="16"
          />
          {{ exportResult }}
        </p>
      </div>

      <!-- Restore Section -->
      <div class="bg-white/80 backdrop-blur rounded-2xl p-6 border-2 border-red-100 shadow-sm flex flex-col items-center justify-center text-center space-y-4">
        <div class="w-16 h-16 bg-red-50 rounded-full flex items-center justify-center mb-2">
          <Upload
            class="text-red-500"
            :size="32"
          />
        </div>
        <h3 class="text-xl font-bold text-slate-900">
          还原备份数据
        </h3>
        <p class="text-sm text-slate-500 px-4">
          从以前保存的备份文件中恢复所有数据。<br><span class="text-red-500 font-bold">{{ t('global.auto_7fb6b2b9') }}</span>
        </p>

        <button
          :disabled="importing"
          class="bg-red-50 hover:bg-red-500 text-red-600 hover:text-white font-bold px-8 py-3 rounded-xl shadow-sm border border-red-200 hover:border-red-500 transition-all flex items-center gap-2 mt-4 disabled:opacity-50"
          @click="handleImport"
        >
          <Loader2
            v-if="importing"
            class="animate-spin"
            :size="18"
          />
          <AlertTriangle
            v-else
            :size="18"
          />
          {{ importing ? '正在还原...' : '从文件还原 (Restore)' }}
        </button>

        <p
          v-if="importResult"
          class="mt-2 text-sm font-bold flex items-center justify-center gap-1"
          :class="importResult.includes('成功') ? 'text-green-600' : 'text-slate-600'"
        >
          <CheckCircle
            v-if="importResult.includes('成功')"
            :size="16"
          />
          {{ importResult }}
        </p>
      </div>
    </div>
  </div>
</template>
