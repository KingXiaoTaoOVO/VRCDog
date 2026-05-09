<script setup lang="ts">
import { ref } from 'vue';
import { VrcApi, DbApi, SysApi, GamelogApi } from "../api";
import { Download, CheckCircle } from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';

const { t } = useI18n();

const exporting = ref(false);
const exportResult = ref('');

const handleExport = async () => {
  exporting.value = true;
  exportResult.value = '';
  try {
    const data: any = await DbApi.exportAll();
    const json = JSON.stringify(data, null, 2);
    const blob = new Blob([json], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `vrcdog_backup_${new Date().toISOString().slice(0, 10)}.json`;
    a.click();
    URL.revokeObjectURL(url);
    exportResult.value = t('export.success');
  } catch (err: any) {
    exportResult.value = t('export.fail') + `: ${err}`;
  } finally {
    exporting.value = false;
  }
};
</script>

<template>
  <div>
    <h2 class="text-2xl font-extrabold text-[#451a03] flex items-center gap-2 mb-6">
      <Download
        class="text-amber-500"
        :size="24"
      /> {{ t('export.title') }}
    </h2>

    <div class="bg-white/80 backdrop-blur rounded-2xl p-6 border-2 border-amber-100 space-y-6">
      <div class="text-center">
        <Download
          class="mx-auto text-amber-400 mb-4"
          :size="48"
        />
        <h3 class="text-lg font-bold text-amber-900 mb-2">
          {{ t('export.subtitle') }}
        </h3>
        <p class="text-sm text-amber-600 mb-6">
          {{ t('export.desc') }}
        </p>

        <button
          :disabled="exporting"
          class="bg-amber-500 hover:bg-amber-600 text-white font-bold px-8 py-3 rounded-2xl shadow-lg shadow-amber-500/30 transition-all flex items-center gap-2 mx-auto disabled:opacity-50"
          @click="handleExport"
        >
          <Download
            v-if="!exporting"
            :size="18"
          />
          <span
            v-else
            class="animate-spin"
          >⏳</span>
          {{ exporting ? t('export.packing') : t('export.button') }}
        </button>

        <p
          v-if="exportResult"
          class="mt-4 text-sm font-bold flex items-center justify-center gap-1"
          :class="exportResult.includes('成功') ? 'text-green-600' : 'text-red-500'"
        >
          <CheckCircle
            v-if="exportResult.includes('成功')"
            :size="16"
          />
          {{ exportResult }}
        </p>
      </div>
    </div>
  </div>
</template>
