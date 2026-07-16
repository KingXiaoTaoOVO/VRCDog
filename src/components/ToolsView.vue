<script setup lang="ts">
import { computed, ref, onMounted, onUnmounted } from 'vue';
import { SysApi, VrcApi } from "../api";
import { Wrench, Trash2, MessageSquare, Play, CheckCircle2, AlertCircle, Loader2, FolderOpen, Image, FileText, Bug, Database } from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';
import { useAuthStore } from '../stores/authStore';
import OscWorkbench from './OscWorkbench.vue';

const { t } = useI18n();
const authStore = useAuthStore();

const isVrcRunning = ref(false);
const cacheStatus = ref({ loading: false, message: '', type: '' });
const rpcForm = ref({ details: 'Using VrcDog', state: 'Chilling in VRChat' });
const rpcStatus = ref({ loading: false, message: '', type: '' });

const dirStatus = ref({ message: '', type: '' });
let vrcStatusTimer: ReturnType<typeof setInterval> | null = null;

type InviteMessageType = 'message' | 'request' | 'response' | 'requestResponse';
type InviteTemplateSlot = {
  slot: number;
  message: string;
  savedMessage: string;
  updatedAt?: string;
};

const inviteMessageTypes: { key: InviteMessageType; label: string; description: string }[] = [
  { key: 'message', label: '邀请', description: '邀请好友来当前实例时使用' },
  { key: 'request', label: '申请加入', description: '向好友请求邀请时使用' },
  { key: 'response', label: '回绝邀请', description: '回应别人邀请时使用' },
  { key: 'requestResponse', label: '回绝加入申请', description: '回应别人加入请求时使用' },
];
const activeInviteMessageType = ref<InviteMessageType>('message');
const inviteTemplateSlots = ref<InviteTemplateSlot[]>([]);
const inviteTemplateLoading = ref(false);
const inviteTemplateSaving = ref('');
const inviteTemplateStatus = ref({ message: '', type: '' });

const activeInviteMessageMeta = computed(() =>
  inviteMessageTypes.find(item => item.key === activeInviteMessageType.value) || inviteMessageTypes[0]
);

function normalizeInviteTemplateSlots(raw: any): InviteTemplateSlot[] {
  const source = raw?.json ?? raw;
  const rows = new globalThis.Map<number, InviteTemplateSlot>();
  const addRow = (value: any, fallbackSlot: number) => {
    const slot = Number(value?.slot ?? fallbackSlot);
    if (!Number.isFinite(slot)) return;
    const message = typeof value === 'string' ? value : String(value?.message ?? '');
    rows.set(slot, {
      slot,
      message,
      savedMessage: message,
      updatedAt: value?.updatedAt || value?.updated_at || value?.updated,
    });
  };

  if (Array.isArray(source)) {
    source.forEach((value, index) => addRow(value, index));
  } else if (source && typeof source === 'object') {
    Object.entries(source).forEach(([key, value]) => addRow(value, Number(key)));
  }

  for (let slot = 0; slot < 4; slot += 1) {
    if (!rows.has(slot)) rows.set(slot, { slot, message: '', savedMessage: '' });
  }

  return Array.from(rows.values()).sort((a, b) => a.slot - b.slot);
}

function inviteTemplateCooldownText(row: InviteTemplateSlot) {
  if (!row.updatedAt) return '可用';
  const updated = new Date(row.updatedAt).getTime();
  if (!Number.isFinite(updated)) return '可用';
  const remaining = updated + 60 * 60 * 1000 - Date.now();
  if (remaining <= 0) return '可用';
  const min = Math.ceil(remaining / 60000);
  return `${min} 分钟`;
}

async function loadInviteTemplates(type: InviteMessageType = activeInviteMessageType.value) {
  activeInviteMessageType.value = type;
  inviteTemplateLoading.value = true;
  inviteTemplateStatus.value = { message: '', type: '' };
  inviteTemplateSlots.value = normalizeInviteTemplateSlots(null);
  try {
    const userId = (authStore.currentUser as any)?.id;
    if (!userId) throw new Error('Missing current user id');
    const result = await VrcApi.getInviteMessages({ userId, messageType: type });
    inviteTemplateSlots.value = normalizeInviteTemplateSlots(result);
  } catch (err: any) {
    inviteTemplateStatus.value = { message: err?.message || String(err), type: 'error' };
  } finally {
    inviteTemplateLoading.value = false;
  }
}

async function saveInviteTemplateSlot(row: InviteTemplateSlot) {
  const userId = (authStore.currentUser as any)?.id;
  if (!userId) return;
  inviteTemplateSaving.value = `${activeInviteMessageType.value}:${row.slot}`;
  inviteTemplateStatus.value = { message: '', type: '' };
  try {
    await VrcApi.editInviteMessage({
      userId,
      messageType: activeInviteMessageType.value,
      slot: row.slot,
      message: row.message.trim(),
    });
    row.savedMessage = row.message.trim();
    row.updatedAt = new Date().toISOString();
    inviteTemplateStatus.value = { message: `Slot ${row.slot + 1} 已保存`, type: 'success' };
  } catch (err: any) {
    inviteTemplateStatus.value = { message: err?.message || String(err), type: 'error' };
  } finally {
    inviteTemplateSaving.value = '';
  }
}

const checkVrc = async () => {
  try {
    isVrcRunning.value = await SysApi.isVrcRunning();
  } catch (err) {
    console.warn(err);
  }
};

const launchVrc = async () => {
  try {
    await SysApi.launchVrc();
    setTimeout(checkVrc, 5000);
  } catch (err) {
    console.warn(err);
  }
};

const clearCache = async () => {
  cacheStatus.value = { loading: true, message: '', type: '' };
  try {
    const deletedBytes = await SysApi.clearVrcCache();
    const mb = (deletedBytes / 1024 / 1024).toFixed(2);
    cacheStatus.value = { loading: false, message: t('tools.cache_success', { size: mb }), type: 'success' };
  } catch (err: any) {
    cacheStatus.value = { loading: false, message: t('tools.cache_fail', { err: err.message || err }), type: 'error' };
  }
};

const setRpc = async () => {
  rpcStatus.value = { loading: true, message: '', type: '' };
  try {
    await SysApi.setDiscordRpc({ details: rpcForm.value.details, state: rpcForm.value.state });
    rpcStatus.value = { loading: false, message: t('tools.rpc_success'), type: 'success' };
  } catch (err: any) {
    rpcStatus.value = { loading: false, message: t('tools.rpc_fail', { err: err.message || err }), type: 'error' };
  }
};

const openDirectory = async (target: string) => {
  dirStatus.value = { message: '', type: '' };
  try {
    await SysApi.openDir({ target });
    dirStatus.value = { message: t('tools.dir_opened'), type: 'success' };
  } catch (err: any) {
    dirStatus.value = { message: t('tools.dir_open_fail', { err: err.message || err }), type: 'error' };
  }
  setTimeout(() => { dirStatus.value = { message: '', type: '' }; }, 3000);
};

onMounted(() => {
  checkVrc();
  loadInviteTemplates();
  vrcStatusTimer = setInterval(checkVrc, 10000);
});

onUnmounted(() => {
  if (vrcStatusTimer) clearInterval(vrcStatusTimer);
  vrcStatusTimer = null;
});
</script>

<template>
  <div class="h-full flex flex-col p-2 space-y-6 overflow-y-auto custom-scrollbar">
    <div class="flex items-center justify-between mb-2">
      <h2 class="text-2xl font-extrabold text-text flex items-center gap-2 tracking-tight">
        <Wrench
          class="text-primary"
          :size="24"
        /> {{ t('tools.title') }}
      </h2>
    </div>

    <!-- 模块化宫格布局 -->
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-5">
      <!-- 游戏启动与控制 -->
      <div class="bg-surface rounded-2xl p-5 border-border-soft shadow-sm relative overflow-hidden group hover:shadow-md transition-shadow flex flex-col">
        <div class="absolute -right-4 -top-4 w-20 h-20 bg-primary/20 rounded-full blur-2xl group-hover:bg-primary/30 transition-colors" />
        <h3 class="font-extrabold text-text mb-4 flex items-center gap-2 relative z-10 text-lg">
          <Play
            class="text-primary"
            :size="20"
          /> {{ t('tools.game_engine') }}
        </h3>
        <div class="space-y-4 relative z-10 flex flex-col flex-1 justify-between">
          <div class="flex items-center justify-between bg-surface-hover p-3 rounded-xl border-border-soft">
            <span class="text-sm font-bold text-text-muted">{{ t('tools.vrc_status') }}</span>
            <span
              v-if="isVrcRunning"
              class="px-2.5 py-1 bg-green-100 text-green-700 text-xs font-bold rounded-md flex items-center gap-1"
            >
              <CheckCircle2 :size="14" /> {{ t('tools.vrc_running') }}
            </span>
            <span
              v-else
              class="px-2.5 py-1 bg-background/20 text-text-muted text-xs font-bold rounded-md flex items-center gap-1"
            >
              <AlertCircle :size="14" /> {{ t('tools.vrc_stopped') }}
            </span>
          </div>
          <button
            class="w-full py-3 bg-primary hover:bg-primary/80 text-white font-bold rounded-xl flex items-center justify-center gap-2 transition-colors shadow-sm"
            @click="launchVrc"
          >
            <Play :size="18" /> {{ isVrcRunning ? t('tools.vrc_restart') : t('tools.vrc_start') }}
          </button>
        </div>
      </div>

      <!-- 快速直达 (文件夹快捷方式) -->
      <div class="bg-surface rounded-2xl p-5 border-border-soft shadow-sm relative overflow-hidden group hover:shadow-md transition-shadow flex flex-col">
        <div class="absolute -right-4 -top-4 w-20 h-20 bg-primary/20 rounded-full blur-2xl group-hover:bg-primary/30 transition-colors" />
        <div class="flex items-center justify-between mb-4 relative z-10">
          <h3 class="font-extrabold text-text flex items-center gap-2 text-lg">
            <FolderOpen
              class="text-primary"
              :size="20"
            /> {{ t('tools.quick_links') }}
          </h3>
          <span
            v-if="dirStatus.message"
            class="text-[10px] font-bold px-2 py-0.5 rounded-md"
            :class="dirStatus.type === 'success' ? 'bg-green-50 text-green-600' : 'bg-primary/20 text-primary'"
          >
            {{ dirStatus.message }}
          </span>
        </div>
        <div class="grid grid-cols-2 gap-3 relative z-10 flex-1">
          <button
            class="flex flex-col items-center justify-center p-3 rounded-xl border-border-soft bg-surface-hover hover:bg-primary/20 hover:border-primary/40 transition-colors gap-2 group/btn"
            @click="openDirectory('logs')"
          >
            <FileText
              class="text-border-strong group-hover/btn:text-primary transition-colors"
              :size="24"
            />
            <span class="text-xs font-bold text-text-muted group-hover/btn:text-primary-active">{{ t('tools.game_logs') }}</span>
          </button>
          <button
            class="flex flex-col items-center justify-center p-3 rounded-xl border-border-soft bg-surface-hover hover:bg-primary/20 hover:border-primary/40 transition-colors gap-2 group/btn"
            @click="openDirectory('screenshots')"
          >
            <Image
              class="text-border-strong group-hover/btn:text-primary transition-colors"
              :size="24"
            />
            <span class="text-xs font-bold text-text-muted group-hover/btn:text-primary-active">{{ t('tools.screenshots') }}</span>
          </button>
          <button
            class="flex flex-col items-center justify-center p-3 rounded-xl border-border-soft bg-surface-hover hover:bg-primary/20 hover:border-primary/40 transition-colors gap-2 group/btn shadow-sm"
            @click="openDirectory('cache')"
          >
            <Database
              class="text-border-strong group-hover/btn:text-primary transition-colors"
              :size="24"
            />
            <span class="text-xs font-bold text-text-muted group-hover/btn:text-primary-active">{{ t('tools.cache_dir') }}</span>
          </button>
          <button
            class="flex flex-col items-center justify-center p-3 rounded-xl border-border-soft bg-surface-hover hover:bg-primary/20 hover:border-primary/40 transition-colors gap-2 group/btn"
            @click="openDirectory('crash_reports')"
          >
            <Bug
              class="text-border-strong group-hover/btn:text-primary transition-colors"
              :size="24"
            />
            <span class="text-xs font-bold text-text-muted group-hover/btn:text-primary-active">{{ t('tools.crash_reports') }}</span>
          </button>
        </div>
      </div>

      <!-- 缓存清理 -->
      <div class="bg-surface rounded-2xl p-5 border-border-soft shadow-sm relative overflow-hidden group hover:shadow-md transition-shadow flex flex-col">
        <div class="absolute -right-4 -top-4 w-20 h-20 bg-primary/20 rounded-full blur-2xl group-hover:bg-primary/30 transition-colors" />
        <h3 class="font-extrabold text-text mb-4 flex items-center gap-2 relative z-10 text-lg">
          <Trash2
            class="text-primary"
            :size="20"
          /> {{ t('tools.cache_title') }}
        </h3>
        <div class="space-y-4 relative z-10 flex flex-col flex-1 justify-between">
          <p class="text-sm text-text-muted font-medium">
            {{ t('tools.cache_desc') }}
          </p>
          <div>
            <div
              v-if="cacheStatus.message"
              class="text-xs font-bold px-3 py-2 rounded-lg mb-2 text-center"
              :class="cacheStatus.type === 'success' ? 'bg-green-50 text-green-600' : 'bg-primary/20 text-primary'"
            >
              {{ cacheStatus.message }}
            </div>
            <button
              :disabled="cacheStatus.loading"
              class="w-full py-3 bg-primary hover:bg-primary/80 disabled:opacity-50 text-white font-bold rounded-xl flex items-center justify-center gap-2 transition-colors shadow-sm"
              @click="clearCache"
            >
              <Loader2
                v-if="cacheStatus.loading"
                class="animate-spin"
                :size="18"
              />
              <Trash2
                v-else
                :size="18"
              /> {{ t('tools.cache_exec') }}
            </button>
          </div>
        </div>
      </div>

      <!-- Discord RPC -->
      <div class="bg-surface rounded-2xl p-5 border-border-soft shadow-sm relative overflow-hidden group hover:shadow-md transition-shadow flex flex-col">
        <div class="absolute -right-4 -top-4 w-20 h-20 bg-primary/20 rounded-full blur-2xl group-hover:bg-primary/30 transition-colors" />
        <h3 class="font-extrabold text-text mb-4 flex items-center gap-2 relative z-10 text-lg">
          <MessageSquare
            class="text-primary"
            :size="20"
          /> {{ t('tools.rpc_title') }}
        </h3>
        <div class="space-y-3 relative z-10">
          <div>
            <label class="block text-xs font-bold text-text-muted mb-1">{{ t('tools.rpc_details') }}</label>
            <input
              v-model="rpcForm.details"
              type="text"
              class="w-full px-3 py-2 bg-surface-hover border-border-soft rounded-lg text-sm outline-none  focus:bg-surface transition-all"
            >
          </div>
          <div>
            <label class="block text-xs font-bold text-text-muted mb-1">{{ t('tools.rpc_state') }}</label>
            <input
              v-model="rpcForm.state"
              type="text"
              class="w-full px-3 py-2 bg-surface-hover border-border-soft rounded-lg text-sm outline-none  focus:bg-surface transition-all"
            >
          </div>
          <button
            :disabled="rpcStatus.loading"
            class="w-full py-2.5 bg-primary hover:bg-primary/80 disabled:opacity-50 text-white font-bold rounded-xl flex items-center justify-center gap-2 transition-colors mt-2"
            @click="setRpc"
          >
            <Loader2
              v-if="rpcStatus.loading"
              class="animate-spin"
              :size="16"
            />
            <MessageSquare
              v-else
              :size="16"
            /> {{ t('tools.rpc_update') }}
          </button>
          <div
            v-if="rpcStatus.message"
            class="text-xs font-bold px-3 py-2 rounded-lg text-center"
            :class="rpcStatus.type === 'success' ? 'bg-green-50 text-green-600' : 'bg-primary/20 text-primary'"
          >
            {{ rpcStatus.message }}
          </div>
        </div>
      </div>

      <OscWorkbench />

      <!-- Invite message templates -->
      <div class="bg-surface rounded-2xl p-5 border-border-soft shadow-sm relative overflow-hidden group hover:shadow-md transition-shadow md:col-span-2 lg:col-span-3">
        <div class="absolute -right-4 -top-4 w-20 h-20 bg-primary/20 rounded-full blur-2xl group-hover:bg-primary/30 transition-colors" />
        <div class="flex flex-wrap items-start justify-between gap-3 mb-4 relative z-10">
          <div>
            <h3 class="font-extrabold text-text flex items-center gap-2 text-lg">
              <MessageSquare class="text-primary" :size="20" /> 邀请消息模板
            </h3>
            <p class="text-xs text-text-muted font-medium mt-1">
              管理 VRChat 的 4 类邀请/回应消息，每类 4 个槽位。
            </p>
          </div>
          <button
            :disabled="inviteTemplateLoading"
            class="px-3 py-2 rounded-xl bg-surface-hover text-text-muted hover:text-primary border-border-soft text-xs font-bold flex items-center gap-2"
            @click="loadInviteTemplates(activeInviteMessageType)"
          >
            <Loader2 v-if="inviteTemplateLoading" class="animate-spin" :size="14" />
            <MessageSquare v-else :size="14" />
            刷新
          </button>
        </div>

        <div class="grid grid-cols-2 md:grid-cols-4 gap-2 mb-4 relative z-10">
          <button
            v-for="item in inviteMessageTypes"
            :key="item.key"
            class="px-3 py-2 rounded-xl text-left border-border-soft transition-all"
            :class="activeInviteMessageType === item.key ? 'bg-primary text-white' : 'bg-surface-hover text-text hover:text-primary'"
            @click="loadInviteTemplates(item.key)"
          >
            <div class="text-sm font-extrabold">{{ item.label }}</div>
            <div class="text-[10px] opacity-70 truncate">{{ item.description }}</div>
          </button>
        </div>

        <div class="relative z-10">
          <div class="flex items-center justify-between mb-3">
            <div>
              <div class="text-sm font-bold text-text">{{ activeInviteMessageMeta.label }}</div>
              <div class="text-xs text-text-muted">{{ activeInviteMessageMeta.description }}</div>
            </div>
            <span
              v-if="inviteTemplateStatus.message"
              class="text-[11px] font-bold px-3 py-1 rounded-lg"
              :class="inviteTemplateStatus.type === 'success' ? 'bg-green-50 text-green-600' : 'bg-red-50 text-red-600'"
            >
              {{ inviteTemplateStatus.message }}
            </span>
          </div>

          <div class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-4 gap-3">
            <div
              v-for="row in inviteTemplateSlots"
              :key="`${activeInviteMessageType}-${row.slot}`"
              class="bg-surface-hover rounded-xl border-border-soft p-3"
            >
              <div class="flex items-center justify-between mb-2">
                <span class="text-sm font-extrabold text-text">Slot {{ row.slot + 1 }}</span>
                <span class="text-[10px] font-bold text-text-muted">{{ inviteTemplateCooldownText(row) }}</span>
              </div>
              <textarea
                v-model="row.message"
                maxlength="64"
                rows="3"
                class="w-full p-2 rounded-lg bg-surface border-border-soft text-sm text-text outline-none resize-none"
                placeholder="输入消息内容"
              />
              <div class="flex items-center justify-between mt-2">
                <span class="text-[10px] text-text-muted">{{ row.message.length }}/64</span>
                <button
                  :disabled="inviteTemplateSaving === `${activeInviteMessageType}:${row.slot}` || row.message.trim() === row.savedMessage"
                  class="px-3 py-1.5 rounded-lg bg-primary text-white text-xs font-bold disabled:opacity-40 flex items-center gap-1.5"
                  @click="saveInviteTemplateSlot(row)"
                >
                  <Loader2
                    v-if="inviteTemplateSaving === `${activeInviteMessageType}:${row.slot}`"
                    class="animate-spin"
                    :size="12"
                  />
                  保存
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>


