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
const exportSuccess = ref(false);
const importing = ref(false);
const importResult = ref('');
const importSuccess = ref(false);

const handleExport = async () => {
  exporting.value = true;
  exportResult.value = '';
  exportSuccess.value = false;
  try {
    const filePath = await save({
      filters: [{ name: 'VrcDog Backup', extensions: ['db', 'sqlite'] }],
      defaultPath: `vrcdog_backup_${new Date().toISOString().slice(0, 10).replace(/-/g, '')}.db`,
    });
    
    if (filePath) {
      await SysApi.backupDatabase({ destPath: filePath });
      exportResult.value = t('export.success') || t('auto_d51cba82');
      exportSuccess.value = true;
    } else {
      exportResult.value = t('auto_20fdb3a3');
    }
  } catch (err: any) {
    exportResult.value = t('export.fail') + `: ${err}`;
  } finally {
    exporting.value = false;
  }
};

const handleImport = async () => {
  if (!confirm(t('auto_e6be0a1c'))) {
    return;
  }
  
  importing.value = true;
  importResult.value = '';
  importSuccess.value = false;
  try {
    const filePath = await open({
      filters: [{ name: 'VrcDog Backup', extensions: ['db', 'sqlite'] }],
      multiple: false,
    });
    
    if (filePath && typeof filePath === 'string') {
      await SysApi.restoreDatabase({ srcPath: filePath });
      importResult.value = t('auto_1a05ef0f');
      importSuccess.value = true;
      setTimeout(() => {
        invoke('process::restart');
      }, 2000);
    } else {
      importResult.value = t('auto_20fdb3a3');
    }
  } catch (err: any) {
    importResult.value = t('auto_0919c89c') + String(err);
  } finally {
    importing.value = false;
  }
};
</script>

<template>
  <div class="h-full flex flex-col p-6 bg-surface-hover rounded-3xl relative overflow-hidden">
    <!-- Subtle Background Glow -->
    <div class="absolute top-0 right-0 w-96 h-96 bg-primary/10 rounded-full blur-[100px] pointer-events-none -z-10" />
    <div class="absolute bottom-0 left-0 w-[500px] h-[500px] bg-blue-500/5 rounded-full blur-[120px] pointer-events-none -z-10" />

    <h2 class="text-3xl font-extrabold text-text tracking-tight flex items-center gap-3 mb-8">
      <span class="inline-flex items-center justify-center p-2 bg-primary/10 rounded-2xl shadow-sm border-primary/20">
        <DatabaseBackup class="w-6 h-6 text-primary" />
      </span>
      {{ t('export.title') }}
    </h2>

    <div class="grid grid-cols-1 md:grid-cols-2 gap-6 max-w-5xl">
      <!-- Backup Section -->
      <div class="bg-surface backdrop-blur rounded-2xl p-6 border-2 border-border-soft shadow-sm flex flex-col items-center justify-center text-center space-y-4">
        <div class="w-16 h-16 bg-primary/10 rounded-full flex items-center justify-center mb-2">
          <Download
            class="text-primary"
            :size="32"
          />
        </div>
        <h3 class="text-xl font-bold text-text">
          {{ t('export.create_backup') || 'Create Backup' }}
        </h3>
        <p class="text-sm text-text-muted px-4">
          {{ t('export.create_backup_desc') || 'Export all data' }}
        </p>

        <button
          :disabled="exporting"
          class="bg-primary hover:bg-primary-hover backdrop-blur-md text-white font-bold px-8 py-3 rounded-xl shadow-lg transition-all flex items-center gap-2 mt-4 disabled:opacity-50"
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
          {{ exporting ? t('export.packing') : t('export.backup_to_file') || 'Backup to file' }}
        </button>

        <p
          v-if="exportResult"
          class="mt-2 text-sm font-bold flex items-center justify-center gap-1"
          :class="exportSuccess ? 'text-green-600' : 'text-text-muted'"
        >
          <CheckCircle
            v-if="exportSuccess"
            :size="16"
          />
          {{ exportResult }}
        </p>
      </div>

      <!-- Restore Section -->
      <div class="bg-surface backdrop-blur rounded-2xl p-6 border-2 border-red-500/20 shadow-sm flex flex-col items-center justify-center text-center space-y-4">
        <div class="w-16 h-16 bg-red-500/10 rounded-full flex items-center justify-center mb-2">
          <Upload
            class="text-red-500"
            :size="32"
          />
        </div>
        <h3 class="text-xl font-bold text-text">
          {{ t('export.restore_backup') || 'Restore Backup' }}
        </h3>
        <p class="text-sm text-text-muted px-4">
          {{ $t('export.restore_desc_line1') || 'Restore data from a saved backup file.' }}<br><span class="text-red-500 font-bold">{{ t('global.auto_7fb6b2b9') }}</span>
        </p>

        <button
          :disabled="importing"
          class="bg-red-500/10 hover:bg-red-500 text-red-500 hover:text-white font-bold px-8 py-3 rounded-xl shadow-sm border-red-500/30 hover:border-red-500 transition-all flex items-center gap-2 mt-4 disabled:opacity-50"
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
          {{ importing ? t('app.restoring') : t('app.restore_from_file') }}
        </button>

        <p
          v-if="importResult"
          class="mt-2 text-sm font-bold flex items-center justify-center gap-1"
          :class="importSuccess ? 'text-green-600' : 'text-text-muted'"
        >
          <CheckCircle
            v-if="importSuccess"
            :size="16"
          />
          {{ importResult }}
        </p>
      </div>
    </div>
  </div>
</template>
