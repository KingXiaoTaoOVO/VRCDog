<script setup lang="ts">
import { computed, ref, onMounted, onUnmounted } from 'vue';
import { isTauri } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { SysApi, VrcApi } from "../api";
import { Wrench, Trash2, MessageSquare, Play, CheckCircle2, AlertCircle, Loader2, FolderOpen, Image, FileText, Bug, Database, Send, ExternalLink, MapPin, Clock3, RotateCcw, Bell, BellOff, Sparkles, XCircle } from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';
import { useAuthStore } from '../stores/authStore';
import OscWorkbench from './OscWorkbench.vue';

const { t } = useI18n();
const { locale } = useI18n();
const l = (zh: string, en: string) => locale.value.startsWith('zh') ? zh : en;
const authStore = useAuthStore();

const isVrcRunning = ref(false);
const cacheStatus = ref({ loading: false, message: '', type: '' });
const rpcForm = ref({ details: 'Using VrcDog', state: 'Chilling in VRChat' });
const rpcStatus = ref({ loading: false, message: '', type: '' });
const chatboxText = ref('');
const chatboxStatus = ref({ loading: false, message: '', type: '' });
const chatboxSendDelay = ref(0);
const chatboxKeepalive = ref(false);
const chatboxNotification = ref(false);
const chatboxEmojiShuffle = ref(false);
const chatboxAutocomplete = ref(false);
const chatboxAutocompleteIndex = ref(-1);
const chatHistory = ref<{ text: string; sentAt: string }[]>([]);
const statusMessages = ref<string[]>(['Using VrcDog', 'Chilling in VRChat', 'AFK']);
const statusCycleEnabled = ref(false);
const statusCycleInterval = ref(30);
let statusCycleTimer: ReturnType<typeof setInterval> | null = null;
const instanceTarget = ref('');
const instanceStatus = ref({ loading: false, message: '', type: '' });

const dirStatus = ref({ message: '', type: '' });
let vrcStatusTimer: ReturnType<typeof setInterval> | null = null;
let launchCheckTimeout: ReturnType<typeof setTimeout> | null = null;
let dirStatusTimeout: ReturnType<typeof setTimeout> | null = null;
let chatboxTypingTimer: ReturnType<typeof setTimeout> | null = null;
let unlistenKeepalive: UnlistenFn | null = null;

type InviteMessageType = 'message' | 'request' | 'response' | 'requestResponse';
type InviteTemplateSlot = {
  slot: number;
  message: string;
  savedMessage: string;
  updatedAt?: string;
};

const inviteMessageTypes = computed<{ key: InviteMessageType; label: string; description: string }[]>(() => [
  { key: 'message', label: t('tools.invite_type_message'), description: t('tools.invite_type_message_desc') },
  { key: 'request', label: t('tools.invite_type_request'), description: t('tools.invite_type_request_desc') },
  { key: 'response', label: t('tools.invite_type_response'), description: t('tools.invite_type_response_desc') },
  { key: 'requestResponse', label: t('tools.invite_type_request_response'), description: t('tools.invite_type_request_response_desc') },
]);
const activeInviteMessageType = ref<InviteMessageType>('message');
const inviteTemplateSlots = ref<InviteTemplateSlot[]>([]);
const inviteTemplateLoading = ref(false);
const inviteTemplateSaving = ref('');
const inviteTemplateStatus = ref({ message: '', type: '' });

const activeInviteMessageMeta = computed(() =>
  inviteMessageTypes.value.find(item => item.key === activeInviteMessageType.value) || inviteMessageTypes.value[0]
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
  if (!row.updatedAt) return t('tools.invite_available');
  const updated = new Date(row.updatedAt).getTime();
  if (!Number.isFinite(updated)) return t('tools.invite_available');
  const remaining = updated + 60 * 60 * 1000 - Date.now();
  if (remaining <= 0) return t('tools.invite_available');
  const min = Math.ceil(remaining / 60000);
  return t('tools.invite_minutes', { count: min });
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
    inviteTemplateStatus.value = { message: t('tools.invite_saved', { slot: row.slot + 1 }), type: 'success' };
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
    if (launchCheckTimeout) clearTimeout(launchCheckTimeout);
    launchCheckTimeout = setTimeout(checkVrc, 5000);
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

const sendChatboxMessage = async () => {
  const text = chatboxText.value.trim();
  if (!text) return;
  chatboxStatus.value = { loading: true, message: '', type: '' };
  if (chatboxTypingTimer) { clearTimeout(chatboxTypingTimer); chatboxTypingTimer = null; }
  await SysApi.sendOscTyping({ typing: false }).catch(() => {});
  try {
    const prefix = chatboxEmojiShuffle.value ? `${getRandomEmoji()} ` : '';
    const fullText = prefix + text;
    await SysApi.sendOscChatbox({ text: fullText, complete: true, delaySecs: chatboxSendDelay.value > 0 ? chatboxSendDelay.value : undefined, notification: chatboxNotification.value || undefined });
    chatHistory.value.unshift({ text: fullText, sentAt: new Date().toISOString() });
    if (chatHistory.value.length > 50) chatHistory.value = chatHistory.value.slice(0, 50);
    chatboxText.value = '';
    chatboxAutocompleteIndex.value = -1;
    chatboxStatus.value = { loading: false, message: t('tools.chatbox_success'), type: 'success' };
  } catch (err: any) {
    chatboxStatus.value = { loading: false, message: t('tools.chatbox_fail', { err: err?.message || err }), type: 'error' };
  }
};

const autocompleteSuggestions = computed(() => {
  if (!chatboxAutocomplete.value || !chatboxText.value.trim()) return [];
  const query = chatboxText.value.trim().toLowerCase();
  const unique = Array.from(new Set(chatHistory.value.map(h => h.text)));
  return unique.filter(t => t.toLowerCase().startsWith(query) && t.toLowerCase() !== query).slice(0, 5);
});

const applyAutocomplete = (text: string) => {
  chatboxText.value = text;
  chatboxAutocompleteIndex.value = -1;
};

const onChatboxInput = () => {
  if (chatboxTypingTimer) clearTimeout(chatboxTypingTimer);
  if (!chatboxText.value.trim()) {
    SysApi.sendOscTyping({ typing: false }).catch(() => {});
    chatboxAutocompleteIndex.value = -1;
    return;
  }
  SysApi.sendOscTyping({ typing: true }).catch(() => {});
  chatboxTypingTimer = setTimeout(() => {
    SysApi.sendOscTyping({ typing: false }).catch(() => {});
    chatboxTypingTimer = null;
  }, 3000);
};

const onChatboxKeydown = (e: KeyboardEvent) => {
  if (!chatboxAutocomplete.value || !autocompleteSuggestions.value.length) return;
  if (e.key === 'Tab') {
    e.preventDefault();
    const suggestions = autocompleteSuggestions.value;
    if (chatboxAutocompleteIndex.value < 0) {
      chatboxAutocompleteIndex.value = 0;
    } else {
      chatboxAutocompleteIndex.value = (chatboxAutocompleteIndex.value + 1) % suggestions.length;
    }
    applyAutocomplete(suggestions[chatboxAutocompleteIndex.value]);
  } else if (e.key === 'Escape') {
    chatboxAutocompleteIndex.value = -1;
  }
};

const resendChatbox = async (text: string) => {
  chatboxStatus.value = { loading: true, message: '', type: '' };
  try {
    await SysApi.sendOscChatbox({ text, complete: true, delaySecs: chatboxSendDelay.value > 0 ? chatboxSendDelay.value : undefined, notification: chatboxNotification.value || undefined });
    chatboxStatus.value = { loading: false, message: t('tools.chatbox_success'), type: 'success' };
  } catch (err: any) {
    chatboxStatus.value = { loading: false, message: t('tools.chatbox_fail', { err: err?.message || err }), type: 'error' };
  }
};

const clearChatHistory = () => {
  chatHistory.value = [];
};

const toggleKeepalive = async () => {
  chatboxKeepalive.value = !chatboxKeepalive.value;
  if (chatboxKeepalive.value) {
    await SysApi.startChatboxKeepalive();
  } else {
    await SysApi.stopChatboxKeepalive();
  }
};

const clearChatbox = async () => {
  chatboxStatus.value = { loading: true, message: '', type: '' };
  try {
    await SysApi.sendOscChatbox({ text: '', complete: true, notification: chatboxNotification.value || undefined });
    chatboxStatus.value = { loading: false, message: t('tools.chatbox_clear_success'), type: 'success' };
  } catch (err: any) {
    chatboxStatus.value = { loading: false, message: t('tools.chatbox_fail', { err: err?.message || err }), type: 'error' };
  }
};

const emojis = ['👋', '✨', '🎮', '🌟', '🔥', '💫', '🚀', '⭐', '💖', '🎉', '🌈', '🎵', '🤖', '👾', '🎲', '🧩'];
const getRandomEmoji = () => emojis[Math.floor(Math.random() * emojis.length)];

const addStatusMessage = () => {
  const text = prompt(t('tools.status_add_prompt'));
  if (text && text.trim()) {
    statusMessages.value.push(text.trim());
  }
};

const removeStatusMessage = (index: number) => {
  statusMessages.value.splice(index, 1);
};

const toggleStatusCycle = () => {
  statusCycleEnabled.value = !statusCycleEnabled.value;
  if (statusCycleEnabled.value) {
    cycleStatus();
    statusCycleTimer = setInterval(cycleStatus, statusCycleInterval.value * 1000);
  } else {
    if (statusCycleTimer) clearInterval(statusCycleTimer);
    statusCycleTimer = null;
  }
};

const cycleStatus = async () => {
  if (!statusMessages.value.length) return;
  const current = rpcForm.value.state;
  let idx = statusMessages.value.indexOf(current);
  if (idx < 0) idx = 0; else idx = (idx + 1) % statusMessages.value.length;
  rpcForm.value.state = statusMessages.value[idx];
  await setRpc();
};

const launchInstance = async () => {
  const value = instanceTarget.value.trim();
  if (!value) return;
  instanceStatus.value = { loading: true, message: '', type: '' };
  try {
    let url = value;
    if (!/^https?:\/\//i.test(value)) {
      const [worldId, ...instanceParts] = value.split(':');
      if (!worldId.startsWith('wrld_')) throw new Error(t('tools.instance_invalid'));
      const query = new URLSearchParams({ worldId });
      const instanceId = instanceParts.join(':').trim();
      if (instanceId) query.set('instanceId', instanceId);
      url = `https://vrchat.com/home/launch?${query.toString()}`;
    }
    await SysApi.openUrl({ url });
    instanceStatus.value = { loading: false, message: t('tools.instance_opened'), type: 'success' };
  } catch (err: any) {
    instanceStatus.value = { loading: false, message: t('tools.instance_fail', { err: err?.message || err }), type: 'error' };
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
  if (dirStatusTimeout) clearTimeout(dirStatusTimeout);
  dirStatusTimeout = setTimeout(() => { dirStatus.value = { message: '', type: '' }; }, 3000);
};

onMounted(async () => {
  checkVrc();
  loadInviteTemplates();
  vrcStatusTimer = setInterval(checkVrc, 10000);
  if (isTauri()) {
    unlistenKeepalive = await listen<{ tick: boolean }>('chatbox-keepalive-tick', () => {
      if (!chatboxKeepalive.value) return;
      SysApi.sendOscTyping({ typing: true }).catch(() => {});
    });
  }
});

onUnmounted(() => {
  if (vrcStatusTimer) clearInterval(vrcStatusTimer);
  if (chatboxTypingTimer) clearTimeout(chatboxTypingTimer);
  if (unlistenKeepalive) unlistenKeepalive();
  if (chatboxKeepalive.value) {
    SysApi.stopChatboxKeepalive().catch(() => {});
  }
  if (statusCycleTimer) clearInterval(statusCycleTimer);
  vrcStatusTimer = null;
  if (launchCheckTimeout) clearTimeout(launchCheckTimeout);
  launchCheckTimeout = null;
  if (dirStatusTimeout) clearTimeout(dirStatusTimeout);
  dirStatusTimeout = null;
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

      <!-- Status cycling -->
      <div class="bg-surface rounded-lg p-5 border-border-soft shadow-sm relative overflow-hidden group hover:shadow-md transition-shadow flex flex-col">
        <h3 class="font-extrabold text-text mb-2 flex items-center gap-2 text-lg">
          <RotateCcw class="text-primary" :size="20" /> {{ t('tools.status_title') }}
        </h3>
        <p class="text-sm text-text-muted font-medium mb-4">{{ t('tools.status_desc') }}</p>
        <div class="mt-auto space-y-2">
          <div class="flex items-center gap-2">
            <input v-model="rpcForm.state" type="text" class="flex-1 px-3 py-2 bg-surface-hover border border-border-soft rounded-lg text-sm text-text outline-none focus:border-primary" :placeholder="t('tools.status_placeholder')">
            <button class="text-[10px] font-bold px-2 py-1 bg-primary text-white rounded" @click="addStatusMessage">{{ l('添加', 'Add') }}</button>
          </div>
          <div class="flex flex-wrap gap-1">
            <span v-for="(msg, idx) in statusMessages" :key="idx" class="inline-flex items-center gap-1 px-2 py-1 bg-surface-hover border border-border-soft rounded text-[10px] font-bold text-text">
              {{ msg }}
              <button class="hover:text-red-500" @click="removeStatusMessage(idx)"><XCircle :size="10" /></button>
            </span>
          </div>
          <div class="flex items-center gap-2">
            <label class="text-[10px] text-text-muted font-bold whitespace-nowrap">{{ l('间隔', 'Interval') }}</label>
            <input v-model.number="statusCycleInterval" type="number" min="5" max="300" step="5" class="w-16 px-2 py-1 bg-surface-hover border border-border-soft rounded text-[10px] font-bold text-text outline-none">
            <span class="text-[10px] text-text-muted font-bold">{{ l('秒', 'sec') }}</span>
            <button class="text-[10px] font-bold px-2 py-1 rounded border border-border-soft" :class="statusCycleEnabled ? 'bg-emerald-500 text-white' : 'bg-surface-hover text-text-muted'" @click="toggleStatusCycle">
              {{ statusCycleEnabled ? l('循环中', 'Cycling') : l('开始循环', 'Start cycle') }}
            </button>
          </div>
        </div>
      </div>

      <!-- Chatbox quick send -->
      <div class="bg-surface rounded-lg p-5 border-border-soft shadow-sm relative overflow-hidden group hover:shadow-md transition-shadow flex flex-col">
        <h3 class="font-extrabold text-text mb-2 flex items-center gap-2 text-lg">
          <MessageSquare class="text-primary" :size="20" /> {{ t('tools.chatbox_title') }}
        </h3>
        <p class="text-sm text-text-muted font-medium mb-4">{{ t('tools.chatbox_desc') }}</p>
        <div class="mt-auto space-y-2">
          <textarea
            v-model="chatboxText"
            class="w-full min-h-20 px-3 py-2 bg-surface-hover border border-border-soft rounded-lg text-sm text-text outline-none resize-none focus:border-primary"
            :placeholder="t('tools.chatbox_placeholder')"
            maxlength="144"
            @input="onChatboxInput"
            @keydown.tab.prevent="onChatboxKeydown"
            @keydown.ctrl.enter.prevent="sendChatboxMessage"
          />
          <div v-if="chatboxAutocomplete && autocompleteSuggestions.length" class="flex flex-wrap gap-1">
            <button v-for="(suggestion, idx) in autocompleteSuggestions" :key="idx" class="text-[10px] font-bold px-2 py-1 rounded border border-border-soft bg-surface-hover" :class="chatboxAutocompleteIndex === idx ? 'border-primary text-primary' : 'text-text-muted'" @click="applyAutocomplete(suggestion)">
              {{ suggestion }}
            </button>
          </div>
          <div class="flex items-center justify-between text-[10px] text-text-muted font-bold">
            <span v-if="chatboxStatus.message" :class="chatboxStatus.type === 'success' ? 'text-emerald-600' : 'text-red-600'">{{ chatboxStatus.message }}</span>
            <span v-else>{{ t('tools.chatbox_hint') }}</span>
            <span>{{ chatboxText.length }}/144</span>
          </div>
          <div class="flex items-center gap-2">
            <label class="text-[10px] text-text-muted font-bold whitespace-nowrap">{{ l('发送延迟', 'Send delay') }}</label>
            <input v-model.number="chatboxSendDelay" type="number" min="0" max="5" step="0.1" class="w-16 px-2 py-1 bg-surface-hover border border-border-soft rounded text-[10px] font-bold text-text outline-none">
            <span class="text-[10px] text-text-muted font-bold">{{ l('秒', 'sec') }}</span>
            <label class="flex items-center gap-1 cursor-pointer ml-auto">
              <input v-model="chatboxNotification" type="checkbox" class="accent-primary">
              <Bell :size="12" />
              <span class="text-[10px] font-bold text-text-muted">{{ l('提示音', 'Notification') }}</span>
            </label>
          </div>
          <div class="flex items-center gap-2">
            <label class="flex items-center gap-1 cursor-pointer">
              <input v-model="chatboxEmojiShuffle" type="checkbox" class="accent-primary">
              <Sparkles :size="12" />
              <span class="text-[10px] font-bold text-text-muted">{{ l('随机表情', 'Emoji shuffle') }}</span>
            </label>
            <button class="text-[10px] font-bold px-2 py-1 rounded border border-border-soft bg-surface-hover text-text-muted hover:text-red-500" @click="clearChatbox" :disabled="chatboxStatus.loading">
              <XCircle :size="12" class="inline mr-1" /> {{ l('清空聊天框', 'Clear chatbox') }}
            </button>
          </div>
          <button
            :disabled="chatboxStatus.loading || !chatboxText.trim()"
            class="w-full py-2.5 bg-primary hover:bg-primary/80 disabled:opacity-50 text-white font-bold rounded-lg flex items-center justify-center gap-2 transition-colors"
            @click="sendChatboxMessage"
          >
            <Loader2 v-if="chatboxStatus.loading" class="animate-spin" :size="16" />
            <Send v-else :size="16" />
            {{ t('tools.chatbox_send') }}
          </button>
        </div>
        <div class="flex items-center gap-2 mt-2">
          <button class="text-[10px] font-bold px-2 py-1 rounded border border-border-soft" :class="chatboxKeepalive ? 'bg-emerald-500 text-white' : 'bg-surface-hover text-text-muted'" @click="toggleKeepalive">
            <Clock3 :size="12" class="inline mr-1" /> {{ chatboxKeepalive ? l('保活中', 'Keepalive ON') : l('保活', 'Keepalive') }}
          </button>
          <span class="text-[10px] text-text-muted font-bold">{{ l('防止chatbox超时', 'Prevent chatbox timeout') }}</span>
        </div>
      </div>

      <!-- Chat history -->
      <div v-if="chatHistory.length" class="bg-surface rounded-lg p-5 border-border-soft shadow-sm">
        <h3 class="font-extrabold text-text mb-2 flex items-center gap-2 text-lg">
          <RotateCcw :size="18" class="text-primary" /> {{ l('发送历史', 'Chat history') }}
        </h3>
        <div class="space-y-1 max-h-60 overflow-y-auto">
          <div v-for="(item, index) in chatHistory.slice(0, 20)" :key="index" class="flex items-center justify-between gap-2 py-1 border-b border-border-soft last:border-0">
            <span class="text-xs text-text truncate flex-1">{{ item.text }}</span>
            <button class="text-[10px] font-bold px-2 py-1 bg-primary text-white rounded" @click="resendChatbox(item.text)">{{ l('重发', 'Resend') }}</button>
          </div>
        </div>
        <button class="mt-2 text-[10px] font-bold text-red-500 hover:text-red-700" @click="clearChatHistory">{{ l('清空历史', 'Clear history') }}</button>
      </div>

      <!-- Instance launcher -->
      <div class="bg-surface rounded-lg p-5 border-border-soft shadow-sm relative overflow-hidden group hover:shadow-md transition-shadow flex flex-col">
        <h3 class="font-extrabold text-text mb-2 flex items-center gap-2 text-lg">
          <MapPin class="text-primary" :size="20" /> {{ t('tools.instance_title') }}
        </h3>
        <p class="text-sm text-text-muted font-medium mb-4">{{ t('tools.instance_desc') }}</p>
        <div class="mt-auto space-y-2">
          <input
            v-model="instanceTarget"
            type="text"
            class="w-full px-3 py-2.5 bg-surface-hover border border-border-soft rounded-lg text-sm text-text font-mono outline-none focus:border-primary"
            :placeholder="t('tools.instance_placeholder')"
            @keydown.enter.prevent="launchInstance"
          >
          <div v-if="instanceStatus.message" class="text-[10px] font-bold" :class="instanceStatus.type === 'success' ? 'text-emerald-600' : 'text-red-600'">
            {{ instanceStatus.message }}
          </div>
          <button
            :disabled="instanceStatus.loading || !instanceTarget.trim()"
            class="w-full py-2.5 bg-primary hover:bg-primary/80 disabled:opacity-50 text-white font-bold rounded-lg flex items-center justify-center gap-2 transition-colors"
            @click="launchInstance"
          >
            <Loader2 v-if="instanceStatus.loading" class="animate-spin" :size="16" />
            <ExternalLink v-else :size="16" />
            {{ t('tools.instance_open') }}
          </button>
        </div>
      </div>

      <OscWorkbench />

      <!-- Invite message templates -->
      <div class="bg-surface rounded-2xl p-5 border-border-soft shadow-sm relative overflow-hidden group hover:shadow-md transition-shadow md:col-span-2 lg:col-span-3">
        <div class="absolute -right-4 -top-4 w-20 h-20 bg-primary/20 rounded-full blur-2xl group-hover:bg-primary/30 transition-colors" />
        <div class="flex flex-wrap items-start justify-between gap-3 mb-4 relative z-10">
          <div>
            <h3 class="font-extrabold text-text flex items-center gap-2 text-lg">
              <MessageSquare class="text-primary" :size="20" /> {{ t('tools.invite_title') }}
            </h3>
            <p class="text-xs text-text-muted font-medium mt-1">
              {{ t('tools.invite_desc') }}
            </p>
          </div>
          <button
            :disabled="inviteTemplateLoading"
            class="px-3 py-2 rounded-xl bg-surface-hover text-text-muted hover:text-primary border-border-soft text-xs font-bold flex items-center gap-2"
            @click="loadInviteTemplates(activeInviteMessageType)"
          >
            <Loader2 v-if="inviteTemplateLoading" class="animate-spin" :size="14" />
            <MessageSquare v-else :size="14" />
            {{ t('tools.invite_refresh') }}
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
                :placeholder="t('tools.invite_placeholder')"
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
                  {{ t('tools.invite_save') }}
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>


