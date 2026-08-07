<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue';
import { isTauri } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { useI18n } from 'vue-i18n';
import {
  Activity,
  Check,
  Clock3,
  Cpu,
  Gauge,
  HardDrive,
  HeartPulse,
  History,
  MemoryStick,
  MessageSquare,
  Monitor,
  Pause,
  Play,
  Plus,
  Radio,
  RefreshCcw,
  Route,
  Save,
  Send,
  Server,
  SlidersHorizontal,
  Square,
  Trash2,
  X,
} from 'lucide-vue-next';
import {
  OscApi,
  type OscAutomationMapping,
  type OscMonitorEvent,
  type OscRouteRule,
  type OscSystemSnapshot,
  type OscValueType,
} from '../api';
import { useAuthStore } from '../stores/authStore';
import {
  formatDuration,
  formatGigabytes,
  formatHardwareSpecs,
  formatHardwareUsage,
  oscTextLength,
  renderOscTemplate,
  truncateOscText,
} from '../utils/oscFormatting';

type OscTab = 'send' | 'monitor' | 'chatbox' | 'automation' | 'router';
type AutoChatMode = 'repeat' | 'queue';

interface OscPreset {
  id: string;
  name: string;
  address: string;
  valueType: OscValueType;
  value: string;
}

interface SendHistoryItem extends OscPreset {
  sentAt: string;
}

const authStore = useAuthStore();
const { locale } = useI18n();
const l = (zh: string, en: string) => locale.value.startsWith('zh') ? zh : en;
const storageKey = 'vrcdog.osc.workbench.v2';
const legacyStorageKey = 'livehime.osc.workbench.v2';
const activeTab = ref<OscTab>('send');
const hydrated = ref(false);

const endpoint = ref({
  host: '127.0.0.1',
  port: 9000,
  monitorHost: '127.0.0.1',
  monitorPort: 9001,
});

const sender = ref({
  address: '/avatar/parameters/SystemCPU',
  valueType: 'float' as OscValueType,
  value: '1',
});

const presetName = ref('');
const presets = ref<OscPreset[]>([
  { id: 'cpu', name: l('CPU 参数', 'CPU parameter'), address: '/avatar/parameters/SystemCPU', valueType: 'float', value: '0.5' },
  { id: 'ram', name: l('RAM 参数', 'RAM parameter'), address: '/avatar/parameters/SystemRAM', valueType: 'float', value: '0.5' },
  { id: 'chat-typing', name: l('Chatbox 输入中', 'Chatbox typing'), address: '/chatbox/typing', valueType: 'bool', value: 'true' },
]);
const sendHistory = ref<SendHistoryItem[]>([]);
const actionStatus = ref({ type: '', message: '' });
const sending = ref(false);

const monitorRunning = ref(false);
const monitorPaused = ref(false);
const monitorFilter = ref('');
const monitorEvents = ref<OscMonitorEvent[]>([]);
const monitorError = ref('');

const emptySnapshot = (): OscSystemSnapshot => ({
  cpuUsage: 0,
  cpuName: '',
  cpuPhysicalCores: 0,
  cpuLogicalCores: 0,
  cpuFrequencyMhz: 0,
  ramUsage: 0,
  memoryUsedGb: 0,
  memoryTotalGb: 0,
  gpuName: '',
  gpuUsage: null,
  gpuMemoryUsedGb: null,
  gpuMemoryTotalGb: null,
  diskUsage: 0,
  diskUsedGb: 0,
  diskTotalGb: 0,
  osName: '',
  hostName: '',
  systemUptimeSeconds: 0,
  idleSeconds: 0,
  activeWindow: '',
  localTime: '--:--:--',
  localDate: '----/--/--',
  vrcRunning: false,
});
const snapshot = ref<OscSystemSnapshot>(emptySnapshot());
const snapshotLoading = ref(false);
const snapshotError = ref('');

const chatMessage = ref('');
const chatTemplate = ref('{message}\n{time}  {hardware}\n{heart_rate} {music}');
const musicText = ref('');
const heartRate = ref(0);
const heartRateAddress = ref('/avatar/parameters/HeartRate');
const chatNotify = ref(false);
const typingEnabled = ref(true);
const chatStatus = ref({ type: '', message: '' });
const autoChatMode = ref<AutoChatMode>('repeat');
const autoChatInterval = ref(8);
const autoChatRunning = ref(false);
const chatQueue = ref<string[]>([]);
const chatHistory = ref<{ text: string; sentAt: string }[]>([]);
let autoChatTimer: number | null = null;
let typingTimer: number | null = null;

const automationRunning = ref(false);
const automationInterval = ref(1500);
const automationError = ref('');
const mappings = ref<OscAutomationMapping[]>([
  {
    enabled: true,
    address: '/avatar/parameters/SystemCPU',
    source: 'cpu_usage',
    scale: 1,
    offset: 0,
    min: 0,
    max: 1,
    valueType: 'float',
  },
  {
    enabled: true,
    address: '/avatar/parameters/SystemRAM',
    source: 'ram_usage',
    scale: 1,
    offset: 0,
    min: 0,
    max: 1,
    valueType: 'float',
  },
  {
    enabled: false,
    address: '/avatar/parameters/DateTimeHour',
    source: 'local_hour',
    scale: 1,
    offset: 0,
    min: 0,
    max: 23,
    valueType: 'int',
  },
]);

const routes = ref<OscRouteRule[]>([]);
const routeStatus = ref('');
const avatarProfileStatus = ref('');

const tabs = computed<{ id: OscTab; label: string; icon: any }[]>(() => [
  { id: 'send', label: l('参数发送', 'Send'), icon: Send },
  { id: 'monitor', label: l('实时监听', 'Monitor'), icon: Monitor },
  { id: 'chatbox', label: 'Chatbox', icon: MessageSquare },
  { id: 'automation', label: l('自动映射', 'Automation'), icon: Gauge },
  { id: 'router', label: l('路由', 'Router'), icon: Route },
]);

const valueTypes: { id: OscValueType; label: string }[] = [
  { id: 'float', label: 'Float' },
  { id: 'int', label: 'Int' },
  { id: 'bool', label: 'Bool' },
  { id: 'string', label: 'String' },
  { id: 'impulse', label: 'Impulse' },
];

const sourceOptions = computed(() => [
  { value: 'cpu_usage', label: l('CPU 使用率 0-1', 'CPU usage 0-1') },
  { value: 'ram_usage', label: l('RAM 使用率 0-1', 'RAM usage 0-1') },
  { value: 'gpu_usage', label: l('GPU 使用率 0-1', 'GPU usage 0-1') },
  { value: 'memory_used_gb', label: l('已用内存 GB', 'Memory used (GB)') },
  { value: 'memory_total_gb', label: l('总内存 GB', 'Total memory (GB)') },
  { value: 'gpu_memory_used_gb', label: l('已用显存 GB', 'VRAM used (GB)') },
  { value: 'gpu_memory_total_gb', label: l('总显存 GB', 'Total VRAM (GB)') },
  { value: 'disk_usage', label: l('磁盘使用率 0-1', 'Disk usage 0-1') },
  { value: 'disk_used_gb', label: l('已用磁盘 GB', 'Disk used (GB)') },
  { value: 'disk_total_gb', label: l('总磁盘 GB', 'Total disk (GB)') },
  { value: 'cpu_physical_cores', label: l('CPU 物理核心数', 'CPU physical cores') },
  { value: 'cpu_logical_cores', label: l('CPU 线程数', 'CPU threads') },
  { value: 'cpu_frequency_mhz', label: l('CPU 频率 MHz', 'CPU frequency (MHz)') },
  { value: 'system_uptime_seconds', label: l('系统运行秒数', 'System uptime (seconds)') },
  { value: 'idle_seconds', label: l('挂机秒数', 'Idle time (seconds)') },
  { value: 'vrc_running', label: l('VRChat 运行状态', 'VRChat running') },
  { value: 'local_hour', label: l('本地小时', 'Local hour') },
  { value: 'local_minute', label: l('本地分钟', 'Local minute') },
  { value: 'local_second', label: l('本地秒', 'Local second') },
  { value: 'local_time_of_day', label: l('当天进度 0-1', 'Time of day 0-1') },
  { value: 'utc_hour', label: l('UTC 小时', 'UTC hour') },
  { value: 'random', label: l('随机值 0-1', 'Random value 0-1') },
]);

const filteredMonitorEvents = computed(() => {
  const query = monitorFilter.value.trim().toLowerCase();
  if (!query) return monitorEvents.value;
  return monitorEvents.value.filter((event) =>
    event.address.toLowerCase().includes(query)
    || event.sender.toLowerCase().includes(query)
    || event.args.some((arg) => String(arg.value).toLowerCase().includes(query))
  );
});

const formatIdle = (seconds: number) => {
  if (seconds < 60) return `${seconds}s`;
  const minutes = Math.floor(seconds / 60);
  const remainder = seconds % 60;
  return `${minutes}m ${remainder}s`;
};

const hardwareText = computed(() => formatHardwareUsage(snapshot.value));
const hardwareSpecs = computed(() => formatHardwareSpecs(snapshot.value));

const renderedChatbox = computed(() => {
  const replacements: Record<string, string> = {
    '{message}': chatMessage.value.trim(),
    '{time}': snapshot.value.localTime,
    '{date}': snapshot.value.localDate,
    '{cpu}': `${snapshot.value.cpuUsage.toFixed(0)}%`,
    '{ram}': `${snapshot.value.ramUsage.toFixed(0)}%`,
    '{gpu}': snapshot.value.gpuUsage == null ? '' : `${snapshot.value.gpuUsage.toFixed(0)}%`,
    '{hardware}': hardwareText.value,
    '{specs}': hardwareSpecs.value,
    '{memory}': `${formatGigabytes(snapshot.value.memoryUsedGb)} / ${formatGigabytes(snapshot.value.memoryTotalGb)}`,
    '{vram}': snapshot.value.gpuMemoryTotalGb == null
      ? ''
      : `${formatGigabytes(snapshot.value.gpuMemoryUsedGb)} / ${formatGigabytes(snapshot.value.gpuMemoryTotalGb)}`,
    '{disk}': `${formatGigabytes(snapshot.value.diskUsedGb, 0)} / ${formatGigabytes(snapshot.value.diskTotalGb, 0)}`,
    '{cpu_name}': snapshot.value.cpuName,
    '{gpu_name}': snapshot.value.gpuName,
    '{os}': snapshot.value.osName,
    '{host}': snapshot.value.hostName,
    '{uptime}': formatDuration(snapshot.value.systemUptimeSeconds),
    '{idle}': snapshot.value.idleSeconds > 0 ? `[${l('挂机', 'Idle')} ${formatIdle(snapshot.value.idleSeconds)}]` : '',
    '{window}': snapshot.value.activeWindow ? `[${l('窗口', 'Window')} ${truncateOscText(snapshot.value.activeWindow, 36)}]` : '',
    '{heart_rate}': heartRate.value > 0 ? `[${l('心率', 'Heart rate')} ${heartRate.value} BPM]` : '',
    '{music}': musicText.value.trim() ? `[${l('音乐', 'Music')} ${musicText.value.trim()}]` : '',
  };
  return renderOscTemplate(chatTemplate.value, replacements);
});
const renderedChatboxLength = computed(() => oscTextLength(renderedChatbox.value));

const currentAvatarId = computed(() => {
  const user = authStore.currentUser as any;
  return String(user?.currentAvatar || user?.currentAvatarId || user?.currentAvatarAssetUrl || '').trim();
});

function valueForApi(type: OscValueType, raw: string): unknown {
  if (type === 'bool') return ['true', '1', 'yes', 'on'].includes(raw.trim().toLowerCase());
  if (type === 'int' || type === 'long') return Math.round(Number(raw));
  if (type === 'float' || type === 'double') return Number(raw);
  if (type === 'impulse') return null;
  return raw;
}

function normalizeValueType(type: string): OscValueType {
  if (['float', 'double', 'int', 'long', 'bool', 'string', 'impulse'].includes(type)) {
    return type as OscValueType;
  }
  return 'string';
}

async function sendCurrentParameter() {
  sending.value = true;
  actionStatus.value = { type: '', message: '' };
  try {
    const value = valueForApi(sender.value.valueType, sender.value.value);
    if (typeof value === 'number' && !Number.isFinite(value)) throw new Error(l('请输入有效数值', 'Enter a valid number'));
    await OscApi.sendMessage({
      host: endpoint.value.host,
      port: Number(endpoint.value.port),
      address: sender.value.address,
      valueType: sender.value.valueType,
      value,
    });
    sendHistory.value.unshift({
      id: crypto.randomUUID(),
      name: sender.value.address,
      address: sender.value.address,
      valueType: sender.value.valueType,
      value: sender.value.value,
      sentAt: new Date().toISOString(),
    });
    sendHistory.value = sendHistory.value.slice(0, 30);
    actionStatus.value = { type: 'success', message: l('OSC 参数已发送', 'OSC parameter sent') };
  } catch (error: any) {
    actionStatus.value = { type: 'error', message: error?.message || String(error) };
  } finally {
    sending.value = false;
  }
}

function applyPreset(preset: OscPreset) {
  sender.value = {
    address: preset.address,
    valueType: preset.valueType,
    value: preset.value,
  };
}

function savePreset() {
  const addressParts = sender.value.address.split('/').filter(Boolean);
  const name = presetName.value.trim() || addressParts[addressParts.length - 1] || 'OSC Preset';
  presets.value.unshift({
    id: crypto.randomUUID(),
    name,
    address: sender.value.address,
    valueType: sender.value.valueType,
    value: sender.value.value,
  });
  presets.value = presets.value.slice(0, 40);
  presetName.value = '';
}

async function toggleMonitor() {
  monitorError.value = '';
  try {
    if (monitorRunning.value) {
      await OscApi.stopMonitor();
      monitorRunning.value = false;
    } else {
      await OscApi.startMonitor({
        host: endpoint.value.monitorHost,
        port: Number(endpoint.value.monitorPort),
        routes: routes.value,
      });
      monitorRunning.value = true;
    }
  } catch (error: any) {
    monitorError.value = error?.message || String(error);
  }
}

async function applyRoutes() {
  routeStatus.value = '';
  try {
    if (monitorRunning.value) {
      await OscApi.startMonitor({
        host: endpoint.value.monitorHost,
        port: Number(endpoint.value.monitorPort),
        routes: routes.value,
      });
    }
    routeStatus.value = monitorRunning.value
      ? l('路由规则已应用，监听器已重启', 'Routes applied and monitor restarted')
      : l('路由规则已保存，启动监听后生效', 'Routes saved and will apply when monitoring starts');
  } catch (error: any) {
    routeStatus.value = error?.message || String(error);
  }
}

function useMonitorEvent(event: OscMonitorEvent) {
  const first = event.args[0];
  sender.value.address = event.address;
  sender.value.valueType = normalizeValueType(first?.valueType || 'impulse');
  sender.value.value = first?.value == null ? '' : String(first.value);
  activeTab.value = 'send';
}

function addRoute() {
  routes.value.push({
    enabled: true,
    sourceAddress: '/avatar/parameters/*',
    targetHost: endpoint.value.host,
    targetPort: endpoint.value.port,
    targetAddress: '',
  });
}

function addMapping() {
  mappings.value.push({
    enabled: true,
    address: '/avatar/parameters/CustomValue',
    source: 'cpu_usage',
    scale: 1,
    offset: 0,
    min: 0,
    max: 1,
    valueType: 'float',
  });
}

async function toggleAutomation() {
  automationError.value = '';
  try {
    if (automationRunning.value) {
      await OscApi.stopAutomation();
      automationRunning.value = false;
    } else {
      await OscApi.startAutomation({
        config: {
          host: endpoint.value.host,
          port: Number(endpoint.value.port),
          intervalMs: Number(automationInterval.value),
          mappings: mappings.value,
        },
      });
      automationRunning.value = true;
    }
  } catch (error: any) {
    automationError.value = error?.message || String(error);
  }
}

function sourceRawValue(source: string): number | null {
  const now = new Date();
  const values: Record<string, number | null> = {
    cpu_usage: snapshot.value.cpuUsage / 100,
    ram_usage: snapshot.value.ramUsage / 100,
    gpu_usage: snapshot.value.gpuUsage == null ? null : snapshot.value.gpuUsage / 100,
    memory_used_gb: snapshot.value.memoryUsedGb,
    memory_total_gb: snapshot.value.memoryTotalGb,
    gpu_memory_used_gb: snapshot.value.gpuMemoryUsedGb,
    gpu_memory_total_gb: snapshot.value.gpuMemoryTotalGb,
    disk_usage: snapshot.value.diskUsage / 100,
    disk_used_gb: snapshot.value.diskUsedGb,
    disk_total_gb: snapshot.value.diskTotalGb,
    cpu_physical_cores: snapshot.value.cpuPhysicalCores,
    cpu_logical_cores: snapshot.value.cpuLogicalCores,
    cpu_frequency_mhz: snapshot.value.cpuFrequencyMhz,
    system_uptime_seconds: snapshot.value.systemUptimeSeconds,
    idle_seconds: snapshot.value.idleSeconds,
    vrc_running: snapshot.value.vrcRunning ? 1 : 0,
    local_hour: now.getHours(),
    local_minute: now.getMinutes(),
    local_second: now.getSeconds(),
    local_time_of_day: (now.getHours() * 3600 + now.getMinutes() * 60 + now.getSeconds()) / 86400,
    utc_hour: now.getUTCHours(),
    random: 0.5,
  };
  return values[source] ?? null;
}

function mappingPreview(mapping: OscAutomationMapping) {
  const raw = sourceRawValue(mapping.source);
  if (raw == null) return '--';
  let value = raw * Number(mapping.scale || 0) + Number(mapping.offset || 0);
  if (mapping.min != null) value = Math.max(value, Number(mapping.min));
  if (mapping.max != null) value = Math.min(value, Number(mapping.max));
  if (mapping.valueType === 'bool') return value >= 0.5 ? 'true' : 'false';
  if (mapping.valueType === 'int' || mapping.valueType === 'long') return String(Math.round(value));
  return value.toFixed(3);
}

async function refreshSnapshot() {
  snapshotLoading.value = true;
  snapshotError.value = '';
  try {
    snapshot.value = await OscApi.getSystemSnapshot();
  } catch (error: any) {
    snapshotError.value = error?.message || l('系统信息检测失败', 'System information check failed');
  } finally {
    snapshotLoading.value = false;
  }
}

async function setTyping(active: boolean) {
  try {
    await OscApi.sendMessage({
      host: endpoint.value.host,
      port: Number(endpoint.value.port),
      address: '/chatbox/typing',
      valueType: 'bool',
      value: active,
    });
  } catch {
    // Typing state is best-effort and should not block message editing.
  }
}

async function sendChatText(text: string) {
  const trimmed = truncateOscText(text.trim());
  if (!trimmed) return;
  await OscApi.sendChatbox({
    host: endpoint.value.host,
    port: Number(endpoint.value.port),
    text: trimmed,
    send: true,
    notify: chatNotify.value,
  });
  await setTyping(false);
  chatHistory.value.unshift({ text: trimmed, sentAt: new Date().toISOString() });
  chatHistory.value = chatHistory.value.slice(0, 30);
}

async function sendChatbox() {
  chatStatus.value = { type: '', message: '' };
  try {
    await refreshSnapshot();
    await sendChatText(renderedChatbox.value);
    chatStatus.value = { type: 'success', message: l('Chatbox 消息已发送', 'Chatbox message sent') };
  } catch (error: any) {
    chatStatus.value = { type: 'error', message: error?.message || String(error) };
  }
}

function addChatToQueue() {
  const text = renderedChatbox.value.trim();
  if (!text) return;
  chatQueue.value.push(text);
  chatMessage.value = '';
}

async function sendNextQueue() {
  const next = chatQueue.value[0];
  if (!next) {
    stopAutoChat();
    return;
  }
  await sendChatText(next);
  chatQueue.value.shift();
  if (chatQueue.value.length === 0) stopAutoChat();
}

function stopAutoChat() {
  if (autoChatTimer != null) window.clearInterval(autoChatTimer);
  autoChatTimer = null;
  autoChatRunning.value = false;
}

function toggleAutoChat() {
  if (autoChatRunning.value) {
    stopAutoChat();
    return;
  }
  const interval = Math.max(3, Number(autoChatInterval.value) || 8) * 1000;
  autoChatRunning.value = true;
  const run = async () => {
    try {
      if (autoChatMode.value === 'queue') await sendNextQueue();
      else await sendChatbox();
    } catch (error: any) {
      chatStatus.value = { type: 'error', message: error?.message || String(error) };
      stopAutoChat();
    }
  };
  void run();
  autoChatTimer = window.setInterval(run, interval);
}

function insertTemplateVariable(variable: string) {
  chatTemplate.value += chatTemplate.value.endsWith(' ') || chatTemplate.value.endsWith('\n') ? variable : ` ${variable}`;
}

function profilePayload() {
  return {
    sender: sender.value,
    presets: presets.value,
    chatTemplate: chatTemplate.value,
    mappings: mappings.value,
    routes: routes.value,
  };
}

function saveAvatarProfile() {
  if (!currentAvatarId.value) {
    avatarProfileStatus.value = l('当前用户没有可识别的头像 ID', 'No avatar ID is available for the current user');
    return;
  }
  localStorage.setItem(`vrcdog.osc.avatar.${currentAvatarId.value}`, JSON.stringify(profilePayload()));
  avatarProfileStatus.value = l('已保存当前头像 OSC 配置', 'Current avatar OSC profile saved');
}

function loadAvatarProfile() {
  if (!currentAvatarId.value) {
    avatarProfileStatus.value = l('当前用户没有可识别的头像 ID', 'No avatar ID is available for the current user');
    return;
  }
  const profileKey = `vrcdog.osc.avatar.${currentAvatarId.value}`;
  const legacyProfileKey = `livehime.osc.avatar.${currentAvatarId.value}`;
  const raw = localStorage.getItem(profileKey) || localStorage.getItem(legacyProfileKey);
  if (!raw) {
    avatarProfileStatus.value = l('当前头像还没有保存配置', 'No saved profile exists for the current avatar');
    return;
  }
  try {
    const data = JSON.parse(raw);
    localStorage.setItem(profileKey, raw);
    if (data.sender) sender.value = data.sender;
    if (Array.isArray(data.presets)) presets.value = data.presets;
    if (typeof data.chatTemplate === 'string') chatTemplate.value = data.chatTemplate;
    if (Array.isArray(data.mappings)) mappings.value = data.mappings;
    if (Array.isArray(data.routes)) routes.value = data.routes;
    avatarProfileStatus.value = l('已载入当前头像 OSC 配置', 'Current avatar OSC profile loaded');
  } catch {
    avatarProfileStatus.value = l('头像配置损坏，无法载入', 'The avatar profile is damaged and could not be loaded');
  }
}

function saveLocalState() {
  if (!hydrated.value) return;
  localStorage.setItem(storageKey, JSON.stringify({
    endpoint: endpoint.value,
    sender: sender.value,
    presets: presets.value,
    sendHistory: sendHistory.value,
    chatMessage: chatMessage.value,
    chatTemplate: chatTemplate.value,
    musicText: musicText.value,
    heartRateAddress: heartRateAddress.value,
    chatNotify: chatNotify.value,
    typingEnabled: typingEnabled.value,
    autoChatMode: autoChatMode.value,
    autoChatInterval: autoChatInterval.value,
    chatQueue: chatQueue.value,
    chatHistory: chatHistory.value,
    automationInterval: automationInterval.value,
    mappings: mappings.value,
    routes: routes.value,
  }));
}

function loadLocalState() {
  const raw = localStorage.getItem(storageKey) || localStorage.getItem(legacyStorageKey);
  if (!raw) return;
  try {
    const data = JSON.parse(raw);
    localStorage.setItem(storageKey, raw);
    if (data.endpoint) endpoint.value = { ...endpoint.value, ...data.endpoint };
    if (data.sender) sender.value = { ...sender.value, ...data.sender };
    if (Array.isArray(data.presets)) presets.value = data.presets;
    if (Array.isArray(data.sendHistory)) sendHistory.value = data.sendHistory;
    if (typeof data.chatMessage === 'string') chatMessage.value = data.chatMessage;
    if (typeof data.chatTemplate === 'string') chatTemplate.value = data.chatTemplate;
    if (typeof data.musicText === 'string') musicText.value = data.musicText;
    if (typeof data.heartRateAddress === 'string') heartRateAddress.value = data.heartRateAddress;
    if (typeof data.chatNotify === 'boolean') chatNotify.value = data.chatNotify;
    if (typeof data.typingEnabled === 'boolean') typingEnabled.value = data.typingEnabled;
    if (data.autoChatMode === 'repeat' || data.autoChatMode === 'queue') autoChatMode.value = data.autoChatMode;
    if (Number.isFinite(Number(data.autoChatInterval))) autoChatInterval.value = Number(data.autoChatInterval);
    if (Array.isArray(data.chatQueue)) chatQueue.value = data.chatQueue;
    if (Array.isArray(data.chatHistory)) chatHistory.value = data.chatHistory;
    if (Number.isFinite(Number(data.automationInterval))) automationInterval.value = Number(data.automationInterval);
    if (Array.isArray(data.mappings)) mappings.value = data.mappings;
    if (Array.isArray(data.routes)) routes.value = data.routes;
  } catch {
    localStorage.removeItem(storageKey);
    localStorage.removeItem(legacyStorageKey);
  }
}

watch(
  [
    endpoint,
    sender,
    presets,
    sendHistory,
    chatMessage,
    chatTemplate,
    musicText,
    heartRateAddress,
    chatNotify,
    typingEnabled,
    autoChatMode,
    autoChatInterval,
    chatQueue,
    chatHistory,
    automationInterval,
    mappings,
    routes,
  ],
  saveLocalState,
  { deep: true },
);

watch(chatMessage, () => {
  if (!typingEnabled.value) return;
  if (typingTimer != null) window.clearTimeout(typingTimer);
  void setTyping(Boolean(chatMessage.value.trim()));
  typingTimer = window.setTimeout(() => void setTyping(false), 1800);
});

watch(typingEnabled, (enabled) => {
  if (!enabled) {
    if (typingTimer != null) window.clearTimeout(typingTimer);
    typingTimer = null;
    void setTyping(false);
  }
});

let snapshotTimer: number | null = null;
const unlisteners: UnlistenFn[] = [];

onMounted(async () => {
  loadLocalState();
  hydrated.value = true;
  await refreshSnapshot();
  try {
    const status = await OscApi.getStatus();
    monitorRunning.value = status.monitorRunning;
    automationRunning.value = status.automationRunning;
  } catch {
    // Browser preview and older backend versions use the local defaults.
  }

  snapshotTimer = window.setInterval(refreshSnapshot, 5000);

  if (isTauri()) {
    unlisteners.push(await listen<OscMonitorEvent>('osc-monitor-event', ({ payload }) => {
      const first = payload.args[0];
      if (payload.address === heartRateAddress.value && first) {
        const value = Number(first.value);
        if (Number.isFinite(value) && value > 0) heartRate.value = Math.round(value);
      }
      if (!monitorPaused.value) {
        monitorEvents.value.unshift(payload);
        monitorEvents.value = monitorEvents.value.slice(0, 300);
      }
    }));
    unlisteners.push(await listen<boolean>('osc-monitor-status', ({ payload }) => {
      monitorRunning.value = payload;
    }));
    unlisteners.push(await listen<boolean>('osc-automation-status', ({ payload }) => {
      automationRunning.value = payload;
    }));
    unlisteners.push(await listen<string>('osc-monitor-error', ({ payload }) => {
      monitorError.value = payload;
    }));
    unlisteners.push(await listen<string>('osc-automation-error', ({ payload }) => {
      automationError.value = payload;
    }));
    unlisteners.push(await listen<OscSystemSnapshot>('osc-system-snapshot', ({ payload }) => {
      snapshot.value = payload;
    }));
  }
});

onUnmounted(() => {
  if (snapshotTimer != null) window.clearInterval(snapshotTimer);
  if (typingTimer != null) window.clearTimeout(typingTimer);
  void setTyping(false);
  stopAutoChat();
  unlisteners.forEach((unlisten) => unlisten());
});
</script>

<template>
  <section class="osc-workbench">
    <header class="osc-head">
      <div class="osc-title">
        <span class="title-icon"><Radio :size="20" /></span>
        <div>
          <h3>{{ l('OSC 工作台', 'OSC Workbench') }}</h3>
          <p>{{ l('参数、监听、Chatbox、硬件映射与路由集中管理', 'Manage parameters, monitoring, Chatbox, hardware mapping, and routing') }}</p>
        </div>
      </div>

      <div class="runtime-badges">
        <span :class="{ active: monitorRunning }">
          <Monitor :size="13" /> {{ monitorRunning ? l('监听中', 'Monitoring') : l('未监听', 'Not monitoring') }}
        </span>
        <span :class="{ active: automationRunning }">
          <Activity :size="13" /> {{ automationRunning ? l('映射中', 'Mapping') : l('未映射', 'Not mapping') }}
        </span>
        <span :class="{ active: snapshot.vrcRunning }">
          <Radio :size="13" /> {{ snapshot.vrcRunning ? l('VRChat 运行中', 'VRChat running') : l('VRChat 未运行', 'VRChat not running') }}
        </span>
      </div>
    </header>

    <div class="endpoint-bar">
      <label>
        <span>{{ l('目标', 'Target') }}</span>
        <input v-model.trim="endpoint.host" :aria-label="l('OSC 目标主机', 'OSC target host')">
        <input v-model.number="endpoint.port" class="port-input" type="number" min="1" max="65535" :aria-label="l('OSC 目标端口', 'OSC target port')">
      </label>
      <label>
        <span>{{ l('监听', 'Monitor') }}</span>
        <input v-model.trim="endpoint.monitorHost" :aria-label="l('OSC 监听主机', 'OSC monitor host')">
        <input v-model.number="endpoint.monitorPort" class="port-input" type="number" min="1" max="65535" :aria-label="l('OSC 监听端口', 'OSC monitor port')">
      </label>
      <div class="profile-actions">
        <button :title="l('保存当前头像配置', 'Save current avatar profile')" :disabled="!currentAvatarId" @click="saveAvatarProfile">
          <Save :size="15" /> {{ l('保存头像配置', 'Save avatar profile') }}
        </button>
        <button :title="l('载入当前头像配置', 'Load current avatar profile')" :disabled="!currentAvatarId" @click="loadAvatarProfile">
          <RefreshCcw :size="15" /> {{ l('载入', 'Load') }}
        </button>
      </div>
    </div>

    <p v-if="avatarProfileStatus" class="inline-status">{{ avatarProfileStatus }}</p>

    <nav class="osc-tabs" :aria-label="l('OSC 功能', 'OSC tools')">
      <button
        v-for="tab in tabs"
        :key="tab.id"
        :class="{ active: activeTab === tab.id }"
        @click="activeTab = tab.id"
      >
        <component :is="tab.icon" :size="15" />
        {{ tab.label }}
      </button>
    </nav>

    <div class="osc-body">
      <template v-if="activeTab === 'send'">
        <div class="send-layout">
          <div class="editor-pane">
            <div class="section-heading">
              <div>
                <strong>{{ l('参数编辑器', 'Parameter editor') }}</strong>
                <span>{{ l('支持 VRChat Avatar 参数与任意 OSC 地址', 'Supports VRChat avatar parameters and any OSC address') }}</span>
              </div>
            </div>

            <label class="field">
              <span>{{ l('OSC 地址', 'OSC address') }}</span>
              <input v-model.trim="sender.address" placeholder="/avatar/parameters/ParameterName" @keydown.enter="sendCurrentParameter">
            </label>

            <div class="field">
              <span>{{ l('参数类型', 'Value type') }}</span>
              <div class="segmented">
                <button
                  v-for="type in valueTypes"
                  :key="type.id"
                  :class="{ active: sender.valueType === type.id }"
                  @click="sender.valueType = type.id"
                >
                  {{ type.label }}
                </button>
              </div>
            </div>

            <label v-if="sender.valueType === 'bool'" class="field">
              <span>{{ l('布尔值', 'Boolean value') }}</span>
              <div class="segmented compact">
                <button :class="{ active: sender.value === 'true' }" @click="sender.value = 'true'">True</button>
                <button :class="{ active: sender.value === 'false' }" @click="sender.value = 'false'">False</button>
              </div>
            </label>
            <label v-else-if="sender.valueType !== 'impulse'" class="field">
              <span>{{ l('参数值', 'Value') }}</span>
              <input
                v-model="sender.value"
                :type="['float', 'double', 'int', 'long'].includes(sender.valueType) ? 'number' : 'text'"
                step="0.01"
                @keydown.enter="sendCurrentParameter"
              >
            </label>

            <div class="primary-actions">
              <button class="primary" :disabled="sending || !sender.address" @click="sendCurrentParameter">
                <RefreshCcw v-if="sending" class="spin" :size="16" />
                <Send v-else :size="16" />
                {{ l('发送参数', 'Send parameter') }}
              </button>
              <input v-model="presetName" :placeholder="l('预设名称', 'Preset name')">
              <button :title="l('保存为预设', 'Save as preset')" @click="savePreset"><Save :size="16" /></button>
            </div>

            <p v-if="actionStatus.message" class="inline-status" :class="actionStatus.type">
              {{ actionStatus.message }}
            </p>
          </div>

          <aside class="side-pane">
            <div class="section-heading">
              <div>
                <strong>{{ l('快捷预设', 'Quick presets') }}</strong>
                <span>{{ presets.length }}</span>
              </div>
            </div>
            <div class="preset-list">
              <button v-for="preset in presets" :key="preset.id" class="preset-row" @click="applyPreset(preset)">
                <span>
                  <strong>{{ preset.name }}</strong>
                  <small>{{ preset.address }}</small>
                </span>
                <code>{{ preset.valueType }} {{ preset.value }}</code>
                <X :size="14" @click.stop="presets = presets.filter((item) => item.id !== preset.id)" />
              </button>
            </div>

            <div class="section-heading history-heading">
              <div>
                <strong>{{ l('发送历史', 'Send history') }}</strong>
                <span>{{ l('最近', 'Latest') }} {{ sendHistory.length }}</span>
              </div>
              <button :title="l('清空历史', 'Clear history')" @click="sendHistory = []"><Trash2 :size="14" /></button>
            </div>
            <div class="history-list">
              <button v-for="item in sendHistory.slice(0, 10)" :key="item.id" @click="applyPreset(item)">
                <span>{{ item.address }}</span>
                <code>{{ item.value }}</code>
              </button>
              <p v-if="sendHistory.length === 0" class="empty-text">{{ l('发送后会保留快捷历史', 'Recently sent parameters appear here') }}</p>
            </div>
          </aside>
        </div>
      </template>

      <template v-else-if="activeTab === 'monitor'">
        <div class="monitor-toolbar">
          <button class="primary" :class="{ danger: monitorRunning }" @click="toggleMonitor">
            <Square v-if="monitorRunning" :size="15" />
            <Play v-else :size="15" />
            {{ monitorRunning ? l('停止监听', 'Stop monitor') : l('开始监听', 'Start monitor') }}
          </button>
          <label class="filter-field">
            <span>{{ l('过滤', 'Filter') }}</span>
            <input v-model="monitorFilter" placeholder="/avatar/parameters/">
          </label>
          <button :class="{ active: monitorPaused }" :title="l('暂停日志显示', 'Pause log display')" @click="monitorPaused = !monitorPaused">
            <Pause :size="15" /> {{ monitorPaused ? l('已暂停', 'Paused') : l('暂停', 'Pause') }}
          </button>
          <button :title="l('清空日志', 'Clear log')" @click="monitorEvents = []"><Trash2 :size="15" /></button>
          <span class="event-count">{{ filteredMonitorEvents.length }}</span>
        </div>

        <p v-if="monitorError" class="inline-status error">{{ monitorError }}</p>

        <div class="monitor-table">
          <div class="monitor-row monitor-header">
            <span>{{ l('时间', 'Time') }}</span>
            <span>{{ l('地址', 'Address') }}</span>
            <span>{{ l('值', 'Value') }}</span>
            <span>{{ l('来源', 'Source') }}</span>
          </div>
          <button
            v-for="(event, index) in filteredMonitorEvents"
            :key="`${event.timestamp}-${event.address}-${index}`"
            class="monitor-row"
            @click="useMonitorEvent(event)"
          >
            <time>{{ new Date(event.timestamp).toLocaleTimeString() }}</time>
            <code>{{ event.address }}</code>
            <span class="event-values">
              <code v-for="(arg, argIndex) in event.args" :key="argIndex">{{ arg.valueType }}:{{ arg.value }}</code>
              <code v-if="event.args.length === 0">impulse</code>
            </span>
            <small>{{ event.sender }}</small>
          </button>
          <div v-if="filteredMonitorEvents.length === 0" class="monitor-empty">
            <Radio :size="30" />
            <strong>{{ l('等待 OSC 数据', 'Waiting for OSC data') }}</strong>
            <span>{{ l('VRChat 常用回传端口为 9001', 'VRChat commonly sends data on port 9001') }}</span>
          </div>
        </div>

        <div class="heart-capture">
          <HeartPulse :size="17" />
          <label>
            <span>{{ l('心率捕获地址', 'Heart-rate address') }}</span>
            <input v-model.trim="heartRateAddress" placeholder="/avatar/parameters/HeartRate">
          </label>
          <strong>{{ heartRate > 0 ? `${heartRate} BPM` : l('等待数据', 'Waiting for data') }}</strong>
        </div>
      </template>

      <template v-else-if="activeTab === 'chatbox'">
        <div class="chat-layout">
          <div class="editor-pane">
            <div class="section-heading">
              <div>
                <strong>{{ l('Chatbox 模板', 'Chatbox template') }}</strong>
                <span>{{ l('最终内容自动限制为 144 字符', 'Final output is limited to 144 characters') }}</span>
              </div>
              <span class="char-count" :class="{ warn: renderedChatboxLength >= 136 }">{{ renderedChatboxLength }}/144</span>
            </div>

            <label class="field">
              <span>{{ l('消息正文', 'Message') }}</span>
              <textarea v-model="chatMessage" rows="3" :placeholder="l('输入消息，或只使用模板变量', 'Enter a message or use template variables')"></textarea>
            </label>
            <label class="field">
              <span>{{ l('模板', 'Template') }}</span>
              <textarea v-model="chatTemplate" rows="4"></textarea>
            </label>

            <div class="variable-bar">
              <button
                v-for="variable in ['{message}', '{time}', '{date}', '{hardware}', '{specs}', '{cpu}', '{ram}', '{gpu}', '{memory}', '{vram}', '{disk}', '{uptime}', '{idle}', '{window}', '{heart_rate}', '{music}']"
                :key="variable"
                @click="insertTemplateVariable(variable)"
              >
                {{ variable }}
              </button>
            </div>

            <label class="field">
              <span>{{ l('音乐信息', 'Music') }}</span>
              <input v-model="musicText" :placeholder="l('可选：歌曲名 - 艺术家', 'Optional: track - artist')">
            </label>

            <div class="toggle-row">
              <label><input v-model="typingEnabled" type="checkbox"> {{ l('输入时同步 typing 状态', 'Sync typing state while editing') }}</label>
              <label><input v-model="chatNotify" type="checkbox"> {{ l('Chatbox 提示音', 'Chatbox notification sound') }}</label>
            </div>

            <div class="primary-actions">
              <button class="primary" :disabled="!renderedChatbox" @click="sendChatbox">
                <Send :size="16" /> {{ l('立即发送', 'Send now') }}
              </button>
              <button :disabled="!renderedChatbox" @click="addChatToQueue">
                <Plus :size="16" /> {{ l('加入队列', 'Add to queue') }}
              </button>
            </div>
            <p v-if="chatStatus.message" class="inline-status" :class="chatStatus.type">
              {{ chatStatus.message }}
            </p>
          </div>

          <aside class="chat-preview">
            <div class="section-heading">
              <div>
                <strong>{{ l('实时预览', 'Live preview') }}</strong>
                <span>{{ snapshot.localTime }}</span>
              </div>
              <button :title="l('刷新系统信息', 'Refresh system information')" :disabled="snapshotLoading" @click="refreshSnapshot">
                <RefreshCcw :size="14" :class="{ spin: snapshotLoading }" />
              </button>
            </div>
            <pre>{{ renderedChatbox || l('输入消息或插入模板变量', 'Enter a message or insert template variables') }}</pre>
            <p v-if="snapshotError" class="inline-status error">{{ snapshotError }}</p>

            <div class="auto-chat-controls">
              <div class="segmented compact">
                <button :class="{ active: autoChatMode === 'repeat' }" @click="autoChatMode = 'repeat'">{{ l('循环模板', 'Repeat template') }}</button>
                <button :class="{ active: autoChatMode === 'queue' }" @click="autoChatMode = 'queue'">{{ l('发送队列', 'Send queue') }}</button>
              </div>
              <label>
                <Clock3 :size="14" />
                <input v-model.number="autoChatInterval" type="number" min="3" max="300">
                {{ l('秒', 'sec') }}
              </label>
              <button class="primary" :class="{ danger: autoChatRunning }" @click="toggleAutoChat">
                <Square v-if="autoChatRunning" :size="15" />
                <Play v-else :size="15" />
                {{ autoChatRunning ? l('停止', 'Stop') : l('开始定时', 'Start timer') }}
              </button>
            </div>

            <div class="queue-head">
              <strong>{{ l('队列', 'Queue') }} {{ chatQueue.length }}</strong>
              <button v-if="chatQueue.length" @click="chatQueue = []"><Trash2 :size="13" /> {{ l('清空', 'Clear') }}</button>
            </div>
            <div class="queue-list">
              <div v-for="(item, index) in chatQueue" :key="`${item}-${index}`">
                <span>{{ item }}</span>
                <button @click="chatQueue.splice(index, 1)"><X :size="13" /></button>
              </div>
              <p v-if="chatQueue.length === 0" class="empty-text">{{ l('队列模式会按顺序发送', 'Queue mode sends messages in order') }}</p>
            </div>
          </aside>
        </div>
      </template>

      <template v-else-if="activeTab === 'automation'">
        <div class="metric-strip">
          <div><Cpu :size="16" /><span>CPU</span><strong>{{ snapshot.cpuUsage.toFixed(0) }}%</strong><small>{{ snapshot.cpuPhysicalCores || '--' }} {{ l('核', 'cores') }} / {{ snapshot.cpuLogicalCores || '--' }} {{ l('线程', 'threads') }}</small></div>
          <div><Gauge :size="16" /><span>GPU</span><strong>{{ snapshot.gpuUsage == null ? '--' : `${snapshot.gpuUsage.toFixed(0)}%` }}</strong><small>{{ snapshot.gpuUsage == null ? l('负载不可用', 'Load unavailable') : l('实时负载', 'Live load') }}</small></div>
          <div><MemoryStick :size="16" /><span>{{ l('内存', 'Memory') }}</span><strong>{{ formatGigabytes(snapshot.memoryUsedGb) }} / {{ formatGigabytes(snapshot.memoryTotalGb) }}</strong><small>{{ snapshot.ramUsage.toFixed(0) }}%</small></div>
          <div><Server :size="16" /><span>{{ l('显存', 'VRAM') }}</span><strong>{{ formatGigabytes(snapshot.gpuMemoryUsedGb) }} / {{ formatGigabytes(snapshot.gpuMemoryTotalGb) }}</strong><small>{{ snapshot.gpuMemoryTotalGb == null ? l('未提供遥测', 'Telemetry unavailable') : l('专用显存', 'Dedicated VRAM') }}</small></div>
          <div><HardDrive :size="16" /><span>{{ l('磁盘', 'Disk') }}</span><strong>{{ formatGigabytes(snapshot.diskUsedGb, 0) }} / {{ formatGigabytes(snapshot.diskTotalGb, 0) }}</strong><small>{{ snapshot.diskUsage.toFixed(0) }}%</small></div>
          <div><Clock3 :size="16" /><span>{{ l('运行时间', 'Uptime') }}</span><strong>{{ formatDuration(snapshot.systemUptimeSeconds) }}</strong><small>{{ l('空闲', 'Idle') }} {{ formatDuration(snapshot.idleSeconds) }}</small></div>
        </div>

        <div class="hardware-profile">
          <div class="hardware-profile-head">
            <div><Activity :size="16" /><strong>{{ l('自动检测到的设备', 'Detected device') }}</strong><span>{{ l('实时', 'Live') }}</span></div>
            <button :title="l('重新检测系统信息', 'Detect system information again')" :disabled="snapshotLoading" @click="refreshSnapshot">
              <RefreshCcw :size="14" :class="{ spin: snapshotLoading }" />
            </button>
          </div>
          <dl>
            <div><dt>{{ l('处理器', 'Processor') }}</dt><dd :title="snapshot.cpuName">{{ snapshot.cpuName || l('未检测到', 'Not detected') }}</dd></div>
            <div><dt>{{ l('显卡', 'Graphics') }}</dt><dd :title="snapshot.gpuName">{{ snapshot.gpuName || l('未检测到', 'Not detected') }}</dd></div>
            <div><dt>{{ l('系统', 'System') }}</dt><dd :title="snapshot.osName">{{ snapshot.osName || l('未检测到', 'Not detected') }}</dd></div>
            <div><dt>{{ l('设备', 'Device') }}</dt><dd>{{ snapshot.hostName || l('未检测到', 'Not detected') }}</dd></div>
          </dl>
        </div>
        <p v-if="snapshotError" class="inline-status error">{{ snapshotError }}</p>

        <div class="automation-toolbar">
          <label>
            <span>{{ l('刷新间隔', 'Refresh interval') }}</span>
            <input v-model.number="automationInterval" type="number" min="250" max="60000" step="250">
            ms
          </label>
          <button @click="addMapping"><Plus :size="15" /> {{ l('添加映射', 'Add mapping') }}</button>
          <button class="primary" :class="{ danger: automationRunning }" @click="toggleAutomation">
            <Square v-if="automationRunning" :size="15" />
            <Play v-else :size="15" />
            {{ automationRunning ? l('停止映射', 'Stop mapping') : l('启动映射', 'Start mapping') }}
          </button>
        </div>

        <p v-if="automationError" class="inline-status error">{{ automationError }}</p>

        <div class="mapping-table">
          <div class="mapping-row mapping-header">
            <span>{{ l('启用', 'Enabled') }}</span><span>{{ l('数据源', 'Source') }}</span><span>{{ l('目标地址', 'Target address') }}</span><span>{{ l('类型', 'Type') }}</span><span>{{ l('缩放', 'Scale') }}</span><span>{{ l('偏移', 'Offset') }}</span><span>{{ l('范围', 'Range') }}</span><span>{{ l('预览', 'Preview') }}</span><span></span>
          </div>
          <div v-for="(mapping, index) in mappings" :key="index" class="mapping-row">
            <input v-model="mapping.enabled" type="checkbox">
            <select v-model="mapping.source">
              <option v-for="option in sourceOptions" :key="option.value" :value="option.value">{{ option.label }}</option>
            </select>
            <input v-model.trim="mapping.address" placeholder="/avatar/parameters/...">
            <select v-model="mapping.valueType">
              <option value="float">Float</option>
              <option value="int">Int</option>
              <option value="bool">Bool</option>
            </select>
            <input v-model.number="mapping.scale" type="number" step="0.01">
            <input v-model.number="mapping.offset" type="number" step="0.01">
            <span class="range-inputs">
              <input v-model.number="mapping.min" type="number" step="0.01" placeholder="min">
              <input v-model.number="mapping.max" type="number" step="0.01" placeholder="max">
            </span>
            <code>{{ mappingPreview(mapping) }}</code>
            <button :title="l('删除映射', 'Delete mapping')" @click="mappings.splice(index, 1)"><Trash2 :size="14" /></button>
          </div>
        </div>
      </template>

      <template v-else>
        <div class="router-intro">
          <div>
            <strong>{{ l('OSC 路由器', 'OSC router') }}</strong>
            <span>{{ l('支持精确地址和以 * 结尾的前缀匹配，转发时保留原始参数类型', 'Supports exact addresses and prefix matching ending in *. Original parameter types are preserved.') }}</span>
          </div>
          <div>
            <button @click="addRoute"><Plus :size="15" /> {{ l('添加规则', 'Add rule') }}</button>
            <button class="primary" @click="applyRoutes"><Check :size="15" /> {{ l('应用规则', 'Apply rules') }}</button>
          </div>
        </div>

        <p v-if="routeStatus" class="inline-status">{{ routeStatus }}</p>

        <div class="route-table">
          <div class="route-row route-header">
            <span>{{ l('启用', 'Enabled') }}</span><span>{{ l('来源地址', 'Source address') }}</span><span>{{ l('目标主机', 'Target host') }}</span><span>{{ l('端口', 'Port') }}</span><span>{{ l('改写地址', 'Rewrite address') }}</span><span></span>
          </div>
          <div v-for="(rule, index) in routes" :key="index" class="route-row">
            <input v-model="rule.enabled" type="checkbox">
            <input v-model.trim="rule.sourceAddress" placeholder="/avatar/parameters/*">
            <input v-model.trim="rule.targetHost">
            <input v-model.number="rule.targetPort" type="number" min="1" max="65535">
            <input v-model.trim="rule.targetAddress" :placeholder="l('留空则保持原地址', 'Leave blank to keep original address')">
            <button :title="l('删除规则', 'Delete rule')" @click="routes.splice(index, 1)"><Trash2 :size="14" /></button>
          </div>
          <div v-if="routes.length === 0" class="route-empty">
            <Route :size="28" />
            <strong>{{ l('还没有路由规则', 'No routing rules yet') }}</strong>
            <span>{{ l('例如把 `/avatar/parameters/*` 转发到其他应用或设备', 'For example, forward `/avatar/parameters/*` to another app or device') }}</span>
          </div>
        </div>
      </template>
    </div>
  </section>
</template>

<style scoped>
.osc-workbench {
  grid-column: 1 / -1;
  min-width: 0;
  border: 1px solid var(--theme-border-soft);
  border-radius: 8px;
  color: var(--theme-text);
  background: color-mix(in srgb, var(--theme-surface) 94%, transparent);
  box-shadow: var(--theme-shadow-sm);
  overflow: hidden;
}

.osc-head,
.endpoint-bar,
.osc-tabs,
.osc-body {
  min-width: 0;
}

.osc-head {
  min-height: 68px;
  padding: 14px 16px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  border-bottom: 1px solid var(--theme-border-soft);
}

.osc-title,
.runtime-badges,
.endpoint-bar,
.endpoint-bar label,
.profile-actions,
.osc-tabs,
.section-heading,
.primary-actions,
.monitor-toolbar,
.heart-capture,
.toggle-row,
.auto-chat-controls,
.automation-toolbar,
.router-intro,
.router-intro > div,
.queue-head {
  display: flex;
  align-items: center;
}

.osc-title {
  min-width: 0;
  gap: 10px;
}

.title-icon {
  width: 36px;
  height: 36px;
  flex: 0 0 auto;
  display: grid;
  place-items: center;
  border-radius: 8px;
  color: var(--theme-primary);
  background: color-mix(in srgb, var(--theme-primary) 12%, var(--theme-surface));
}

.osc-title h3 {
  margin: 0;
  font-size: 18px;
  font-weight: 850;
}

.osc-title p,
.section-heading span,
.router-intro span {
  margin: 3px 0 0;
  color: var(--theme-text-muted);
  font-size: 11px;
  font-weight: 650;
}

.runtime-badges {
  justify-content: flex-end;
  flex-wrap: wrap;
  gap: 6px;
}

.runtime-badges span {
  min-height: 26px;
  padding: 0 8px;
  display: inline-flex;
  align-items: center;
  gap: 5px;
  border: 1px solid var(--theme-border-soft);
  border-radius: 6px;
  color: var(--theme-text-muted);
  background: var(--theme-surface-hover);
  font-size: 11px;
  font-weight: 750;
}

.runtime-badges span.active {
  color: #15803d;
  border-color: color-mix(in srgb, #22c55e 35%, transparent);
  background: color-mix(in srgb, #22c55e 10%, var(--theme-surface));
}

.endpoint-bar {
  min-height: 52px;
  padding: 8px 16px;
  gap: 14px;
  border-bottom: 1px solid var(--theme-border-soft);
  background: var(--theme-surface-hover);
}

.endpoint-bar label {
  min-width: 0;
  flex: 1;
  gap: 6px;
}

.endpoint-bar label > span,
.field > span,
.filter-field > span,
.automation-toolbar label > span {
  flex: 0 0 auto;
  color: var(--theme-text-muted);
  font-size: 11px;
  font-weight: 800;
}

input,
textarea,
select {
  min-width: 0;
  border: 1px solid var(--theme-border-soft);
  border-radius: 6px;
  color: var(--theme-text);
  background: var(--theme-surface);
  outline: none;
  font: inherit;
}

input,
select {
  min-height: 34px;
  padding: 0 9px;
}

textarea {
  padding: 9px 10px;
  line-height: 1.5;
  resize: vertical;
}

input:focus,
textarea:focus,
select:focus {
  border-color: color-mix(in srgb, var(--theme-primary) 65%, var(--theme-border-soft));
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--theme-primary) 12%, transparent);
}

.endpoint-bar label input:first-of-type {
  flex: 1;
}

.port-input {
  width: 84px;
}

.profile-actions {
  flex: 0 0 auto;
  gap: 6px;
}

button {
  min-height: 32px;
  border: 1px solid var(--theme-border-soft);
  border-radius: 6px;
  padding: 0 10px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  color: var(--theme-text-muted);
  background: var(--theme-surface);
  font: inherit;
  font-size: 12px;
  font-weight: 800;
  cursor: pointer;
  transition: border-color 0.16s ease, color 0.16s ease, background 0.16s ease;
}

button:hover:not(:disabled) {
  color: var(--theme-primary);
  border-color: color-mix(in srgb, var(--theme-primary) 40%, var(--theme-border-soft));
  background: var(--theme-surface-hover);
}

button:disabled {
  cursor: not-allowed;
  opacity: 0.45;
}

button.primary {
  color: white;
  border-color: var(--theme-primary);
  background: var(--theme-primary);
}

button.primary:hover:not(:disabled) {
  color: white;
  background: var(--theme-primary-hover);
}

button.danger {
  color: white;
  border-color: #dc2626;
  background: #dc2626;
}

.osc-tabs {
  padding: 0 12px;
  gap: 2px;
  border-bottom: 1px solid var(--theme-border-soft);
}

.osc-tabs button {
  min-height: 42px;
  border: 0;
  border-bottom: 2px solid transparent;
  border-radius: 0;
  padding: 0 14px;
  background: transparent;
}

.osc-tabs button.active {
  color: var(--theme-primary);
  border-bottom-color: var(--theme-primary);
}

.osc-body {
  min-height: 430px;
  padding: 16px;
}

.send-layout,
.chat-layout {
  display: grid;
  grid-template-columns: minmax(0, 1.5fr) minmax(280px, 0.8fr);
  gap: 16px;
}

.editor-pane,
.side-pane,
.chat-preview {
  min-width: 0;
}

.side-pane,
.chat-preview {
  padding-left: 16px;
  border-left: 1px solid var(--theme-border-soft);
}

.section-heading {
  min-height: 34px;
  justify-content: space-between;
  gap: 10px;
  margin-bottom: 10px;
}

.section-heading > div {
  min-width: 0;
  display: grid;
}

.section-heading strong,
.router-intro strong {
  font-size: 13px;
  font-weight: 850;
}

.field {
  min-width: 0;
  display: grid;
  gap: 6px;
  margin-bottom: 12px;
}

.segmented {
  min-width: 0;
  display: grid;
  grid-template-columns: repeat(5, minmax(0, 1fr));
  gap: 4px;
  padding: 4px;
  border-radius: 7px;
  background: var(--theme-surface-hover);
}

.segmented.compact {
  grid-template-columns: repeat(2, minmax(0, 1fr));
}

.segmented button {
  border: 0;
  background: transparent;
}

.segmented button.active {
  color: var(--theme-primary);
  background: var(--theme-surface);
  box-shadow: var(--theme-shadow-sm);
}

.primary-actions {
  min-width: 0;
  gap: 8px;
  margin-top: 14px;
}

.primary-actions input {
  flex: 1;
}

.inline-status {
  margin: 8px 16px;
  color: var(--theme-text-muted);
  font-size: 11px;
  font-weight: 750;
}

.osc-body > .inline-status,
.editor-pane > .inline-status {
  margin: 8px 0 0;
}

.inline-status.success {
  color: #15803d;
}

.inline-status.error {
  color: #dc2626;
}

.preset-list,
.history-list,
.queue-list {
  max-height: 180px;
  overflow-y: auto;
}

.preset-row {
  width: 100%;
  min-height: 46px;
  margin-bottom: 5px;
  padding: 6px 8px;
  justify-content: flex-start;
  text-align: left;
}

.preset-row > span {
  min-width: 0;
  flex: 1;
  display: grid;
}

.preset-row strong,
.preset-row small {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.preset-row small {
  margin-top: 2px;
  color: var(--theme-text-muted);
  font-size: 10px;
}

code {
  font-family: Consolas, "SFMono-Regular", monospace;
  font-size: 11px;
}

.history-heading {
  margin-top: 16px;
}

.history-list button {
  width: 100%;
  min-height: 30px;
  padding: 0 6px;
  justify-content: space-between;
  border-width: 0 0 1px;
  border-radius: 0;
  background: transparent;
}

.history-list button span {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.empty-text {
  margin: 18px 0;
  color: var(--theme-text-muted);
  font-size: 11px;
  text-align: center;
}

.monitor-toolbar,
.automation-toolbar {
  min-width: 0;
  gap: 8px;
  margin-bottom: 12px;
}

.filter-field {
  min-width: 220px;
  flex: 1;
  display: flex;
  align-items: center;
  gap: 7px;
}

.filter-field input {
  flex: 1;
}

.monitor-toolbar button.active {
  color: var(--theme-primary);
  border-color: var(--theme-primary);
}

.event-count {
  margin-left: auto;
  color: var(--theme-text-muted);
  font-size: 11px;
  font-weight: 800;
}

.monitor-table {
  min-height: 330px;
  max-height: 440px;
  overflow: auto;
  border: 1px solid var(--theme-border-soft);
  border-radius: 7px;
  background: var(--theme-terminal-bg, #111827);
}

.monitor-row {
  width: 100%;
  min-height: 34px;
  display: grid;
  grid-template-columns: 86px minmax(220px, 1.2fr) minmax(180px, 1fr) 150px;
  align-items: center;
  gap: 10px;
  border: 0;
  border-bottom: 1px solid rgba(255, 255, 255, 0.07);
  border-radius: 0;
  padding: 6px 10px;
  color: rgba(255, 255, 255, 0.78);
  background: transparent;
  text-align: left;
}

.monitor-row:hover:not(.monitor-header) {
  color: white;
  background: rgba(255, 255, 255, 0.06);
}

.monitor-header {
  position: sticky;
  top: 0;
  z-index: 1;
  color: rgba(255, 255, 255, 0.5);
  background: #111827;
  font-size: 10px;
  font-weight: 850;
}

.monitor-row time,
.monitor-row small {
  color: rgba(255, 255, 255, 0.48);
  font-size: 10px;
}

.event-values {
  min-width: 0;
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
}

.event-values code {
  padding: 2px 5px;
  border-radius: 4px;
  color: #a7f3d0;
  background: rgba(16, 185, 129, 0.12);
}

.monitor-empty,
.route-empty {
  min-height: 260px;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 7px;
  color: rgba(255, 255, 255, 0.32);
}

.monitor-empty span,
.route-empty span {
  font-size: 11px;
}

.heart-capture {
  min-width: 0;
  gap: 10px;
  margin-top: 10px;
  padding: 9px 10px;
  border: 1px solid color-mix(in srgb, #ef4444 22%, var(--theme-border-soft));
  border-radius: 7px;
  color: #dc2626;
  background: color-mix(in srgb, #ef4444 6%, var(--theme-surface));
}

.heart-capture label {
  min-width: 0;
  flex: 1;
  display: flex;
  align-items: center;
  gap: 8px;
}

.heart-capture label span {
  flex: 0 0 auto;
  font-size: 11px;
  font-weight: 800;
}

.heart-capture input {
  flex: 1;
}

.char-count {
  font-family: Consolas, monospace;
  font-size: 11px;
  font-weight: 800;
}

.char-count.warn {
  color: #dc2626;
}

.variable-bar {
  display: flex;
  flex-wrap: wrap;
  gap: 5px;
  margin: -2px 0 12px;
}

.variable-bar button {
  min-height: 26px;
  padding: 0 7px;
  color: var(--theme-primary);
  font-family: Consolas, monospace;
  font-size: 10px;
}

.toggle-row {
  flex-wrap: wrap;
  gap: 16px;
  color: var(--theme-text-muted);
  font-size: 11px;
  font-weight: 750;
}

.toggle-row label {
  display: inline-flex;
  align-items: center;
  gap: 6px;
}

.toggle-row input,
.mapping-row > input[type="checkbox"],
.route-row > input[type="checkbox"] {
  min-height: auto;
  accent-color: var(--theme-primary);
}

.chat-preview pre {
  min-height: 150px;
  margin: 0 0 12px;
  padding: 12px;
  overflow: auto;
  border: 1px solid var(--theme-border-soft);
  border-radius: 7px;
  color: var(--theme-text);
  background: var(--theme-surface-hover);
  font: 600 13px/1.55 system-ui, sans-serif;
  white-space: pre-wrap;
  word-break: break-word;
}

.auto-chat-controls {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  align-items: center;
  gap: 7px;
}

.auto-chat-controls .segmented {
  width: 100%;
  grid-column: 1 / -1;
}

.auto-chat-controls > .primary {
  justify-self: end;
}

.auto-chat-controls label,
.automation-toolbar label {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  color: var(--theme-text-muted);
  font-size: 11px;
  font-weight: 800;
}

.auto-chat-controls label input,
.automation-toolbar label input {
  width: 68px;
}

.queue-head {
  justify-content: space-between;
  margin: 16px 0 6px;
  font-size: 12px;
}

.queue-head button {
  min-height: 26px;
}

.queue-list > div {
  min-height: 34px;
  padding: 5px 4px;
  display: flex;
  align-items: center;
  gap: 6px;
  border-bottom: 1px solid var(--theme-border-soft);
}

.queue-list > div span {
  min-width: 0;
  flex: 1;
  overflow: hidden;
  color: var(--theme-text-muted);
  font-size: 11px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.queue-list > div button {
  min-width: 26px;
  min-height: 26px;
  padding: 0;
}

.metric-strip {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 1px;
  margin-bottom: 12px;
  overflow: hidden;
  border: 1px solid var(--theme-border-soft);
  border-radius: 7px;
  background: var(--theme-border-soft);
}

.metric-strip > div {
  min-width: 0;
  min-height: 72px;
  padding: 9px 11px;
  display: grid;
  grid-template-columns: auto 1fr;
  align-items: center;
  column-gap: 7px;
  color: var(--theme-text-muted);
  background: var(--theme-surface);
}

.metric-strip span {
  font-size: 10px;
  font-weight: 800;
}

.metric-strip strong {
  grid-column: 1 / -1;
  overflow: hidden;
  color: var(--theme-text);
  font-size: 13px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.metric-strip small {
  grid-column: 1 / -1;
  overflow: hidden;
  color: var(--theme-text-faint);
  font-size: 10px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.hardware-profile {
  margin-bottom: 12px;
  border-top: 1px solid var(--theme-border-soft);
  border-bottom: 1px solid var(--theme-border-soft);
}

.hardware-profile-head {
  min-height: 40px;
  padding: 5px 2px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.hardware-profile-head > div {
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 7px;
  color: var(--theme-text);
  font-size: 12px;
}

.hardware-profile-head span {
  padding: 2px 6px;
  border: 1px solid color-mix(in srgb, var(--theme-success) 34%, transparent);
  border-radius: 999px;
  color: var(--theme-success);
  background: color-mix(in srgb, var(--theme-success) 8%, transparent);
  font-size: 9px;
  font-weight: 800;
}

.hardware-profile dl {
  margin: 0;
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  column-gap: 18px;
}

.hardware-profile dl > div {
  min-width: 0;
  min-height: 38px;
  display: grid;
  grid-template-columns: 52px minmax(0, 1fr);
  align-items: center;
  gap: 8px;
  border-top: 1px solid var(--theme-border-soft);
}

.hardware-profile dt {
  color: var(--theme-text-faint);
  font-size: 10px;
  font-weight: 800;
}

.hardware-profile dd {
  min-width: 0;
  margin: 0;
  overflow: hidden;
  color: var(--theme-text);
  font-size: 11px;
  font-weight: 700;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.automation-toolbar {
  justify-content: flex-end;
}

.mapping-table,
.route-table {
  overflow-x: auto;
  border: 1px solid var(--theme-border-soft);
  border-radius: 7px;
}

.mapping-row {
  min-width: 1080px;
  min-height: 46px;
  padding: 6px 8px;
  display: grid;
  grid-template-columns: 42px 170px minmax(230px, 1fr) 80px 72px 72px 150px 72px 34px;
  align-items: center;
  gap: 6px;
  border-bottom: 1px solid var(--theme-border-soft);
}

.mapping-header,
.route-header {
  min-height: 34px;
  color: var(--theme-text-muted);
  background: var(--theme-surface-hover);
  font-size: 10px;
  font-weight: 850;
}

.mapping-row select,
.mapping-row input,
.route-row input {
  width: 100%;
  min-height: 31px;
  font-size: 11px;
}

.range-inputs {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 4px;
}

.mapping-row > button,
.route-row > button {
  min-width: 30px;
  padding: 0;
}

.router-intro {
  justify-content: space-between;
  gap: 14px;
  margin-bottom: 12px;
}

.router-intro > div:first-child {
  min-width: 0;
  display: grid;
}

.router-intro > div:last-child {
  gap: 7px;
}

.route-row {
  min-width: 820px;
  min-height: 46px;
  padding: 6px 8px;
  display: grid;
  grid-template-columns: 42px minmax(220px, 1fr) 150px 82px minmax(220px, 1fr) 34px;
  align-items: center;
  gap: 6px;
  border-bottom: 1px solid var(--theme-border-soft);
}

.route-empty {
  min-height: 280px;
  color: var(--theme-text-muted);
}

.spin {
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

@media (max-width: 1100px) {
  .osc-head,
  .endpoint-bar {
    align-items: stretch;
    flex-direction: column;
  }

  .runtime-badges {
    justify-content: flex-start;
  }

  .profile-actions {
    align-self: flex-end;
  }

  .send-layout,
  .chat-layout {
    grid-template-columns: 1fr;
  }

  .side-pane,
  .chat-preview {
    padding-top: 16px;
    padding-left: 0;
    border-top: 1px solid var(--theme-border-soft);
    border-left: 0;
  }

  .metric-strip {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}

@media (max-width: 720px) {
  .endpoint-bar label,
  .monitor-toolbar,
  .automation-toolbar,
  .router-intro,
  .router-intro > div:last-child {
    align-items: stretch;
    flex-direction: column;
  }

  .profile-actions {
    width: 100%;
    align-self: stretch;
  }

  .profile-actions button {
    flex: 1;
  }

  .osc-tabs {
    overflow-x: auto;
  }

  .osc-tabs button {
    flex: 0 0 auto;
  }

  .segmented {
    grid-template-columns: repeat(3, minmax(0, 1fr));
  }

  .primary-actions {
    align-items: stretch;
    flex-direction: column;
  }

  .metric-strip {
    grid-template-columns: 1fr;
  }

  .hardware-profile dl {
    grid-template-columns: 1fr;
  }
}
</style>
