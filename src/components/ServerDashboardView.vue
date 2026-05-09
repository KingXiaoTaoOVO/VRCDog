<template>
  <div class="h-screen w-screen flex flex-col bg-gray-900 text-gray-100 font-mono p-4 relative overflow-hidden">
    <!-- Top Bar -->
    <div class="flex items-center justify-between mb-3 pb-3 border-b border-gray-700 shrink-0">
      <div class="flex items-center gap-3">
        <Server class="w-7 h-7 transition-colors" :class="isRunning ? 'text-emerald-400' : 'text-gray-500'" />
        <div>
          <h1 class="text-lg font-bold">{{ t('role.dashboard_title') }}</h1>
          <p class="text-xs" :class="isRunning ? 'text-emerald-400' : 'text-gray-500'">
            {{ isRunning ? t('role.dashboard_running', { port: serverPort }) : t('role.server_stopped') }}
          </p>
        </div>
      </div>
      <div class="flex gap-2">
        <button @click="cycleLanguage" class="flex items-center gap-1 px-2 py-1 bg-gray-800 hover:bg-gray-700 rounded border border-gray-600 text-xs text-gray-300">
          <Globe class="w-3.5 h-3.5" /> {{ currentLangLabel }}
        </button>
        <button @click="openNewClient" class="flex items-center gap-1 px-3 py-1 bg-indigo-500/20 text-indigo-400 hover:bg-indigo-500/40 rounded border border-indigo-500/30 text-xs font-bold">
          <Monitor class="w-3.5 h-3.5" /> {{ t('role.dashboard_open_client') }}
        </button>
        <button @click="stopAndExit" class="flex items-center gap-1 px-3 py-1 bg-gray-800 hover:bg-gray-700 text-gray-300 rounded border border-gray-600 text-xs font-bold">
          <LogOut class="w-3.5 h-3.5" /> {{ t('role.back') }}
        </button>
      </div>
    </div>

    <!-- Main Area -->
    <div class="flex-1 flex gap-4 overflow-hidden">
      <!-- Left Panel: Server Config + Online Clients -->
      <div class="w-60 flex flex-col gap-3 shrink-0 overflow-hidden">
        <!-- Connection Info / Start -->
        <div class="bg-gray-800 rounded-lg p-3 border border-gray-700">
          <h2 class="text-xs text-gray-400 mb-2 uppercase tracking-wider">{{ t('role.dashboard_conn_info') }}</h2>
          <div v-if="!isRunning" class="space-y-2">
            <div>
              <label class="text-xs text-gray-500 block mb-1">IP</label>
              <input v-model="serverHost" class="w-full bg-gray-900 border border-gray-600 rounded px-2 py-1 text-xs outline-none focus:border-emerald-500" />
            </div>
            <div>
              <label class="text-xs text-gray-500 block mb-1">Port</label>
              <input v-model.number="serverPort" type="number" class="w-full bg-gray-900 border border-gray-600 rounded px-2 py-1 text-xs outline-none focus:border-emerald-500" />
            </div>
            <button @click="startLocalServer" class="w-full mt-2 py-1.5 bg-emerald-500 hover:bg-emerald-600 text-white font-bold rounded text-xs flex justify-center items-center gap-1">
              <Play class="w-3.5 h-3.5" /> {{ t('role.start_server_btn') }}
            </button>
          </div>
          <div v-else class="space-y-2">
            <div class="text-lg font-bold text-white">{{ serverHost }}:{{ serverPort }}</div>
            <p class="text-xs text-gray-500">{{ t('role.dashboard_listening') }}</p>
            <button @click="stopLocalServer" class="w-full py-1.5 bg-amber-500/20 text-amber-500 hover:bg-amber-500/40 font-bold rounded text-xs border border-amber-500/30 flex justify-center items-center gap-1">
              <Square class="w-3.5 h-3.5" /> {{ t('role.stop_server_btn') }}
            </button>
          </div>
        </div>

        <!-- Online Clients -->
        <div class="bg-gray-800 rounded-lg p-3 border border-gray-700 flex-1 overflow-hidden flex flex-col">
          <h2 class="text-xs text-gray-400 mb-2 uppercase tracking-wider">
            {{ t('role.dashboard_clients') }} ({{ onlineClients.length }})
          </h2>
          <div class="flex-1 overflow-y-auto space-y-1.5" v-if="onlineClients.length > 0">
            <div v-for="c in onlineClients" :key="c.user_id"
              class="flex items-center gap-2 p-2 bg-gray-900 rounded border border-gray-700 hover:border-gray-500 cursor-pointer transition-colors"
              @click="selectUser(c.user_id)">
              <div class="w-2 h-2 bg-emerald-400 rounded-full shrink-0 animate-pulse"></div>
              <div class="min-w-0 flex-1">
                <div class="text-xs font-bold truncate">{{ c.display_name }}</div>
                <div class="text-[10px] text-gray-500 truncate">{{ c.ip_address }}</div>
              </div>
            </div>
          </div>
          <div v-else class="flex-1 flex items-center justify-center text-gray-600 text-xs text-center px-2">
            {{ t('role.dashboard_no_clients') }}
          </div>
        </div>
      </div>

      <!-- Right Panel: Tabs -->
      <div class="flex-1 flex flex-col overflow-hidden">
        <!-- Tab Buttons -->
        <div class="flex gap-1 mb-3 shrink-0">
          <button v-for="tab in tabs" :key="tab.key" @click="activeTab = tab.key"
            class="px-3 py-1.5 rounded text-xs font-bold transition-colors"
            :class="activeTab === tab.key ? 'bg-emerald-500/20 text-emerald-400 border border-emerald-500/40' : 'bg-gray-800 text-gray-400 hover:bg-gray-700 border border-gray-700'">
            {{ tab.label }}
          </button>
        </div>

        <!-- Tab: Terminal Logs -->
        <div v-show="activeTab === 'logs'" class="flex-1 bg-black rounded-lg border border-gray-700 p-3 flex flex-col overflow-hidden relative">
          <div class="flex justify-between items-center mb-2">
            <span class="text-xs text-gray-400">{{ t('role.dashboard_logs') }}</span>
            <button @click="logs = []" class="text-xs text-gray-500 hover:text-white">{{ t('role.dashboard_clear_logs') }}</button>
          </div>
          <div class="flex-1 overflow-y-auto text-xs" ref="logContainer">
            <div v-for="(log, idx) in logs" :key="idx" class="mb-0.5 leading-relaxed break-all">
              <span class="text-gray-500 mr-1">[{{ log.time }}]</span>
              <span :class="{'text-blue-400': log.level==='INFO','text-red-400': log.level==='ERROR','text-yellow-400': log.level==='WARN','text-green-400': log.level==='SUCCESS'}">{{ log.content }}</span>
            </div>
            <div v-if="logs.length===0" class="text-gray-600 mt-4 text-center">{{ t('role.dashboard_no_logs') }}</div>
          </div>
        </div>

        <!-- Tab: User Management -->
        <div v-show="activeTab === 'users'" class="flex-1 bg-gray-800 rounded-lg border border-gray-700 p-3 flex flex-col overflow-hidden">
          <div class="flex-1 overflow-y-auto">
            <table class="w-full text-xs" v-if="allUsers.length > 0">
              <thead class="sticky top-0 bg-gray-800 z-10">
                <tr class="text-gray-400 text-left border-b border-gray-700">
                  <th class="py-2 px-2">{{ t('role.user_name') }}</th>
                  <th class="py-2 px-1">{{ t('role.status') }}</th>
                  <th class="py-2 px-1">{{ t('role.login_count') }}</th>
                  <th class="py-2 px-1">{{ t('role.role') }}</th>
                  <th class="py-2 px-1 text-right">{{ t('role.action') }}</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="u in allUsers" :key="u.user_id" class="border-b border-gray-700/50 hover:bg-gray-700/30">
                  <td class="py-2 px-2">
                    <div class="font-bold">{{ u.display_name }}</div>
                    <div class="text-[10px] text-gray-500 truncate max-w-[140px]">{{ u.user_id }}</div>
                  </td>
                  <td class="py-2 px-1">
                    <div v-if="banMap[u.user_id]">
                      <span class="px-1.5 py-0.5 bg-red-500/20 text-red-400 rounded text-[10px]">{{ t('role.ban') }}</span>
                      <div class="text-[9px] text-red-400/70 mt-1 whitespace-nowrap">
                        <span v-if="banMap[u.user_id].duration_hours">{{ banMap[u.user_id].duration_hours }}{{ t('role.hours') }}</span>
                        <span v-else>{{ t('role.permanent') }}</span>
                        <div v-if="banMap[u.user_id].expires_at" class="scale-90 origin-left mt-0.5">{{ banMap[u.user_id].expires_at?.substring(5) }}</div>
                      </div>
                    </div>
                    <div v-else-if="freezeMap[u.user_id]">
                      <span class="px-1.5 py-0.5 bg-blue-500/20 text-blue-400 rounded text-[10px]">{{ t('role.freeze') }}</span>
                    </div>
                    <span v-else-if="u.is_online" class="px-1.5 py-0.5 bg-emerald-500/20 text-emerald-400 rounded text-[10px]">{{ t('role.online') }}</span>
                    <span v-else class="px-1.5 py-0.5 bg-gray-600/30 text-gray-500 rounded text-[10px]">{{ t('role.offline') }}</span>
                  </td>
                  <td class="py-2 px-1 text-gray-400">{{ u.login_count }}</td>
                  <td class="py-2 px-1">
                    <select v-model="u.role_id" @change="setUserRole(u.user_id, u.role_id)" class="bg-gray-900 border border-gray-600 rounded text-[10px] px-1 py-0.5 outline-none focus:border-emerald-500 text-gray-300">
                      <option :value="null">{{ t('role.default_role') }}</option>
                      <option v-for="r in allRoles" :key="r.role_id" :value="r.role_id">{{ r.role_name }}</option>
                    </select>
                  </td>
                  <td class="py-2 px-1 text-right">
                    <div class="flex gap-1 justify-end flex-wrap">
                      <button v-if="u.is_online" @click="kickUser(u.user_id)" class="px-1.5 py-0.5 bg-yellow-500/20 text-yellow-400 hover:bg-yellow-500/40 rounded text-[10px]">{{ t('role.kick') }}</button>
                      <button v-if="!banMap[u.user_id]" @click="openBanDialog(u)" class="px-1.5 py-0.5 bg-red-500/20 text-red-400 hover:bg-red-500/40 rounded text-[10px]">{{ t('role.ban') }}</button>
                      <button v-else @click="unbanUser(u.user_id)" class="px-1.5 py-0.5 bg-green-500/20 text-green-400 hover:bg-green-500/40 rounded text-[10px]">{{ t('role.unban') }}</button>
                      <button v-if="!freezeMap[u.user_id]" @click="openFreezeDialog(u)" class="px-1.5 py-0.5 bg-blue-500/20 text-blue-400 hover:bg-blue-500/40 rounded text-[10px]">{{ t('role.freeze') }}</button>
                      <button v-else @click="unfreezeUser(u.user_id)" class="px-1.5 py-0.5 bg-cyan-500/20 text-cyan-400 hover:bg-cyan-500/40 rounded text-[10px]">{{ t('role.unfreeze') }}</button>
                      <button @click="removeUser(u.user_id)" class="px-1.5 py-0.5 bg-gray-600/30 text-gray-400 hover:bg-gray-500/40 rounded text-[10px]">{{ t('role.remove') }}</button>
                    </div>
                  </td>
                </tr>
              </tbody>
            </table>
            <div v-else class="flex-1 flex items-center justify-center text-gray-600 text-sm h-full">{{ t('role.no_user_records') }}</div>
          </div>
        </div>

        <!-- Tab: Roles -->
        <div v-show="activeTab === 'features'" class="flex-1 bg-gray-800 rounded-lg border border-gray-700 p-3 flex overflow-hidden gap-4">
          <!-- Roles List -->
          <div class="w-1/3 flex flex-col border-r border-gray-700 pr-3">
            <div class="flex justify-between items-center mb-2 shrink-0">
              <h3 class="text-sm font-bold text-gray-300">{{ t('role.dashboard_roles_list') || '角色列表' }}</h3>
              <button @click="createNewRole" class="px-2 py-1 bg-emerald-500/20 text-emerald-400 hover:bg-emerald-500/40 rounded text-[10px]">+</button>
            </div>
            <div class="flex-1 overflow-y-auto space-y-2">
              <div v-for="r in allRoles" :key="r.role_id"
                @click="selectRole(r)"
                class="p-2 rounded border cursor-pointer flex justify-between items-center"
                :class="selectedRole?.role_id === r.role_id ? 'bg-emerald-500/20 border-emerald-500/50 text-emerald-400' : 'bg-gray-900 border-gray-700 text-gray-400 hover:border-gray-500'">
                <div class="text-xs font-bold">{{ r.role_name }} <span v-if="r.is_default" class="text-[9px] bg-gray-700 px-1 rounded ml-1">{{ t('role.default') }}</span></div>
                <div class="flex gap-1" v-if="selectedRole?.role_id === r.role_id">
                  <button v-if="!r.is_default" @click.stop="setDefaultRole(r.role_id)" class="text-[10px] hover:text-white" :title="t('role.set_default')">👑</button>
                  <button v-if="!r.is_default" @click.stop="deleteRole(r.role_id)" class="text-[10px] hover:text-red-400" :title="t('role.delete')">🗑️</button>
                </div>
              </div>
            </div>
          </div>
          <!-- Role Editor -->
          <div class="flex-1 flex flex-col overflow-y-auto" v-if="selectedRole">
            <div class="mb-3 shrink-0">
               <label class="text-xs text-gray-400 block mb-1">{{ t('role.role_name') }}</label>
               <input v-model="selectedRole.role_name" class="w-full bg-gray-900 border border-gray-600 rounded px-2 py-1.5 text-sm outline-none focus:border-emerald-500" />
            </div>
            <div class="mb-4">
              <h3 class="text-sm font-bold text-gray-300 mb-2">{{ t('role.feature_menus') }}</h3>
              <div class="grid grid-cols-3 gap-2">
                <label v-for="(enabled, key) in selectedRole.features.menus" :key="key"
                  class="flex items-center gap-2 p-2 bg-gray-900 rounded border border-gray-700 hover:border-gray-500 cursor-pointer text-xs">
                  <input type="checkbox" v-model="selectedRole.features.menus[key]" class="accent-emerald-500" />
                  <span>{{ t('sidebar.' + key) || key }}</span>
                </label>
              </div>
            </div>
            <div class="mb-4">
              <h3 class="text-sm font-bold text-gray-300 mb-2">{{ t('role.feature_modes') }}</h3>
              <div class="grid grid-cols-2 gap-2">
                <label v-for="(enabled, key) in selectedRole.features.modes" :key="key"
                  class="flex items-center gap-2 p-2 bg-gray-900 rounded border border-gray-700 hover:border-gray-500 cursor-pointer text-xs">
                  <input type="checkbox" v-model="selectedRole.features.modes[key]" class="accent-emerald-500" />
                  <span>{{ t('role.mode_' + key) || (key === 'pc' ? 'PC Desktop' : key === 'vr' ? 'VR Overlay' : key) }}</span>
                </label>
              </div>
            </div>
            <div class="mb-4">
              <h3 class="text-sm font-bold text-gray-300 mb-2">{{ t('role.feature_themes') }}</h3>
              <div class="grid grid-cols-3 gap-2">
                <label v-for="(enabled, key) in selectedRole.features.themes" :key="key"
                  class="flex items-center gap-2 p-2 bg-gray-900 rounded border border-gray-700 hover:border-gray-500 cursor-pointer text-xs">
                  <input type="checkbox" v-model="selectedRole.features.themes[key]" class="accent-emerald-500" />
                  <span>{{ t('role.theme_' + key) || (key.charAt(0).toUpperCase() + key.slice(1)) }}</span>
                </label>
              </div>
            </div>
            <div class="mt-auto pt-3 shrink-0">
               <button @click="saveRole" class="px-4 py-1.5 bg-emerald-500 hover:bg-emerald-600 text-white font-bold rounded text-xs w-full">{{ t('role.save_role') }}</button>
            </div>
          </div>
          <div v-else class="flex-1 flex items-center justify-center text-gray-500 text-sm">{{ t('role.select_or_create_role') }}</div>
        </div>
      </div>
    </div>

    <!-- Ban Dialog -->
    <div v-if="showBanDialog" class="fixed inset-0 bg-black/60 flex items-center justify-center z-[999]" @click.self="showBanDialog=false">
      <div class="bg-gray-800 rounded-xl p-5 w-96 border border-gray-600 shadow-2xl">
        <h3 class="text-sm font-bold mb-3 text-red-400">{{ t('role.ban_user') }} {{ dialogUser?.display_name }}</h3>
        <div class="space-y-3">
          <div>
            <label class="text-xs text-gray-400 block mb-1">{{ t('role.ban_reason') }}</label>
            <input v-model="banReason" class="w-full bg-gray-900 border border-gray-600 rounded px-2 py-1.5 text-sm outline-none focus:border-red-500" :placeholder="t('role.ban_reason_ph')" />
          </div>
          <div>
            <label class="text-xs text-gray-400 block mb-1">{{ t('role.ban_duration') }}</label>
            <input v-model.number="banDuration" type="number" class="w-full bg-gray-900 border border-gray-600 rounded px-2 py-1.5 text-sm outline-none focus:border-red-500" :placeholder="t('role.ban_duration_ph')" />
          </div>
          <div class="flex gap-2 justify-end mt-4">
            <button @click="showBanDialog=false" class="px-4 py-1.5 bg-gray-700 hover:bg-gray-600 rounded text-xs">{{ t('role.cancel') }}</button>
            <button @click="confirmBan" class="px-4 py-1.5 bg-red-500 hover:bg-red-600 text-white font-bold rounded text-xs">{{ t('role.confirm_ban') }}</button>
          </div>
        </div>
      </div>
    </div>

    <!-- Freeze Dialog -->
    <div v-if="showFreezeDialog" class="fixed inset-0 bg-black/60 flex items-center justify-center z-[999]" @click.self="showFreezeDialog=false">
      <div class="bg-gray-800 rounded-xl p-5 w-96 border border-gray-600 shadow-2xl">
        <h3 class="text-sm font-bold mb-3 text-blue-400">{{ t('role.freeze_user') }} {{ dialogUser?.display_name }}</h3>
        <div class="space-y-3">
          <div>
            <label class="text-xs text-gray-400 block mb-1">{{ t('role.freeze_reason') }}</label>
            <input v-model="freezeReason" class="w-full bg-gray-900 border border-gray-600 rounded px-2 py-1.5 text-sm outline-none focus:border-blue-500" :placeholder="t('role.freeze_reason_ph')" />
          </div>
          <div class="flex gap-2 justify-end mt-4">
            <button @click="showFreezeDialog=false" class="px-4 py-1.5 bg-gray-700 hover:bg-gray-600 rounded text-xs">{{ t('role.cancel') }}</button>
            <button @click="confirmFreeze" class="px-4 py-1.5 bg-blue-500 hover:bg-blue-600 text-white font-bold rounded text-xs">{{ t('role.confirm_freeze') }}</button>
          </div>
        </div>
      </div>
    </div>

    <!-- Toast -->
    <div v-if="toastMessage" class="fixed top-5 left-1/2 transform -translate-x-1/2 px-4 py-2 bg-emerald-500 text-white rounded shadow-lg z-[1000] text-sm font-bold transition-all animate-bounce">
      {{ toastMessage }}
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, nextTick, computed, reactive } from 'vue';
import { Server, Globe, Monitor, LogOut, Play, Square } from 'lucide-vue-next';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { useI18n } from 'vue-i18n';
import { SysApi, DbApi, VrcApi } from '../api';
import { useStorage } from '@vueuse/core';

const { t, locale } = useI18n();
const langMap: Record<string, string> = { 'zh-CN': '简体中文', 'en-US': 'English', 'ja-JP': '日本語' };
const currentLangLabel = computed(() => langMap[locale.value] || 'Language');
const cycleLanguage = () => {
  const keys = Object.keys(langMap);
  locale.value = keys[(keys.indexOf(locale.value) + 1) % keys.length];
  localStorage.setItem('vrcdog-locale', locale.value);
  DbApi.saveSetting({ key: 'language', value: JSON.stringify(locale.value) }).catch(() => {});
};

const emit = defineEmits(['exit']);
const serverHost = useStorage('vrc_dashboard_host', '0.0.0.0');
const serverPort = useStorage('vrc_dashboard_port', 11451);
const isRunning = ref(false);
const activeTab = ref('logs');
const tabs = computed(() => [
  { key: 'logs', label: t('role.dashboard_logs_tab') || '终端日志' },
  { key: 'users', label: t('role.dashboard_users_tab') || '用户管理' },
  { key: 'features', label: t('role.dashboard_roles_tab') || '角色与权限' },
]);

const toastMessage = ref('');
const showToast = (msg: string) => {
  toastMessage.value = msg;
  setTimeout(() => toastMessage.value = '', 3000);
};

// === Server URL helper ===
const serverUrl = computed(() => {
  const h = serverHost.value === '0.0.0.0' ? '127.0.0.1' : serverHost.value;
  return `http://${h}:${serverPort.value}`;
});

// === Logs ===
interface LogEntry { time: string; level: string; content: string; }
const logs = ref<LogEntry[]>([]);
const logContainer = ref<HTMLElement | null>(null);
let unlistenLog: UnlistenFn | null = null;
let unlistenClients: UnlistenFn | null = null;

const addLog = (content: string) => {
  const time = new Date().toTimeString().split(' ')[0];
  let level = 'INFO', clean = content;
  if (content.startsWith('[ERROR]')) { level = 'ERROR'; clean = content.substring(7).trim(); }
  else if (content.startsWith('[WARN]')) { level = 'WARN'; clean = content.substring(6).trim(); }
  else if (content.startsWith('[INFO]')) { level = 'INFO'; clean = content.substring(6).trim(); }
  logs.value.push({ time, level, content: clean });
  if (logs.value.length > 500) logs.value.shift();
  nextTick(() => { if (logContainer.value) logContainer.value.scrollTop = logContainer.value.scrollHeight; });
};

// === Online Clients ===
interface ClientInfo { user_id: string; display_name: string; avatar_url: string; ip_address: string; connected_at: string; last_heartbeat: string; }
interface UserRecord { user_id: string; display_name: string; avatar_url: string; first_seen: string; last_seen: string; login_count: number; is_online: boolean; role_id: string | null; }
interface BanInfo { user_id: string; reason: string; banned_at: string; duration_hours: number | null; expires_at: string | null; }
interface FreezeInfo { user_id: string; reason: string; frozen_at: string; }
interface FeatureConfig { menus: Record<string, boolean>; themes: Record<string, boolean>; modes: Record<string, boolean>; }
interface Role { role_id: string; role_name: string; is_default: boolean; features: FeatureConfig; }

const onlineClients = ref<ClientInfo[]>([]);
const allUsers = ref<UserRecord[]>([]);
const banMap = ref<Record<string, BanInfo>>({});
const freezeMap = ref<Record<string, FreezeInfo>>({});
const allRoles = ref<Role[]>([]);
const selectedRole = ref<Role | null>(null);

let pollTimer: ReturnType<typeof setInterval> | null = null;

const fetchClients = async () => {
  if (!isRunning.value) return;
  try {
    const data = await VrcApi.request(`${serverUrl.value}/api/admin/clients`, 'GET');
    onlineClients.value = data.clients || [];
  } catch { /* ignore */ }
};
const fetchUsers = async () => {
  if (!isRunning.value) return;
  try {
    const data = await VrcApi.request(`${serverUrl.value}/api/admin/users`, 'GET');
    allUsers.value = data.users || [];
    banMap.value = data.bans || {};
    freezeMap.value = data.frozen || {};
  } catch { /* ignore */ }
};
const fetchRoles = async () => {
  if (!isRunning.value) return;
  try {
    const data = await VrcApi.request(`${serverUrl.value}/api/admin/roles`, 'GET');
    allRoles.value = data.roles || [];
  } catch { /* ignore */ }
};
const startPolling = () => {
  fetchClients(); fetchUsers(); fetchRoles();
  pollTimer = setInterval(() => { fetchClients(); fetchUsers(); fetchRoles(); }, 5000);
};
const stopPolling = () => { if (pollTimer) { clearInterval(pollTimer); pollTimer = null; } };

// === User Actions ===
const selectedUserId = ref<string | null>(null);
const selectUser = (uid: string) => { selectedUserId.value = uid; activeTab.value = 'users'; };

const adminPost = async (endpoint: string, body: any) => {
  try {
    const data = await VrcApi.request(`${serverUrl.value}${endpoint}`, 'POST', body);
    
    if (data && data.success === false) {
       throw new Error(data.message || '操作未成功');
    }
    
    addLog(`[INFO] ${data?.message || 'OK'}`);
    showToast(data?.message || '操作成功');
    fetchClients(); 
    fetchUsers();
  } catch (e: any) { 
    addLog(`[ERROR] ${e.message || e}`); 
    showToast(`操作失败: ${e.message || e}`);
  }
};

const kickUser = (uid: string) => adminPost('/api/admin/kick', { user_id: uid });
const unbanUser = (uid: string) => adminPost('/api/admin/unban', { user_id: uid });
const unfreezeUser = (uid: string) => adminPost('/api/admin/unfreeze', { user_id: uid });
const removeUser = (uid: string) => { if (confirm(t('role.confirm_remove'))) adminPost('/api/admin/remove', { user_id: uid }); };

// Ban Dialog
const showBanDialog = ref(false);
const dialogUser = ref<UserRecord | null>(null);
const banReason = ref('');
const banDuration = ref<number | undefined>(undefined);
const openBanDialog = (u: UserRecord) => { dialogUser.value = u; banReason.value = ''; banDuration.value = undefined; showBanDialog.value = true; };
const confirmBan = () => {
  if (!dialogUser.value) return;
  if (!banReason.value.trim()) {
    alert(t('role.please_input_ban_reason'));
    return;
  }
  adminPost('/api/admin/ban', { user_id: dialogUser.value.user_id, reason: banReason.value.trim(), duration_hours: banDuration.value || null });
  showBanDialog.value = false;
};

// Freeze Dialog
const showFreezeDialog = ref(false);
const freezeReason = ref('');
const openFreezeDialog = (u: UserRecord) => { dialogUser.value = u; freezeReason.value = ''; showFreezeDialog.value = true; };
const confirmFreeze = () => {
  if (!dialogUser.value) return;
  if (!freezeReason.value.trim()) {
    alert(t('role.please_input_freeze_reason'));
    return;
  }
  adminPost('/api/admin/freeze', { user_id: dialogUser.value.user_id, reason: freezeReason.value.trim() });
  showFreezeDialog.value = false;
};

// Roles Management
const createNewRole = () => {
  const newRole: Role = {
    role_id: 'role_' + Date.now(),
    role_name: t('role.new_role'),
    is_default: false,
    features: {
      menus: { "dashboard":true, "feed":true, "friendlog":true, "locations":true, "charts":true, "playerlist":true, "gallery":true, "social":true, "search":true, "notifications":true, "groups":true, "avatars":true, "favorites":true, "moderation":true, "heatmap":true, "gamelog":true, "notes":true, "presets":true, "tools":true, "translator":true, "ovr":true, "env":true, "export":true, "settings":true },
      themes: { "dog":true, "cat":true, "helmet":true, "mono":true },
      modes: { "pc":true, "vr":true }
    }
  };
  selectedRole.value = newRole;
};

const selectRole = (r: Role) => { selectedRole.value = JSON.parse(JSON.stringify(r)); };

const saveRole = async () => {
  if (!selectedRole.value) return;
  try {
    const data = await VrcApi.request(`${serverUrl.value}/api/admin/roles`, 'POST', selectedRole.value);
    if(data.success) {
      alert(t('settings.saved'));
      fetchRoles();
    }
  } catch (e: any) { 
    addLog(`[ERROR] 保存失败: ${e.message || e}`);
    showToast(`保存失败: ${e.message || e}`);
  }
};

const deleteRole = async (role_id: string) => {
  if(!confirm('确定删除此角色吗？')) return;
  try {
    const data = await VrcApi.request(`${serverUrl.value}/api/admin/roles/delete`, 'POST', { role_id });
    if(data.success) {
       if(selectedRole.value?.role_id === role_id) selectedRole.value = null;
       showToast('删除成功');
       fetchRoles(); fetchUsers();
    } else {
       addLog(`[WARN] ${data.message}`);
       alert(data.message);
    }
  } catch {}
};

const setDefaultRole = async (role_id: string) => {
  try {
    await VrcApi.request(`${serverUrl.value}/api/admin/roles/set_default`, 'POST', { role_id });
    showToast('默认角色已更改');
    fetchRoles();
  } catch {}
};

const setUserRole = async (user_id: string, role_id: string | null) => {
  try {
    const data = await VrcApi.request(`${serverUrl.value}/api/admin/users/set_role`, 'POST', { user_id, role_id });
    if (data.success) {
      showToast('角色分配成功！');
    }
  } catch {}
};

// === Server Lifecycle ===
const startLocalServer = async () => {
  try {
    addLog('[INFO] 正在启动服务端...');
    await SysApi.startServer({ host: serverHost.value, port: serverPort.value });
    isRunning.value = true;
    setTimeout(startPolling, 500);
  } catch (err: any) { addLog('[ERROR] 启动失败: ' + (err.message || err)); }
};
const stopLocalServer = async () => {
  try {
    addLog('[INFO] 正在停止服务端...');
    await SysApi.stopServer();
    isRunning.value = false;
    stopPolling();
    addLog('[INFO] 服务端已成功停止');
  } catch (err: any) {
    addLog('[ERROR] 停止失败: ' + (err.message || err));
  }
};

// === Lifecycle ===
onMounted(async () => {
  addLog('[INFO] ' + t('role.dashboard_title') + ' 已加载。请配置后点击启动。');
  unlistenLog = await listen<string>('server_log', (e) => addLog(e.payload));
  unlistenClients = await listen<string>('clients_updated', () => { fetchClients(); fetchUsers(); });
  
  try {
    const running = await SysApi.isServerRunning();
    if (running) {
      isRunning.value = true;
      addLog('[INFO] 检测到服务端已经在后台运行中...');
      setTimeout(startPolling, 500);
    }
  } catch (err: any) {
    console.warn("检查服务端状态失败", err);
  }
});
onUnmounted(() => { unlistenLog?.(); unlistenClients?.(); stopPolling(); });

const openNewClient = async () => {
  try { addLog('[INFO] 正在启动新客户端窗口...'); await SysApi.openNewClient(); }
  catch (err: any) { addLog('[ERROR] 启动客户端失败: ' + (err.message || err)); }
};
const stopAndExit = () => { stopPolling(); emit('exit'); };
</script>
