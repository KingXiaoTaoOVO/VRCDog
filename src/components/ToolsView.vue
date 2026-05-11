<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from 'vue';
import { SysApi } from "../api";
import { Wrench, Trash2, MessageSquare, Radio, Play, CheckCircle2, AlertCircle, Loader2, Activity, Keyboard, Send, FolderOpen, Image, FileText, Bug, Database } from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';

const { t } = useI18n();

const isVrcRunning = ref(false);
const cacheStatus = ref({ loading: false, message: '', type: '' });
const rpcForm = ref({ details: 'Using VrcDog', state: 'Chilling in VRChat' });
const rpcStatus = ref({ loading: false, message: '', type: '' });
const oscForm = ref({ address: '/avatar/parameters/Jump', value: 1 });
const oscStatus = ref({ loading: false, message: '', type: '' });
const isOscAutoSync = ref(false);

const chatboxText = ref('');
const chatboxStatus = ref({ loading: false, message: '', type: '' });
let typingTimer: any = null;

const dirStatus = ref({ message: '', type: '' });

// Send typing indicator when text changes
watch(chatboxText, (newVal) => {
  if (typingTimer) clearTimeout(typingTimer);
  SysApi.sendOscChatbox({ text: newVal, complete: false }).catch(() => {});
  typingTimer = setTimeout(() => {
    SysApi.sendOscChatbox({ text: newVal, complete: true }).catch(() => {});
  }, 2000);
});

const sendChatbox = async () => {
  if (!chatboxText.value) return;
  chatboxStatus.value = { loading: true, message: '', type: '' };
  try {
    await SysApi.sendOscChatbox({ text: chatboxText.value, complete: true });
    chatboxStatus.value = { loading: false, message: t('tools.chatbox_success'), type: 'success' };
    chatboxText.value = ''; 
  } catch (err: any) {
    chatboxStatus.value = { loading: false, message: t('tools.osc_fail', { err: err.message || err }), type: 'error' };
  }
};

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

const sendOsc = async () => {
  oscStatus.value = { loading: true, message: '', type: '' };
  try {
    await SysApi.sendOscParam({ address: oscForm.value.address, value: Number(oscForm.value.value) });
    oscStatus.value = { loading: false, message: t('tools.osc_success'), type: 'success' };
  } catch (err: any) {
    oscStatus.value = { loading: false, message: t('tools.osc_fail', { err: err.message || err }), type: 'error' };
  }
};

const toggleOscSync = async () => {
  try {
    if (isOscAutoSync.value) {
      await SysApi.stopOscAutomation();
      isOscAutoSync.value = false;
      oscStatus.value = { loading: false, message: t('tools.osc_sync_stopped'), type: 'success' };
    } else {
      await SysApi.startOscAutomation();
      isOscAutoSync.value = true;
      oscStatus.value = { loading: false, message: t('tools.osc_sync_running'), type: 'success' };
    }
  } catch (err: any) {
    oscStatus.value = { loading: false, message: t('tools.osc_fail', { err: err.message || err }), type: 'error' };
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
  setInterval(checkVrc, 10000);
});

onUnmounted(() => {
  if (isOscAutoSync.value) {
    SysApi.stopOscAutomation();
  }
});
</script>

<template>
  <div class="h-full flex flex-col p-2 space-y-6 overflow-y-auto custom-scrollbar">
    <div class="flex items-center justify-between mb-2">
      <h2 class="text-2xl font-extrabold text-[#451a03] flex items-center gap-2 tracking-tight">
        <Wrench
          class="text-amber-500"
          :size="24"
        /> {{ t('tools.title') }}
      </h2>
    </div>

    <!-- 妯″潡鍖栧鏍煎竷灞€ -->
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-5">
      <!-- 娓告垙鍚姩涓庢帶鍒?-->
      <div class="bg-surface rounded-2xl p-5 border border-border-soft shadow-sm relative overflow-hidden group hover:shadow-md transition-shadow flex flex-col">
        <div class="absolute -right-4 -top-4 w-20 h-20 bg-blue-50 rounded-full blur-2xl group-hover:bg-blue-100 transition-colors" />
        <h3 class="font-extrabold text-text mb-4 flex items-center gap-2 relative z-10 text-lg">
          <Play
            class="text-blue-500"
            :size="20"
          /> {{ t('tools.game_engine') }}
        </h3>
        <div class="space-y-4 relative z-10 flex flex-col flex-1 justify-between">
          <div class="flex items-center justify-between bg-surface-hover p-3 rounded-xl border border-border-soft">
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
            class="w-full py-3 bg-blue-500 hover:bg-blue-600 text-white font-bold rounded-xl flex items-center justify-center gap-2 transition-colors shadow-sm"
            @click="launchVrc"
          >
            <Play :size="18" /> {{ isVrcRunning ? t('tools.vrc_restart') : t('tools.vrc_start') }}
          </button>
        </div>
      </div>

      <!-- 蹇€熺洿閫?(鏂囦欢澶瑰揩鎹锋柟寮? -->
      <div class="bg-surface rounded-2xl p-5 border border-border-soft shadow-sm relative overflow-hidden group hover:shadow-md transition-shadow flex flex-col">
        <div class="absolute -right-4 -top-4 w-20 h-20 bg-amber-50 rounded-full blur-2xl group-hover:bg-amber-100 transition-colors" />
        <div class="flex items-center justify-between mb-4 relative z-10">
          <h3 class="font-extrabold text-text flex items-center gap-2 text-lg">
            <FolderOpen
              class="text-amber-500"
              :size="20"
            /> {{ t('tools.quick_links') }}
          </h3>
          <span
            v-if="dirStatus.message"
            class="text-[10px] font-bold px-2 py-0.5 rounded-md"
            :class="dirStatus.type === 'success' ? 'bg-green-50 text-green-600' : 'bg-red-50 text-red-600'"
          >
            {{ dirStatus.message }}
          </span>
        </div>
        <div class="grid grid-cols-2 gap-3 relative z-10 flex-1">
          <button
            class="flex flex-col items-center justify-center p-3 rounded-xl border border-border-soft bg-surface-hover hover:bg-amber-50 hover:border-amber-200 transition-colors gap-2 group/btn"
            @click="openDirectory('logs')"
          >
            <FileText
              class="text-border-strong group-hover/btn:text-amber-500 transition-colors"
              :size="24"
            />
            <span class="text-xs font-bold text-text-muted group-hover/btn:text-amber-700">{{ t('tools.game_logs') }}</span>
          </button>
          <button
            class="flex flex-col items-center justify-center p-3 rounded-xl border border-border-soft bg-surface-hover hover:bg-amber-50 hover:border-amber-200 transition-colors gap-2 group/btn"
            @click="openDirectory('screenshots')"
          >
            <Image
              class="text-border-strong group-hover/btn:text-amber-500 transition-colors"
              :size="24"
            />
            <span class="text-xs font-bold text-text-muted group-hover/btn:text-amber-700">{{ t('tools.screenshots') }}</span>
          </button>
          <button
            class="flex flex-col items-center justify-center p-3 rounded-xl border border-border-soft bg-surface-hover hover:bg-amber-50 hover:border-amber-200 transition-colors gap-2 group/btn shadow-sm"
            @click="openDirectory('cache')"
          >
            <Database
              class="text-border-strong group-hover/btn:text-amber-500 transition-colors"
              :size="24"
            />
            <span class="text-xs font-bold text-text-muted group-hover/btn:text-amber-700">{{ t('tools.cache_dir') }}</span>
          </button>
          <button
            class="flex flex-col items-center justify-center p-3 rounded-xl border border-border-soft bg-surface-hover hover:bg-amber-50 hover:border-amber-200 transition-colors gap-2 group/btn"
            @click="openDirectory('crash_reports')"
          >
            <Bug
              class="text-border-strong group-hover/btn:text-amber-500 transition-colors"
              :size="24"
            />
            <span class="text-xs font-bold text-text-muted group-hover/btn:text-amber-700">{{ t('tools.crash_reports') }}</span>
          </button>
        </div>
      </div>

      <!-- 缂撳瓨娓呯悊 -->
      <div class="bg-surface rounded-2xl p-5 border border-border-soft shadow-sm relative overflow-hidden group hover:shadow-md transition-shadow flex flex-col">
        <div class="absolute -right-4 -top-4 w-20 h-20 bg-red-50 rounded-full blur-2xl group-hover:bg-red-100 transition-colors" />
        <h3 class="font-extrabold text-text mb-4 flex items-center gap-2 relative z-10 text-lg">
          <Trash2
            class="text-red-500"
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
              :class="cacheStatus.type === 'success' ? 'bg-green-50 text-green-600' : 'bg-red-50 text-red-600'"
            >
              {{ cacheStatus.message }}
            </div>
            <button
              :disabled="cacheStatus.loading"
              class="w-full py-3 bg-red-500 hover:bg-red-600 disabled:opacity-50 text-white font-bold rounded-xl flex items-center justify-center gap-2 transition-colors shadow-sm"
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
      <div class="bg-surface rounded-2xl p-5 border border-border-soft shadow-sm relative overflow-hidden group hover:shadow-md transition-shadow flex flex-col">
        <div class="absolute -right-4 -top-4 w-20 h-20 bg-indigo-50 rounded-full blur-2xl group-hover:bg-indigo-100 transition-colors" />
        <h3 class="font-extrabold text-text mb-4 flex items-center gap-2 relative z-10 text-lg">
          <MessageSquare
            class="text-indigo-500"
            :size="20"
          /> {{ t('tools.rpc_title') }}
        </h3>
        <div class="space-y-3 relative z-10">
          <div>
            <label class="block text-xs font-bold text-text-muted mb-1">{{ t('tools.rpc_details') }}</label>
            <input
              v-model="rpcForm.details"
              type="text"
              class="w-full px-3 py-2 bg-surface-hover border border-border-soft rounded-lg text-sm outline-none focus:border-indigo-400 focus:bg-surface transition-all"
            >
          </div>
          <div>
            <label class="block text-xs font-bold text-text-muted mb-1">{{ t('tools.rpc_state') }}</label>
            <input
              v-model="rpcForm.state"
              type="text"
              class="w-full px-3 py-2 bg-surface-hover border border-border-soft rounded-lg text-sm outline-none focus:border-indigo-400 focus:bg-surface transition-all"
            >
          </div>
          <button
            :disabled="rpcStatus.loading"
            class="w-full py-2.5 bg-indigo-500 hover:bg-indigo-600 disabled:opacity-50 text-white font-bold rounded-xl flex items-center justify-center gap-2 transition-colors mt-2"
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
            :class="rpcStatus.type === 'success' ? 'bg-green-50 text-green-600' : 'bg-red-50 text-red-600'"
          >
            {{ rpcStatus.message }}
          </div>
        </div>
      </div>

      <!-- OSC 鎺у埗 -->
      <div class="bg-surface rounded-2xl p-5 border border-border-soft shadow-sm relative overflow-hidden group hover:shadow-md transition-shadow flex flex-col">
        <div class="absolute -right-4 -top-4 w-20 h-20 bg-emerald-50 rounded-full blur-2xl group-hover:bg-emerald-100 transition-colors" />
        <h3 class="font-extrabold text-text mb-4 flex items-center gap-2 relative z-10 text-lg">
          <Radio
            class="text-emerald-500"
            :size="20"
          /> {{ t('tools.osc_title') }}
        </h3>
        <div class="space-y-3 relative z-10">
          <div>
            <label class="block text-xs font-bold text-text-muted mb-1">{{ t('tools.osc_address') }}</label>
            <input
              v-model="oscForm.address"
              type="text"
              placeholder="/avatar/parameters/..."
              class="w-full px-3 py-2 bg-surface-hover border border-border-soft rounded-lg text-sm outline-none focus:border-emerald-400 focus:bg-surface transition-all"
            >
          </div>
          <div>
            <label class="block text-xs font-bold text-text-muted mb-1">{{ t('tools.osc_value') }}</label>
            <input
              v-model="oscForm.value"
              type="number"
              step="0.1"
              class="w-full px-3 py-2 bg-surface-hover border border-border-soft rounded-lg text-sm outline-none focus:border-emerald-400 focus:bg-surface transition-all"
            >
          </div>
          <div class="flex flex-col gap-2 mt-3">
            <button
              :disabled="oscStatus.loading"
              class="w-full py-2.5 bg-emerald-500 hover:bg-emerald-600 disabled:opacity-50 text-white font-bold rounded-xl flex items-center justify-center gap-2 transition-colors shadow-sm"
              @click="sendOsc"
            >
              <Loader2
                v-if="oscStatus.loading"
                class="animate-spin"
                :size="16"
              />
              <Send
                v-else
                :size="16"
              /> {{ t('tools.osc_send') }}
            </button>
            <button
              class="w-full py-2.5 font-bold rounded-xl flex items-center justify-center gap-2 transition-colors shadow-sm"
              :class="isOscAutoSync ? 'bg-red-50 text-red-600 hover:bg-red-100 border border-red-200' : 'bg-fuchsia-50 text-fuchsia-600 hover:bg-fuchsia-100 border border-fuchsia-200'"
              @click="toggleOscSync"
            >
              <Activity :size="16" /> {{ isOscAutoSync ? t('tools.osc_sync_stop') : t('tools.osc_sync_start') }}
            </button>
          </div>
          <div
            v-if="oscStatus.message"
            class="text-xs font-bold px-3 py-2 rounded-lg text-center"
            :class="oscStatus.type === 'success' ? 'bg-green-50 text-green-600' : 'bg-red-50 text-red-600'"
          >
            {{ oscStatus.message }}
          </div>
        </div>
      </div>

      <!-- VRChat 娴┖鎵撳瓧鏈?(鍗犱綅瀹? -->
      <div class="bg-surface rounded-2xl p-5 border border-border-soft shadow-sm relative overflow-hidden group hover:shadow-md transition-shadow md:col-span-2 lg:col-span-1">
        <div class="absolute -right-4 -top-4 w-20 h-20 bg-pink-50 rounded-full blur-2xl group-hover:bg-pink-100 transition-colors" />
        <h3 class="font-extrabold text-text mb-4 flex items-center gap-2 relative z-10 text-lg">
          <Keyboard
            class="text-pink-500"
            :size="20"
          /> {{ t('tools.chatbox_title') }}
        </h3>
        <div class="space-y-3 relative z-10 flex flex-col flex-1">
          <p class="text-xs text-text-muted font-medium leading-relaxed mb-1">
            {{ t('tools.chatbox_desc') }}
          </p>
          <textarea
            v-model="chatboxText"
            :placeholder="t('tools.chatbox_placeholder')"
            class="w-full flex-1 min-h-[60px] p-3 bg-surface-hover border border-border-soft rounded-xl text-sm font-medium outline-none focus:border-pink-400 focus:bg-surface transition-all resize-none custom-scrollbar"
            @keyup.enter.prevent="sendChatbox"
          />
          <div class="flex items-center justify-between mt-auto">
            <span
              v-if="chatboxStatus.message"
              class="text-[10px] font-bold px-2 py-1 rounded-md"
              :class="chatboxStatus.type === 'success' ? 'bg-green-50 text-green-600' : 'bg-red-50 text-red-600'"
            >
              {{ chatboxStatus.message }}
            </span>
            <span
              v-else
              class="text-[10px] text-border-strong"
            >{{ t('tools.chatbox_hint') }}</span>
            
            <button
              :disabled="chatboxStatus.loading || !chatboxText.trim()"
              class="px-5 py-2 bg-surface hover:bg-black disabled:opacity-50 text-white font-bold rounded-xl flex items-center justify-center gap-2 transition-colors ml-auto"
              @click="sendChatbox"
            >
              <Loader2
                v-if="chatboxStatus.loading"
                class="animate-spin"
                :size="16"
              />
              <Send
                v-else
                :size="16"
              /> {{ t('tools.chatbox_send') }}
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.custom-scrollbar::-webkit-scrollbar { width: 6px; }
.custom-scrollbar::-webkit-scrollbar-track { background: transparent; }
.custom-scrollbar::-webkit-scrollbar-thumb { background: #cbd5e1; border-radius: 10px; }
.custom-scrollbar::-webkit-scrollbar-thumb:hover { background: #94a3b8; }
</style>
