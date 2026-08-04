<script setup lang="ts">
import { computed, onMounted, onUnmounted, reactive, ref, watch } from 'vue';
import { useStorage } from '@vueuse/core';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { DbApi, DanmakuApi, OvrApi, type DanmakuConfig, type DanmakuMessage, type DanmakuStatus } from '../api';

const STORAGE_KEY = 'danmaku_config_v2';
const liveThemeEnabled = useStorage('danmaku-live-theme', false);
const liveThemeLabel = computed(() => (liveThemeEnabled.value ? '直播姬风格' : '软件主题'));

const toggleLiveTheme = () => {
  liveThemeEnabled.value = !liveThemeEnabled.value;
};

const hmdDefault = {
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
};

const handDefault = {
  x: 0,
  y: 0.08,
  z: -0.1,
  pitch: -30,
  yaw: 0,
  roll: 0,
  overlay_width_m: 0.25,
  overlay_alpha: 0.95,
  bg_alpha: 0.9,
  font_size: 12,
};

const presets = {
  left: { x: -0.4, y: 0.1, z: -0.8, pitch: 0, yaw: 15, roll: 0 },
  center: { x: 0, y: 0, z: -0.8, pitch: 0, yaw: 0, roll: 0 },
  right: { x: 0.4, y: 0.1, z: -0.8, pitch: 0, yaw: -15, roll: 0 },
};

const defaultConfig = (): DanmakuConfig => ({
  enable_bilibili: true,
  room_id: 0,
  bili_sessdata: '',
  enable_osc_input: false,
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
  vr_menu_visible: false,
  attach_mode: 'hmd',
  toggle_hand: 'left',
  ...hmdDefault,
  text_color: '#FFFFFF',
  bg_color: '#121216',
  max_messages: 50,
  show_danmaku: true,
  show_gift: true,
  show_enter: true,
  show_follow: true,
  show_guard: true,
  show_sc: true,
  vr_input_text: '',
});

const config = ref<DanmakuConfig>(defaultConfig());
const modeMemory = reactive({
  hmd: { ...hmdDefault },
  hand: { ...handDefault },
});

const status = ref<DanmakuStatus>({
  running: false,
  bili_connected: false,
  osc_input_running: false,
  vr_initialized: false,
  overlay_visible: true,
  vr_menu_visible: false,
  room_id: 0,
  online: 0,
  message_count: 0,
  last_error: '',
  last_event: '',
  vr_input_text: '',
  vr_keyboard_open: false,
});

const logs = ref<string[]>([]);
const messages = ref<DanmakuMessage[]>([]);
const roomInput = ref('');
const loading = ref(false);
const saving = ref(false);
const saved = ref(false);
const error = ref('');
const biliLoggedIn = ref(false);
const qrModalOpen = ref(false);
const qrCodeUrl = ref('');
const qrKey = ref('');
const qrStatusText = ref('');
const qrLoading = ref(false);
const previewText = ref('这是一条测试弹幕，会同步到 VR 弹幕窗口。');
const unlisteners: Array<() => void> = [];
let liveConfigTimer: number | null = null;
let qrPollTimer: number | null = null;

const positionKeys = ['x', 'y', 'z', 'pitch', 'yaw', 'roll', 'overlay_width_m', 'overlay_alpha', 'bg_alpha', 'font_size'] as const;

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
  const normalizeOscAddress = (value: unknown) => {
    const text = String(value || '').trim();
    return !text || text === '/vrcdog/danmaku' ? '/vrcdog/danmaku' : text;
  };
  return {
    ...base,
    room_id: integerInRange(base.room_id, 0, 0, Number.MAX_SAFE_INTEGER),
    osc_input_port: integerInRange(base.osc_input_port, 9011, 1, 65535),
    osc_output_port: integerInRange(base.osc_output_port, 9000, 1, 65535),
    vrc_chatbox_port: integerInRange(base.vrc_chatbox_port, 9000, 1, 65535),
    osc_input_address: normalizeOscAddress(base.osc_input_address),
    osc_output_address: normalizeOscAddress(base.osc_output_address),
    chatbox_interval_ms: integerInRange(base.chatbox_interval_ms, 1600, 250, 60_000),
    x: finiteNumber(base.x, hmdDefault.x),
    y: finiteNumber(base.y, hmdDefault.y),
    z: finiteNumber(base.z, hmdDefault.z),
    pitch: finiteNumber(base.pitch, 0),
    yaw: finiteNumber(base.yaw, 15),
    roll: finiteNumber(base.roll, 0),
    overlay_width_m: Math.min(0.8, Math.max(0.15, finiteNumber(base.overlay_width_m, 0.4))),
    overlay_alpha: Math.min(1, Math.max(0.3, finiteNumber(base.overlay_alpha, 0.92))),
    bg_alpha: Math.min(1, Math.max(0, finiteNumber(base.bg_alpha, 0.85))),
    font_size: integerInRange(base.font_size, 14, 10, 20),
    max_messages: integerInRange(base.max_messages, 50, 10, 500),
  };
};

const connectedText = computed(() => {
  if (status.value.bili_connected) return status.value.online ? `在线 ${status.value.online}` : '已连接';
  if (status.value.running) return '连接中';
  return '未连接';
});

const isConnected = computed(() => status.value.running && (status.value.bili_connected || status.value.osc_input_running));
const statusClass = computed(() => (isConnected.value ? 'connected' : status.value.running ? 'connecting' : ''));
const activeMessages = computed(() => [...messages.value].reverse().slice(0, 120));

const addLog = (message: string, level: 'info' | 'success' | 'warning' | 'error' = 'info') => {
  const time = new Date().toLocaleTimeString('zh-CN', { hour12: false });
  const prefix = level === 'error' ? '[错误] ' : level === 'success' ? '[成功] ' : level === 'warning' ? '[警告] ' : '';
  logs.value.push(`${time} ${prefix}${message}`);
  logs.value = logs.value.slice(-500);
};

const clearLog = () => {
  logs.value = [];
};

const persistModePosition = () => {
  const mode = config.value.attach_mode === 'hand' || config.value.attach_mode === 'left_hand' ? 'hand' : 'hmd';
  const target = mode === 'hand' ? modeMemory.hand : modeMemory.hmd;
  for (const key of positionKeys) {
    (target as any)[key] = (config.value as any)[key];
  }
};

const applyPosition = (next: Partial<typeof hmdDefault>) => {
  Object.assign(config.value, next);
};

const setAttachMode = (mode: 'hmd' | 'hand') => {
  persistModePosition();
  config.value.attach_mode = mode;
  applyPosition(mode === 'hand' ? modeMemory.hand : modeMemory.hmd);
  addLog(`绑定模式切换为${mode === 'hand' ? '左手' : '头显'}`);
};

const setToggleHand = (hand: 'left' | 'right' | 'always_on') => {
  config.value.toggle_hand = hand;
  addLog(hand === 'always_on' ? '弹幕窗口已设置为常开' : `Grip 切换手柄改为${hand === 'left' ? '左手柄' : '右手柄'}`);
};

const applyPreset = (name: keyof typeof presets) => {
  if (config.value.attach_mode !== 'hmd') setAttachMode('hmd');
  applyPosition(presets[name]);
  addLog(`已应用预设位置：${name === 'left' ? '左前方' : name === 'center' ? '正前方' : '右前方'}`);
};

const resetPosition = () => {
  applyPosition(config.value.attach_mode === 'hand' ? handDefault : hmdDefault);
  addLog('已重置当前位置参数');
};

const saveSettings = async () => {
  saving.value = true;
  saved.value = false;
  try {
    persistModePosition();
    const nextConfig = runtimeConfig();
    await DbApi.saveSetting({ key: STORAGE_KEY, value: JSON.stringify({ config: nextConfig, modeMemory }) });
    await DbApi.saveSetting({ key: 'bili_sessdata', value: nextConfig.bili_sessdata || '' });
    status.value = await DanmakuApi.setConfig({ config: nextConfig });
    saved.value = true;
    window.setTimeout(() => { saved.value = false; }, 1500);
  } catch (e: any) {
    error.value = e.message || String(e);
  } finally {
    saving.value = false;
  }
};

const loadSettings = async () => {
  try {
    const stored = await DbApi.getSetting({ key: STORAGE_KEY });
    if (stored) {
      const parsed = JSON.parse(stored);
      if (parsed?.config) config.value = { ...defaultConfig(), ...parsed.config };
      else config.value = { ...defaultConfig(), ...parsed };
      if (parsed?.modeMemory?.hmd) Object.assign(modeMemory.hmd, parsed.modeMemory.hmd);
      if (parsed?.modeMemory?.hand) Object.assign(modeMemory.hand, parsed.modeMemory.hand);
    }
    const sessdata = await DbApi.getSetting({ key: 'bili_sessdata' });
    if (sessdata && !config.value.bili_sessdata) config.value.bili_sessdata = sessdata;
    roomInput.value = config.value.room_id ? String(config.value.room_id) : '';
  } catch (e) {
    addLog(`读取配置失败：${String(e)}`, 'error');
  }
};

const refreshSnapshot = async () => {
  try {
    const [nextStatus, nextMessages] = await Promise.all([DanmakuApi.getStatus(), DanmakuApi.getMessages()]);
    status.value = nextStatus;
    messages.value = nextMessages;
  } catch {
    // Browser preview or backend not ready.
  }
};

const start = async () => {
  const room = roomInput.value.trim();
  if (config.value.enable_bilibili && (!room || !/^\d+$/.test(room))) {
    error.value = '请输入有效的 Bilibili 直播间房间号。';
    addLog(error.value, 'error');
    return;
  }

  loading.value = true;
  error.value = '';
  try {
    config.value.room_id = room ? Number(room) : 0;
    await saveSettings();
    status.value = await DanmakuApi.start({ config: runtimeConfig() });
    addLog(`正在连接直播间 ${config.value.room_id || 'OSC 输入'}...`);
    void recoverOpenVrConflict();
  } catch (e: any) {
    error.value = e.message || String(e);
    addLog(error.value, 'error');
  } finally {
    loading.value = false;
  }
};

const stop = async () => {
  loading.value = true;
  error.value = '';
  try {
    status.value = await DanmakuApi.stop();
    addLog('已断开直播弹幕连接');
  } catch (e: any) {
    error.value = e.message || String(e);
  } finally {
    loading.value = false;
  }
};

const toggleConnect = () => {
  if (status.value.running) void stop();
  else void start();
};

const toggleOverlay = async () => {
  try {
    const visible = !config.value.overlay_visible;
    config.value.overlay_visible = visible;
    status.value = await DanmakuApi.setOverlayVisible({ visible });
    addLog(visible ? 'VR 弹幕窗口已显示' : 'VR 弹幕窗口已隐藏');
  } catch (e: any) {
    error.value = e.message || String(e);
  }
};

const toggleVrMenu = async () => {
  try {
    config.value.vr_menu_visible = !config.value.vr_menu_visible;
    status.value = await DanmakuApi.setConfig({ config: runtimeConfig() });
    addLog(config.value.vr_menu_visible ? 'VR 调整菜单已调出' : 'VR 调整菜单已隐藏');
  } catch (e: any) {
    error.value = e.message || String(e);
  }
};

const sendTest = async (messageType: string) => {
  try {
    const text = messageType === 'danmaku' ? previewText.value.trim() : undefined;
    const msg = await DanmakuApi.sendTest(text ? { messageType, text } : { messageType });
    if (msg.source === 'browser') messages.value.push(msg);
    addLog(`已发送测试${messageType === 'sc' ? ' SC' : messageType === 'gift' ? '礼物' : messageType === 'enter' ? '进入' : messageType === 'warning' ? '警告' : '弹幕'}`, 'success');
  } catch (e: any) {
    error.value = e.message || String(e);
  }
};

const clearMessages = async () => {
  await DanmakuApi.clearMessages().catch(() => {});
  messages.value = [];
  addLog('已清空弹幕列表');
};

const scheduleLiveConfigApply = () => {
  if (!status.value.running) return;
  if (liveConfigTimer !== null) window.clearTimeout(liveConfigTimer);
  liveConfigTimer = window.setTimeout(async () => {
    liveConfigTimer = null;
    try {
      persistModePosition();
      status.value = await DanmakuApi.setConfig({ config: runtimeConfig() });
    } catch (e: any) {
      error.value = e.message || String(e);
    }
  }, 80);
};

const checkBiliLogin = async () => {
  const sessdata = (config.value.bili_sessdata || '').trim();
  biliLoggedIn.value = false;
  if (!sessdata) return;
  try {
    biliLoggedIn.value = await invoke<boolean>('bili_check_login', { sessdata });
    addLog(biliLoggedIn.value ? 'Bilibili 登录凭证有效' : 'Bilibili 登录凭证无效或已过期', biliLoggedIn.value ? 'success' : 'warning');
  } catch (e) {
    addLog(`登录状态检查失败：${String(e)}`, 'warning');
  }
};

const generateQrUrl = (url: string) => `https://api.qrserver.com/v1/create-qr-code/?size=240x240&data=${encodeURIComponent(url)}`;

const stopQrPolling = () => {
  if (qrPollTimer !== null) window.clearInterval(qrPollTimer);
  qrPollTimer = null;
};

const openBiliLogin = async () => {
  stopQrPolling();
  qrModalOpen.value = true;
  qrLoading.value = true;
  qrCodeUrl.value = '';
  qrStatusText.value = '正在生成登录二维码...';
  try {
    const res: any = await invoke('bili_new_qr');
    if (res?.code !== 0 || !res.data?.url || !res.data?.qrcode_key) {
      throw new Error(res?.message || '二维码生成失败');
    }
    qrCodeUrl.value = res.data.qr_image_data_url || generateQrUrl(res.data.url);
    qrKey.value = res.data.qrcode_key;
    qrStatusText.value = '请使用哔哩哔哩 APP 扫码登录';
    addLog('Bilibili 登录二维码已生成');

    qrPollTimer = window.setInterval(async () => {
      const pollRes: any = await invoke('bili_get_qr_status', { qrKey: qrKey.value });
      const code = pollRes?.data?.code;
      if (code === 0) {
        stopQrPolling();
        const sessdata = pollRes.sessdata_extracted;
        if (!sessdata) throw new Error('登录成功但未获得 SESSDATA');
        config.value.bili_sessdata = sessdata;
        await saveSettings();
        biliLoggedIn.value = true;
        qrStatusText.value = '登录成功';
        qrLoading.value = false;
        addLog('Bilibili 扫码登录成功', 'success');
        window.setTimeout(() => { qrModalOpen.value = false; }, 900);
      } else if (code === 86090) {
        qrStatusText.value = '已扫码，请在手机上确认';
      } else if (code === 86038) {
        stopQrPolling();
        qrLoading.value = false;
        qrStatusText.value = '二维码已过期，请重新生成';
      } else {
        qrStatusText.value = '等待扫码...';
      }
    }, 1600);
  } catch (e) {
    qrLoading.value = false;
    qrStatusText.value = String(e);
    addLog(`扫码登录失败：${String(e)}`, 'error');
  }
};

const closeBiliLogin = () => {
  stopQrPolling();
  qrModalOpen.value = false;
  qrLoading.value = false;
};

const logoutBili = async () => {
  config.value.bili_sessdata = '';
  biliLoggedIn.value = false;
  await saveSettings();
  addLog('已退出 Bilibili 登录');
};

const eventLabel = (message: DanmakuMessage) => {
  if (message.message_type === 'sc') return message.price ? `SC ${message.price}` : 'SC';
  if (message.message_type === 'gift') return '礼物';
  if (message.message_type === 'enter') return '进入';
  if (message.message_type === 'follow') return '关注';
  if (message.message_type === 'guard' || message.message_type === 'vip_enter') return '舰长';
  if (message.message_type === 'warning') return '警告';
  return '弹幕';
};

const formatMessageTime = (message: DanmakuMessage) => new Date(message.timestamp_ms).toLocaleTimeString('zh-CN', {
  hour: '2-digit',
  minute: '2-digit',
  hour12: false,
});

const messageClass = (message: DanmakuMessage) => `type-${message.message_type.replace(/[^a-z0-9_-]/gi, '')}`;

const sleep = (ms: number) => new Promise((resolve) => window.setTimeout(resolve, ms));

const recoverOpenVrConflict = async () => {
  if (!config.value.enable_vr_overlay) return false;
  for (let i = 0; i < 6; i += 1) {
    await sleep(420);
    const latest = await DanmakuApi.getStatus();
    status.value = latest;
    if (!latest.last_error) return false;
    if (latest.last_error.toLowerCase().includes('already initialized')) {
      addLog('检测到 OVR 翻译器占用了 OpenVR，正在释放后重启 VR 弹幕。', 'warning');
      await DanmakuApi.stop().catch(() => {});
      await OvrApi.shutdown().catch(() => {});
      await sleep(450);
      status.value = await DanmakuApi.start({ config: runtimeConfig() });
      addLog('已重新启动 VR 弹幕 Overlay。', 'success');
      return true;
    }
  }
  return false;
};

watch(config, scheduleLiveConfigApply, { deep: true });

onMounted(async () => {
  await loadSettings();
  await checkBiliLogin();
  await refreshSnapshot();
  addLog('VrcDog界面已加载');

  try {
    unlisteners.push(await listen<DanmakuStatus>('danmaku_status', (event) => {
      status.value = event.payload;
    }));
    unlisteners.push(await listen<DanmakuMessage>('danmaku_message', (event) => {
      messages.value.push(event.payload);
      messages.value = messages.value.slice(-500);
    }));
    unlisteners.push(await listen<DanmakuConfig>('danmaku_config', (event) => {
      config.value = { ...config.value, ...event.payload };
      persistModePosition();
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
  if (liveConfigTimer !== null) window.clearTimeout(liveConfigTimer);
  unlisteners.forEach((unlisten) => unlisten());
});
</script>

<template>
  <div class="vrcdog-shell" :class="{ 'live-theme': liveThemeEnabled }">
    <aside class="vrcdog-sidebar">
      <header class="vrcdog-header">
        <strong>VrcDog</strong>
        <span class="status-badge">
          <i class="status-dot" :class="statusClass" />
          {{ connectedText }}
        </span>
      </header>

      <div class="sidebar-scroll">
        <section class="section">
          <div class="section-title">直播间</div>
          <div class="input-row">
            <input
              v-model="roomInput"
              :disabled="status.running"
              class="dark-input"
              placeholder="输入房间号"
              inputmode="numeric"
              @keydown.enter="toggleConnect"
            >
            <button class="primary-btn" :disabled="loading" @click="toggleConnect">
              {{ status.running ? '断开' : '连接' }}
            </button>
          </div>
        </section>

        <section class="section">
          <div class="section-title">账号</div>
          <div class="account-row">
            <i class="status-dot" :class="{ connected: biliLoggedIn }" />
            <span>{{ biliLoggedIn ? '已登录' : '未登录' }}</span>
            <button v-if="!biliLoggedIn" class="small-btn" @click="openBiliLogin">扫码登录</button>
            <button v-else class="small-btn" @click="logoutBili">退出登录</button>
          </div>
        </section>

        <section class="section">
          <div class="section-title">绑定模式</div>
          <div class="toggle-group">
            <button :class="{ active: config.attach_mode === 'hmd' }" @click="setAttachMode('hmd')">头显</button>
            <button :class="{ active: config.attach_mode !== 'hmd' }" @click="setAttachMode('hand')">左手</button>
          </div>
        </section>

        <section class="section">
          <div class="section-title">显示切换</div>
          <div class="toggle-group three">
            <button :class="{ active: config.toggle_hand === 'left' }" @click="setToggleHand('left')">左手柄</button>
            <button :class="{ active: config.toggle_hand === 'right' }" @click="setToggleHand('right')">右手柄</button>
            <button :class="{ active: config.toggle_hand === 'always_on' }" @click="setToggleHand('always_on')">常开</button>
          </div>
        </section>

        <section v-if="config.attach_mode === 'hmd'" class="section">
          <div class="section-title">预设位置</div>
          <div class="btn-row">
            <button class="small-btn" @click="applyPreset('left')">左前方</button>
            <button class="small-btn" @click="applyPreset('center')">正前方</button>
            <button class="small-btn" @click="applyPreset('right')">右前方</button>
          </div>
        </section>

        <section class="section">
          <div class="section-title">{{ config.attach_mode === 'hmd' ? '位置' : '位置微调' }}</div>
          <div class="slider-row">
            <span>{{ config.attach_mode === 'hmd' ? '水平' : '左右' }}</span>
            <input v-model.number="config.x" type="range" :min="config.attach_mode === 'hmd' ? -1 : -0.1" :max="config.attach_mode === 'hmd' ? 1 : 0.1" :step="config.attach_mode === 'hmd' ? 0.02 : 0.01">
            <b>{{ config.x.toFixed(2) }}</b>
          </div>
          <div class="slider-row">
            <span>{{ config.attach_mode === 'hmd' ? '垂直' : '上下' }}</span>
            <input v-model.number="config.y" type="range" :min="config.attach_mode === 'hmd' ? -0.8 : 0" :max="config.attach_mode === 'hmd' ? 0.8 : 0.15" step="0.01">
            <b>{{ config.y.toFixed(2) }}</b>
          </div>
          <div class="slider-row">
            <span>{{ config.attach_mode === 'hmd' ? '距离' : '前后' }}</span>
            <input v-model.number="config.z" type="range" :min="config.attach_mode === 'hmd' ? -1.5 : -0.1" :max="config.attach_mode === 'hmd' ? -0.3 : 0.1" step="0.01">
            <b>{{ config.z.toFixed(2) }}</b>
          </div>
        </section>

        <section v-if="config.attach_mode === 'hmd'" class="section">
          <div class="section-title">角度</div>
          <div class="slider-row">
            <span>俯仰</span>
            <input v-model.number="config.pitch" type="range" min="-30" max="30" step="1">
            <b>{{ Math.round(config.pitch) }}°</b>
          </div>
          <div class="slider-row">
            <span>偏航</span>
            <input v-model.number="config.yaw" type="range" min="-30" max="30" step="1">
            <b>{{ Math.round(config.yaw) }}°</b>
          </div>
          <div class="slider-row">
            <span>翻滚</span>
            <input v-model.number="config.roll" type="range" min="-20" max="20" step="1">
            <b>{{ Math.round(config.roll) }}°</b>
          </div>
        </section>

        <section class="section">
          <div class="section-title">外观</div>
          <div class="slider-row">
            <span>大小</span>
            <input v-model.number="config.overlay_width_m" type="range" min="0.15" max="0.8" step="0.01">
            <b>{{ config.overlay_width_m.toFixed(2) }}</b>
          </div>
          <div class="slider-row">
            <span>透明</span>
            <input v-model.number="config.overlay_alpha" type="range" min="0.3" max="1" step="0.02">
            <b>{{ config.overlay_alpha.toFixed(2) }}</b>
          </div>
          <div class="slider-row">
            <span>背景</span>
            <input v-model.number="config.bg_alpha" type="range" min="0" max="1" step="0.05">
            <b>{{ config.bg_alpha.toFixed(2) }}</b>
          </div>
          <div class="slider-row">
            <span>字号</span>
            <input v-model.number="config.font_size" type="range" min="10" max="20" step="1">
            <b>{{ Math.round(config.font_size) }}</b>
          </div>
        </section>

        <section class="section">
          <div class="section-title">显示内容</div>
          <div class="checkbox-grid">
            <label><input v-model="config.show_danmaku" type="checkbox">弹幕</label>
            <label><input v-model="config.show_gift" type="checkbox">礼物</label>
            <label><input v-model="config.show_sc" type="checkbox">SC</label>
            <label><input v-model="config.show_guard" type="checkbox">舰长</label>
            <label><input v-model="config.show_follow" type="checkbox">关注</label>
            <label><input v-model="config.show_enter" type="checkbox">进入</label>
          </div>
        </section>

        <section class="section">
          <div class="section-title">VR 调整</div>
          <div class="hint-box">
            <p>启动后会创建真实 SteamVR 弹幕 Overlay。</p>
            <p>在 VR 内按应用菜单键调出调整菜单；摇杆/触控板调位置，按住扳机调距离和大小，Grip 显示/隐藏弹幕窗口。</p>
          </div>
          <div class="btn-row">
            <button class="small-btn" @click="toggleOverlay">{{ config.overlay_visible ? '隐藏窗口' : '显示窗口' }}</button>
            <button class="small-btn" @click="toggleVrMenu">{{ config.vr_menu_visible ? '隐藏 VR 菜单' : '调出 VR 菜单' }}</button>
            <button class="small-btn" @click="resetPosition">重置位置</button>
            <button class="small-btn" @click="saveSettings">{{ saved ? '已保存' : '保存配置' }}</button>
          </div>
        </section>

        <section class="section">
          <div class="section-title">测试</div>
          <input v-model="previewText" class="dark-input full" placeholder="测试弹幕内容">
          <div class="btn-row test-row">
            <button class="small-btn" @click="sendTest('danmaku')">弹幕</button>
            <button class="small-btn" @click="sendTest('sc')">SC</button>
            <button class="small-btn" @click="sendTest('gift')">礼物</button>
            <button class="small-btn" @click="sendTest('enter')">进入</button>
            <button class="small-btn" @click="sendTest('warning')">警告</button>
          </div>
        </section>
      </div>
    </aside>

    <main class="vrcdog-main">
      <header class="log-header live-header">
        <div>
          <strong>直播互动</strong>
          <span>桌面与 SteamVR 实时同步</span>
        </div>
        <div class="live-summary">
          <span>人气 {{ status.online }}</span>
          <span>消息 {{ status.message_count }}</span>
          <button class="small-btn" :title="`切换到${liveThemeEnabled ? '软件主题' : '直播姬风格'}`" @click="toggleLiveTheme">{{ liveThemeLabel }}</button>
          <button class="small-btn" @click="clearMessages">清空消息</button>
        </div>
      </header>

      <div v-if="error || status.last_error" class="error-banner">
        {{ error || status.last_error }}
      </div>

      <div class="live-toolbar" aria-label="消息分类">
        <span class="filter-pill active">全部</span>
        <span class="filter-pill">弹幕</span>
        <span class="filter-pill">礼物</span>
        <span class="filter-pill">SC</span>
        <span class="filter-pill">舰长</span>
      </div>

      <div class="live-feed">
        <div v-if="!activeMessages.length" class="live-empty">
          <strong>{{ status.running ? '正在等待真实直播消息' : '连接直播间后显示互动消息' }}</strong>
          <span>普通弹幕、礼物、SC、舰长、进入与关注会分层显示，并同步到 VR 面板。</span>
        </div>

        <article
          v-for="message in activeMessages"
          :key="message.id"
          class="message-card"
          :class="messageClass(message)"
        >
          <div class="message-avatar" aria-hidden="true">{{ message.user.trim().charAt(0) || '哔' }}</div>
          <div class="message-content">
            <div class="message-meta">
              <strong :title="message.user">{{ message.user || '系统' }}</strong>
              <span v-if="message.medal_name" class="medal-badge">
                {{ message.medal_name }} {{ message.medal_level || '' }}
              </span>
              <span class="event-badge">{{ eventLabel(message) }}</span>
              <time>{{ formatMessageTime(message) }}</time>
            </div>
            <p v-if="message.text" :title="message.text">{{ message.text }}</p>
            <p v-else class="message-placeholder">{{ eventLabel(message) }}消息</p>
            <div v-if="message.price || message.gift_count" class="message-value">
              <span v-if="message.price">¥{{ message.price.toFixed(2) }}</span>
              <span v-if="message.gift_count">× {{ message.gift_count }}</span>
            </div>
          </div>
        </article>
      </div>

      <details class="runtime-log">
        <summary>运行日志 <span>{{ logs.length }}</span></summary>
        <div class="runtime-log-actions">
          <button class="small-btn" @click="clearLog">清空日志</button>
        </div>
        <div class="runtime-log-body">
          <div v-for="line in logs" :key="line" class="log-entry">{{ line }}</div>
        </div>
      </details>
    </main>

    <div v-if="qrModalOpen" class="modal-mask">
      <div class="qr-modal">
        <h2>Bilibili 扫码登录</h2>
        <p>{{ qrStatusText }}</p>
        <div class="qr-box">
          <span v-if="!qrCodeUrl">生成中...</span>
          <img v-else :src="qrCodeUrl" alt="Bilibili login QR code">
        </div>
        <div class="btn-row">
          <button class="small-btn" @click="closeBiliLogin">关闭</button>
          <button class="primary-btn" :disabled="qrLoading" @click="openBiliLogin">重新生成</button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.vrcdog-shell {
  --dm-bg: var(--theme-bg-main, #f7f5ef);
  --dm-side: var(--theme-surface, #fffdf7);
  --dm-panel: var(--theme-surface-hover, #f1eadf);
  --dm-hover: color-mix(in srgb, var(--theme-primary, #d97706) 12%, var(--dm-panel));
  --dm-border: var(--theme-border-soft, rgba(74, 45, 15, 0.14));
  --dm-text: var(--theme-text, #2d2117);
  --dm-muted: var(--theme-text-soft, #76552d);
  --dm-dim: var(--theme-text-muted, #9a7b4f);
  --dm-accent: var(--theme-primary, #d97706);
  --dm-accent-hover: var(--theme-primary-hover, #b45309);
  --dm-success: var(--theme-success, #15803d);
  --dm-info: var(--theme-info, #1d4ed8);
  --dm-warning: var(--theme-warning, #a16207);
  --dm-danger: var(--theme-danger, #b91c1c);
  --dm-gold: var(--theme-gold, #a16207);
  height: 100%;
  min-height: 0;
  display: flex;
  overflow: hidden;
  background: var(--dm-bg);
  color: var(--dm-text);
  border: 1px solid var(--dm-border);
  border-radius: 24px;
  box-shadow: 0 24px 64px rgba(0, 0, 0, 0.34);
  backdrop-filter: blur(18px);
  font-size: 13px;
}

.vrcdog-shell.live-theme {
  --dm-bg: #0f1015;
  --dm-side: #171820;
  --dm-panel: #20212b;
  --dm-hover: #2b2633;
  --dm-border: rgba(255, 255, 255, 0.09);
  --dm-text: #f7f7fa;
  --dm-muted: #b8bac5;
  --dm-dim: #7d808d;
  --dm-accent: #fb7299;
  --dm-accent-hover: #ff85ad;
  --dm-success: #32d583;
  --dm-info: #93c5fd;
  --dm-warning: #facc15;
  --dm-danger: #fca5a5;
  --dm-gold: #ffd17a;
}

.vrcdog-sidebar {
  width: 320px;
  min-width: 320px;
  display: flex;
  flex-direction: column;
  background: var(--dm-side);
  box-shadow: inset -1px 0 0 var(--dm-border);
}
.vrcdog-header,
.log-header {
  min-height: 56px;
  padding: 14px 16px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  box-shadow: inset 0 -1px 0 var(--dm-border);
}
.status-badge,
.account-row {
  display: flex;
  align-items: center;
  gap: 8px;
  color: var(--dm-muted);
  font-size: 12px;
}
.status-dot {
  width: 8px;
  height: 8px;
  border-radius: 999px;
  background: var(--dm-dim);
}
.status-dot.connected {
  background: var(--dm-success);
}
.status-dot.connecting {
  background: #f59e0b;
  animation: pulse 1s infinite;
}
@keyframes pulse {
  50% { opacity: 0.4; }
}
.sidebar-scroll,
.log-body {
  min-height: 0;
  overflow-y: auto;
}
.sidebar-scroll {
  padding: 12px;
}
.section {
  margin-bottom: 16px;
}
.section-title {
  margin-bottom: 8px;
  color: var(--dm-dim);
  font-size: 11px;
  letter-spacing: 0.5px;
}
.input-row,
.btn-row {
  display: flex;
  gap: 8px;
}
.dark-input {
  min-width: 0;
  flex: 1;
  padding: 8px 12px;
  border: 1px solid var(--dm-border);
  border-radius: 6px;
  background: var(--dm-panel);
  color: var(--dm-text);
  outline: none;
}
.dark-input.full {
  width: 100%;
  margin-bottom: 8px;
}
.dark-input:focus {
  border-color: var(--dm-accent);
}
.primary-btn,
.small-btn {
  border: 0;
  border-radius: 6px;
  cursor: pointer;
  font-weight: 700;
}
.primary-btn {
  padding: 8px 16px;
  color: white;
  background: var(--dm-accent);
  box-shadow: 0 8px 18px rgba(245, 158, 11, 0.18);
}
.primary-btn:hover {
  background: var(--dm-accent-hover);
}
.small-btn {
  padding: 6px 12px;
  color: var(--dm-muted);
  background: var(--dm-panel);
  box-shadow: inset 0 0 0 1px var(--dm-border);
}
.small-btn:hover {
  color: var(--dm-text);
  background: var(--dm-hover);
}
.account-row {
  padding: 8px 10px;
  border-radius: 6px;
  background: var(--dm-panel);
}
.account-row span {
  flex: 1;
}
.toggle-group {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 2px;
  padding: 3px;
  border-radius: 6px;
  background: var(--dm-panel);
}
.toggle-group.three {
  grid-template-columns: repeat(3, 1fr);
}
.toggle-group button {
  border: 0;
  border-radius: 4px;
  padding: 7px 8px;
  color: var(--dm-muted);
  background: transparent;
  cursor: pointer;
  font-weight: 700;
}
.toggle-group button.active {
  color: var(--dm-text);
  background: var(--dm-hover);
  box-shadow: inset 0 0 0 1px rgba(245, 158, 11, 0.18);
}
.slider-row {
  display: grid;
  grid-template-columns: 40px 1fr 48px;
  align-items: center;
  gap: 8px;
  margin-bottom: 8px;
}
.slider-row span {
  color: var(--dm-muted);
  font-size: 12px;
}
.slider-row b {
  text-align: right;
  font-family: Consolas, monospace;
  font-size: 12px;
  color: var(--dm-text);
}
.slider-row input[type="range"] {
  appearance: none;
  height: 4px;
  border-radius: 999px;
  background: color-mix(in srgb, var(--dm-text) 10%, transparent);
}
.slider-row input[type="range"]::-webkit-slider-thumb {
  appearance: none;
  width: 12px;
  height: 12px;
  border-radius: 999px;
  background: var(--dm-accent);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--dm-accent) 18%, transparent);
}
.checkbox-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 6px;
}
.checkbox-grid label {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 8px;
  border-radius: 4px;
  color: var(--dm-muted);
  background: var(--dm-panel);
}
.hint-box {
  padding: 10px;
  border-radius: 6px;
  color: var(--dm-muted);
  background: var(--dm-panel);
  line-height: 1.6;
}
.vrcdog-main {
  min-width: 0;
  flex: 1;
  display: flex;
  flex-direction: column;
  background: color-mix(in srgb, var(--dm-bg) 82%, var(--dm-panel));
}
.live-header > div:first-child {
  min-width: 0;
  display: grid;
  gap: 3px;
}
.live-header strong {
  font-size: 16px;
}
.live-header > div:first-child span {
  color: var(--dm-dim);
  font-size: 11px;
}
.live-summary,
.live-toolbar,
.message-meta,
.message-value,
.runtime-log-actions {
  display: flex;
  align-items: center;
}
.live-summary {
  gap: 12px;
  color: var(--dm-muted);
  font-size: 12px;
}
.live-toolbar {
  gap: 8px;
  padding: 12px 16px 0;
}
.filter-pill {
  padding: 6px 12px;
  border: 1px solid var(--dm-border);
  border-radius: 999px;
  color: var(--dm-muted);
  background: var(--dm-panel);
  font-size: 12px;
}
.filter-pill.active {
  color: var(--dm-accent);
  border-color: color-mix(in srgb, var(--dm-accent) 48%, var(--dm-border));
  background: color-mix(in srgb, var(--dm-accent) 13%, transparent);
}
.live-feed {
  min-height: 0;
  flex: 1;
  overflow-y: auto;
  padding: 12px 16px 18px;
}
.live-empty {
  min-height: 240px;
  display: grid;
  place-content: center;
  gap: 8px;
  padding: 30px;
  text-align: center;
  color: var(--dm-dim);
}
.live-empty strong {
  color: var(--dm-muted);
  font-size: 15px;
}
.message-card {
  min-width: 0;
  display: grid;
  grid-template-columns: 36px minmax(0, 1fr);
  gap: 10px;
  margin-bottom: 8px;
  padding: 11px 12px;
  overflow: hidden;
  border: 1px solid var(--dm-border);
  border-radius: 10px;
  background: var(--dm-panel);
  box-shadow: 0 8px 22px color-mix(in srgb, var(--dm-text) 9%, transparent);
}
.message-card.type-sc {
  border-color: color-mix(in srgb, var(--dm-gold) 52%, var(--dm-border));
  background: linear-gradient(90deg, color-mix(in srgb, var(--dm-gold) 14%, transparent), var(--dm-panel) 38%);
}
.message-card.type-gift {
  border-color: color-mix(in srgb, var(--dm-accent) 42%, var(--dm-border));
  background: linear-gradient(90deg, color-mix(in srgb, var(--dm-accent) 13%, transparent), var(--dm-panel) 38%);
}
.message-card.type-guard,
.message-card.type-vip_enter {
  border-color: color-mix(in srgb, var(--dm-warning) 48%, var(--dm-border));
}
.message-card.type-warning {
  border-color: color-mix(in srgb, var(--dm-danger) 48%, var(--dm-border));
  background: color-mix(in srgb, var(--dm-danger) 11%, var(--dm-panel));
}
.message-avatar {
  width: 36px;
  height: 36px;
  display: grid;
  place-items: center;
  border-radius: 50%;
  color: #fff;
  background: linear-gradient(135deg, var(--dm-accent), var(--dm-accent-hover));
  font-weight: 800;
}
.message-content,
.message-meta,
.message-meta strong,
.message-content p {
  min-width: 0;
}
.message-meta {
  gap: 7px;
  color: var(--dm-dim);
  font-size: 11px;
}
.message-meta strong {
  max-width: 180px;
  overflow: hidden;
  color: var(--dm-text);
  text-overflow: ellipsis;
  white-space: nowrap;
}
.message-meta time {
  margin-left: auto;
  white-space: nowrap;
}
.medal-badge,
.event-badge {
  padding: 2px 6px;
  border-radius: 4px;
  white-space: nowrap;
}
.medal-badge {
  color: var(--dm-info);
  background: color-mix(in srgb, var(--dm-info) 14%, transparent);
}
.event-badge {
  color: var(--dm-accent);
  background: color-mix(in srgb, var(--dm-accent) 13%, transparent);
}
.message-content p {
  margin: 5px 0 0;
  overflow-wrap: anywhere;
  color: var(--dm-text);
  line-height: 1.55;
}
.message-placeholder {
  color: var(--dm-muted) !important;
}
.message-value {
  gap: 12px;
  margin-top: 6px;
  color: var(--dm-gold);
  font-weight: 800;
}
.runtime-log {
  flex: none;
  max-height: 180px;
  overflow: hidden;
  border-top: 1px solid var(--dm-border);
  color: var(--dm-muted);
  background: var(--dm-side);
}
.runtime-log summary {
  padding: 9px 16px;
  cursor: pointer;
  user-select: none;
}
.runtime-log summary span {
  color: var(--dm-dim);
}
.runtime-log-actions {
  justify-content: flex-end;
  padding: 0 12px 6px;
}
.runtime-log-body {
  max-height: 118px;
  overflow-y: auto;
  padding: 0 16px 10px;
  font-family: Consolas, "SF Mono", monospace;
  font-size: 11px;
}
.log-entry {
  padding: 3px 0;
  color: var(--dm-muted);
  overflow-wrap: anywhere;
}
.error-banner {
  margin: 12px 16px 0;
  padding: 10px 12px;
  border: 1px solid color-mix(in srgb, var(--dm-danger) 38%, transparent);
  border-radius: 6px;
  background: color-mix(in srgb, var(--dm-danger) 10%, var(--dm-panel));
  color: var(--dm-danger);
}
.modal-mask {
  position: fixed;
  inset: 0;
  z-index: 60;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.72);
}
.qr-modal {
  width: 340px;
  padding: 20px;
  border-radius: 20px;
  background: var(--dm-side);
  box-shadow: 0 20px 60px rgba(74, 45, 15, 0.2), 0 0 0 1px var(--dm-border) inset;
}
.qr-modal h2 {
  margin-bottom: 6px;
  font-size: 18px;
}
.qr-modal p {
  color: var(--dm-muted);
}
.qr-box {
  width: 220px;
  height: 220px;
  margin: 18px auto;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 10px;
  background: white;
  color: #111;
}
.qr-box img {
  width: 100%;
  height: 100%;
  object-fit: contain;
}
::-webkit-scrollbar {
  width: 6px;
}
::-webkit-scrollbar-thumb {
  background: var(--dm-border);
  border-radius: 999px;
}
</style>
