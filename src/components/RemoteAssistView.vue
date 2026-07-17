<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed, nextTick } from 'vue';
import { useI18n } from 'vue-i18n';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { Monitor, Wifi, WifiOff, RefreshCw, Copy, Shield, Send, FileUp, MessageCircle, Settings2, Zap, Globe, Server, Phone, PhoneOff, Eye, EyeOff, X, Maximize2 } from 'lucide-vue-next';

const { t } = useI18n();

// ─── State ───────────────────────────────────────────────────────────────────

interface DeviceInfo {
  id: string;
  password: string;
  hostname: string;
  platform: string;
  nat_type: string;
  online: boolean;
}

interface ServerConfig {
  host: string;
  relay: string;
  api: string;
  key: string;
  is_official: boolean;
  label: string;
  server_type?: 'Official' | 'VrcDogBackend' | string;
  rendezvous_server?: string;
  relay_server?: string;
  api_server?: string;
}

interface ConnectionSession {
  session_id: string;
  peer_id: string;
  peer_name: string;
  started_at: string;
  conn_type: string;
  latency_ms: number;
  status: string;
  peer_hostname?: string;
  connected_at?: string;
  connection_type?: string;
}

interface ChatMessage {
  id: string;
  from: string;
  text: string;
  time: string;
  sender?: string;
  content?: string;
  timestamp?: string;
}

const deviceInfo = ref<DeviceInfo | null>(null);
const servers = ref<ServerConfig[]>([]);
const activeServer = ref<ServerConfig | null>(null);
const sessions = ref<ConnectionSession[]>([]);
const chatMessages = ref<ChatMessage[]>([]);

const serviceRunning = ref(false);
const acceptingConnections = ref(false);
const showPassword = ref(false);
const connecting = ref(false);
const remoteError = ref('');
let unlistenRemoteEvent: UnlistenFn | null = null;
let unlistenRemoteChat: UnlistenFn | null = null;
let unlistenRemoteFrame: UnlistenFn | null = null;
const viewerOpen = ref(false);
const viewerSessionId = ref('');
const viewerFrame = ref('');
const viewerWidth = ref(0);
const viewerHeight = ref(0);
const viewerLoading = ref(false);
const viewerSurface = ref<HTMLElement | null>(null);
let lastPointerSentAt = 0;

// Connection form
const peerIdInput = ref('');
const peerPasswordInput = ref('');

// Custom server form
const showCustomServer = ref(false);
const customServerHost = ref('');
const customServerLabel = ref('');

// Chat
const chatInput = ref('');
const activePanel = ref<'connect' | 'sessions' | 'chat' | 'files' | 'settings'>('connect');

// ─── Initialization ──────────────────────────────────────────────────────────

const errorText = (error: unknown) => {
  if (error instanceof Error) return error.message;
  return typeof error === 'string' ? error : JSON.stringify(error);
};

onMounted(async () => {
  try {
    deviceInfo.value = await invoke('remote_assist_init');
    
    // 获取服务器列表，传入当前连接的后台服务端地址
    const authStore = await import('../stores/authStore').then(m => m.useAuthStore());
    const backendUrl = authStore.clientServerUrl || '';
    servers.value = await invoke('remote_assist_get_servers', { backendUrl: backendUrl || null });
    
    // 默认选择第一个服务器（如果有 VrcDog 服务端则优先选它）
    if (servers.value.length > 0) {
      activeServer.value = servers.value[0];
      await invoke('remote_assist_set_server', { server: servers.value[0] });
    }

    const state = await invoke<{
      service_on: boolean;
      accepting: boolean;
      sessions: ConnectionSession[];
    }>('remote_assist_get_state');
    serviceRunning.value = Boolean(state.service_on);
    acceptingConnections.value = Boolean(state.accepting);
    sessions.value = Array.isArray(state.sessions) ? state.sessions : [];
    chatMessages.value = await invoke<ChatMessage[]>('remote_assist_get_chat');
  } catch (e) {
    console.error('Failed to init remote assist:', e);
    remoteError.value = errorText(e);
  }

  // Listen for status events
  unlistenRemoteEvent = await listen('remote_assist_event', async (event: any) => {
    const data = event.payload;
    if (data.event === 'service_started') {
      serviceRunning.value = true;
      acceptingConnections.value = true;
    } else if (data.event === 'service_stopped') {
      serviceRunning.value = false;
      acceptingConnections.value = false;
    } else if (data.event === 'connecting') {
      connecting.value = true;
    } else if (data.event === 'connection_update') {
      connecting.value = false;
      const idx = sessions.value.findIndex(s => s.session_id === data.session_id);
      if (idx >= 0) {
        sessions.value[idx].status = data.status;
        sessions.value[idx].conn_type = data.conn_type || data.connection_type || sessions.value[idx].conn_type;
      }
    } else if (data.event === 'connected') {
      connecting.value = false;
      serviceRunning.value = true;
      sessions.value = await invoke<ConnectionSession[]>('remote_assist_get_sessions');
    } else if (data.event === 'disconnected') {
      sessions.value = sessions.value.filter(s => s.session_id !== data.session_id);
      if (viewerSessionId.value === data.session_id) {
        viewerOpen.value = false;
        viewerSessionId.value = '';
        viewerFrame.value = '';
      }
    } else if (data.event === 'transport_error') {
      connecting.value = false;
      remoteError.value = data.message || t('remote_assist.connection_failed');
    }
  });

  unlistenRemoteChat = await listen('remote_assist_chat', (event: any) => {
    const data = event.payload;
    if (data.message) {
      chatMessages.value.push(data.message);
    }
  });

  unlistenRemoteFrame = await listen('remote_assist_frame', (event: any) => {
    const data = event.payload;
    if (data.session_id !== viewerSessionId.value || !data.data) return;
    viewerWidth.value = Number(data.width) || 0;
    viewerHeight.value = Number(data.height) || 0;
    viewerFrame.value = `data:image/jpeg;base64,${data.data}`;
    viewerLoading.value = false;
  });
});

onUnmounted(() => {
  if (viewerSessionId.value) {
    void invoke('remote_assist_stop_view', { sessionId: viewerSessionId.value });
  }
  unlistenRemoteEvent?.();
  unlistenRemoteChat?.();
  unlistenRemoteFrame?.();
});

// ─── Actions ─────────────────────────────────────────────────────────────────

const toggleService = async () => {
  remoteError.value = '';
  try {
    if (serviceRunning.value) {
      await invoke('remote_assist_stop_service');
    } else {
      await invoke('remote_assist_start_service');
    }
  } catch (e) {
    console.error('Failed to toggle service:', e);
    remoteError.value = errorText(e);
  }
};

const refreshPassword = async () => {
  try {
    const newPwd = await invoke<string>('remote_assist_refresh_password');
    if (deviceInfo.value) {
      deviceInfo.value.password = newPwd;
    }
  } catch (e) {
    console.error('Failed to refresh password:', e);
  }
};

const copyToClipboard = (text: string) => {
  navigator.clipboard.writeText(text);
};

const connectToPeer = async () => {
  if (!peerIdInput.value.trim()) return;
  connecting.value = true;
  remoteError.value = '';
  try {
    const session = await invoke<ConnectionSession>('remote_assist_connect', {
      peerId: peerIdInput.value.trim(),
      password: peerPasswordInput.value,
    });
    sessions.value.push(session);
    activePanel.value = 'sessions';
  } catch (e) {
    console.error('Connection failed:', e);
    remoteError.value = errorText(e);
    connecting.value = false;
  }
};

const toggleAcceptConnections = async () => {
  const nextValue = !acceptingConnections.value;
  remoteError.value = '';
  try {
    await invoke('remote_assist_toggle_accept', { accept: nextValue });
    acceptingConnections.value = nextValue;
  } catch (error) {
    remoteError.value = errorText(error);
  }
};

const disconnectSession = async (sessionId: string) => {
  try {
    await invoke('remote_assist_disconnect', { sessionId });
  } catch (e) {
    console.error('Disconnect failed:', e);
  }
};

const openViewer = async (sessionId: string) => {
  remoteError.value = '';
  viewerSessionId.value = sessionId;
  viewerFrame.value = '';
  viewerLoading.value = true;
  viewerOpen.value = true;
  try {
    await invoke('remote_assist_start_view', { sessionId });
    await nextTick();
    viewerSurface.value?.focus();
  } catch (error) {
    viewerOpen.value = false;
    viewerSessionId.value = '';
    viewerLoading.value = false;
    remoteError.value = errorText(error);
  }
};

const closeViewer = async () => {
  const sessionId = viewerSessionId.value;
  viewerOpen.value = false;
  viewerSessionId.value = '';
  viewerFrame.value = '';
  viewerLoading.value = false;
  if (sessionId) {
    try {
      await invoke('remote_assist_stop_view', { sessionId });
    } catch (error) {
      remoteError.value = errorText(error);
    }
  }
};

const sendViewerInput = (event: Record<string, unknown>) => {
  if (!viewerSessionId.value) return;
  void invoke('remote_assist_send_input', {
    sessionId: viewerSessionId.value,
    event,
  }).catch((error) => {
    remoteError.value = errorText(error);
  });
};

const remotePointerPosition = (event: MouseEvent) => {
  const element = event.currentTarget as HTMLElement;
  const rect = element.getBoundingClientRect();
  return {
    x: Math.max(0, Math.min(viewerWidth.value - 1, Math.round((event.clientX - rect.left) / rect.width * viewerWidth.value))),
    y: Math.max(0, Math.min(viewerHeight.value - 1, Math.round((event.clientY - rect.top) / rect.height * viewerHeight.value))),
  };
};

const onViewerMouseMove = (event: MouseEvent) => {
  const now = performance.now();
  if (now - lastPointerSentAt < 32 || !viewerWidth.value || !viewerHeight.value) return;
  lastPointerSentAt = now;
  sendViewerInput({ type: 'MouseMove', ...remotePointerPosition(event) });
};

const remoteMouseButton = (button: number) => button === 1 ? 2 : button === 2 ? 1 : 0;

const onViewerMouseDown = (event: MouseEvent) => {
  viewerSurface.value?.focus();
  sendViewerInput({ type: 'MouseDown', button: remoteMouseButton(event.button) });
};

const onViewerMouseUp = (event: MouseEvent) => {
  sendViewerInput({ type: 'MouseUp', button: remoteMouseButton(event.button) });
};

const onViewerWheel = (event: WheelEvent) => {
  event.preventDefault();
  sendViewerInput({ type: 'MouseWheel', delta: -Math.sign(event.deltaY) * 120 });
};

const onViewerKey = (event: KeyboardEvent, down: boolean) => {
  event.preventDefault();
  sendViewerInput({ type: down ? 'KeyDown' : 'KeyUp', code: event.keyCode });
};

const getServerHost = (server: ServerConfig | null) =>
  server?.host || server?.rendezvous_server || '';

const getSessionName = (session: ConnectionSession) =>
  session.peer_name || session.peer_hostname || session.peer_id;

const getSessionType = (session: ConnectionSession) =>
  session.conn_type || session.connection_type || 'relay';

const getMessageSender = (msg: ChatMessage) =>
  msg.from || msg.sender || 'remote';

const getMessageText = (msg: ChatMessage) =>
  msg.text || msg.content || '';

const getMessageTime = (msg: ChatMessage) =>
  msg.time || msg.timestamp || new Date().toISOString();

const selectServer = async (server: ServerConfig) => {
  activeServer.value = server;
  await invoke('remote_assist_set_server', { server });
};

const addCustomServer = async () => {
  if (!customServerHost.value.trim()) return;
  try {
    const server = await invoke<ServerConfig>('remote_assist_add_custom_server', {
      host: customServerHost.value.trim(),
      label: customServerLabel.value.trim() || '',
    });
    servers.value.push(server);
    activeServer.value = server;
    await invoke('remote_assist_set_server', { server });
    showCustomServer.value = false;
    customServerHost.value = '';
    customServerLabel.value = '';
  } catch (e) {
    console.error('Failed to add custom server:', e);
  }
};

const sendChat = async () => {
  if (!chatInput.value.trim() || sessions.value.length === 0) return;
  const activeSession = sessions.value.find(s => s.status === 'connected');
  if (!activeSession) return;
  
  try {
    const msg = await invoke<ChatMessage>('remote_assist_send_chat', {
      sessionId: activeSession.session_id,
      message: chatInput.value.trim(),
    });
    chatInput.value = '';
  } catch (e) {
    console.error('Failed to send chat:', e);
  }
};

const activeSessions = computed(() => sessions.value.filter(s => s.status !== 'disconnected'));
</script>

<template>
  <div class="h-full overflow-y-auto p-6 space-y-6">
    <!-- Header -->
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-2xl font-bold text-text-strong">
          {{ t('remote_assist.title') }}
        </h1>
        <p class="text-sm text-text-muted mt-1">
          {{ t('remote_assist.subtitle') }}
        </p>
      </div>
      <div class="flex items-center gap-2">
        <span
          class="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-full text-xs font-bold"
          :class="serviceRunning
            ? 'bg-emerald-500/15 text-emerald-600 border border-emerald-500/20'
            : 'bg-surface text-text-muted border border-border-soft'"
        >
          <span class="w-2 h-2 rounded-full" :class="serviceRunning ? 'bg-emerald-500 animate-pulse' : 'bg-text-muted'" />
          {{ serviceRunning ? t('remote_assist.service_online') : t('remote_assist.service_offline') }}
        </span>
      </div>
    </div>

    <div
      v-if="remoteError"
      class="rounded-xl border border-red-500/25 bg-red-500/10 px-4 py-3 text-sm font-bold text-red-500"
    >
      {{ remoteError }}
    </div>

    <!-- Tab Navigation -->
    <div class="flex gap-1 p-1 bg-surface/50 rounded-xl border border-border-soft">
      <button
        v-for="panel in ['connect', 'sessions', 'chat', 'files', 'settings'] as const"
        :key="panel"
        class="flex-1 flex items-center justify-center gap-1.5 px-3 py-2 rounded-lg text-xs font-bold transition-all"
        :class="activePanel === panel
          ? 'bg-primary text-white border border-primary shadow-sm shadow-primary/20'
          : 'text-text-muted hover:text-text hover:bg-surface-hover'"
        @click="activePanel = panel"
      >
        <Monitor v-if="panel === 'connect'" :size="14" />
        <Zap v-else-if="panel === 'sessions'" :size="14" />
        <MessageCircle v-else-if="panel === 'chat'" :size="14" />
        <FileUp v-else-if="panel === 'files'" :size="14" />
        <Settings2 v-else-if="panel === 'settings'" :size="14" />
        {{ t(`remote_assist.tab_${panel}`) }}
      </button>
    </div>

    <!-- Connect Panel -->
    <div v-if="activePanel === 'connect'" class="space-y-5">
      <!-- Device Info Card -->
      <div class="bg-surface/60 backdrop-blur-sm rounded-2xl border border-border-soft p-5 space-y-4">
        <div class="flex items-center gap-3">
          <div class="w-10 h-10 rounded-xl bg-primary/15 flex items-center justify-center">
            <Shield :size="20" class="text-primary" />
          </div>
          <div>
            <h3 class="font-bold text-text-strong text-sm">{{ t('remote_assist.your_device') }}</h3>
            <p class="text-xs text-text-muted">{{ deviceInfo?.hostname || '...' }}</p>
          </div>
          <div class="ml-auto">
            <button
              class="px-3 py-1.5 rounded-lg text-xs font-bold transition-all"
              :class="serviceRunning
                ? 'bg-red-500/15 text-red-500 hover:bg-red-500/25 border border-red-500/20'
                : 'bg-primary text-white hover:bg-primary-hover border border-primary shadow-sm shadow-primary/20'"
              @click="toggleService"
            >
              <component :is="serviceRunning ? PhoneOff : Phone" :size="12" class="inline mr-1" />
              {{ serviceRunning ? t('remote_assist.stop_service') : t('remote_assist.start_service') }}
            </button>
          </div>
        </div>

        <!-- ID & Password -->
        <div class="grid grid-cols-2 gap-4">
          <div class="space-y-1.5">
            <label class="text-[11px] font-bold text-text-muted uppercase tracking-wider">
              {{ t('remote_assist.your_id') }}
            </label>
            <div class="flex items-center gap-2">
              <div class="flex-1 bg-surface rounded-lg px-3 py-2 font-mono text-lg font-bold text-text-strong tracking-wider border border-border-soft">
                {{ deviceInfo?.id || '---' }}
              </div>
              <button
                class="p-2 rounded-lg bg-surface hover:bg-surface-hover border border-border-soft transition-colors"
                @click="copyToClipboard(deviceInfo?.id || '')"
                :title="t('remote_assist.copy_id')"
              >
                <Copy :size="14" class="text-text-muted" />
              </button>
            </div>
          </div>
          <div class="space-y-1.5">
            <label class="text-[11px] font-bold text-text-muted uppercase tracking-wider">
              {{ t('remote_assist.your_password') }}
            </label>
            <div class="flex items-center gap-2">
              <div class="flex-1 bg-surface rounded-lg px-3 py-2 font-mono text-lg font-bold text-text-strong tracking-wider border border-border-soft">
                {{ showPassword ? (deviceInfo?.password || '---') : '••••••••' }}
              </div>
              <button
                class="p-2 rounded-lg bg-surface hover:bg-surface-hover border border-border-soft transition-colors"
                @click="showPassword = !showPassword"
              >
                <component :is="showPassword ? EyeOff : Eye" :size="14" class="text-text-muted" />
              </button>
              <button
                class="p-2 rounded-lg bg-surface hover:bg-surface-hover border border-border-soft transition-colors"
                @click="refreshPassword"
                :title="t('remote_assist.refresh_password')"
              >
                <RefreshCw :size="14" class="text-text-muted" />
              </button>
            </div>
          </div>
        </div>

        <!-- NAT Type -->
        <div class="flex items-center gap-2 text-xs text-text-muted">
          <Globe :size="12" />
          <span>NAT: {{ deviceInfo?.nat_type || 'detecting...' }}</span>
          <span class="mx-2">•</span>
          <Server :size="12" />
          <span>{{ activeServer?.label || t('remote_assist.no_server') }}</span>
        </div>
      </div>

      <!-- Connect to Remote -->
      <div class="bg-surface/60 backdrop-blur-sm rounded-2xl border border-border-soft p-5 space-y-4">
        <div class="flex items-center gap-3">
          <div class="w-10 h-10 rounded-xl bg-primary/15 flex items-center justify-center">
            <Monitor :size="20" class="text-primary" />
          </div>
          <div>
            <h3 class="font-bold text-text-strong text-sm">{{ t('remote_assist.connect_remote') }}</h3>
            <p class="text-xs text-text-muted">{{ t('remote_assist.connect_desc') }}</p>
          </div>
        </div>

        <div class="space-y-3">
          <div>
            <label class="text-[11px] font-bold text-text-muted uppercase tracking-wider mb-1.5 block">
              {{ t('remote_assist.peer_id') }}
            </label>
            <input
              v-model="peerIdInput"
              type="text"
              :placeholder="t('remote_assist.peer_id_placeholder')"
              class="w-full bg-surface border border-border-soft rounded-lg px-3 py-2.5 text-sm text-text-strong placeholder:text-text-muted/50 focus:outline-none focus:border-primary/50 focus:ring-1 focus:ring-primary/20 transition-all"
              @keyup.enter="connectToPeer"
            />
          </div>
          <div>
            <label class="text-[11px] font-bold text-text-muted uppercase tracking-wider mb-1.5 block">
              {{ t('remote_assist.peer_password') }}
            </label>
            <input
              v-model="peerPasswordInput"
              type="password"
              :placeholder="t('remote_assist.peer_password_placeholder')"
              class="w-full bg-surface border border-border-soft rounded-lg px-3 py-2.5 text-sm text-text-strong placeholder:text-text-muted/50 focus:outline-none focus:border-primary/50 focus:ring-1 focus:ring-primary/20 transition-all"
              @keyup.enter="connectToPeer"
            />
          </div>
          <button
            class="w-full py-2.5 rounded-xl font-bold text-sm text-white bg-primary hover:bg-primary-hover transition-all disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
            :disabled="!peerIdInput.trim() || connecting"
            @click="connectToPeer"
          >
            <Wifi v-if="!connecting" :size="16" />
            <RefreshCw v-else :size="16" class="animate-spin" />
            {{ connecting ? t('remote_assist.connecting') : t('remote_assist.connect_btn') }}
          </button>
        </div>
      </div>
    </div>

    <!-- Sessions Panel -->
    <div v-else-if="activePanel === 'sessions'" class="space-y-4">
      <div v-if="activeSessions.length === 0" class="text-center py-12">
        <WifiOff :size="48" class="mx-auto text-text-muted/30 mb-4" />
        <p class="text-text-muted font-bold">{{ t('remote_assist.no_sessions') }}</p>
        <p class="text-xs text-text-muted/60 mt-1">{{ t('remote_assist.no_sessions_desc') }}</p>
      </div>
      <div
        v-for="session in activeSessions"
        :key="session.session_id"
        class="bg-surface/60 backdrop-blur-sm rounded-2xl border border-border-soft p-4"
      >
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-3">
            <div
              class="w-10 h-10 rounded-xl flex items-center justify-center"
              :class="session.status === 'connected' ? 'bg-emerald-500/15' : 'bg-amber-500/15'"
            >
              <Monitor :size="20" :class="session.status === 'connected' ? 'text-emerald-500' : 'text-amber-500'" />
            </div>
            <div>
              <p class="font-bold text-sm text-text-strong">{{ getSessionName(session) }}</p>
              <p class="text-xs text-text-muted">
                ID: {{ session.peer_id }} · {{ getSessionType(session).toUpperCase() }}
                <span v-if="session.latency_ms > 0"> • {{ session.latency_ms }}ms</span>
              </p>
            </div>
          </div>
          <div class="flex items-center gap-2">
            <span
              class="px-2 py-1 rounded-full text-[10px] font-bold"
              :class="{
                'bg-emerald-500/15 text-emerald-600': session.status === 'connected',
                'bg-amber-500/15 text-amber-600': session.status === 'connecting',
                'bg-red-500/15 text-red-500': session.status === 'disconnected',
              }"
            >
              {{ session.status }}
            </span>
            <button
              class="p-2 rounded-lg bg-primary/10 hover:bg-primary/20 text-primary transition-colors"
              :title="t('remote_assist.open_viewer')"
              @click="openViewer(session.session_id)"
            >
              <Maximize2 :size="14" />
            </button>
            <button
              class="p-2 rounded-lg bg-red-500/10 hover:bg-red-500/20 text-red-500 transition-colors"
              @click="disconnectSession(session.session_id)"
            >
              <PhoneOff :size="14" />
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- Chat Panel -->
    <div v-else-if="activePanel === 'chat'" class="space-y-4">
      <div class="bg-surface/60 backdrop-blur-sm rounded-2xl border border-border-soft p-4 h-[400px] flex flex-col">
        <div class="flex-1 overflow-y-auto space-y-3 mb-3">
          <div v-if="chatMessages.length === 0" class="text-center py-12">
            <MessageCircle :size="36" class="mx-auto text-text-muted/30 mb-3" />
            <p class="text-text-muted text-sm font-bold">{{ t('remote_assist.no_messages') }}</p>
          </div>
          <div
            v-for="msg in chatMessages"
            :key="msg.id"
            class="flex"
            :class="getMessageSender(msg) === 'local' ? 'justify-end' : 'justify-start'"
          >
            <div
              class="max-w-[70%] px-3 py-2 rounded-xl text-sm"
              :class="getMessageSender(msg) === 'local'
                ? 'bg-primary/15 text-text-strong border border-primary/20'
                : 'bg-surface border border-border-soft text-text'"
            >
              {{ getMessageText(msg) }}
              <p class="text-[10px] text-text-muted mt-1">
                {{ new Date(getMessageTime(msg)).toLocaleTimeString() }}
              </p>
            </div>
          </div>
        </div>
        <div class="flex gap-2">
          <input
            v-model="chatInput"
            type="text"
            :placeholder="t('remote_assist.chat_placeholder')"
            class="flex-1 bg-surface border border-border-soft rounded-lg px-3 py-2 text-sm text-text-strong placeholder:text-text-muted/50 focus:outline-none focus:border-primary/50 transition-all"
            @keyup.enter="sendChat"
          />
          <button
            class="px-4 py-2 rounded-lg bg-primary hover:bg-primary-hover text-white font-bold text-sm transition-all disabled:opacity-50"
            :disabled="!chatInput.trim()"
            @click="sendChat"
          >
            <Send :size="14" />
          </button>
        </div>
      </div>
    </div>

    <!-- Files Panel -->
    <div v-else-if="activePanel === 'files'" class="space-y-4">
      <div class="text-center py-12 bg-surface/60 backdrop-blur-sm rounded-2xl border border-border-soft">
        <FileUp :size="48" class="mx-auto text-text-muted/30 mb-4" />
        <p class="text-text-muted font-bold">{{ t('remote_assist.file_transfer') }}</p>
        <p class="text-xs text-text-muted/60 mt-1">{{ t('remote_assist.file_transfer_desc') }}</p>
        <button
          class="mt-4 px-4 py-2 rounded-lg bg-primary text-white font-bold text-sm hover:bg-primary-hover transition-all border border-primary shadow-sm shadow-primary/20 disabled:opacity-50 disabled:cursor-not-allowed"
          :disabled="activeSessions.length === 0"
        >
          <FileUp :size="14" class="inline mr-1" />
          {{ t('remote_assist.select_files') }}
        </button>
      </div>
    </div>

    <!-- Settings Panel -->
    <div v-else-if="activePanel === 'settings'" class="space-y-5">
      <!-- Server Selection -->
      <div class="bg-surface/60 backdrop-blur-sm rounded-2xl border border-border-soft p-5 space-y-4">
        <div class="flex items-center gap-3">
          <div class="w-10 h-10 rounded-xl bg-primary/15 flex items-center justify-center">
            <Server :size="20" class="text-primary" />
          </div>
          <div>
            <h3 class="font-bold text-text-strong text-sm">{{ t('remote_assist.server_config') }}</h3>
            <p class="text-xs text-text-muted">{{ t('remote_assist.server_config_desc') }}</p>
          </div>
        </div>

        <div class="space-y-2">
          <button
            v-for="server in servers"
            :key="getServerHost(server)"
            class="w-full flex items-center gap-3 px-3 py-2.5 rounded-xl border transition-all text-left"
            :class="getServerHost(activeServer) === getServerHost(server)
              ? 'bg-primary/10 border-primary/30 text-text-strong'
              : 'bg-surface border-border-soft text-text-muted hover:border-primary/20 hover:bg-surface-hover'"
            @click="selectServer(server)"
          >
            <div
              class="w-2 h-2 rounded-full"
              :class="getServerHost(activeServer) === getServerHost(server) ? 'bg-primary' : 'bg-text-muted/30'"
            />
            <div class="flex-1">
              <p class="text-sm font-bold">{{ server.label }}</p>
              <p class="text-[11px] text-text-muted">{{ getServerHost(server) }}</p>
            </div>
            <span
              v-if="server.is_official"
              class="px-2 py-0.5 rounded-full text-[10px] font-bold bg-primary/10 text-primary border border-primary/20"
            >
              {{ t('remote_assist.official_badge') }}
            </span>
            <span
              v-else-if="server.server_type === 'VrcDogBackend'"
              class="px-2 py-0.5 rounded-full text-[10px] font-bold bg-emerald-500/10 text-emerald-600 border border-emerald-500/20"
            >
              VrcDog
            </span>
          </button>
        </div>

        <!-- Add Custom Server -->
        <div class="pt-2 border-t border-border-soft">
          <button v-if="!showCustomServer"
            class="w-full py-2 rounded-lg text-xs font-bold text-primary bg-surface-hover hover:bg-primary hover:text-white transition-all border border-border-soft hover:border-primary"
            @click="showCustomServer = true"
          >
            + {{ t('remote_assist.add_custom_server') }}
          </button>
          <div v-else class="space-y-3 pt-2">
            <p class="text-[11px] text-text-muted">{{ t('remote_assist.custom_server_hint') }}</p>
            <input
              v-model="customServerHost"
              type="text"
              :placeholder="t('remote_assist.server_host_placeholder')"
              class="w-full bg-surface border border-border-soft rounded-lg px-3 py-2 text-sm text-text-strong placeholder:text-text-muted/50 focus:outline-none focus:border-primary/50 transition-all"
            />
            <input
              v-model="customServerLabel"
              type="text"
              :placeholder="t('remote_assist.server_label_placeholder')"
              class="w-full bg-surface border border-border-soft rounded-lg px-3 py-2 text-sm text-text-strong placeholder:text-text-muted/50 focus:outline-none focus:border-primary/50 transition-all"
            />
            <div class="flex gap-2">
              <button
                class="flex-1 py-2 rounded-lg bg-primary text-white font-bold text-xs hover:bg-primary-hover transition-all"
                @click="addCustomServer"
              >
                {{ t('remote_assist.save') }}
              </button>
              <button
                class="flex-1 py-2 rounded-lg bg-surface text-text-muted font-bold text-xs hover:bg-surface-hover transition-all border border-border-soft"
                @click="showCustomServer = false"
              >
                {{ t('remote_assist.cancel') }}
              </button>
            </div>
          </div>
        </div>
      </div>

      <!-- Connection Preferences -->
      <div class="bg-surface/60 backdrop-blur-sm rounded-2xl border border-border-soft p-5 space-y-4">
        <h3 class="font-bold text-text-strong text-sm">{{ t('remote_assist.preferences') }}</h3>
        <div class="space-y-3">
          <label class="flex items-center justify-between cursor-pointer">
            <span class="text-sm text-text">{{ t('remote_assist.accept_connections') }}</span>
            <button
              class="w-10 h-5 rounded-full transition-all relative"
              :class="acceptingConnections ? 'bg-primary' : 'bg-red-400/60'"
              @click="toggleAcceptConnections"
            >
              <span
                class="absolute top-0.5 w-4 h-4 rounded-full bg-white shadow transition-all"
                :class="acceptingConnections ? 'left-5' : 'left-0.5'"
              />
            </button>
          </label>
        </div>
      </div>
    </div>
  </div>

  <Teleport to="body">
    <div
      v-if="viewerOpen"
      class="fixed inset-0 z-[10000] bg-black/85 flex flex-col"
    >
      <div class="h-12 shrink-0 flex items-center gap-3 px-4 bg-surface border-b border-border-soft">
        <Monitor :size="16" class="text-primary" />
        <span class="text-sm font-bold text-text-strong">{{ t('remote_assist.remote_screen') }}</span>
        <span class="text-xs text-text-muted">{{ sessions.find(session => session.session_id === viewerSessionId)?.peer_name }}</span>
        <button
          class="ml-auto p-2 rounded-lg text-text-muted hover:text-text-strong hover:bg-surface-hover transition-colors"
          :title="t('remote_assist.close_viewer')"
          @click="closeViewer"
        >
          <X :size="18" />
        </button>
      </div>
      <div
        ref="viewerSurface"
        tabindex="0"
        class="flex-1 min-h-0 flex items-center justify-center bg-black outline-none overflow-hidden"
        @keydown="onViewerKey($event, true)"
        @keyup="onViewerKey($event, false)"
      >
        <div v-if="viewerLoading" class="flex items-center gap-2 text-white/70 text-sm font-bold">
          <RefreshCw :size="16" class="animate-spin" />
          {{ t('remote_assist.waiting_for_screen') }}
        </div>
        <img
          v-else-if="viewerFrame"
          :src="viewerFrame"
          class="max-w-full max-h-full object-contain select-none"
          draggable="false"
          @mousemove="onViewerMouseMove"
          @mousedown.prevent="onViewerMouseDown"
          @mouseup.prevent="onViewerMouseUp"
          @wheel="onViewerWheel"
          @contextmenu.prevent
        />
      </div>
    </div>
  </Teleport>
</template>
