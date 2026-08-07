<script setup lang="ts">
import { computed, onMounted, onUnmounted, reactive, ref, watch } from 'vue';
import { useStorage } from '@vueuse/core';
import { useI18n } from 'vue-i18n';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { DbApi, DanmakuApi, OvrApi, type DanmakuConfig, type DanmakuMessage, type DanmakuStatus } from '../api';
import {
  BilibiliLiveApi,
  type BiliLiveSession,
  type ContributionRankItem,
  type LiveArea,
  type LiveRoomInfo,
  type StreamEndpoint,
} from '../api/bilibiliLive';

const STORAGE_KEY = 'danmaku_config_v2';
const { locale } = useI18n();
const l = (zh: string, en: string) => locale.value.startsWith('zh') ? zh : en;
const liveThemeEnabled = useStorage('danmaku-live-theme', false);
const liveThemeLabel = computed(() => (liveThemeEnabled.value
  ? l('直播姬风格', 'Streamer theme')
  : l('软件主题', 'App theme')));

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
const previewText = ref(l('这是一条测试弹幕，会同步到 VR 弹幕窗口。', 'This is a test message synchronized to the VR overlay.'));
const biliSession = ref<BiliLiveSession>({ sessdata: '', bili_jct: '', buvid3: '' });
const liveRoom = ref<LiveRoomInfo | null>(null);
const liveAreas = ref<LiveArea[]>([]);
const liveTitleDraft = ref('');
const liveAnnouncementDraft = ref('');
const liveAreaDraft = ref(0);
const liveActionLoading = ref(false);
const liveManageError = ref('');
const streamEndpoints = ref<StreamEndpoint[]>([]);
const visibleStreamKeys = ref<Record<number, boolean>>({});
const contributionRank = ref<ContributionRankItem[]>([]);
const rankLoading = ref(false);
const outgoingDanmaku = ref('');
const sendDanmakuLoading = ref(false);
const activeFilter = ref<'all' | 'danmaku' | 'gift' | 'sc' | 'guard'>('all');
const copyFeedback = ref('');
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
  if (status.value.bili_connected) return status.value.online
    ? `${l('在线', 'Online')} ${status.value.online}`
    : l('已连接', 'Connected');
  if (status.value.running) return l('连接中', 'Connecting');
  return l('未连接', 'Disconnected');
});

const isConnected = computed(() => status.value.running && (status.value.bili_connected || status.value.osc_input_running));
const statusClass = computed(() => (isConnected.value ? 'connected' : status.value.running ? 'connecting' : ''));
const activeMessages = computed(() => [...messages.value]
  .reverse()
  .filter((message) => {
    if (activeFilter.value === 'all') return true;
    if (activeFilter.value === 'guard') return message.message_type === 'guard' || message.message_type === 'vip_enter';
    return message.message_type === activeFilter.value;
  })
  .slice(0, 120));
const groupedAreas = computed(() => {
  const groups = new Map<string, LiveArea[]>();
  for (const area of liveAreas.value) {
    const list = groups.get(area.parent_name) || [];
    list.push(area);
    groups.set(area.parent_name, list);
  }
  return Array.from(groups.entries());
});
const hasLiveSession = computed(() => Boolean(biliSession.value.sessdata && biliSession.value.bili_jct));

const addLog = (message: string, level: 'info' | 'success' | 'warning' | 'error' = 'info') => {
  const time = new Date().toLocaleTimeString('zh-CN', { hour12: false });
  const prefix = level === 'error'
    ? l('[错误] ', '[Error] ')
    : level === 'success'
      ? l('[成功] ', '[Success] ')
      : level === 'warning'
        ? l('[警告] ', '[Warning] ')
        : '';
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
  addLog(l(`绑定模式切换为${mode === 'hand' ? '左手' : '头显'}`, `Attachment mode changed to ${mode === 'hand' ? 'left hand' : 'headset'}`));
};

const setToggleHand = (hand: 'left' | 'right' | 'always_on') => {
  config.value.toggle_hand = hand;
  addLog(hand === 'always_on'
    ? l('弹幕窗口已设置为常开', 'The message overlay is now always visible')
    : l(`Grip 切换手柄改为${hand === 'left' ? '左手柄' : '右手柄'}`, `Grip toggle changed to the ${hand === 'left' ? 'left' : 'right'} controller`));
};

const applyPreset = (name: keyof typeof presets) => {
  if (config.value.attach_mode !== 'hmd') setAttachMode('hmd');
  applyPosition(presets[name]);
  addLog(l(
    `已应用预设位置：${name === 'left' ? '左前方' : name === 'center' ? '正前方' : '右前方'}`,
    `Applied position preset: ${name === 'left' ? 'front left' : name === 'center' ? 'front center' : 'front right'}`,
  ));
};

const resetPosition = () => {
  applyPosition(config.value.attach_mode === 'hand' ? handDefault : hmdDefault);
  addLog(l('已重置当前位置参数', 'Position settings reset'));
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
    const biliJct = await DbApi.getSetting({ key: 'bili_jct' }).catch(() => null);
    const buvid3 = await DbApi.getSetting({ key: 'bili_buvid3' }).catch(() => null);
    biliSession.value = {
      sessdata: config.value.bili_sessdata || sessdata || '',
      bili_jct: biliJct || '',
      buvid3: buvid3 || '',
    };
    roomInput.value = config.value.room_id ? String(config.value.room_id) : '';
  } catch (e) {
    addLog(l(`读取配置失败：${String(e)}`, `Could not load settings: ${String(e)}`), 'error');
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
    error.value = l('请输入有效的 Bilibili 直播间房间号。', 'Enter a valid Bilibili live room ID.');
    addLog(error.value, 'error');
    return;
  }

  loading.value = true;
  error.value = '';
  try {
    config.value.room_id = room ? Number(room) : 0;
    await saveSettings();
    status.value = await DanmakuApi.start({ config: runtimeConfig() });
    addLog(l(`正在连接直播间 ${config.value.room_id || 'OSC 输入'}...`, `Connecting to ${config.value.room_id ? `room ${config.value.room_id}` : 'OSC input'}...`));
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
    addLog(l('已断开直播弹幕连接', 'Live message connection closed'));
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
    addLog(visible ? l('VR 弹幕窗口已显示', 'VR message overlay shown') : l('VR 弹幕窗口已隐藏', 'VR message overlay hidden'));
  } catch (e: any) {
    error.value = e.message || String(e);
  }
};

const toggleVrMenu = async () => {
  try {
    config.value.vr_menu_visible = !config.value.vr_menu_visible;
    status.value = await DanmakuApi.setConfig({ config: runtimeConfig() });
    addLog(config.value.vr_menu_visible ? l('VR 调整菜单已调出', 'VR adjustment menu shown') : l('VR 调整菜单已隐藏', 'VR adjustment menu hidden'));
  } catch (e: any) {
    error.value = e.message || String(e);
  }
};

const sendTest = async (messageType: string) => {
  try {
    const text = messageType === 'danmaku' ? previewText.value.trim() : undefined;
    const msg = await DanmakuApi.sendTest(text ? { messageType, text } : { messageType });
    if (msg.source === 'browser') messages.value.push(msg);
    const label = messageType === 'sc' ? 'SC' : eventLabel({ message_type: messageType } as DanmakuMessage);
    addLog(l(`已发送测试${label}`, `Test ${label} sent`), 'success');
  } catch (e: any) {
    error.value = e.message || String(e);
  }
};

const clearMessages = async () => {
  await DanmakuApi.clearMessages().catch(() => {});
  messages.value = [];
  addLog(l('已清空弹幕列表', 'Message list cleared'));
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
    addLog(biliLoggedIn.value
      ? l('Bilibili 登录凭证有效', 'Bilibili credentials are valid')
      : l('Bilibili 登录凭证无效或已过期', 'Bilibili credentials are invalid or expired'), biliLoggedIn.value ? 'success' : 'warning');
  } catch (e) {
    addLog(l(`登录状态检查失败：${String(e)}`, `Could not check login status: ${String(e)}`), 'warning');
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
  qrStatusText.value = l('正在生成登录二维码...', 'Generating login QR code...');
  try {
    const res: any = await invoke('bili_new_qr');
    if (res?.code !== 0 || !res.data?.url || !res.data?.qrcode_key) {
      throw new Error(res?.message || l('二维码生成失败', 'Could not generate QR code'));
    }
    qrCodeUrl.value = res.data.qr_image_data_url || generateQrUrl(res.data.url);
    qrKey.value = res.data.qrcode_key;
    qrStatusText.value = l('请使用哔哩哔哩 APP 扫码登录', 'Scan with the Bilibili app to sign in');
    addLog(l('Bilibili 登录二维码已生成', 'Bilibili login QR code generated'));

    qrPollTimer = window.setInterval(async () => {
      const pollRes: any = await invoke('bili_get_qr_status', { qrKey: qrKey.value });
      const code = pollRes?.data?.code;
      if (code === 0) {
        stopQrPolling();
        const sessdata = pollRes.sessdata_extracted;
        if (!sessdata) throw new Error(l('登录成功但未获得 SESSDATA', 'Signed in, but no SESSDATA was returned'));
        config.value.bili_sessdata = sessdata;
        biliSession.value = {
          sessdata,
          bili_jct: pollRes.bili_jct_extracted || '',
          buvid3: pollRes.buvid3_extracted || '',
        };
        const savedSession = { ...biliSession.value };
        await Promise.all([
          saveSettings(),
          DbApi.saveSetting({ key: 'bili_jct', value: savedSession.bili_jct }),
          DbApi.saveSetting({ key: 'bili_buvid3', value: savedSession.buvid3 || '' }),
        ]);
        biliSession.value = savedSession;
        biliLoggedIn.value = true;
        await refreshLiveRoom();
        void refreshContributionRank();
        qrStatusText.value = l('登录成功', 'Signed in');
        qrLoading.value = false;
        addLog(l('Bilibili 扫码登录成功', 'Bilibili QR sign-in succeeded'), 'success');
        window.setTimeout(() => { qrModalOpen.value = false; }, 900);
      } else if (code === 86090) {
        qrStatusText.value = l('已扫码，请在手机上确认', 'QR code scanned. Confirm on your phone.');
      } else if (code === 86038) {
        stopQrPolling();
        qrLoading.value = false;
        qrStatusText.value = l('二维码已过期，请重新生成', 'The QR code expired. Generate a new one.');
      } else {
        qrStatusText.value = l('等待扫码...', 'Waiting for scan...');
      }
    }, 1600);
  } catch (e) {
    qrLoading.value = false;
    qrStatusText.value = String(e);
    addLog(l(`扫码登录失败：${String(e)}`, `QR sign-in failed: ${String(e)}`), 'error');
  }
};

const closeBiliLogin = () => {
  stopQrPolling();
  qrModalOpen.value = false;
  qrLoading.value = false;
};

const logoutBili = async () => {
  config.value.bili_sessdata = '';
  biliSession.value = { sessdata: '', bili_jct: '', buvid3: '' };
  biliLoggedIn.value = false;
  liveRoom.value = null;
  streamEndpoints.value = [];
  contributionRank.value = [];
  await Promise.all([
    saveSettings(),
    DbApi.saveSetting({ key: 'bili_jct', value: '' }),
    DbApi.saveSetting({ key: 'bili_buvid3', value: '' }),
  ]);
  addLog(l('已退出 Bilibili 登录', 'Signed out of Bilibili'));
};

const refreshLiveRoom = async () => {
  if (!hasLiveSession.value) return;
  liveActionLoading.value = true;
  liveManageError.value = '';
  try {
    const room = await BilibiliLiveApi.getOwnRoom(biliSession.value);
    liveRoom.value = room;
    liveTitleDraft.value = room.title;
    liveAnnouncementDraft.value = room.announcement;
    liveAreaDraft.value = room.area_id;
    if (!liveAreas.value.length) liveAreas.value = await BilibiliLiveApi.getAreas();
  } catch (e: any) {
    liveManageError.value = e.message || String(e);
  } finally {
    liveActionLoading.value = false;
  }
};

const saveLiveRoomInfo = async () => {
  if (!liveRoom.value || !hasLiveSession.value) return;
  liveActionLoading.value = true;
  liveManageError.value = '';
  try {
    if (liveTitleDraft.value.trim() !== liveRoom.value.title) {
      await BilibiliLiveApi.updateTitle(biliSession.value, liveRoom.value.room_id, liveTitleDraft.value.trim());
    }
    if (liveAreaDraft.value !== liveRoom.value.area_id) {
      await BilibiliLiveApi.updateArea(biliSession.value, liveRoom.value.room_id, liveAreaDraft.value);
    }
    if (liveAnnouncementDraft.value.trim() !== liveRoom.value.announcement) {
      await BilibiliLiveApi.updateAnnouncement(
        biliSession.value,
        liveRoom.value.room_id,
        liveRoom.value.uid,
        liveAnnouncementDraft.value.trim(),
      );
    }
    await refreshLiveRoom();
    addLog(l('直播间资料已更新', 'Live room details updated'), 'success');
  } catch (e: any) {
    liveManageError.value = e.message || String(e);
  } finally {
    liveActionLoading.value = false;
  }
};

const toggleLive = async () => {
  if (!liveRoom.value || !hasLiveSession.value || liveActionLoading.value) return;
  const stopping = liveRoom.value.live_status === 1;
  if (!window.confirm(stopping
    ? l('确认停止 Bilibili 直播？OBS 停止推流不会自动关闭直播间。', 'Stop the Bilibili stream? Stopping OBS does not close the live room automatically.')
    : l('确认开始 Bilibili 直播并获取推流地址？', 'Start the Bilibili stream and retrieve the streaming endpoint?'))) return;
  liveActionLoading.value = true;
  liveManageError.value = '';
  try {
    if (stopping) {
      await BilibiliLiveApi.stop(biliSession.value, liveRoom.value.room_id);
      streamEndpoints.value = [];
      addLog(l('Bilibili 直播已停止', 'Bilibili stream stopped'), 'success');
    } else {
      const result = await BilibiliLiveApi.start(biliSession.value, liveRoom.value.room_id, liveAreaDraft.value);
      if (result.requires_face_auth) {
        throw new Error(result.face_auth_url
          ? l(`开播前需要完成实名认证：${result.face_auth_url}`, `Identity verification is required before streaming: ${result.face_auth_url}`)
          : l('开播前需要完成实名认证', 'Identity verification is required before streaming'));
      }
      streamEndpoints.value = result.endpoints;
      addLog(l('Bilibili 直播已开始，推流信息已安全显示', 'Bilibili stream started; streaming details are now available'), 'success');
    }
    await refreshLiveRoom();
  } catch (e: any) {
    liveManageError.value = e.message || String(e);
  } finally {
    liveActionLoading.value = false;
  }
};

const copyStreamValue = async (value: string, label: string) => {
  await navigator.clipboard.writeText(value);
  copyFeedback.value = l(`${label}已复制`, `${label} copied`);
  window.setTimeout(() => { copyFeedback.value = ''; }, 1300);
};

const refreshContributionRank = async () => {
  if (!liveRoom.value || !hasLiveSession.value) return;
  rankLoading.value = true;
  try {
    contributionRank.value = await BilibiliLiveApi.getContributionRank(biliSession.value, liveRoom.value.room_id);
  } catch (e: any) {
    liveManageError.value = e.message || String(e);
  } finally {
    rankLoading.value = false;
  }
};

const sendLiveDanmaku = async () => {
  const message = outgoingDanmaku.value.trim();
  if (!liveRoom.value || !hasLiveSession.value || !message) return;
  sendDanmakuLoading.value = true;
  liveManageError.value = '';
  try {
    await BilibiliLiveApi.sendDanmaku(biliSession.value, liveRoom.value.room_id, message);
    outgoingDanmaku.value = '';
    addLog(l('弹幕发送成功', 'Message sent'), 'success');
  } catch (e: any) {
    liveManageError.value = e.message || String(e);
  } finally {
    sendDanmakuLoading.value = false;
  }
};

const eventLabel = (message: DanmakuMessage) => {
  if (message.message_type === 'sc') return message.price ? `SC ${message.price}` : 'SC';
  if (message.message_type === 'gift') return l('礼物', 'Gift');
  if (message.message_type === 'enter') return l('进入', 'Joined');
  if (message.message_type === 'follow') return l('关注', 'Follow');
  if (message.message_type === 'guard' || message.message_type === 'vip_enter') return l('舰长', 'Guard');
  if (message.message_type === 'warning') return l('警告', 'Warning');
  return l('弹幕', 'Danmaku');
};

const formatMessageTime = (message: DanmakuMessage) => new Date(message.timestamp_ms).toLocaleTimeString(locale.value.startsWith('zh') ? 'zh-CN' : 'en-US', {
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
      addLog(l('检测到 OVR 翻译器占用了 OpenVR，正在释放后重启 VR 弹幕。', 'The OVR translator is using OpenVR. Releasing it and restarting the VR message overlay.'), 'warning');
      await DanmakuApi.stop().catch(() => {});
      await OvrApi.shutdown().catch(() => {});
      await sleep(450);
      status.value = await DanmakuApi.start({ config: runtimeConfig() });
      addLog(l('已重新启动 VR 弹幕 Overlay。', 'VR message overlay restarted.'), 'success');
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
  if (biliLoggedIn.value && hasLiveSession.value) {
    await refreshLiveRoom();
    void refreshContributionRank();
  }
  addLog(l('VrcDog 界面已加载', 'VrcDog interface loaded'));

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
          <div class="section-title">{{ l('直播间', 'Live room') }}</div>
          <div class="input-row">
            <input
              v-model="roomInput"
              :disabled="status.running"
              class="dark-input"
              :placeholder="l('输入房间号', 'Enter room ID')"
              inputmode="numeric"
              @keydown.enter="toggleConnect"
            >
            <button class="primary-btn" :disabled="loading" @click="toggleConnect">
              {{ status.running ? l('断开', 'Disconnect') : l('连接', 'Connect') }}
            </button>
          </div>
        </section>

        <section class="section">
          <div class="section-title">{{ l('账号', 'Account') }}</div>
          <div class="account-row">
            <i class="status-dot" :class="{ connected: biliLoggedIn }" />
            <span>{{ biliLoggedIn ? (hasLiveSession ? l('已登录 · 可管理直播', 'Signed in · Stream controls enabled') : l('已登录 · 仅监控', 'Signed in · Monitoring only')) : l('未登录', 'Signed out') }}</span>
            <button v-if="!biliLoggedIn" class="small-btn" @click="openBiliLogin">{{ l('扫码登录', 'QR sign-in') }}</button>
            <button v-else class="small-btn" @click="logoutBili">{{ l('退出登录', 'Sign out') }}</button>
          </div>
          <div v-if="biliLoggedIn" class="btn-row account-actions">
            <button class="small-btn" :disabled="liveActionLoading" @click="refreshLiveRoom">{{ l('刷新直播间', 'Refresh live room') }}</button>
          </div>
        </section>

        <section v-if="liveRoom" class="section live-control-card">
          <div class="live-card-title">
            <div>
              <div class="section-title">{{ l('直播控制', 'Stream controls') }}</div>
              <strong>{{ liveRoom.live_status === 1 ? l('直播中', 'Live') : l('未开播', 'Offline') }}</strong>
            </div>
            <i class="status-dot" :class="{ connected: liveRoom.live_status === 1 }" />
          </div>
          <label class="live-field">
            <span>{{ l('直播标题', 'Stream title') }}</span>
            <input v-model="liveTitleDraft" class="dark-input full" maxlength="40">
          </label>
          <label class="live-field">
            <span>{{ l('直播分区', 'Category') }}</span>
            <select v-model.number="liveAreaDraft" class="dark-input full">
              <optgroup v-for="[parent, areas] in groupedAreas" :key="parent" :label="parent">
                <option v-for="area in areas" :key="area.id" :value="area.id">{{ area.name }}</option>
              </optgroup>
            </select>
          </label>
          <label class="live-field">
            <span>{{ l('主播公告', 'Announcement') }}</span>
            <textarea
              v-model="liveAnnouncementDraft"
              class="dark-input full live-announcement"
              maxlength="200"
              rows="3"
              :placeholder="l('留空可隐藏主播公告', 'Leave empty to hide the announcement')"
            />
            <small>{{ liveAnnouncementDraft.length }}/200</small>
          </label>
          <div class="btn-row live-actions">
            <button class="small-btn" :disabled="liveActionLoading" @click="saveLiveRoomInfo">{{ l('保存资料', 'Save details') }}</button>
            <button class="primary-btn" :class="{ stop: liveRoom.live_status === 1 }" :disabled="liveActionLoading" @click="toggleLive">
              {{ liveActionLoading ? l('处理中…', 'Processing...') : liveRoom.live_status === 1 ? l('停止直播', 'Stop stream') : l('开始直播', 'Start stream') }}
            </button>
          </div>
          <small>{{ l('人气', 'Audience') }} {{ liveRoom.online }} · {{ l('房间', 'Room') }} {{ liveRoom.room_id }}</small>
        </section>

        <section v-if="streamEndpoints.length" class="section stream-card">
          <div class="section-title">{{ l('OBS 推流信息', 'OBS streaming details') }}</div>
          <div v-for="(endpoint, index) in streamEndpoints" :key="`${endpoint.protocol}-${index}`" class="endpoint-row">
            <div class="endpoint-head">
              <strong>{{ endpoint.protocol }}</strong>
              <span v-if="copyFeedback">{{ copyFeedback }}</span>
            </div>
            <code>{{ endpoint.address }}</code>
            <div class="secret-row">
              <code>{{ visibleStreamKeys[index] ? endpoint.stream_key : '••••••••••••••••' }}</code>
              <button class="small-btn" @click="visibleStreamKeys[index] = !visibleStreamKeys[index]">{{ visibleStreamKeys[index] ? l('隐藏', 'Hide') : l('显示', 'Show') }}</button>
            </div>
            <div class="btn-row">
              <button class="small-btn" @click="copyStreamValue(endpoint.address, l('地址', 'Address'))">{{ l('复制地址', 'Copy address') }}</button>
              <button class="small-btn" @click="copyStreamValue(endpoint.stream_key, l('密钥', 'Key'))">{{ l('复制密钥', 'Copy key') }}</button>
            </div>
          </div>
        </section>

        <section class="section">
          <div class="section-title">{{ l('绑定模式', 'Attachment mode') }}</div>
          <div class="toggle-group">
            <button :class="{ active: config.attach_mode === 'hmd' }" @click="setAttachMode('hmd')">{{ l('头显', 'Headset') }}</button>
            <button :class="{ active: config.attach_mode !== 'hmd' }" @click="setAttachMode('hand')">{{ l('左手', 'Left hand') }}</button>
          </div>
        </section>

        <section class="section">
          <div class="section-title">{{ l('显示切换', 'Visibility toggle') }}</div>
          <div class="toggle-group three">
            <button :class="{ active: config.toggle_hand === 'left' }" @click="setToggleHand('left')">{{ l('左手柄', 'Left controller') }}</button>
            <button :class="{ active: config.toggle_hand === 'right' }" @click="setToggleHand('right')">{{ l('右手柄', 'Right controller') }}</button>
            <button :class="{ active: config.toggle_hand === 'always_on' }" @click="setToggleHand('always_on')">{{ l('常开', 'Always on') }}</button>
          </div>
        </section>

        <section v-if="config.attach_mode === 'hmd'" class="section">
          <div class="section-title">{{ l('预设位置', 'Position presets') }}</div>
          <div class="btn-row">
            <button class="small-btn" @click="applyPreset('left')">{{ l('左前方', 'Front left') }}</button>
            <button class="small-btn" @click="applyPreset('center')">{{ l('正前方', 'Front center') }}</button>
            <button class="small-btn" @click="applyPreset('right')">{{ l('右前方', 'Front right') }}</button>
          </div>
        </section>

        <section class="section">
          <div class="section-title">{{ config.attach_mode === 'hmd' ? l('位置', 'Position') : l('位置微调', 'Fine positioning') }}</div>
          <div class="slider-row">
            <span>{{ config.attach_mode === 'hmd' ? l('水平', 'Horizontal') : l('左右', 'Left / right') }}</span>
            <input v-model.number="config.x" type="range" :min="config.attach_mode === 'hmd' ? -1 : -0.1" :max="config.attach_mode === 'hmd' ? 1 : 0.1" :step="config.attach_mode === 'hmd' ? 0.02 : 0.01">
            <b>{{ config.x.toFixed(2) }}</b>
          </div>
          <div class="slider-row">
            <span>{{ config.attach_mode === 'hmd' ? l('垂直', 'Vertical') : l('上下', 'Up / down') }}</span>
            <input v-model.number="config.y" type="range" :min="config.attach_mode === 'hmd' ? -0.8 : 0" :max="config.attach_mode === 'hmd' ? 0.8 : 0.15" step="0.01">
            <b>{{ config.y.toFixed(2) }}</b>
          </div>
          <div class="slider-row">
            <span>{{ config.attach_mode === 'hmd' ? l('距离', 'Distance') : l('前后', 'Forward / back') }}</span>
            <input v-model.number="config.z" type="range" :min="config.attach_mode === 'hmd' ? -1.5 : -0.1" :max="config.attach_mode === 'hmd' ? -0.3 : 0.1" step="0.01">
            <b>{{ config.z.toFixed(2) }}</b>
          </div>
        </section>

        <section v-if="config.attach_mode === 'hmd'" class="section">
          <div class="section-title">{{ l('角度', 'Rotation') }}</div>
          <div class="slider-row">
            <span>{{ l('俯仰', 'Pitch') }}</span>
            <input v-model.number="config.pitch" type="range" min="-30" max="30" step="1">
            <b>{{ Math.round(config.pitch) }}°</b>
          </div>
          <div class="slider-row">
            <span>{{ l('偏航', 'Yaw') }}</span>
            <input v-model.number="config.yaw" type="range" min="-30" max="30" step="1">
            <b>{{ Math.round(config.yaw) }}°</b>
          </div>
          <div class="slider-row">
            <span>{{ l('翻滚', 'Roll') }}</span>
            <input v-model.number="config.roll" type="range" min="-20" max="20" step="1">
            <b>{{ Math.round(config.roll) }}°</b>
          </div>
        </section>

        <section class="section">
          <div class="section-title">{{ l('外观', 'Appearance') }}</div>
          <div class="slider-row">
            <span>{{ l('大小', 'Size') }}</span>
            <input v-model.number="config.overlay_width_m" type="range" min="0.15" max="0.8" step="0.01">
            <b>{{ config.overlay_width_m.toFixed(2) }}</b>
          </div>
          <div class="slider-row">
            <span>{{ l('透明', 'Opacity') }}</span>
            <input v-model.number="config.overlay_alpha" type="range" min="0.3" max="1" step="0.02">
            <b>{{ config.overlay_alpha.toFixed(2) }}</b>
          </div>
          <div class="slider-row">
            <span>{{ l('背景', 'Background') }}</span>
            <input v-model.number="config.bg_alpha" type="range" min="0" max="1" step="0.05">
            <b>{{ config.bg_alpha.toFixed(2) }}</b>
          </div>
          <div class="slider-row">
            <span>{{ l('字号', 'Font size') }}</span>
            <input v-model.number="config.font_size" type="range" min="10" max="20" step="1">
            <b>{{ Math.round(config.font_size) }}</b>
          </div>
        </section>

        <section class="section">
          <div class="section-title">{{ l('显示内容', 'Visible events') }}</div>
          <div class="checkbox-grid">
            <label><input v-model="config.show_danmaku" type="checkbox">{{ l('弹幕', 'Danmaku') }}</label>
            <label><input v-model="config.show_gift" type="checkbox">{{ l('礼物', 'Gifts') }}</label>
            <label><input v-model="config.show_sc" type="checkbox">SC</label>
            <label><input v-model="config.show_guard" type="checkbox">{{ l('舰长', 'Guards') }}</label>
            <label><input v-model="config.show_follow" type="checkbox">{{ l('关注', 'Follows') }}</label>
            <label><input v-model="config.show_enter" type="checkbox">{{ l('进入', 'Joins') }}</label>
          </div>
        </section>

        <section class="section">
          <div class="section-title">{{ l('VR 调整', 'VR adjustment') }}</div>
          <div class="hint-box">
            <p>{{ l('启动后会创建真实 SteamVR 弹幕 Overlay。', 'Starting creates a real SteamVR message overlay.') }}</p>
            <p>{{ l('在 VR 内按应用菜单键调出调整菜单；摇杆/触控板调位置，按住扳机调距离和大小，Grip 显示/隐藏弹幕窗口。', 'In VR, open the adjustment menu with the application menu button. Use the stick or touchpad to position it, hold the trigger for distance and size, and use Grip to show or hide the overlay.') }}</p>
          </div>
          <div class="btn-row">
            <button class="small-btn" @click="toggleOverlay">{{ config.overlay_visible ? l('隐藏窗口', 'Hide overlay') : l('显示窗口', 'Show overlay') }}</button>
            <button class="small-btn" @click="toggleVrMenu">{{ config.vr_menu_visible ? l('隐藏 VR 菜单', 'Hide VR menu') : l('调出 VR 菜单', 'Show VR menu') }}</button>
            <button class="small-btn" @click="resetPosition">{{ l('重置位置', 'Reset position') }}</button>
            <button class="small-btn" @click="saveSettings">{{ saved ? l('已保存', 'Saved') : l('保存配置', 'Save settings') }}</button>
          </div>
        </section>

        <section class="section">
          <div class="section-title">{{ l('测试', 'Test') }}</div>
          <input v-model="previewText" class="dark-input full" :placeholder="l('测试弹幕内容', 'Test message')">
          <div class="btn-row test-row">
            <button class="small-btn" @click="sendTest('danmaku')">{{ l('弹幕', 'Danmaku') }}</button>
            <button class="small-btn" @click="sendTest('sc')">SC</button>
            <button class="small-btn" @click="sendTest('gift')">{{ l('礼物', 'Gift') }}</button>
            <button class="small-btn" @click="sendTest('enter')">{{ l('进入', 'Join') }}</button>
            <button class="small-btn" @click="sendTest('warning')">{{ l('警告', 'Warning') }}</button>
          </div>
        </section>
      </div>
    </aside>

    <main class="vrcdog-main">
      <header class="log-header live-header">
        <div>
          <strong>{{ l('直播互动', 'Live Interaction') }}</strong>
          <span>{{ l('桌面与 SteamVR 实时同步', 'Real-time desktop and SteamVR sync') }}</span>
        </div>
        <div class="live-summary">
          <span>{{ l('人气', 'Audience') }} {{ status.online }}</span>
          <span>{{ l('消息', 'Messages') }} {{ status.message_count }}</span>
          <button v-if="liveRoom" class="small-btn" :disabled="rankLoading" @click="refreshContributionRank">{{ rankLoading ? l('榜单加载中', 'Loading ranking') : l('刷新榜单', 'Refresh ranking') }}</button>
          <button class="small-btn" :title="l(`切换到${liveThemeEnabled ? '软件主题' : '直播姬风格'}`, `Switch to ${liveThemeEnabled ? 'app theme' : 'streamer theme'}`)" @click="toggleLiveTheme">{{ liveThemeLabel }}</button>
          <button class="small-btn" @click="clearMessages">{{ l('清空消息', 'Clear messages') }}</button>
        </div>
      </header>

      <div v-if="error || status.last_error || liveManageError" class="error-banner">
        {{ error || status.last_error || liveManageError }}
      </div>

      <div class="live-toolbar" :aria-label="l('消息分类', 'Message categories')">
        <button class="filter-pill" :class="{ active: activeFilter === 'all' }" @click="activeFilter = 'all'">{{ l('全部', 'All') }}</button>
        <button class="filter-pill" :class="{ active: activeFilter === 'danmaku' }" @click="activeFilter = 'danmaku'">{{ l('弹幕', 'Danmaku') }}</button>
        <button class="filter-pill" :class="{ active: activeFilter === 'gift' }" @click="activeFilter = 'gift'">{{ l('礼物', 'Gifts') }}</button>
        <button class="filter-pill" :class="{ active: activeFilter === 'sc' }" @click="activeFilter = 'sc'">SC</button>
        <button class="filter-pill" :class="{ active: activeFilter === 'guard' }" @click="activeFilter = 'guard'">{{ l('舰长', 'Guards') }}</button>
      </div>

      <div v-if="liveRoom && hasLiveSession" class="send-bar">
        <input v-model="outgoingDanmaku" class="dark-input" maxlength="30" :placeholder="l('发送 Bilibili 弹幕（最多 30 字）', 'Send a Bilibili message (30 characters max)')" @keydown.enter="sendLiveDanmaku">
        <span>{{ outgoingDanmaku.length }}/30</span>
        <button class="primary-btn" :disabled="sendDanmakuLoading || !outgoingDanmaku.trim()" @click="sendLiveDanmaku">{{ l('发送', 'Send') }}</button>
      </div>

      <div v-if="contributionRank.length" class="rank-strip">
        <strong>{{ l('在线贡献榜', 'Top Contributors') }}</strong>
        <span v-for="item in contributionRank.slice(0, 5)" :key="item.uid">#{{ item.rank }} {{ item.name }} · {{ item.score }}</span>
      </div>

      <div class="live-feed">
        <div v-if="!activeMessages.length" class="live-empty">
          <strong>{{ status.running ? l('正在等待真实直播消息', 'Waiting for live messages') : l('连接直播间后显示互动消息', 'Connect to a live room to see interactions') }}</strong>
          <span>{{ l('普通弹幕、礼物、SC、舰长、进入与关注会分层显示，并同步到 VR 面板。', 'Messages, gifts, Super Chats, guards, joins, and follows are grouped and synced to the VR panel.') }}</span>
        </div>

        <article
          v-for="message in activeMessages"
          :key="message.id"
          class="message-card"
          :class="messageClass(message)"
        >
          <div class="message-avatar" aria-hidden="true">{{ message.user.trim().charAt(0) || 'B' }}</div>
          <div class="message-content">
            <div class="message-meta">
              <strong :title="message.user">{{ message.user || l('系统', 'System') }}</strong>
              <span v-if="message.medal_name" class="medal-badge">
                {{ message.medal_name }} {{ message.medal_level || '' }}
              </span>
              <span class="event-badge">{{ eventLabel(message) }}</span>
              <time>{{ formatMessageTime(message) }}</time>
            </div>
            <p v-if="message.text" :title="message.text">{{ message.text }}</p>
            <p v-else class="message-placeholder">{{ l(`${eventLabel(message)}消息`, `${eventLabel(message)} event`) }}</p>
            <div v-if="message.price || message.gift_count" class="message-value">
              <span v-if="message.price">¥{{ message.price.toFixed(2) }}</span>
              <span v-if="message.gift_count">× {{ message.gift_count }}</span>
            </div>
          </div>
        </article>
      </div>

      <details class="runtime-log">
        <summary>{{ l('运行日志', 'Runtime log') }} <span>{{ logs.length }}</span></summary>
        <div class="runtime-log-actions">
          <button class="small-btn" @click="clearLog">{{ l('清空日志', 'Clear log') }}</button>
        </div>
        <div class="runtime-log-body">
          <div v-for="line in logs" :key="line" class="log-entry">{{ line }}</div>
        </div>
      </details>
    </main>

    <div v-if="qrModalOpen" class="modal-mask">
      <div class="qr-modal">
        <h2>{{ l('Bilibili 扫码登录', 'Bilibili QR sign-in') }}</h2>
        <p>{{ qrStatusText }}</p>
        <div class="qr-box">
          <span v-if="!qrCodeUrl">{{ l('生成中...', 'Generating...') }}</span>
          <img v-else :src="qrCodeUrl" alt="Bilibili login QR code">
        </div>
        <div class="btn-row">
          <button class="small-btn" @click="closeBiliLogin">{{ l('关闭', 'Close') }}</button>
          <button class="primary-btn" :disabled="qrLoading" @click="openBiliLogin">{{ l('重新生成', 'Generate again') }}</button>
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
.live-announcement {
  box-sizing: border-box;
  min-height: 68px;
  resize: vertical;
  font: inherit;
  line-height: 1.45;
}
.live-field > small {
  margin-top: -5px;
  color: var(--dm-dim);
  text-align: right;
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
.account-actions {
  margin-top: 7px;
}
.live-control-card,
.stream-card {
  padding: 11px;
  border: 1px solid var(--dm-border);
  border-radius: 10px;
  background: color-mix(in srgb, var(--dm-panel) 78%, transparent);
}
.live-card-title,
.endpoint-head,
.secret-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}
.live-card-title {
  margin-bottom: 10px;
}
.live-card-title .section-title {
  margin-bottom: 2px;
}
.live-field {
  display: grid;
  gap: 5px;
  margin-bottom: 8px;
  color: var(--dm-muted);
  font-size: 11px;
}
.live-field select {
  appearance: auto;
}
.live-actions .primary-btn {
  flex: 1;
}
.primary-btn.stop {
  background: var(--dm-danger);
}
.live-control-card > small {
  display: block;
  margin-top: 8px;
  color: var(--dm-dim);
}
.endpoint-row {
  display: grid;
  gap: 7px;
  margin-top: 8px;
  padding: 9px;
  border-radius: 7px;
  background: var(--dm-side);
}
.endpoint-head span {
  color: var(--dm-success);
  font-size: 10px;
}
.endpoint-row code {
  min-width: 0;
  overflow: hidden;
  color: var(--dm-muted);
  font-family: Consolas, monospace;
  font-size: 10px;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.secret-row code {
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
  cursor: pointer;
}
.filter-pill.active {
  color: var(--dm-accent);
  border-color: color-mix(in srgb, var(--dm-accent) 48%, var(--dm-border));
  background: color-mix(in srgb, var(--dm-accent) 13%, transparent);
}
.send-bar {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto auto;
  align-items: center;
  gap: 8px;
  padding: 10px 16px 0;
}
.send-bar > span {
  color: var(--dm-dim);
  font-size: 11px;
}
.rank-strip {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 9px 16px 0;
  overflow-x: auto;
  color: var(--dm-muted);
  font-size: 11px;
}
.rank-strip strong {
  flex: 0 0 auto;
  color: var(--dm-gold);
}
.rank-strip span {
  flex: 0 0 auto;
  padding: 4px 8px;
  border-radius: 999px;
  background: var(--dm-panel);
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

@media (max-width: 860px) {
  .vrcdog-shell {
    flex-direction: column;
  }

  .vrcdog-sidebar {
    width: 100%;
    min-width: 0;
    max-height: 46%;
    box-shadow: inset 0 -1px 0 var(--dm-border);
  }

  .vrcdog-main {
    min-height: 0;
  }

  .live-header {
    align-items: flex-start;
    gap: 10px;
  }

  .live-summary {
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 6px;
  }
}

@media (max-width: 560px) {
  .vrcdog-sidebar { max-height: 50%; }
  .live-header { flex-direction: column; }
  .live-summary { justify-content: flex-start; }
  .live-toolbar { overflow-x: auto; }
  .send-bar { grid-template-columns: minmax(0, 1fr) auto; }
  .send-bar > span { display: none; }
  .message-meta { flex-wrap: wrap; }
  .message-meta time { width: 100%; margin-left: 0; }
}
</style>
