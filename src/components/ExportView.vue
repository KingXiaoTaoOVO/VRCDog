<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue';
import { SysApi } from "../api";
import { Download, Upload, CheckCircle, DatabaseBackup, Loader2, AlertTriangle, FolderOpen, ShieldCheck, Sparkles, FileWarning, Clock, Database, Copy } from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';
import { save, open } from '@tauri-apps/plugin-dialog';
import { invoke } from '@tauri-apps/api/core';

const { t } = useI18n();

const exporting = ref(false);
const exportResult = ref('');
const exportSuccess = ref(false);
const exporting2 = ref(false);
const exporting2Result = ref('');
const exporting2Success = ref(false);
const importing = ref(false);
const importResult = ref('');
const importSuccess = ref(false);

const now = ref(new Date());
let clockTimer: ReturnType<typeof setInterval> | null = null;

const suggestedFileName = computed(() => {
  const d = now.value;
  const ymd = `${d.getFullYear()}${String(d.getMonth() + 1).padStart(2, '0')}${String(d.getDate()).padStart(2, '0')}`;
  const hm = `${String(d.getHours()).padStart(2, '0')}${String(d.getMinutes()).padStart(2, '0')}`;
  return `vrcdog_backup_${ymd}_${hm}.db`;
});

const formattedNow = computed(() => {
  const d = now.value;
  const yyyy = d.getFullYear();
  const mm = String(d.getMonth() + 1).padStart(2, '0');
  const dd = String(d.getDate()).padStart(2, '0');
  const hh = String(d.getHours()).padStart(2, '0');
  const mi = String(d.getMinutes()).padStart(2, '0');
  const ss = String(d.getSeconds()).padStart(2, '0');
  return `${yyyy}-${mm}-${dd} ${hh}:${mi}:${ss}`;
});

const handleExport = async () => {
  exporting.value = true;
  exportResult.value = '';
  exportSuccess.value = false;
  try {
    const filePath = await save({
      filters: [{ name: 'VrcDog Backup', extensions: ['db', 'sqlite'] }],
      defaultPath: suggestedFileName.value,
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

const handleExportTo = async () => {
  // 二次确认模式：导出到应用数据目录，文件名带时间戳
  exporting2.value = true;
  exporting2Result.value = '';
  exporting2Success.value = false;
  try {
    const fileName = suggestedFileName.value;
    // 简单调用：让用户选目录
    const dirPath = await open({
      directory: true,
      multiple: false,
      title: t('export.pick_dir') || '选择备份目录',
    });
    if (!dirPath) {
      exporting2Result.value = t('auto_20fdb3a3');
      return;
    }
    const fullPath = `${dirPath}/${fileName}`;
    await SysApi.backupDatabase({ destPath: fullPath });
    exporting2Result.value = (t('export.success') || t('auto_d51cba82')) + ` → ${fullPath}`;
    exporting2Success.value = true;
  } catch (err: any) {
    exporting2Result.value = t('export.fail') + `: ${err}`;
  } finally {
    exporting2.value = false;
  }
};

const copySuggestedName = async () => {
  // 浏览器剪贴板即可，Tauri 环境下同样可用
  try {
    await navigator.clipboard.writeText(suggestedFileName.value);
  } catch (err) {
    // 兜底：忽略
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

const openBackupFolder = async (_folderPath?: string) => {
  // 暂未启用：保留以备后续添加「在文件管理器中打开」按钮
  // 使用 opener 插件 openPath(path) 即可
};

onMounted(() => {
  clockTimer = setInterval(() => { now.value = new Date(); }, 1000);
});

onUnmounted(() => {
  if (clockTimer) clearInterval(clockTimer);
  clockTimer = null;
});
</script>

<template>
  <div class="h-full flex flex-col p-2 space-y-4 overflow-y-auto custom-scrollbar">
    <!-- 标题区 -->
    <div class="flex flex-wrap items-end justify-between gap-3 mb-1">
      <h2 class="text-2xl font-extrabold text-text flex items-center gap-2 tracking-tight">
        <span class="inline-flex items-center justify-center p-1.5 bg-primary/10 rounded-xl border border-primary/20">
          <DatabaseBackup class="w-5 h-5 text-primary" />
        </span>
        {{ t('export.title') }}
      </h2>
      <div class="flex items-center gap-1.5 text-xs text-text-muted font-mono bg-surface px-3 py-1.5 rounded-lg border border-border-soft">
        <Clock :size="14" class="text-primary" />
        {{ formattedNow }}
      </div>
    </div>

    <!-- 主操作区：紧凑水平卡片 -->
    <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
      <!-- 创建完整备份 -->
      <div class="bg-surface rounded-2xl border border-border-soft shadow-sm relative overflow-hidden group hover:shadow-md transition-shadow">
        <div class="absolute -right-8 -top-8 w-32 h-32 bg-primary/10 rounded-full blur-2xl pointer-events-none" />
        <div class="relative z-10 p-4 flex items-center gap-4">
          <div class="shrink-0 w-12 h-12 bg-primary/10 rounded-xl flex items-center justify-center">
            <Download class="text-primary" :size="22" />
          </div>
          <div class="flex-1 min-w-0">
            <h3 class="font-extrabold text-text text-base flex items-center gap-2">
              {{ t('export.create_backup') || '创建完整备份' }}
              <span class="text-[10px] font-bold px-1.5 py-0.5 rounded-md bg-primary/10 text-primary">
                {{ t('export.recommended') || '推荐' }}
              </span>
            </h3>
            <p class="text-xs text-text-muted mt-0.5 leading-snug">
              {{ t('export.create_backup_desc') || '将所有本地数据库完整导出为一个独立备份文件。' }}
            </p>
          </div>
          <button
            :disabled="exporting"
            class="shrink-0 bg-primary hover:bg-primary-hover text-white font-bold px-4 py-2 rounded-lg shadow-sm transition-all flex items-center gap-1.5 disabled:opacity-50 text-sm"
            @click="handleExport"
          >
            <Loader2 v-if="exporting" class="animate-spin" :size="16" />
            <Download v-else :size="16" />
            {{ exporting ? (t('export.packing') || '打包中…') : (t('export.backup_to_file') || '备份到文件') }}
          </button>
        </div>
        <div
          v-if="exportResult"
          class="px-4 pb-3 -mt-1 text-xs font-bold flex items-center gap-1.5"
          :class="exportSuccess ? 'text-emerald-600' : 'text-text-muted'"
        >
          <CheckCircle v-if="exportSuccess" :size="14" />
          <AlertTriangle v-else :size="14" />
          <span class="truncate">{{ exportResult }}</span>
        </div>
      </div>

      <!-- 还原备份 -->
      <div class="bg-surface rounded-2xl border-2 border-red-500/30 shadow-sm relative overflow-hidden group hover:shadow-md transition-shadow">
        <div class="absolute -right-8 -top-8 w-32 h-32 bg-red-500/10 rounded-full blur-2xl pointer-events-none" />
        <div class="relative z-10 p-4 flex items-center gap-4">
          <div class="shrink-0 w-12 h-12 bg-red-500/10 rounded-xl flex items-center justify-center">
            <Upload class="text-red-500" :size="22" />
          </div>
          <div class="flex-1 min-w-0">
            <h3 class="font-extrabold text-text text-base flex items-center gap-2">
              {{ t('export.restore_backup') || '还原备份' }}
              <span class="text-[10px] font-bold px-1.5 py-0.5 rounded-md bg-red-500/10 text-red-500">
                {{ t('export.danger') || '危险' }}
              </span>
            </h3>
            <p class="text-xs text-text-muted mt-0.5 leading-snug">
              <span class="text-red-500 font-bold">{{ t('global.auto_7fb6b2b9') }}</span>
            </p>
          </div>
          <button
            :disabled="importing"
            class="shrink-0 bg-red-500/10 hover:bg-red-500 text-red-500 hover:text-white font-bold px-4 py-2 rounded-lg border border-red-500/30 hover:border-red-500 transition-all flex items-center gap-1.5 disabled:opacity-50 text-sm"
            @click="handleImport"
          >
            <Loader2 v-if="importing" class="animate-spin" :size="16" />
            <AlertTriangle v-else :size="16" />
            {{ importing ? (t('app.restoring') || '还原中…') : (t('app.restore_from_file') || '从文件还原') }}
          </button>
        </div>
        <div
          v-if="importResult"
          class="px-4 pb-3 -mt-1 text-xs font-bold flex items-center gap-1.5"
          :class="importSuccess ? 'text-emerald-600' : 'text-text-muted'"
        >
          <CheckCircle v-if="importSuccess" :size="14" />
          <AlertTriangle v-else :size="14" />
          <span class="truncate">{{ importResult }}</span>
        </div>
      </div>
    </div>

    <!-- 信息 / 辅助区：3 列 -->
    <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
      <!-- 快速导出：建议文件名 -->
      <div class="bg-surface rounded-2xl border border-border-soft p-4 shadow-sm flex flex-col">
        <h4 class="font-extrabold text-text text-sm flex items-center gap-2 mb-2">
          <Sparkles class="text-primary" :size="16" />
          {{ t('export.suggested_name') || '建议文件名' }}
        </h4>
        <div class="bg-surface-hover rounded-lg px-3 py-2 font-mono text-xs text-text flex items-center gap-2 border border-border-soft">
          <span class="truncate flex-1">{{ suggestedFileName }}</span>
          <button
            class="shrink-0 text-text-muted hover:text-primary transition-colors p-1 rounded"
            :title="t('export.copy') || '复制'"
            @click="copySuggestedName"
          >
            <Copy :size="13" />
          </button>
        </div>
        <button
          :disabled="exporting2"
          class="mt-2 w-full bg-surface-hover hover:bg-primary/20 text-text-muted hover:text-primary text-xs font-bold py-2 rounded-lg border border-border-soft transition-colors flex items-center justify-center gap-1.5 disabled:opacity-50"
          @click="handleExportTo"
        >
          <Loader2 v-if="exporting2" class="animate-spin" :size="13" />
          <FolderOpen v-else :size="13" />
          {{ t('export.export_to_folder') || '导出到指定目录' }}
        </button>
        <p
          v-if="exporting2Result"
          class="mt-2 text-[10px] font-bold flex items-center gap-1 truncate"
          :class="exporting2Success ? 'text-emerald-600' : 'text-text-muted'"
        >
          {{ exporting2Result }}
        </p>
      </div>

      <!-- 备份最佳实践 -->
      <div class="bg-surface rounded-2xl border border-border-soft p-4 shadow-sm">
        <h4 class="font-extrabold text-text text-sm flex items-center gap-2 mb-2">
          <ShieldCheck class="text-primary" :size="16" />
          {{ t('export.best_practice') || '备份最佳实践' }}
        </h4>
        <ul class="space-y-1.5 text-xs text-text-muted leading-relaxed">
          <li class="flex gap-1.5">
            <span class="text-primary mt-0.5">·</span>
            <span>{{ t('export.tip_periodic') || '建议每次大规模设置改动前各备份一次。' }}</span>
          </li>
          <li class="flex gap-1.5">
            <span class="text-primary mt-0.5">·</span>
            <span>{{ t('export.tip_cloud') || '将备份文件复制到网盘/外部硬盘，避免本机硬盘损坏丢失。' }}</span>
          </li>
          <li class="flex gap-1.5">
            <span class="text-primary mt-0.5">·</span>
            <span>{{ t('export.tip_versioning') || '保留多个历史版本（按日期命名），便于回滚到任意时点。' }}</span>
          </li>
          <li class="flex gap-1.5">
            <span class="text-primary mt-0.5">·</span>
            <span>{{ t('export.tip_verify') || '升级 VRCDog 前务必先备份。' }}</span>
          </li>
        </ul>
      </div>

      <!-- 还原前检查清单 -->
      <div class="bg-surface rounded-2xl border border-red-500/20 p-4 shadow-sm relative overflow-hidden">
        <div class="absolute -right-4 -top-4 w-20 h-20 bg-red-500/10 rounded-full blur-2xl pointer-events-none" />
        <h4 class="font-extrabold text-text text-sm flex items-center gap-2 mb-2 relative z-10">
          <FileWarning class="text-red-500" :size="16" />
          {{ t('export.restore_checklist') || '还原前检查清单' }}
        </h4>
        <ol class="space-y-1.5 text-xs text-text-muted leading-relaxed relative z-10">
          <li class="flex gap-1.5">
            <span class="text-red-500 font-extrabold shrink-0 w-4">1.</span>
            <span>{{ t('export.check_1') || '确认备份文件来源可信，未被第三方修改。' }}</span>
          </li>
          <li class="flex gap-1.5">
            <span class="text-red-500 font-extrabold shrink-0 w-4">2.</span>
            <span>{{ t('export.check_2') || '关闭 VRChat、退出当前账号会话，避免文件锁冲突。' }}</span>
          </li>
          <li class="flex gap-1.5">
            <span class="text-red-500 font-extrabold shrink-0 w-4">3.</span>
            <span>{{ t('export.check_3') || '先导出一份当前数据作为「回滚保险」。' }}</span>
          </li>
          <li class="flex gap-1.5">
            <span class="text-red-500 font-extrabold shrink-0 w-4">4.</span>
            <span>{{ t('export.check_4') || '还原完成后会自动重启应用，请提前保存工作。' }}</span>
          </li>
        </ol>
      </div>
    </div>

    <!-- 数据库信息：直接读前端已知的应用数据目录 -->
    <div class="bg-surface rounded-2xl border border-border-soft p-4 shadow-sm">
      <h4 class="font-extrabold text-text text-sm flex items-center gap-2 mb-2">
        <Database class="text-primary" :size="16" />
        {{ t('export.db_info') || '数据库信息' }}
      </h4>
      <div class="grid grid-cols-1 md:grid-cols-3 gap-3 text-xs">
        <div class="bg-surface-hover rounded-lg p-3 border border-border-soft">
          <div class="text-text-muted mb-1">{{ t('export.db_name') || '数据文件' }}</div>
          <div class="font-mono text-text font-bold">vrcdog.db</div>
          <div class="text-[10px] text-text-muted mt-1">{{ t('export.db_name_hint') || '应用首次启动时自动创建，存放于 AppData 目录。' }}</div>
        </div>
        <div class="bg-surface-hover rounded-lg p-3 border border-border-soft">
          <div class="text-text-muted mb-1">{{ t('export.db_location') || '存储位置' }}</div>
          <div class="font-mono text-text text-[11px] leading-snug break-all">
            %APPDATA%\VRCDog\vrcdog.db
          </div>
          <div class="text-[10px] text-text-muted mt-1">Windows · {{ t('export.db_loc_hint') || '如需手动备份可直接复制该文件。' }}</div>
        </div>
        <div class="bg-surface-hover rounded-lg p-3 border border-border-soft">
          <div class="text-text-muted mb-1">{{ t('export.db_contains') || '包含内容' }}</div>
          <div class="flex flex-wrap gap-1 mt-1">
            <span class="px-1.5 py-0.5 rounded-md bg-primary/10 text-primary font-bold text-[10px]">{{ t('export.contains_friends') || '好友' }}</span>
            <span class="px-1.5 py-0.5 rounded-md bg-primary/10 text-primary font-bold text-[10px]">{{ t('export.contains_logs') || '游戏日志' }}</span>
            <span class="px-1.5 py-0.5 rounded-md bg-primary/10 text-primary font-bold text-[10px]">{{ t('export.contains_settings') || '应用设置' }}</span>
            <span class="px-1.5 py-0.5 rounded-md bg-primary/10 text-primary font-bold text-[10px]">{{ t('export.contains_notes') || '笔记' }}</span>
            <span class="px-1.5 py-0.5 rounded-md bg-primary/10 text-primary font-bold text-[10px]">{{ t('export.contains_avatars') || 'Avatar 收藏' }}</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
