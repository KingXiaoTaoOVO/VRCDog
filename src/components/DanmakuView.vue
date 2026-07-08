<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import {
  Activity,
  Bell,
  CheckCircle2,
  Eye,
  EyeOff,
  Gift,
  Glasses,
  Loader2,
  LogOut,
  MessageSquare,
  MonitorUp,
  Play,
  QrCode,
  Radio,
  Save,
  Send,
  Settings2,
  Square,
  Trash2,
  Wifi,
  WifiOff,
} from 'lucide-vue-next';
import { DanmakuApi, DbApi, OvrApi, type DanmakuConfig, type DanmakuMessage, type DanmakuStatus } from '../api';

const STORAGE_KEY = 'danmaku_config_v1';

const defaultConfig = (): DanmakuConfig => ({
  enable_bilibili: true,
  room_id: 0,
  bili_sessdata: '',
  enable_osc_input: true,
  osc_input_host: '127.0.0.1',
  osc_input_port: 9011,
  osc_input_address: '/vrcdog/danmaku',
  enable_osc_output: false,
  osc_output_host: '127.0.0.1',
  osc_output_port: 9000,
  osc_output_address: '/vrcdog/danmaku',
  enable_vrc_chatbox: false,
  vrc_chatbox_port: 9000,
  chatbox_interval_ms: 1600,
  enable_vr_overlay: true,
  overlay_visible: true,
  attach_mode: 'hmd',
  toggle_hand: 'left',
  x: -0.4,
  y: 0.1,
  z: -0.8,
  pitch: 0,
  yaw: 15,
  roll: 0,
  overlay_width_m: 0.4,
  overlay_alpha: 0.92,
  bg_alpha: 0.85,
  font_size: 14,
  text_color: '#FFFFFF',
  bg_color: '#10141F',
  max_messages: 50,
  show_danmaku: true,
  show_gift: true,
  show_enter: true,
  show_follow: true,
  show_guard: true,
  show_sc: true,
});

const config = ref<DanmakuConfig>(defaultConfig());
const status = ref<DanmakuStatus>({
  running: false,
  bili_connected: false,
  osc_input_running: false,
  vr_initialized: false,
  overlay_visible: true,
  room_id: 0,
  online: 0,
  message_count: 0,
  last_error: '',
  last_event: '',
});
const messages = ref<DanmakuMessage[]>([]);
const logs = ref<string[]>([]);
const loading = ref(false);
const saving = ref(false);
const saved = ref(false);
const error = ref('');
const previewInput = ref('VRDanmaku is integrated into VrcDog.');
const unlisteners: Array<() => void> = [];
const biliLoggedIn = ref(false);
const biliLoginChecking = ref(false);
const showQrModal = ref(false);
const qrCodeUrl = ref('');
const qrKey = ref('');
const qrStatusText = ref('');
const qrLoginLoading = ref(false);
let qrPollTimer: number | null = null;
let liveConfigTimer: number | null = null;
let restartTimer: number | null = null;

const finiteNumber = (value: unknown, fallback: number) => {
  const numberValue = Number(value);
  return Number.isFinite(numberValue) ? numberValue : fallback;
};

const integerInRange = (value: unknown, fallback: number, min: number, max: number) => {
  const numberValue = Math.trunc(finiteNumber(value, fallback));
  return Math.min(max, Math.max(min, numberValue));
};

const runtimeConfig = (): DanmakuConfig => {
  const base = { ...defaultConfig(), ...config.value };
  return {
    ...base,
    room_id: integerInRange(base.room_id, 0, 0, Number.MAX_SAFE_INTEGER),
    osc_input_port: integerInRange(base.osc_input_port, 9011, 1, 65535),
    osc_output_port: integerInRange(base.osc_output_port, 9000, 1, 65535),
    vrc_chatbox_port: integerInRange(base.vrc_chatbox_port, 9000, 1, 65535),
    chatbox_interval_ms: integerInRange(base.chatbox_interval_ms, 1600, 250, 60_000),
    x: finiteNumber(base.x, -0.4),
    y: finiteNumber(base.y, 0.1),
    z: finiteNumber(base.z, -0.8),
    pitch: finiteNumber(base.pitch, 0),
    yaw: finiteNumber(base.yaw, 15),
    roll: finiteNumber(base.roll, 0),
    overlay_width_m: finiteNumber(base.overlay_width_m, 0.4),
    overlay_alpha: Math.min(1, Math.max(0.05, finiteNumber(base.overlay_alpha, 0.92))),
    bg_alpha: Math.min(1, Math.max(0, finiteNumber(base.bg_alpha, 0.85))),
    font_size: finiteNumber(base.font_size, 14),
    max_messages: integerInRange(base.max_messages, 50, 10, 500),
  };
};

const clearRuntimeTimers = () => {
  if (liveConfigTimer !== null) {
    window.clearTimeout(liveConfigTimer);
    liveConfigTimer = null;
  }
  if (restartTimer !== null) {
    window.clearTimeout(restartTimer);
    restartTimer = null;
  }
};

const runningLabel = computed(() => {
  if (status.value.running && status.value.bili_connected) return '直播弹幕已连接';
  if (status.value.running && status.value.osc_input_running) return 'OSC 监听中';
  if (status.value.running) return '服务运行中';
  return '未启动';
});

const connectionPills = computed(() => [
  {
    label: 'Bilibili',
    active: status.value.bili_connected,
    detail: status.value.room_id ? `#${status.value.room_id}` : '未连接',
  },
  {
    label: 'OSC',
    active: status.value.osc_input_running,
    detail: `${config.value.osc_input_host}:${config.value.osc_input_port}`,
  },
  {
    label: 'SteamVR',
    active: status.value.vr_initialized,
    detail: status.value.overlay_visible ? '窗口显示' : '窗口隐藏',
  },
]);

const recentMessages = computed(() => [...messages.value].reverse().slice(0, 80));

const addLog = (line: string, level = 'info') => {
  const prefix = level === 'error' ? '[ERROR] ' : level === 'success' ? '[OK] ' : '';
  logs.value.unshift(`[${new Date().toLocaleTimeString()}] ${prefix}${line}`);
  logs.value = logs.value.slice(0, 60);
};

const formatTime = (timestamp: number) => {
  const date = new Date(timestamp);
  return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
};

const sleep = (ms: number) => new Promise((resolve) => window.setTimeout(resolve, ms));

const generateQrUrl = (url: string) => {
  return `https://api.qrserver.com/v1/create-qr-code/?size=240x240&data=${encodeURIComponent(url)}`;
};

const stopQrPolling = () => {
  if (qrPollTimer !== null) {
    window.clearInterval(qrPollTimer);
    qrPollTimer = null;
  }
};

const checkBiliLogin = async () => {
  const sessdata = (config.value.bili_sessdata || '').trim();
  biliLoggedIn.value = false;
  if (!sessdata) return;

  biliLoginChecking.value = true;
  try {
    biliLoggedIn.value = await invoke<boolean>('bili_check_login', { sessdata });
    if (biliLoggedIn.value) {
      await DbApi.saveSetting({ key: 'bili_sessdata', value: sessdata });
      await saveSettings();
      addLog('Bilibili 登录有效，已同步到直播弹幕配置', 'success');
    } else {
      addLog('Bilibili 登录已失效，请重新扫码登录');
    }
  } catch (e) {
    addLog(`Bilibili 登录状态检查失败: ${String(e)}`);
  } finally {
    biliLoginChecking.value = false;
  }
};

const openBiliLogin = async () => {
  stopQrPolling();
  showQrModal.value = true;
  qrCodeUrl.value = '';
  qrKey.value = '';
  qrStatusText.value = '正在生成登录二维码...';
  qrLoginLoading.value = true;
  addLog('正在生成 Bilibili 登录二维码...');

  try {
    const res: any = await invoke('bili_new_qr');
    if (res?.code === 0 && res.data?.url && res.data?.qrcode_key) {
      qrCodeUrl.value = res.data.qr_image_data_url || generateQrUrl(res.data.url);
      qrKey.value = res.data.qrcode_key;
      qrStatusText.value = '二维码已生成，请使用哔哩哔哩 APP 扫描';
      addLog('二维码已生成，请使用哔哩哔哩 APP 扫描');

      qrPollTimer = window.setInterval(async () => {
        try {
          const pollRes: any = await invoke('bili_get_qr_status', { qrKey: qrKey.value });
          const code = pollRes?.data?.code;
          if (code === 0) {
            stopQrPolling();
            const sessdata = pollRes.sessdata_extracted;
            if (!sessdata) {
              qrStatusText.value = '登录成功，但没有获取到 SESSDATA，请重试';
              qrLoginLoading.value = false;
              addLog('Bilibili 登录成功但未返回 SESSDATA', 'error');
              return;
            }

            config.value.bili_sessdata = sessdata;
            await DbApi.saveSetting({ key: 'bili_sessdata', value: sessdata });
            await saveSettings();
            biliLoggedIn.value = true;
            qrStatusText.value = '登录成功，已同步到直播弹幕配置';
            qrLoginLoading.value = false;
            addLog('Bilibili 扫码登录成功，已同步到直播弹幕配置');
            window.setTimeout(() => {
              showQrModal.value = false;
            }, 900);
          } else if (code === 86090) {
            qrStatusText.value = '已扫码，请在手机上确认登录';
          } else if (code === 86038) {
            stopQrPolling();
            qrStatusText.value = '二维码已过期，请重新扫码';
            qrLoginLoading.value = false;
            addLog('Bilibili 登录二维码已过期');
          } else if (code === 86101) {
            qrStatusText.value = '等待扫码，请使用哔哩哔哩 APP 扫描';
          }
        } catch (e) {
          stopQrPolling();
          qrStatusText.value = `登录轮询失败: ${String(e)}`;
          qrLoginLoading.value = false;
          addLog(qrStatusText.value);
        }
      }, 1600);
    } else {
      qrStatusText.value = res?.message || '二维码生成失败';
      qrLoginLoading.value = false;
      addLog(qrStatusText.value);
    }
  } catch (e) {
    qrStatusText.value = `二维码生成失败: ${String(e)}`;
    qrLoginLoading.value = false;
    addLog(qrStatusText.value);
  }
};

const closeBiliLogin = () => {
  stopQrPolling();
  showQrModal.value = false;
  qrLoginLoading.value = false;
};

const logoutBili = async () => {
  stopQrPolling();
  config.value.bili_sessdata = '';
  biliLoggedIn.value = false;
  await DbApi.saveSetting({ key: 'bili_sessdata', value: '' });
  await saveSettings();
  addLog('已退出 Bilibili 登录');
};

const badgeClass = (type: string) => {
  switch (type) {
    case 'sc': return 'bg-amber-100 text-amber-700 border-amber-200';
    case 'gift': return 'bg-orange-100 text-orange-700 border-orange-200';
    case 'enter': return 'bg-emerald-100 text-emerald-700 border-emerald-200';
    case 'follow': return 'bg-pink-100 text-pink-700 border-pink-200';
    case 'guard':
    case 'vip_enter': return 'bg-yellow-100 text-yellow-700 border-yellow-200';
    case 'warning': return 'bg-red-100 text-red-700 border-red-200';
    case 'osc': return 'bg-primary/10 text-primary border-primary/20';
    default: return 'bg-primary/10 text-primary border-primary/20';
  }
};

const messageLabel = (message: DanmakuMessage) => {
  if (message.message_type === 'sc') return `SC ${message.price ? `¥${message.price}` : ''}`;
  if (message.message_type === 'gift') return '礼物';
  if (message.message_type === 'enter') return '进入';
  if (message.message_type === 'follow') return '关注';
  if (message.message_type === 'guard' || message.message_type === 'vip_enter') return '舰长';
  if (message.message_type === 'warning') return '警告';
  if (message.source === 'osc') return 'OSC';
  return '弹幕';
};

const loadSettings = async () => {
  try {
    const stored = await DbApi.getSetting({ key: STORAGE_KEY });
    if (stored) {
      config.value = { ...defaultConfig(), ...JSON.parse(stored) };
    }
    const sessdata = await DbApi.getSetting({ key: 'bili_sessdata' });
    if (sessdata && !config.value.bili_sessdata) {
      config.value.bili_sessdata = sessdata;
    }
  } catch (e) {
    addLog(`读取配置失败: ${String(e)}`);
  }
};

const saveSettings = async () => {
  saving.value = true;
  saved.value = false;
  try {
    const nextConfig = runtimeConfig();
    await DbApi.saveSetting({ key: STORAGE_KEY, value: JSON.stringify(nextConfig) });
    await DanmakuApi.setConfig({ config: nextConfig });
    saved.value = true;
    setTimeout(() => { saved.value = false; }, 1800);
  } catch (e: any) {
    error.value = e.message || String(e);
  } finally {
    saving.value = false;
  }
};

const refreshSnapshot = async () => {
  try {
    const [nextStatus, nextMessages] = await Promise.all([
      DanmakuApi.getStatus(),
      DanmakuApi.getMessages(),
    ]);
    status.value = nextStatus;
    messages.value = nextMessages;
  } catch {
    // Browser preview or backend not ready.
  }
};

const start = async () => {
  loading.value = true;
  error.value = '';
  try {
    clearRuntimeTimers();
    await saveSettings();
    status.value = await DanmakuApi.start({ config: runtimeConfig() });
    if (config.value.enable_vr_overlay) {
      await sleep(700);
      const snapshot = await DanmakuApi.getStatus();
      status.value = snapshot;
      if (snapshot.last_error.includes('already initialized')) {
        addLog('SteamVR overlay 被 OVR 翻译后端占用，正在释放并重试弹幕窗口');
        await DanmakuApi.stop();
        await OvrApi.shutdown().catch(() => {});
        await sleep(500);
        status.value = await DanmakuApi.start({ config: runtimeConfig() });
      }
    }
    addLog('弹幕服务已启动');
  } catch (e: any) {
    error.value = e.message || String(e);
  } finally {
    loading.value = false;
  }
};

const stop = async () => {
  loading.value = true;
  error.value = '';
  try {
    clearRuntimeTimers();
    status.value = await DanmakuApi.stop();
    addLog('弹幕服务已停止');
  } catch (e: any) {
    error.value = e.message || String(e);
  } finally {
    loading.value = false;
  }
};

const toggleOverlay = async () => {
  try {
    const visible = !status.value.overlay_visible;
    status.value = await DanmakuApi.setOverlayVisible({ visible });
    config.value.overlay_visible = visible;
  } catch (e: any) {
    error.value = e.message || String(e);
  }
};

const clearMessages = async () => {
  try {
    await DanmakuApi.clearMessages();
    messages.value = [];
  } catch (e: any) {
    error.value = e.message || String(e);
  }
};

const sendTest = async (type = 'danmaku') => {
  try {
    const text = type === 'danmaku' ? previewInput.value.trim() : undefined;
    const message = await DanmakuApi.sendTest(text ? { messageType: type, text } : { messageType: type });
    if (message.source === 'browser') {
      messages.value.push(message);
    }
    addLog(`已发送测试${type === 'sc' ? ' SC' : type === 'gift' ? '礼物' : '弹幕'}，VR overlay 将同步刷新`, 'success');
  } catch (e: any) {
    error.value = e.message || String(e);
  }
};

const scheduleLiveConfigApply = () => {
  if (!status.value.running) return;
  if (liveConfigTimer !== null) {
    window.clearTimeout(liveConfigTimer);
  }
  liveConfigTimer = window.setTimeout(async () => {
    liveConfigTimer = null;
    try {
      status.value = await DanmakuApi.setConfig({ config: runtimeConfig() });
    } catch (e: any) {
      error.value = e.message || String(e);
    }
  }, 120);
};

const scheduleRunningRestart = () => {
  if (!status.value.running) return;
  if (restartTimer !== null) {
    window.clearTimeout(restartTimer);
  }
  restartTimer = window.setTimeout(async () => {
    restartTimer = null;
    if (!status.value.running) return;
    if (liveConfigTimer !== null) {
      window.clearTimeout(liveConfigTimer);
      liveConfigTimer = null;
    }
    try {
      loading.value = true;
      status.value = await DanmakuApi.start({ config: runtimeConfig() });
      addLog('已按新的房间号/输入源/VR 窗口配置重连直播姬', 'success');
    } catch (e: any) {
      error.value = e.message || String(e);
    } finally {
      loading.value = false;
    }
  }, 700);
};

watch(
  () => config.value,
  scheduleLiveConfigApply,
  { deep: true },
);

watch(
  () => [
    config.value.enable_bilibili,
    config.value.room_id,
    config.value.bili_sessdata,
    config.value.enable_osc_input,
    config.value.osc_input_host,
    config.value.osc_input_port,
    config.value.osc_input_address,
    config.value.enable_vr_overlay,
  ],
  scheduleRunningRestart,
);

onMounted(async () => {
  await loadSettings();
  await checkBiliLogin();
  await refreshSnapshot();
  try {
    unlisteners.push(await listen<DanmakuStatus>('danmaku_status', (event) => {
      status.value = event.payload;
    }));
    unlisteners.push(await listen<DanmakuMessage>('danmaku_message', (event) => {
      messages.value.push(event.payload);
      messages.value = messages.value.slice(-120);
    }));
    unlisteners.push(await listen<string>('danmaku_log', (event) => {
      addLog(event.payload);
    }));
    unlisteners.push(await listen<boolean>('danmaku_cleared', () => {
      messages.value = [];
    }));
  } catch {
    // Non-Tauri browser preview.
  }
});

onUnmounted(() => {
  stopQrPolling();
  clearRuntimeTimers();
  unlisteners.forEach((unlisten) => unlisten());
  unlisteners.length = 0;
});
</script>

<template>
  <div class="h-full min-h-0 flex flex-col bg-surface-hover rounded-3xl overflow-hidden">
    <header class="px-6 py-5 border-b border-border-soft bg-surface flex items-center justify-between gap-4">
      <div class="min-w-0">
        <div class="flex items-center gap-3">
          <span class="w-10 h-10 rounded-xl bg-primary/10 text-primary flex items-center justify-center">
            <Radio class="w-5 h-5" />
          </span>
          <div>
            <h1 class="text-2xl font-extrabold text-text leading-tight">
              直播弹幕
            </h1>
            <p class="text-sm text-text-muted font-medium">
              Bilibili 直播弹幕、第三方 OSC 推流与 SteamVR 弹幕窗口
            </p>
          </div>
        </div>
      </div>

      <div class="flex items-center gap-2">
        <button
          class="px-4 py-2 rounded-xl border border-border-soft bg-surface text-text-muted hover:text-primary font-bold text-sm flex items-center gap-2"
          :disabled="saving"
          @click="saveSettings"
        >
          <CheckCircle2 v-if="saved" class="w-4 h-4 text-emerald-500" />
          <Loader2 v-else-if="saving" class="w-4 h-4 animate-spin" />
          <Save v-else class="w-4 h-4" />
          保存
        </button>
        <button
          v-if="!status.running"
          class="px-5 py-2 rounded-xl bg-primary text-white font-bold text-sm flex items-center gap-2 shadow-md hover:brightness-110"
          :disabled="loading"
          @click="start"
        >
          <Loader2 v-if="loading" class="w-4 h-4 animate-spin" />
          <Play v-else class="w-4 h-4" />
          启动
        </button>
        <button
          v-else
          class="px-5 py-2 rounded-xl bg-red-500 text-white font-bold text-sm flex items-center gap-2 shadow-md hover:bg-red-600"
          :disabled="loading"
          @click="stop"
        >
          <Loader2 v-if="loading" class="w-4 h-4 animate-spin" />
          <Square v-else class="w-4 h-4" />
          停止
        </button>
      </div>
    </header>

    <div class="grid grid-cols-3 gap-3 px-6 py-4 bg-background/20 border-b border-border-soft">
      <div
        v-for="pill in connectionPills"
        :key="pill.label"
        class="rounded-2xl bg-surface border border-border-soft px-4 py-3 flex items-center justify-between"
      >
        <div>
          <p class="text-xs text-text-muted font-bold uppercase tracking-wide">
            {{ pill.label }}
          </p>
          <p class="text-sm text-text font-bold truncate">
            {{ pill.detail }}
          </p>
        </div>
        <Wifi v-if="pill.active" class="w-5 h-5 text-emerald-500" />
        <WifiOff v-else class="w-5 h-5 text-border-strong" />
      </div>
    </div>

    <div class="flex-1 min-h-0 grid grid-cols-[380px_1fr] gap-0">
      <aside class="min-h-0 overflow-y-auto border-r border-border-soft bg-surface p-5 space-y-5">
        <section class="space-y-3">
          <div class="flex items-center gap-2 text-text font-extrabold">
            <MessageSquare class="w-4 h-4 text-primary" />
            Bilibili 直播间
          </div>
          <label class="block">
            <span class="text-xs font-bold text-text-muted">房间号</span>
            <input
              v-model.number="config.room_id"
              type="number"
              min="0"
              class="mt-1 w-full rounded-xl bg-surface-hover border border-border-soft px-3 py-2 text-text font-bold outline-none focus:border-primary"
              placeholder="例如 22603245"
            >
          </label>
          <label class="flex items-center justify-between gap-3 rounded-xl border border-border-soft bg-surface-hover px-3 py-2">
            <span class="text-sm font-bold text-text-muted">启用 B 站弹幕源</span>
            <input v-model="config.enable_bilibili" type="checkbox" class="w-4 h-4">
          </label>
          <div class="rounded-xl border border-border-soft bg-surface-hover px-3 py-3 space-y-3">
            <div class="flex items-center justify-between gap-3">
              <div class="min-w-0 flex items-center gap-2">
                <span
                  class="w-2.5 h-2.5 rounded-full shrink-0"
                  :class="biliLoggedIn ? 'bg-emerald-500 shadow-sm shadow-emerald-500/30' : 'bg-border-strong'"
                />
                <div class="min-w-0">
                  <p class="text-sm font-extrabold text-text">
                    {{ biliLoggedIn ? 'Bilibili 已登录' : 'Bilibili 未登录' }}
                  </p>
                  <p class="text-[11px] text-text-muted truncate">
                    登录后直播间鉴权、舰长/进场等事件更完整
                  </p>
                </div>
              </div>
              <div class="flex items-center gap-2">
                <button
                  v-if="!biliLoggedIn"
                  class="px-3 py-2 rounded-lg bg-primary text-white text-xs font-bold flex items-center gap-1.5 shadow-sm hover:brightness-110 disabled:opacity-60"
                  :disabled="biliLoginChecking || qrLoginLoading"
                  @click="openBiliLogin"
                >
                  <Loader2 v-if="biliLoginChecking || qrLoginLoading" class="w-3.5 h-3.5 animate-spin" />
                  <QrCode v-else class="w-3.5 h-3.5" />
                  扫码登录
                </button>
                <button
                  v-else
                  class="px-3 py-2 rounded-lg bg-surface border border-border-soft text-text-muted hover:text-red-500 text-xs font-bold flex items-center gap-1.5"
                  @click="logoutBili"
                >
                  <LogOut class="w-3.5 h-3.5" />
                  退出
                </button>
              </div>
            </div>
            <label class="block">
              <span class="text-[11px] font-bold text-text-muted">SESSDATA（可手动粘贴，扫码成功会自动填入）</span>
              <div class="mt-1 flex gap-2">
                <input
                  v-model="config.bili_sessdata"
                  type="password"
                  class="min-w-0 flex-1 rounded-xl bg-surface border border-border-soft px-3 py-2 text-text outline-none focus:border-primary"
                  placeholder="匿名可用；登录后弹幕事件更完整"
                >
                <button
                  class="px-3 py-2 rounded-xl bg-surface border border-border-soft text-text-muted hover:text-primary font-bold text-xs"
                  @click="checkBiliLogin"
                >
                  检查
                </button>
              </div>
            </label>
          </div>
        </section>

        <section class="space-y-3">
          <div class="flex items-center gap-2 text-text font-extrabold">
            <MonitorUp class="w-4 h-4 text-primary" />
            第三方 OSC 推流
          </div>
          <label class="flex items-center justify-between gap-3 rounded-xl border border-border-soft bg-surface-hover px-3 py-2">
            <span class="text-sm font-bold text-text-muted">接收外部 OSC 弹幕</span>
            <input v-model="config.enable_osc_input" type="checkbox" class="w-4 h-4">
          </label>
          <div class="grid grid-cols-2 gap-2">
            <label>
              <span class="text-xs font-bold text-text-muted">监听地址</span>
              <input v-model="config.osc_input_host" class="mt-1 w-full rounded-xl bg-surface-hover border border-border-soft px-3 py-2 text-text outline-none focus:border-primary">
            </label>
            <label>
              <span class="text-xs font-bold text-text-muted">监听端口</span>
              <input v-model.number="config.osc_input_port" type="number" class="mt-1 w-full rounded-xl bg-surface-hover border border-border-soft px-3 py-2 text-text outline-none focus:border-primary">
            </label>
          </div>
          <label>
            <span class="text-xs font-bold text-text-muted">OSC 地址</span>
            <input v-model="config.osc_input_address" class="mt-1 w-full rounded-xl bg-surface-hover border border-border-soft px-3 py-2 text-text outline-none focus:border-primary">
          </label>
          <label class="flex items-center justify-between gap-3 rounded-xl border border-border-soft bg-surface-hover px-3 py-2">
            <span class="text-sm font-bold text-text-muted">转发到 VRChat Chatbox</span>
            <input v-model="config.enable_vrc_chatbox" type="checkbox" class="w-4 h-4">
          </label>
          <label class="flex items-center justify-between gap-3 rounded-xl border border-border-soft bg-surface-hover px-3 py-2">
            <span class="text-sm font-bold text-text-muted">转发为 OSC 数据流</span>
            <input v-model="config.enable_osc_output" type="checkbox" class="w-4 h-4">
          </label>
          <div class="grid grid-cols-2 gap-2">
            <label>
              <span class="text-xs font-bold text-text-muted">转发地址</span>
              <input v-model="config.osc_output_host" class="mt-1 w-full rounded-xl bg-surface-hover border border-border-soft px-3 py-2 text-text outline-none focus:border-primary">
            </label>
            <label>
              <span class="text-xs font-bold text-text-muted">转发端口</span>
              <input v-model.number="config.osc_output_port" type="number" class="mt-1 w-full rounded-xl bg-surface-hover border border-border-soft px-3 py-2 text-text outline-none focus:border-primary">
            </label>
          </div>
          <label>
            <span class="text-xs font-bold text-text-muted">转发 OSC 地址</span>
            <input v-model="config.osc_output_address" class="mt-1 w-full rounded-xl bg-surface-hover border border-border-soft px-3 py-2 text-text outline-none focus:border-primary">
          </label>
          <div class="grid grid-cols-2 gap-2">
            <label>
              <span class="text-xs font-bold text-text-muted">Chatbox 端口</span>
              <input v-model.number="config.vrc_chatbox_port" type="number" class="mt-1 w-full rounded-xl bg-surface-hover border border-border-soft px-3 py-2 text-text outline-none focus:border-primary">
            </label>
            <label>
              <span class="text-xs font-bold text-text-muted">Chatbox 间隔(ms)</span>
              <input v-model.number="config.chatbox_interval_ms" type="number" step="100" class="mt-1 w-full rounded-xl bg-surface-hover border border-border-soft px-3 py-2 text-text outline-none focus:border-primary">
            </label>
          </div>
        </section>

        <section class="space-y-3">
          <div class="flex items-center gap-2 text-text font-extrabold">
            <Glasses class="w-4 h-4 text-primary" />
            SteamVR 弹幕窗口
          </div>
          <label class="flex items-center justify-between gap-3 rounded-xl border border-border-soft bg-surface-hover px-3 py-2">
            <span class="text-sm font-bold text-text-muted">启用 SteamVR overlay</span>
            <input v-model="config.enable_vr_overlay" type="checkbox" class="w-4 h-4">
          </label>
          <div class="grid grid-cols-2 gap-2">
            <label>
              <span class="text-xs font-bold text-text-muted">挂载位置</span>
              <select v-model="config.attach_mode" class="mt-1 w-full rounded-xl bg-surface-hover border border-border-soft px-3 py-2 text-text outline-none focus:border-primary">
                <option value="hmd">头显前方</option>
                <option value="left_hand">左手手腕</option>
                <option value="right_hand">右手手腕</option>
              </select>
            </label>
            <label>
              <span class="text-xs font-bold text-text-muted">Grip 切换手</span>
              <select v-model="config.toggle_hand" class="mt-1 w-full rounded-xl bg-surface-hover border border-border-soft px-3 py-2 text-text outline-none focus:border-primary">
                <option value="left">左手</option>
                <option value="right">右手</option>
                <option value="always_on">常开</option>
              </select>
            </label>
          </div>
          <div class="grid grid-cols-3 gap-2">
            <label>
              <span class="text-xs font-bold text-text-muted uppercase">x</span>
              <input v-model.number="config.x" type="number" step="0.01" class="mt-1 w-full rounded-xl bg-surface-hover border border-border-soft px-3 py-2 text-text outline-none focus:border-primary">
            </label>
            <label>
              <span class="text-xs font-bold text-text-muted uppercase">y</span>
              <input v-model.number="config.y" type="number" step="0.01" class="mt-1 w-full rounded-xl bg-surface-hover border border-border-soft px-3 py-2 text-text outline-none focus:border-primary">
            </label>
            <label>
              <span class="text-xs font-bold text-text-muted uppercase">z</span>
              <input v-model.number="config.z" type="number" step="0.01" class="mt-1 w-full rounded-xl bg-surface-hover border border-border-soft px-3 py-2 text-text outline-none focus:border-primary">
            </label>
          </div>
          <div class="grid grid-cols-3 gap-2">
            <label>
              <span class="text-xs font-bold text-text-muted">Pitch</span>
              <input v-model.number="config.pitch" type="number" step="1" class="mt-1 w-full rounded-xl bg-surface-hover border border-border-soft px-3 py-2 text-text outline-none focus:border-primary">
            </label>
            <label>
              <span class="text-xs font-bold text-text-muted">Yaw</span>
              <input v-model.number="config.yaw" type="number" step="1" class="mt-1 w-full rounded-xl bg-surface-hover border border-border-soft px-3 py-2 text-text outline-none focus:border-primary">
            </label>
            <label>
              <span class="text-xs font-bold text-text-muted">Roll</span>
              <input v-model.number="config.roll" type="number" step="1" class="mt-1 w-full rounded-xl bg-surface-hover border border-border-soft px-3 py-2 text-text outline-none focus:border-primary">
            </label>
          </div>
          <div class="grid grid-cols-2 gap-2">
            <label>
              <span class="text-xs font-bold text-text-muted">宽度(m)</span>
              <input v-model.number="config.overlay_width_m" type="number" step="0.01" class="mt-1 w-full rounded-xl bg-surface-hover border border-border-soft px-3 py-2 text-text outline-none focus:border-primary">
            </label>
            <label>
              <span class="text-xs font-bold text-text-muted">字号</span>
              <input v-model.number="config.font_size" type="number" class="mt-1 w-full rounded-xl bg-surface-hover border border-border-soft px-3 py-2 text-text outline-none focus:border-primary">
            </label>
          </div>
          <div class="grid grid-cols-2 gap-2">
            <label>
              <span class="text-xs font-bold text-text-muted">窗口透明度</span>
              <input v-model.number="config.overlay_alpha" type="number" step="0.01" min="0.05" max="1" class="mt-1 w-full rounded-xl bg-surface-hover border border-border-soft px-3 py-2 text-text outline-none focus:border-primary">
            </label>
            <label>
              <span class="text-xs font-bold text-text-muted">背景透明度</span>
              <input v-model.number="config.bg_alpha" type="number" step="0.01" min="0" max="1" class="mt-1 w-full rounded-xl bg-surface-hover border border-border-soft px-3 py-2 text-text outline-none focus:border-primary">
            </label>
          </div>
          <div class="grid grid-cols-3 gap-2">
            <label>
              <span class="text-xs font-bold text-text-muted">文字色</span>
              <input v-model="config.text_color" type="color" class="mt-1 w-full h-10 rounded-xl bg-surface-hover border border-border-soft px-2 py-1">
            </label>
            <label>
              <span class="text-xs font-bold text-text-muted">背景色</span>
              <input v-model="config.bg_color" type="color" class="mt-1 w-full h-10 rounded-xl bg-surface-hover border border-border-soft px-2 py-1">
            </label>
            <label>
              <span class="text-xs font-bold text-text-muted">保留条数</span>
              <input v-model.number="config.max_messages" type="number" min="10" class="mt-1 w-full rounded-xl bg-surface-hover border border-border-soft px-3 py-2 text-text outline-none focus:border-primary">
            </label>
          </div>
          <div class="flex gap-2">
            <button class="flex-1 px-3 py-2 rounded-xl bg-surface-hover border border-border-soft text-text-muted hover:text-primary font-bold text-sm flex items-center justify-center gap-2" @click="toggleOverlay">
              <EyeOff v-if="status.overlay_visible" class="w-4 h-4" />
              <Eye v-else class="w-4 h-4" />
              {{ status.overlay_visible ? '隐藏窗口' : '显示窗口' }}
            </button>
          </div>
        </section>

        <section class="space-y-3">
          <div class="flex items-center gap-2 text-text font-extrabold">
            <Settings2 class="w-4 h-4 text-primary" />
            显示过滤
          </div>
          <div class="grid grid-cols-2 gap-2">
            <label class="rounded-xl border border-border-soft bg-surface-hover px-3 py-2 flex items-center justify-between">
              <span class="text-xs font-bold text-text-muted">普通弹幕</span>
              <input v-model="config.show_danmaku" type="checkbox">
            </label>
            <label class="rounded-xl border border-border-soft bg-surface-hover px-3 py-2 flex items-center justify-between">
              <span class="text-xs font-bold text-text-muted">SC</span>
              <input v-model="config.show_sc" type="checkbox">
            </label>
            <label class="rounded-xl border border-border-soft bg-surface-hover px-3 py-2 flex items-center justify-between">
              <span class="text-xs font-bold text-text-muted">礼物</span>
              <input v-model="config.show_gift" type="checkbox">
            </label>
            <label class="rounded-xl border border-border-soft bg-surface-hover px-3 py-2 flex items-center justify-between">
              <span class="text-xs font-bold text-text-muted">进入</span>
              <input v-model="config.show_enter" type="checkbox">
            </label>
            <label class="rounded-xl border border-border-soft bg-surface-hover px-3 py-2 flex items-center justify-between">
              <span class="text-xs font-bold text-text-muted">关注</span>
              <input v-model="config.show_follow" type="checkbox">
            </label>
            <label class="rounded-xl border border-border-soft bg-surface-hover px-3 py-2 flex items-center justify-between">
              <span class="text-xs font-bold text-text-muted">舰长</span>
              <input v-model="config.show_guard" type="checkbox">
            </label>
          </div>
        </section>
      </aside>

      <main class="min-h-0 flex flex-col">
        <div class="px-6 py-4 border-b border-border-soft bg-surface flex items-center justify-between gap-4">
          <div>
            <div class="flex items-center gap-2">
              <span
                class="w-2.5 h-2.5 rounded-full"
                :class="status.running ? 'bg-emerald-500 animate-pulse' : 'bg-border-strong'"
              />
              <span class="font-extrabold text-text">{{ runningLabel }}</span>
            </div>
            <p class="text-xs text-text-muted mt-1">
              消息 {{ status.message_count }} 条，观众 {{ status.online }}，最后事件 {{ status.last_event || 'none' }}
            </p>
          </div>
          <div class="flex flex-wrap items-center justify-end gap-2">
            <input
              v-model="previewInput"
              class="w-64 max-w-full rounded-xl bg-surface-hover border border-border-soft px-3 py-2 text-sm text-text outline-none focus:border-primary"
              placeholder="测试弹幕内容"
            >
            <button class="px-3 py-2 rounded-xl bg-surface-hover border border-border-soft text-text-muted hover:text-primary font-bold text-sm flex items-center gap-2" @click="sendTest('danmaku')">
              <Send class="w-4 h-4" />
              测试弹幕
            </button>
            <button class="px-3 py-2 rounded-xl bg-surface-hover border border-border-soft text-text-muted hover:text-amber-600 font-bold text-sm flex items-center gap-2" @click="sendTest('sc')">
              <Bell class="w-4 h-4" />
              测试 SC
            </button>
            <button class="px-3 py-2 rounded-xl bg-surface-hover border border-border-soft text-text-muted hover:text-orange-600 font-bold text-sm flex items-center gap-2" @click="sendTest('gift')">
              <Gift class="w-4 h-4" />
              测试礼物
            </button>
            <button class="p-2 rounded-xl bg-surface-hover border border-border-soft text-text-muted hover:text-red-500" @click="clearMessages">
              <Trash2 class="w-4 h-4" />
            </button>
          </div>
        </div>

        <div v-if="error || status.last_error" class="mx-6 mt-4 rounded-xl bg-red-50 border border-red-200 text-red-700 px-4 py-3 text-sm font-bold">
          {{ error || status.last_error }}
        </div>

        <div class="flex-1 min-h-0 grid grid-cols-[1fr_300px] gap-0">
          <section class="min-h-0 overflow-y-auto p-6 space-y-3">
            <div
              v-if="recentMessages.length === 0"
              class="h-full min-h-[360px] rounded-3xl border border-dashed border-border-soft bg-surface flex flex-col items-center justify-center text-text-muted"
            >
              <Activity class="w-10 h-10 mb-3 opacity-60" />
              <p class="font-bold">
                等待弹幕流入
              </p>
              <p class="text-xs mt-1">
                启动后可接收 Bilibili 或第三方 OSC 推流消息
              </p>
            </div>

            <div
              v-for="message in recentMessages"
              :key="message.id"
              class="rounded-2xl bg-surface border border-border-soft px-4 py-3 shadow-sm"
            >
              <div class="flex items-center gap-2 mb-2">
                <span class="px-2 py-0.5 rounded-lg border text-[11px] font-extrabold" :class="badgeClass(message.message_type)">
                  {{ messageLabel(message) }}
                </span>
                <span class="font-extrabold text-text truncate">{{ message.user }}</span>
                <span v-if="message.medal_name" class="text-[11px] text-amber-700 bg-amber-50 px-2 py-0.5 rounded">
                  {{ message.medal_name }} {{ message.medal_level }}
                </span>
                <span class="ml-auto text-[11px] text-text-muted font-mono">{{ formatTime(message.timestamp_ms) }}</span>
              </div>
              <p class="text-sm text-text-muted leading-relaxed break-words">
                {{ message.text }}
              </p>
            </div>
          </section>

          <aside class="min-h-0 overflow-y-auto border-l border-border-soft bg-surface p-4">
            <h2 class="font-extrabold text-text mb-3">
              运行日志
            </h2>
            <div class="space-y-2">
              <div
                v-for="line in logs"
                :key="line"
                class="rounded-xl bg-surface-hover border border-border-soft px-3 py-2 text-xs text-text-muted font-mono break-words"
              >
                {{ line }}
              </div>
              <div v-if="logs.length === 0" class="text-xs text-text-muted">
                暂无日志。
              </div>
            </div>
          </aside>
        </div>
      </main>
    </div>

    <div
      v-if="showQrModal"
      class="fixed inset-0 z-50 flex items-center justify-center bg-background/80 backdrop-blur-xl"
    >
      <div class="w-[360px] max-w-[calc(100vw-32px)] rounded-2xl bg-surface border border-border-soft shadow-2xl p-6 text-center">
        <div class="mx-auto mb-4 w-11 h-11 rounded-xl bg-primary/10 text-primary flex items-center justify-center border border-primary/20">
          <QrCode class="w-5 h-5" />
        </div>
        <h2 class="text-xl font-extrabold text-text">
          Bilibili 扫码登录
        </h2>
        <p class="mt-1 text-xs text-text-muted">
          使用哔哩哔哩 APP 扫描二维码，确认后会自动写入直播弹幕配置
        </p>

        <div class="mx-auto mt-5 w-56 h-56 rounded-2xl bg-white border border-border-soft p-3 flex items-center justify-center shadow-inner">
          <Loader2
            v-if="!qrCodeUrl"
            class="w-8 h-8 animate-spin text-primary"
          />
          <img
            v-else
            :src="qrCodeUrl"
            class="w-full h-full rounded-xl"
            alt="Bilibili login QR code"
          >
        </div>

        <div class="mt-5 rounded-xl bg-primary/10 border border-primary/20 px-3 py-2 text-sm font-bold text-primary flex items-center justify-center gap-2">
          <Loader2
            v-if="qrLoginLoading"
            class="w-4 h-4 animate-spin"
          />
          <span>{{ qrStatusText }}</span>
        </div>

        <div class="mt-5 flex gap-2">
          <button
            class="flex-1 px-4 py-2 rounded-xl bg-surface-hover border border-border-soft text-text-muted hover:text-text font-bold text-sm"
            @click="closeBiliLogin"
          >
            取消
          </button>
          <button
            class="flex-1 px-4 py-2 rounded-xl bg-primary text-white font-bold text-sm hover:brightness-110"
            @click="openBiliLogin"
          >
            重新生成
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
