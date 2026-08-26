<script setup lang="ts">
import CustomSelect from './CustomSelect.vue';
import { ref, computed, onMounted, onUnmounted, watch } from 'vue';
import { Languages, ScanEye, Settings, Save, Layers, Cpu, Check, PlaySquare, Watch, Eye, EyeOff, GripVertical, RotateCcw, HelpCircle, Info, MessageSquare, Gamepad2, Hand, Globe, User, Loader2, X, Headphones, MonitorSpeaker, Volume2, VolumeX, Mic, MicOff, Sun, Palette, Sliders, Gauge, Move3d, Compass, ArrowUpDown, BarChart3, Timer, Keyboard, Bell, SkipBack, Play, SquareIcon, SkipForward, Power, BatteryCharging, RefreshCw, Shield, Crosshair, Box, Footprints, Rotate3d, Move, Activity, Wind, Monitor, ChevronDown, ChevronRight } from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';
import { DbApi, OvrApi } from '../api';
import { currentTheme, setTheme, themes, type ThemeId } from '../theme';
import OvrAdvPanels from './OvrAdvPanels.vue';
import OvrAdvVrDashPanels from './OvrAdvVrDashPanels.vue';

const { t } = useI18n();

const translationProviderOptions = [
  { label: 'Google Translate Free', value: 'google_free' },
  { label: 'Microsoft Translator', value: 'microsoft' },
  { label: 'DeepL Free', value: 'deepl_free' },
  { label: 'DeepL Pro', value: 'deepl' },
  { label: 'Tencent Translate', value: 'tencent' },
  { label: 'Baidu Translate', value: 'baidu' },
  { label: 'Papago', value: 'papago' },
  { label: 'Gemini', value: 'gemini' },
  { label: 'OpenAI', value: 'openai' },
  { label: 'DeepSeek', value: 'deepseek' },
  { label: 'SiliconFlow', value: 'siliconflow' },
  { label: 'Moonshot', value: 'moonshot' },
  { label: 'ZhiPu GLM', value: 'zhipu' },
  { label: 'Groq', value: 'groq' },
  { label: 'OpenRouter', value: 'openrouter' },
  { label: 'Plamo', value: 'plamo' },
  { label: 'Ollama Local', value: 'ollama' },
  { label: 'LM Studio Local', value: 'lmstudio' },
  { label: 'Custom OpenAI API', value: 'custom_llm' },
];

const llmPromptServices = [
  'openai',
  'deepseek',
  'siliconflow',
  'moonshot',
  'zhipu',
  'groq',
  'openrouter',
  'plamo',
  'ollama',
  'lmstudio',
  'custom_llm',
  'gemini',
];

const apiTestUrls: Record<string, string> = {
  deepseek: 'https://api.deepseek.com/models',
  openai: 'https://api.openai.com/v1/models',
  groq: 'https://api.groq.com/openai/v1/models',
  openrouter: 'https://openrouter.ai/api/v1/models',
  plamo: 'https://api.platform.preferredai.jp/v1/models',
  gemini: 'https://generativelanguage.googleapis.com/v1beta/models',
};

const activeSubTab = ref('basic');
const isSaving = ref(false);
const saved = ref(false);

const createDefaultConfig = () => ({
  general: {
    enabled: true,
    dualDisplay: true,
    theme: 'dark',
    wristMode: false,
    triggerKey: 'trigger',
    clearKey: 'left_stick',
    ocrSharpen: true,
    ocrDenoise: false,
    ocrMergeHeightTol: 0.2,
    ocrMergeWidthTol: 0.1,
    transMode: 'builtin',
    transService: 'tencent',
    transSourceLang: 'auto',
    transTargetLang: 'zh',
    transApiKey: '',
     transLlmModel: '',
     transLlmPrompt: t('ovr.trans_llm_prompt_default'),
     overlayTextColor: '#FFFFFF',
     overlayBgColor: '#101826',
     overlayBgOpacity: 0.46,
     overlayGlass: true,
     overlayBorderOpacity: 0.42,
     overlayCornerRadius: 18,
     overlayShadowStrength: 0.35,
     transPanelMaxWidth: 640,
     overlayFontSize: 30,
     overlayStatusColor: '#00FF00',
     overlayLockMode: 'world',
     menuWidthM: 0.55,
     menuOffsetX: 0,
     menuOffsetY: -0.06,
     menuOffsetZ: -0.75,
     resultWidthM: 0.72,
     resultOffsetX: 0,
     resultOffsetY: -0.42,
     resultOffsetZ: -1.10,
     scanFrameWidthM: 0.34,
     scanFrameDistanceM: 0.72,
    advCpuAccel: true,
    advGpuAccel: false,
    advDebugMode: false,
    advAutoStart: false,
    desktopMode: false,         
    autoScanEnabled: false,     
    autoScanInterval: 5,        // 鑷姩鎵弿闂撮殧 (绉?,
    ttsEnabled: true,           
    oscChatboxEnabled: true,    
    spaceAdjustChap: true,
    motionDragComfort: 0,
    motionDragForceBounds: true,
    motionDragMultiplier: 1.0,
    motionGravityOn: false,
    motionGravityStrength: 9.81,
    motionGravityFriction: 0.1,
    motionSaveMomentum: false,
    motionFlingStrength: 1.0,
    captureMode: 'dynamic',
    captureQuality: 'balanced',
    captureAutoSave: false,
    captureFormat: 'png',
    rotationAutoTurn: false,
    rotationActivationDist: 0.5,
    rotationDeactivationDist: 0.3,
    rotationUseCornerAngle: true,
    rotationUseSmooth: false,
    rotationTurnSpeed: 90,
    rotationRedirectedWalk: false,
    rotationRedirectRadius: 5.0,
    rotationViewRatchet: 0,
    rotationSpaceTurnLeft: true,
    rotationSpaceTurnRight: true,
    rotationTurnComfort: 0,
    rotationTurnForceBounds: true,
    rotationSnapTurnAngle: 45,
    rotationSmoothTurnRate: 100,
    snapTurnEnabled: true,
    smoothTurnEnabled: false,
    lockXEnabled: false,
    lockYEnabled: false,
    lockZEnabled: false,
    comfortTurnEnabled: false,
    settingsUniverseRotation: false,
    settingsCrashRecovery: true,
    settingsVersionCheck: false,
    settingsForceSteamvrChap: false,
    settingsAutoChapProfile: false,
    settingsForceDisableOculus: false,
    settingsExclusiveInput: false,
    settingsDisableVsync: false,
  },
  ocr: {
    model: 'zh-en-ja',
    speedMode: 'standard',
    enhanceContrast: false,
  },
  chaperone: {
    visibility: 70,
    fadeDistance: 0.7,
    height: 2.5,
    centerMarker: false,
    playspaceMarker: false,
    forceBounds: false,
    disable: false,
    beginnerMode: false,
    hapticFeedback: false,
    audioWarning: false,
    loopAudio: false,
    audioVolume: false,
    openDashboard: false,
    colorR: 0,
    colorG: 255,
    colorB: 128,
    colorHex: '#00FF80',
    floorAlways: false,
    activationDistance: 0.5,
  },
  playspace: {
    offsetX: 0,
    offsetY: 0,
    offsetZ: 0,
    rotation: 0,
    dragLeft: true,
    dragRight: true,
    heightToggle: false,
    heightOffset: 0.3,
  },
  video: {
    brightnessOn: false,
    brightnessValue: 100,
    colorR: 100,
    colorG: 100,
    colorB: 100,
    sSOverride: false,
    superSampling: 1.0,
    motionSmooth: true,
    advSSFilter: true,
    overlayColor: false,
    overlayOpacity: 0.5,
  },
  audio: {
    playbackOverride: false,
    mirrorVolume: 100,
    mirrorMute: false,
    micOverride: false,
    micVolume: 100,
    micMute: false,
    proxSensor: false,
    pTT: false,
    pTTNotif: true,
    pTM: false,
  },
  steamvr: {
    timingOverlay: false,
    multiDriver: false,
    disableNotifs: false,
    requireHmd: true,
    systemButton: false,
    controllerPower: true,
    noFadeGrid: false,
    cameraEnable: false,
    cameraBounds: false,
    cameraController: false,
    perAppBinds: false,
  },
  utilities: {
    mediaKeys: true,
    keyShortcut1: 'Ctrl+Shift+M',
    keyShortcut2: '',
    keyShortcut3: '',
    alarmEnabled: false,
    alarmTime: '08:00',
    trackerBattery: false,
    keyboard: false,
  },
});

const config = ref(createDefaultConfig());

const legacyConfigKeyMap: Record<string, [string, string]> = {
  ovrEnabled: ['general', 'enabled'],
  ovrDualDisplay: ['general', 'dualDisplay'],
  ovrTheme: ['general', 'theme'],
  ovrWristMode: ['general', 'wristMode'],
  ovrTriggerKey: ['general', 'triggerKey'],
  ovrClearKey: ['general', 'clearKey'],
  ocrModel: ['ocr', 'model'],
  ocrSpeedMode: ['ocr', 'speedMode'],
  ocrEnhanceContrast: ['ocr', 'enhanceContrast'],
  ocrSharpen: ['general', 'ocrSharpen'],
  ocrDenoise: ['general', 'ocrDenoise'],
  ocrMergeHeightTol: ['general', 'ocrMergeHeightTol'],
  ocrMergeWidthTol: ['general', 'ocrMergeWidthTol'],
  transMode: ['general', 'transMode'],
  transService: ['general', 'transService'],
  transSourceLang: ['general', 'transSourceLang'],
  transTargetLang: ['general', 'transTargetLang'],
  transApiKey: ['general', 'transApiKey'],
  transLlmModel: ['general', 'transLlmModel'],
  transLlmPrompt: ['general', 'transLlmPrompt'],
  overlayTextColor: ['general', 'overlayTextColor'],
  overlayBgColor: ['general', 'overlayBgColor'],
  overlayBgOpacity: ['general', 'overlayBgOpacity'],
  overlayGlass: ['general', 'overlayGlass'],
  overlayBorderOpacity: ['general', 'overlayBorderOpacity'],
  overlayCornerRadius: ['general', 'overlayCornerRadius'],
  overlayShadowStrength: ['general', 'overlayShadowStrength'],
  transPanelMaxWidth: ['general', 'transPanelMaxWidth'],
  overlayFontSize: ['general', 'overlayFontSize'],
  overlayStatusColor: ['general', 'overlayStatusColor'],
  overlayLockMode: ['general', 'overlayLockMode'],
  menuWidthM: ['general', 'menuWidthM'],
  menu_width_m: ['general', 'menuWidthM'],
  menuOffsetX: ['general', 'menuOffsetX'],
  menu_offset_x: ['general', 'menuOffsetX'],
  menuOffsetY: ['general', 'menuOffsetY'],
  menu_offset_y: ['general', 'menuOffsetY'],
  menuOffsetZ: ['general', 'menuOffsetZ'],
  menu_offset_z: ['general', 'menuOffsetZ'],
  resultWidthM: ['general', 'resultWidthM'],
  result_width_m: ['general', 'resultWidthM'],
  resultOffsetX: ['general', 'resultOffsetX'],
  result_offset_x: ['general', 'resultOffsetX'],
  resultOffsetY: ['general', 'resultOffsetY'],
  result_offset_y: ['general', 'resultOffsetY'],
  resultOffsetZ: ['general', 'resultOffsetZ'],
  result_offset_z: ['general', 'resultOffsetZ'],
  scanFrameWidthM: ['general', 'scanFrameWidthM'],
  scan_frame_width_m: ['general', 'scanFrameWidthM'],
  scanFrameDistanceM: ['general', 'scanFrameDistanceM'],
  scan_frame_distance_m: ['general', 'scanFrameDistanceM'],
  overlay_lock_mode: ['general', 'overlayLockMode'],
  overlay_bg_opacity: ['general', 'overlayBgOpacity'],
  dual_display: ['general', 'dualDisplay'],
  wrist_mode: ['general', 'wristMode'],
  tts_enabled: ['general', 'ttsEnabled'],
  osc_chatbox_enabled: ['general', 'oscChatboxEnabled'],
  height_toggle_enabled: ['playspace', 'heightToggle'],
  playspace_offset_x: ['playspace', 'offsetX'],
  playspace_offset_y: ['playspace', 'offsetY'],
  playspace_offset_z: ['playspace', 'offsetZ'],
  playspace_rotation: ['playspace', 'rotation'],
  advCpuAccel: ['general', 'advCpuAccel'],
  advGpuAccel: ['general', 'advGpuAccel'],
  advDebugMode: ['general', 'advDebugMode'],
  advAutoStart: ['general', 'advAutoStart'],
  spaceAdjustChap: ['general', 'spaceAdjustChap'],
  motionGravity: ['general', 'motionGravityOn'],
  motionGravityOn: ['general', 'motionGravityOn'],
  motionGravityStrength: ['general', 'motionGravityStrength'],
  motionGravityFriction: ['general', 'motionGravityFriction'],
  motionSaveMomentum: ['general', 'motionSaveMomentum'],
  motionFlingStrength: ['general', 'motionFlingStrength'],
  rotationAutoTurn: ['general', 'rotationAutoTurn'],
  rotationActivationDist: ['general', 'rotationActivationDist'],
  rotationDeactivationDist: ['general', 'rotationDeactivationDist'],
  rotationUseCornerAngle: ['general', 'rotationUseCornerAngle'],
  rotationUseSmooth: ['general', 'rotationUseSmooth'],
  rotationTurnSpeed: ['general', 'rotationTurnSpeed'],
  rotationRedirectedWalk: ['general', 'rotationRedirectedWalk'],
  rotationRedirectRadius: ['general', 'rotationRedirectRadius'],
  rotationViewRatchet: ['general', 'rotationViewRatchet'],
  rotationSpaceTurnLeft: ['general', 'rotationSpaceTurnLeft'],
  rotationSpaceTurnRight: ['general', 'rotationSpaceTurnRight'],
  rotationTurnComfort: ['general', 'rotationTurnComfort'],
  rotationTurnForceBounds: ['general', 'rotationTurnForceBounds'],
  rotationSnapTurnAngle: ['general', 'rotationSnapTurnAngle'],
  rotationSmoothTurnRate: ['general', 'rotationSmoothTurnRate'],
  settingsUniverseRotation: ['general', 'settingsUniverseRotation'],
  settingsCrashRecovery: ['general', 'settingsCrashRecovery'],
  settingsVersionCheck: ['general', 'settingsVersionCheck'],
  settingsForceSteamvrChap: ['general', 'settingsForceSteamvrChap'],
  settingsAutoChapProfile: ['general', 'settingsAutoChapProfile'],
  settingsForceDisableOculus: ['general', 'settingsForceDisableOculus'],
  settingsExclusiveInput: ['general', 'settingsExclusiveInput'],
  settingsDisableVsync: ['general', 'settingsDisableVsync'],
  steamvrTimingOverlay: ['steamvr', 'timingOverlay'],
  steamvrMultiDriver: ['steamvr', 'multiDriver'],
  steamvrDisableNotifs: ['steamvr', 'disableNotifs'],
  steamvrRequireHmd: ['steamvr', 'requireHmd'],
  steamvrSystemButton: ['steamvr', 'systemButton'],
  steamvrControllerPower: ['steamvr', 'controllerPower'],
  steamvrNoFadeGrid: ['steamvr', 'noFadeGrid'],
  steamvrCameraEnable: ['steamvr', 'cameraEnable'],
  steamvrCameraBounds: ['steamvr', 'cameraBounds'],
  steamvrCameraController: ['steamvr', 'cameraController'],
  steamvrPerAppBinds: ['steamvr', 'perAppBinds'],
  chapVisibility: ['chaperone', 'visibility'],
  chapFadeDistance: ['chaperone', 'fadeDistance'],
  chapHeight: ['chaperone', 'height'],
  chapCenterMarker: ['chaperone', 'centerMarker'],
  chapPlayspaceMarker: ['chaperone', 'playspaceMarker'],
  chapForceBounds: ['chaperone', 'forceBounds'],
  chapDisable: ['chaperone', 'disable'],
  chapBeginnerMode: ['chaperone', 'beginnerMode'],
  chapHapticFeedback: ['chaperone', 'hapticFeedback'],
  chapAudioWarning: ['chaperone', 'audioWarning'],
  chapLoopAudio: ['chaperone', 'loopAudio'],
  chapAudioVolume: ['chaperone', 'audioVolume'],
  chapOpenDashboard: ['chaperone', 'openDashboard'],
  chapColorR: ['chaperone', 'colorR'],
  chapColorG: ['chaperone', 'colorG'],
  chapColorB: ['chaperone', 'colorB'],
  chapFloorAlways: ['chaperone', 'floorAlways'],
  chapActivationDistance: ['chaperone', 'activationDistance'],
  spaceOffsetX: ['playspace', 'offsetX'],
  spaceOffsetY: ['playspace', 'offsetY'],
  spaceOffsetZ: ['playspace', 'offsetZ'],
  spaceRotation: ['playspace', 'rotation'],
  motionDragLeft: ['playspace', 'dragLeft'],
  motionDragRight: ['playspace', 'dragRight'],
  motionHeightToggle: ['playspace', 'heightToggle'],
  motionHeightOffset: ['playspace', 'heightOffset'],
  audioPlaybackOverride: ['audio', 'playbackOverride'],
  audioMirrorVolume: ['audio', 'mirrorVolume'],
  audioMirrorMute: ['audio', 'mirrorMute'],
  audioMicOverride: ['audio', 'micOverride'],
  audioMicVolume: ['audio', 'micVolume'],
  audioMicMute: ['audio', 'micMute'],
  audioProxSensor: ['audio', 'proxSensor'],
  audioPTT: ['audio', 'pTT'],
  audioPTTNotif: ['audio', 'pTTNotif'],
  audioPTM: ['audio', 'pTM'],
  videoBrightnessOn: ['video', 'brightnessOn'],
  videoBrightnessValue: ['video', 'brightnessValue'],
  videoColorR: ['video', 'colorR'],
  videoColorG: ['video', 'colorG'],
  videoColorB: ['video', 'colorB'],
  videoSSOverride: ['video', 'sSOverride'],
  videoSSValue: ['video', 'superSampling'],
  videoSuperSampling: ['video', 'superSampling'],
  videoMotionSmooth: ['video', 'motionSmooth'],
  videoAdvSSFilter: ['video', 'advSSFilter'],
  videoOverlayColor: ['video', 'overlayColor'],
  videoOverlayOpacity: ['video', 'overlayOpacity'],
  utilMediaKeys: ['utilities', 'mediaKeys'],
  utilKeyShortcut1: ['utilities', 'keyShortcut1'],
  utilKeyShortcut2: ['utilities', 'keyShortcut2'],
  utilKeyShortcut3: ['utilities', 'keyShortcut3'],
  utilAlarmEnabled: ['utilities', 'alarmEnabled'],
  utilAlarmTime: ['utilities', 'alarmTime'],
  utilTrackerBattery: ['utilities', 'trackerBattery'],
};

const isPlainObject = (value: unknown): value is Record<string, unknown> =>
  Boolean(value) && typeof value === 'object' && !Array.isArray(value);

const parseStoredValue = (value: unknown) => {
  if (typeof value !== 'string') return value;

  try {
    return JSON.parse(value);
  } catch {
    return value;
  }
};

const setConfigValue = (section: string, key: string, value: unknown) => {
  const target = (config.value as Record<string, any>)[section];
  if (isPlainObject(target)) {
    target[key] = value;
  }
};

const applyConfigPatch = (patch: Record<string, unknown>) => {
  const root = config.value as Record<string, any>;

  for (const [key, rawValue] of Object.entries(patch)) {
    const value = parseStoredValue(rawValue);
    const legacyTarget = legacyConfigKeyMap[key];

    if (legacyTarget) {
      setConfigValue(legacyTarget[0], legacyTarget[1], value);
      continue;
    }

    if (!(key in root)) continue;

    if (isPlainObject(root[key]) && isPlainObject(value)) {
      Object.assign(root[key], value);
    } else {
      root[key] = value;
    }
  }
};

const ovrLayoutGroups = [
  {
    id: 'menu',
    title: '内部菜单窗口',
    description: '推荐在头显前方偏下，不挡视线；VR 内可用右 Grip + 双摇杆/触控板微调。',
    controls: [
      { key: 'menuWidthM', label: '宽度', min: 0.25, max: 1.4, step: 0.01, unit: 'm', digits: 2 },
      { key: 'menuOffsetX', label: '水平', min: -1.5, max: 1.5, step: 0.01, unit: 'm', digits: 2 },
      { key: 'menuOffsetY', label: '高度', min: -0.9, max: 0.8, step: 0.01, unit: 'm', digits: 2 },
      { key: 'menuOffsetZ', label: '距离', min: -2.5, max: -0.25, step: 0.01, unit: 'm', digits: 2 },
    ],
  },
  {
    id: 'result',
    title: '底部翻译结果',
    description: '默认落在 VR 视野底部，抬眼能看见但不贴脸；右 Grip 可在结果显示时微调。',
    controls: [
      { key: 'resultWidthM', label: '宽度', min: 0.25, max: 1.6, step: 0.01, unit: 'm', digits: 2 },
      { key: 'resultOffsetX', label: '水平', min: -1.5, max: 1.5, step: 0.01, unit: 'm', digits: 2 },
      { key: 'resultOffsetY', label: '高度', min: -1.2, max: 0.6, step: 0.01, unit: 'm', digits: 2 },
      { key: 'resultOffsetZ', label: '距离', min: -2.8, max: -0.3, step: 0.01, unit: 'm', digits: 2 },
    ],
  },
  {
    id: 'scan',
    title: '右手截图框',
    description: '绿色田字格只在右扳机按住时出现，拉动手柄改变框大小，松开后执行 OCR 翻译。',
    controls: [
      { key: 'scanFrameWidthM', label: '初始大小', min: 0.12, max: 1.6, step: 0.01, unit: 'm', digits: 2 },
      { key: 'scanFrameDistanceM', label: '瞄准距离', min: 0.3, max: 2.0, step: 0.01, unit: 'm', digits: 2 },
    ],
  },
] as const;

const ovrLayoutPresets: Record<string, Record<string, number>> = {
  menu: {
    menuWidthM: 0.55,
    menuOffsetX: 0,
    menuOffsetY: -0.06,
    menuOffsetZ: -0.75,
  },
  result: {
    resultWidthM: 0.72,
    resultOffsetX: 0,
    resultOffsetY: -0.42,
    resultOffsetZ: -1.10,
  },
  scan: {
    scanFrameWidthM: 0.34,
    scanFrameDistanceM: 0.72,
  },
  all: {
    menuWidthM: 0.55,
    menuOffsetX: 0,
    menuOffsetY: -0.06,
    menuOffsetZ: -0.75,
    resultWidthM: 0.72,
    resultOffsetX: 0,
    resultOffsetY: -0.42,
    resultOffsetZ: -1.10,
    scanFrameWidthM: 0.34,
    scanFrameDistanceM: 0.72,
  },
};

const generalNumber = (key: string) => {
  const value = (config.value.general as Record<string, unknown>)[key];
  const numberValue = Number(value);
  return Number.isFinite(numberValue) ? numberValue : 0;
};

const setGeneralNumber = (key: string, value: unknown) => {
  const numberValue = Number(value);
  if (Number.isFinite(numberValue)) {
    (config.value.general as unknown as Record<string, number>)[key] = numberValue;
  }
};

const setGeneralNumberFromEvent = (key: string, event: Event) => {
  setGeneralNumber(key, (event.target as HTMLInputElement).value);
};

const applyOvrLayoutPreset = (id: string) => {
  const preset = ovrLayoutPresets[id];
  if (!preset) return;
  Object.assign(config.value.general, preset);
  void syncConfigToBackend();
};

const patchOvrLayoutFromBackend = (payload: Record<string, unknown>) => {
  for (const [snakeKey, rawValue] of Object.entries(payload)) {
    const target = legacyConfigKeyMap[snakeKey];
    if (!target || target[0] !== 'general') continue;
    const numberValue = Number(rawValue);
    if (Number.isFinite(numberValue)) {
      setConfigValue('general', target[1], numberValue);
    }
  }
};

// ========== VR Preview Simulator ==========
const showVrPreview = ref(true);
// Simulated translated bounds in VR view
const vrShowOriginal = ref(false);

// Pre-generate random particle styles for performance (avoids Math.random() in template)
const vrParticles = ref(Array.from({ length: 20 }, () => ({
  left: (Math.random() * 100) + '%',
  top: (Math.random() * 80) + '%',
  animationDelay: (Math.random() * 8) + 's',
  animationDuration: (4 + Math.random() * 6) + 's'
})));

const vrStatusText = ref('Ready');
const vrIsScanning = ref(false);
const vrDashboardOpen = ref(true);

// VR dashboard tab list (OVR Settings only)
import { markRaw } from 'vue';

const vrDashboardTabs = computed(() => {
  return [
    { key: 'basic', icon: markRaw(Settings), label: 'ovr.tab_basic' },
    { key: 'desktop', icon: markRaw(MonitorSpeaker), label: 'ovr.tab_desktop' },
    { key: 'ocr', icon: markRaw(ScanEye), label: 'ovr.tab_ocr' },
    { key: 'trans', icon: markRaw(Languages), label: 'ovr.tab_trans' },
    { key: 'overlay', icon: markRaw(Layers), label: 'ovr.tab_overlay' },
    { key: 'adv', icon: markRaw(Cpu), label: 'ovr.tab_adv' },
    { key: 'steamvr', icon: markRaw(Power), label: 'ovr.tab_steamvr' },
    { key: 'chaperone', icon: markRaw(Shield), label: 'ovr.tab_chaperone' },
    { key: 'playspace', icon: markRaw(Move3d), label: 'ovr.tab_playspace' },
    { key: 'audio', icon: markRaw(Headphones), label: 'ovr.tab_audio' },
    { key: 'video', icon: markRaw(Sun), label: 'ovr.tab_video' },
    { key: 'utilities', icon: markRaw(Keyboard), label: 'ovr.tab_utilities' },
    { key: 'statistics', icon: markRaw(BarChart3), label: 'ovr.tab_statistics' },
    { key: 'lang', icon: markRaw(MessageSquare), label: 'ovr.tab_lang' },
    { key: 'help', icon: markRaw(HelpCircle), label: 'ovr.tab_help' },
    { key: 'about', icon: markRaw(Info), label: 'ovr.tab_about' }
  ];
});

const vrDashboardTab = ref('basic');
const vrLogoLoadFailed = ref(false);

watch(() => currentTheme.value.id, () => {
  vrLogoLoadFailed.value = false;
});

// Sample OCR demo content
const ocrSamples = [
  { original: 'Welcome to this world!\nPlease enjoy your stay.', translated: t('ovr.sim_text_1_trans') },
  { original: 'Press trigger to interact\nwith objects around you.', translated: t('ovr.sim_text_2_trans') },
  { original: 'This avatar is private.\nYou cannot clone it.', translated: t('ovr.sim_text_3_trans') },
  { original: 'Instance capacity: 20/40\nRegion: US West', translated: t('ovr.sim_text_4_trans') },
];
const currentSampleIdx = ref(0);
const currentSample = computed(() => ocrSamples[currentSampleIdx.value]);

// Simulate OCR scan animation
let scanTimeout1: ReturnType<typeof setTimeout>;
let scanTimeout2: ReturnType<typeof setTimeout>;
let saveTimeout: ReturnType<typeof setTimeout>;

const triggerScan = () => {
  if (!config.value.general.enabled || vrIsScanning.value) return;
  vrIsScanning.value = true;
  vrStatusText.value = 'Scanning...';
  
  triggerDesktopScan(); // Execute real backend scan

  scanTimeout1 = setTimeout(() => {
    currentSampleIdx.value = (currentSampleIdx.value + 1) % ocrSamples.length;
    vrStatusText.value = 'Translated';
    vrIsScanning.value = false;
    scanTimeout2 = setTimeout(() => { vrStatusText.value = 'Ready'; }, 2000);
  }, 1200);
};

const toggleOriginal = () => {
  if (config.value.general.dualDisplay) {
    vrShowOriginal.value = !vrShowOriginal.value;
  }
};

// Overlay computed styles bound to config
const overlayBgStyle = computed(() => {
  const raw = config.value.general.overlayBgColor || '#101826';
  const hex = /^#[0-9a-fA-F]{6}$/.test(raw) ? raw : '#101826';
  const r = parseInt(hex.slice(1, 3), 16);
  const g = parseInt(hex.slice(3, 5), 16);
  const b = parseInt(hex.slice(5, 7), 16);
  return `rgba(${r}, ${g}, ${b}, ${config.value.general.overlayBgOpacity})`;
});

// Draggable overlay position
const overlayPos = ref({ x: 0, y: 0 });
const isDragging = ref(false);
const dragStart = ref({ x: 0, y: 0 });
const posStart = ref({ x: 0, y: 0 });

const startDrag = (e: MouseEvent) => {
  isDragging.value = true;
  dragStart.value = { x: e.clientX, y: e.clientY };
  posStart.value = { ...overlayPos.value };
  e.preventDefault();
};
const stopDrag = () => { isDragging.value = false; isWristDragging.value = false; };
const resetPosition = () => { overlayPos.value = { x: 0, y: 0 }; wristPos.value = { x: 0, y: 0 }; };

// Wrist overlay
const wristPos = ref({ x: 0, y: 0 });
const isWristDragging = ref(false);
const wristDragStart = ref({ x: 0, y: 0 });
const wristPosStart = ref({ x: 0, y: 0 });
const startWristDrag = (e: MouseEvent) => {
  isWristDragging.value = true;
  wristDragStart.value = { x: e.clientX, y: e.clientY };
  wristPosStart.value = { ...wristPos.value };
  e.preventDefault();
};

// Unified drag handler for both main overlay and wrist overlay
const onGlobalDrag = (e: MouseEvent) => {
  if (isDragging.value) {
    overlayPos.value = {
      x: posStart.value.x + (e.clientX - dragStart.value.x),
      y: posStart.value.y + (e.clientY - dragStart.value.y),
    };
  }
  if (isWristDragging.value) {
    wristPos.value = {
      x: wristPosStart.value.x + (e.clientX - wristDragStart.value.x),
      y: wristPosStart.value.y + (e.clientY - wristDragStart.value.y),
    };
  }
};


// Global Toast System
const toasts = ref<{id: number; message: string; type: 'success'|'error'|'info'}[]>([]);
let toastIdCounter = 0;
const showToast = (message: string, type: 'success'|'error'|'info' = 'info') => {
  const id = ++toastIdCounter;
  toasts.value.push({ id, message, type });
  setTimeout(() => {
    toasts.value = toasts.value.filter(t => t.id !== id);
  }, 3000);
};

const openSteamVrBindings = async () => {
  try {
    await OvrApi.openBindingUi();
    showToast('已打开 SteamVR 按键绑定编辑器', 'success');
  } catch (error) {
    showToast(`无法打开 SteamVR 按键绑定编辑器：${String(error)}`, 'error');
  }
};

const toggleNativeVrMenu = async () => {
  try {
    await OvrApi.toggleMenu();
  } catch (error) {
    showToast(`无法切换 VR 菜单：${String(error)}`, 'error');
  }
};

// Auto-sync configuration to backend with debounce.
// Watch a serialized snapshot instead of deep-watching the whole config object,
// which avoids deep proxy traversal on every nested mutation.
let configSyncTimeout: ReturnType<typeof setTimeout> | null = null;
let desktopScanSafetyTimer: ReturnType<typeof setTimeout> | null = null;
let ovrasSyncChecked = false;
let ovrasSyncAvailable = true;
watch(() => JSON.stringify(config.value), () => {
  if (configSyncTimeout) clearTimeout(configSyncTimeout);
  configSyncTimeout = setTimeout(() => {
    syncConfigToBackend();
  }, 500);
});

// ========== OVR Backend Integration ==========
const ovrConnected = ref(false);
const ovrHmdModel = ref('');
const ovrLogs = ref<string[]>([]);
let ovrUnlisteners: (() => void)[] = [];

// ========== Desktop Mirror Translation ==========
const autoScanRunning = ref(false);
const desktopTranslationResult = ref<{original: string; translated: string} | null>(null);
const desktopScanLoading = ref(false);

const toggleAutoScan = async () => {
  if (autoScanRunning.value) {
    await OvrApi.stopAutoScan();
    autoScanRunning.value = false;
    showToast(t('ovr.toast_scan_stopped'), 'info');
  } else {
    // Sync config first so backend has latest interval
    await syncConfigToBackend();
    await OvrApi.startAutoScan();
    autoScanRunning.value = true;
    showToast(t('ovr.toast_scan_started').replace('{interval}', String(config.value.general.autoScanInterval || 5)), 'success');
  }
};

const triggerDesktopScan = async () => {
  if (desktopScanLoading.value) return;
  desktopScanLoading.value = true;
  try {
    await syncConfigToBackend();
    await OvrApi.desktopScanOnce();
    showToast(t('ovr.toast_scan_loading'), 'info');
  } catch (err: any) {
    showToast(t('ovr.toast_scan_failed').replace('{error}', String(err)), 'error');
  }
  // Loading will be cleared when ovr_desktop_translation event fires
  desktopScanSafetyTimer = setTimeout(() => { desktopScanLoading.value = false; }, 35000); // Safety timeout
};

const initOvrBackend = async () => {
  try {
    const status = await OvrApi.init();
    if (status?.initialized) {
      ovrConnected.value = true;
      ovrHmdModel.value = status.hmd_model || 'Unknown';
      ovrLogs.value.push((t('ovr.log_ovr_connected') || t('auto_2ef2d51a')).replace('{model}', ovrHmdModel.value));
      // The native thread starts before the settings watcher can flush. Push the
      // persisted values immediately so SteamVR never starts with stale defaults.
      await syncConfigToBackend();
    }
  } catch (err: any) {
    ovrLogs.value.push(t('ovr.log_ovr_init_fail').replace('{error}', String(err?.message || err)));
    console.warn('OVR init failed (VR may not be running):', err);
  }
};

const syncConfigToBackend = async () => {
  void syncConfigToOvrasIni();
  if (!ovrConnected.value) return;
  try {
    await OvrApi.setConfig({
      config: {
        enabled: config.value.general.enabled,
        dual_display: config.value.general.dualDisplay,
        wrist_mode: config.value.general.wristMode,
        trigger_key: config.value.general.triggerKey,
        clear_key: config.value.general.clearKey,
        overlay_text_color: config.value.general.overlayTextColor,
        overlay_bg_color: config.value.general.overlayBgColor,
        overlay_bg_opacity: config.value.general.overlayBgOpacity,
        overlay_glass: config.value.general.overlayGlass !== false,
        overlay_border_opacity: config.value.general.overlayBorderOpacity ?? 0.42,
        overlay_corner_radius: config.value.general.overlayCornerRadius ?? 18,
        overlay_shadow_strength: config.value.general.overlayShadowStrength ?? 0.35,
        overlay_lock_mode: config.value.general.overlayLockMode,
        status_color: config.value.general.overlayStatusColor,
        trans_service: config.value.general.transService,
        trans_api_key: config.value.general.transApiKey,
        trans_llm_model: config.value.general.transLlmModel,
        trans_llm_prompt: config.value.general.transLlmPrompt,
        trans_source_lang: config.value.general.transSourceLang,
        trans_target_lang: config.value.general.transTargetLang,
        // Desktop mirror mode
        desktop_mode: config.value.general.desktopMode || false,
        auto_scan_enabled: config.value.general.autoScanEnabled || false,
        auto_scan_interval: config.value.general.autoScanInterval || 5,
        tts_enabled: config.value.general.ttsEnabled !== false,
        osc_chatbox_enabled: config.value.general.oscChatboxEnabled !== false,
        trans_panel_max_width: config.value.general.transPanelMaxWidth || 640,
        overlay_font_size: config.value.general.overlayFontSize || 30,
        ocr_language: config.value.ocr.model || 'zh-en-ja',
        ocr_speed_mode: config.value.ocr.speedMode || 'standard',
        ocr_image_enhance: config.value.ocr.enhanceContrast || false,
        ocr_sharpen: config.value.general.ocrSharpen || false,
        ocr_denoise: config.value.general.ocrDenoise || false,
        ocr_merge_tolerance_x: config.value.general.ocrMergeWidthTol || 0.1,
        ocr_merge_tolerance_y: config.value.general.ocrMergeHeightTol || 0.2,
        auto_start_steamvr: config.value.general.advAutoStart || false,
        playspace_offset_x: config.value.playspace.offsetX || 0,
        playspace_offset_y: config.value.playspace.offsetY || 0,
        playspace_offset_z: config.value.playspace.offsetZ || 0,
        playspace_rotation: config.value.playspace.rotation || 0,
        height_toggle_enabled: config.value.playspace.heightToggle || false,
        height_toggle_offset: config.value.playspace.heightOffset || 0.3,
        menu_width_m: generalNumber('menuWidthM') || 0.55,
        menu_offset_x: generalNumber('menuOffsetX'),
        menu_offset_y: generalNumber('menuOffsetY') || -0.06,
        menu_offset_z: generalNumber('menuOffsetZ') || -0.75,
        result_width_m: generalNumber('resultWidthM') || 0.72,
        result_offset_x: generalNumber('resultOffsetX'),
        result_offset_y: generalNumber('resultOffsetY') || -0.42,
        result_offset_z: generalNumber('resultOffsetZ') || -1.10,
        scan_frame_width_m: generalNumber('scanFrameWidthM') || 0.34,
        scan_frame_distance_m: generalNumber('scanFrameDistanceM') || 0.72,
        gravity_enabled: config.value.general.motionGravityOn || false,
        gravity_strength: (config.value.general.motionGravityStrength || 9.81) / 20.0,
        fling_strength: config.value.general.motionFlingStrength || 1.0,
        snap_turn_enabled: config.value.general.snapTurnEnabled !== false,
        snap_turn_angle: config.value.general.rotationSnapTurnAngle || 30,
        smooth_turn_enabled: config.value.general.smoothTurnEnabled || false,
        smooth_turn_rate: config.value.general.rotationSmoothTurnRate || 90,
        lock_x_enabled: config.value.general.lockXEnabled || false,
        lock_y_enabled: config.value.general.lockYEnabled || false,
        lock_z_enabled: config.value.general.lockZEnabled || false,
        drag_multiplier: config.value.general.motionDragMultiplier || 1.0,
        comfort_turn_enabled: config.value.general.comfortTurnEnabled || false,
        capture_mode: config.value.general.captureMode || 'dynamic',
        capture_quality: config.value.general.captureQuality || 'balanced',
        capture_auto_save: config.value.general.captureAutoSave || false,
        capture_format: config.value.general.captureFormat || 'png',
      }
    });
  } catch (err) {
    console.warn('OVR config sync failed:', err);
  }
};

// App.vue owns the global theme -> VR-menu sync.

const syncConfigToOvrasIni = async () => {
  if (ovrasSyncChecked && !ovrasSyncAvailable) return;

  try {
    await OvrApi.syncOvrasIni({ payload: JSON.stringify(config.value) });
    ovrasSyncAvailable = true;
  } catch (err) {
    ovrasSyncAvailable = false;
    if (!ovrasSyncChecked) {
      console.warn('OVRAS compatibility sync unavailable:', err);
    }
  } finally {
    ovrasSyncChecked = true;
  }
};

// 竞态防护：onMounted 里有多个 await，组件可能在监听器注册完成前就被卸载。
// 若已卸载则立即注销刚拿到的监听器，避免永久泄漏。
let ovrDisposed = false;

onMounted(async () => {
  ovrDisposed = false;
  window.addEventListener('mousemove', onGlobalDrag);
  window.addEventListener('mouseup', stopDrag);
  await loadSettings();

  // Initialize OVR backend (non-blocking, VR is optional)
  initOvrBackend();

  // Listen for OVR backend events via Tauri
  try {
    const { listen } = await import('@tauri-apps/api/event');
    const u1 = await listen<string>('ovr_log', (e) => {
      ovrLogs.value.push(e.payload);
      if (ovrLogs.value.length > 50) ovrLogs.value.shift();
    });
    const u2 = await listen<string>('ovr_error', (e) => {
      ovrLogs.value.push(`[错误] ${e.payload}`);
    });
    const u3 = await listen<any>('ovr_heartbeat', (e) => {
      if (e.payload) {
        ovrConnected.value = e.payload.initialized;
        ovrHmdModel.value = e.payload.hmd_model || '';
      }
    });
    const u4 = await listen<{original: string; translated: string}>('ovr_desktop_translation', (e) => {
      desktopTranslationResult.value = e.payload;
      desktopScanLoading.value = false;
    });
    const u5 = await listen<boolean>('ovr_auto_scan_status', (e) => {
      autoScanRunning.value = e.payload;
    });
    const u6 = await listen<Record<string, unknown>>('ovr_layout_config_changed', (e) => {
      if (isPlainObject(e.payload)) {
        patchOvrLayoutFromBackend(e.payload);
      }
    });
    const u7 = await listen<Record<string, unknown>>('ovr_config_changed', (e) => {
      const payload = e.payload;
      if (!isPlainObject(payload)) return;
      patchOvrLayoutFromBackend(payload);
      if (typeof payload.overlay_lock_mode === 'string') config.value.general.overlayLockMode = payload.overlay_lock_mode;
      if (typeof payload.overlay_bg_opacity === 'number') config.value.general.overlayBgOpacity = payload.overlay_bg_opacity;
      if (typeof payload.dual_display === 'boolean') config.value.general.dualDisplay = payload.dual_display;
      if (typeof payload.wrist_mode === 'boolean') config.value.general.wristMode = payload.wrist_mode;
      if (typeof payload.tts_enabled === 'boolean') config.value.general.ttsEnabled = payload.tts_enabled;
      if (typeof payload.osc_chatbox_enabled === 'boolean') config.value.general.oscChatboxEnabled = payload.osc_chatbox_enabled;
      void saveSettings();
    });
    ovrUnlisteners = [u1, u2, u3, u4, u5, u6, u7];
    if (ovrDisposed) {
      // 组件在监听器注册过程中已被卸载，立即注销防止泄漏
      ovrUnlisteners.forEach(u => u());
      ovrUnlisteners = [];
    }
  } catch {
    // Tauri events not available (dev mode / non-Tauri env)
  }
});
onUnmounted(() => {
  ovrDisposed = true;
  window.removeEventListener('mousemove', onGlobalDrag);
  window.removeEventListener('mouseup', stopDrag);
  clearTimeout(scanTimeout1);
  clearTimeout(scanTimeout2);
  clearTimeout(saveTimeout);
  if (configSyncTimeout) clearTimeout(configSyncTimeout);
  if (desktopScanSafetyTimer) clearTimeout(desktopScanSafetyTimer);
  // Cleanup OVR event listeners
  ovrUnlisteners.forEach(u => u());
  // Keep the native OpenVR session alive while navigating between app views.
  // Stopping it here made every VR menu, shortcut and overlay disappear as soon
  // as the user left the translator settings page.
});
const loadSettings = async () => {
  try {
    const all = await DbApi.getAllSettings();
    if (all && typeof all === 'object') {
      applyConfigPatch(all as Record<string, unknown>);
      const nativeRuntime = parseStoredValue((all as Record<string, unknown>).ovr_native_runtime_config);
      if (isPlainObject(nativeRuntime)) applyConfigPatch(nativeRuntime);
    }

    // Two-way sync: Override with native OVR INI if they changed it in VR
    try {
      const iniJsonStr = await OvrApi.loadOvrasIni();
      const iniData = JSON.parse(iniJsonStr || '{}');
      if (isPlainObject(iniData)) {
        applyConfigPatch(iniData);
      }
    } catch (e) {
      console.warn('Failed to load native OVR INI:', e);
    }
  } catch (err) {
    console.warn('Failed to load OVR settings:', err);
  }
};

const saveSettings = async () => {
  isSaving.value = true;
  try {
    const settings = Object.entries(config.value).map(
      ([key, val]) => [key, JSON.stringify(val)] as [string, string],
    );
    await DbApi.saveSettings({ settings });
    saved.value = true;
    saveTimeout = setTimeout(() => { saved.value = false; }, 2000);
    // Sync to OVR backend
    syncConfigToBackend();
  } catch (err) {
    console.error('Failed to save OVR settings:', err);
  } finally {
    isSaving.value = false;
  }
};

const restoreDefaults = () => {
  config.value = createDefaultConfig();
  saveSettings();
};

// ========== Statistics (simulated) ==========
const statsData = ref({
  hmdDistance: 0,
  hmdRotations: 0,
  leftControllerSpeed: 0,
  rightControllerSpeed: 0,
  presentedFrames: 0,
  droppedFrames: 0,
  reprojectedFrames: 0,
  timedOut: 0,
});
const reprojectionRatio = computed(() => {
  if (!statsData.value.presentedFrames) return '0%';
  return ((statsData.value.reprojectedFrames / statsData.value.presentedFrames) * 100).toFixed(1) + '%';
});
const resetStats = () => {
  statsData.value = { hmdDistance: 0, hmdRotations: 0, leftControllerSpeed: 0, rightControllerSpeed: 0, presentedFrames: 0, droppedFrames: 0, reprojectedFrames: 0, timedOut: 0 };
};

// API Connection Testing
const isTestingApi = ref(false);
const apiTestResult = ref<'idle' | 'success' | 'error'>('idle');
const apiTestMsg = ref('');

const testApiConnection = async () => {
  if (!config.value.general.transApiKey) {
    apiTestResult.value = 'error';
    apiTestMsg.value = t('ovr.api_test_empty');
    return;
  }
  
  isTestingApi.value = true;
  apiTestResult.value = 'idle';
  
  try {
    const service = config.value.general.transService;
    const url = apiTestUrls[service];
    if (!url) {
      if (config.value.general.transApiKey.length > 10) {
        apiTestResult.value = 'success';
        apiTestMsg.value = t('ovr.api_test_success');
      } else {
        apiTestResult.value = 'error';
        apiTestMsg.value = t('ovr.api_test_fail');
      }
      isTestingApi.value = false;
      return;
    }

    const headers: Record<string, string> = {};
    if (service === 'gemini') {
      headers['x-goog-api-key'] = config.value.general.transApiKey;
    } else {
      headers.Authorization = `Bearer ${config.value.general.transApiKey}`;
    }

    const res = await fetch(url, {
      method: 'GET',
      headers,
    });

    if (res.ok) {
      apiTestResult.value = 'success';
      apiTestMsg.value = t('ovr.api_test_success');
    } else {
      apiTestResult.value = 'error';
      apiTestMsg.value = t('ovr.api_test_fail') + ` (Status: ${res.status})`;
    }
  } catch (e: any) {
    apiTestResult.value = 'error';
    apiTestMsg.value = t('ovr.api_test_fail') + ` (Network Error)`;
  } finally {
    isTestingApi.value = false;
  }
};

const getKeyDisplay = (val: string) => {
  const map: Record<string, string> = {
    trigger: t('ovr.trigger'),
    grip: t('ovr.grip'),
    a_button: t('ovr.a_button'),
    b_button: t('ovr.b_button'),
    left_stick: t('ovr.left_stick'),
    right_stick: t('ovr.right_stick')
  };
  return map[val] || val;
};

</script>

<template>
  <div class="flex-1 overflow-y-auto overflow-x-hidden flex flex-col pr-2 custom-scrollbar">
    <header class="mb-6 flex justify-between items-end flex-shrink-0">
      <div>
        <h1 class="text-3xl font-extrabold text-text tracking-tight flex items-center gap-3">
          {{ t('ovr.title') }} <span class="bg-primary/10 text-primary px-2 py-0.5 rounded text-xs font-bold uppercase tracking-wider">{{ t('ovr.badge') }}</span>
        </h1>
        <p class="text-text-muted font-medium mt-1">
          {{ t('ovr.subtitle') }}
        </p>
        <!-- VR Backend Connection Status -->
        <div class="mt-2 flex items-center gap-2">
          <div
            class="w-2.5 h-2.5 rounded-full"
            :class="ovrConnected ? 'bg-green-500 animate-pulse' : 'bg-surface'"
          />
          <span
            class="text-xs font-bold"
            :class="ovrConnected ? 'text-green-700' : 'text-text-muted'"
          >
            {{ ovrConnected ? t('ovr.ovr_connected').replace('{model}', ovrHmdModel) : t('ovr.ovr_disconnected') }}
          </span>
        </div>
      </div>
      <div class="flex items-center gap-3">
        <button
          class="px-5 py-2 rounded-full font-bold text-primary bg-surface-hover hover:bg-primary hover:text-white border border-primary/40 shadow-sm transition-colors flex items-center gap-2"
          @click="restoreDefaults"
        >
          <RotateCcw class="w-4 h-4" /> {{ t('ovr.restore_defaults') }}
        </button>
        <button
          class="px-6 py-2 rounded-full font-bold shadow-md transition-all flex items-center gap-2"
          :class="saved ? 'bg-green-500 text-white shadow-green-500/30' : 'bg-primary text-white hover:brightness-110 shadow-primary/30'"
          @click="saveSettings"
        >
          <Check
            v-if="saved"
            class="w-4 h-4"
          />
          <Save
            v-else
            class="w-4 h-4"
            :class="{'animate-spin': isSaving}"
          />
          {{ saved ? t('ovr.saved') : t('ovr.save') }}
        </button>
      </div>
    </header>

    <div class="h-[650px] flex-shrink-0 mb-8 bg-surface backdrop-blur-md border-border-soft rounded-3xl shadow-lg flex overflow-hidden">
      <!-- 宸︿晶瀵艰埅 -->
      <div class="w-48 bg-surface border-primary p-4 space-y-2 overflow-y-auto custom-scrollbar">
        <button
          :class="activeSubTab === 'basic' ? 'bg-primary text-white font-bold shadow-sm' : 'text-text-muted hover:text-primary hover:bg-surface-hover/80'"
          class="w-full text-left px-4 py-3 rounded-xl transition-colors flex items-center gap-3 text-sm"
          @click="activeSubTab = 'basic'"
        >
          <Settings :size="16" /> {{ t('ovr.tab_basic') }}
        </button>
        <button
          :class="activeSubTab === 'desktop' ? 'bg-primary text-white font-bold shadow-sm' : 'text-text-muted hover:text-primary hover:bg-surface-hover/80'"
          class="w-full text-left px-4 py-3 rounded-xl transition-colors flex items-center gap-3 text-sm"
          @click="activeSubTab = 'desktop'"
        >
          <MonitorSpeaker :size="16" /> {{ t('ovr.tab_desktop') }}
        </button>
        <button
          :class="activeSubTab === 'ocr' ? 'bg-primary text-white font-bold shadow-sm' : 'text-text-muted hover:text-primary hover:bg-surface-hover/80'"
          class="w-full text-left px-4 py-3 rounded-xl transition-colors flex items-center gap-3 text-sm"
          @click="activeSubTab = 'ocr'"
        >
          <ScanEye :size="16" /> {{ t('ovr.tab_ocr') }}
        </button>
        <button
          :class="activeSubTab === 'trans' ? 'bg-primary text-white font-bold shadow-sm' : 'text-text-muted hover:text-primary hover:bg-surface-hover/80'"
          class="w-full text-left px-4 py-3 rounded-xl transition-colors flex items-center gap-3 text-sm"
          @click="activeSubTab = 'trans'"
        >
          <Languages :size="16" /> {{ t('ovr.tab_trans') }}
        </button>
        <button
          :class="activeSubTab === 'overlay' ? 'bg-primary text-white font-bold shadow-sm' : 'text-text-muted hover:text-primary hover:bg-surface-hover/80'"
          class="w-full text-left px-4 py-3 rounded-xl transition-colors flex items-center gap-3 text-sm"
          @click="activeSubTab = 'overlay'"
        >
          <Layers :size="16" /> {{ t('ovr.tab_overlay') }}
        </button>
        <button
          :class="activeSubTab === 'adv' ? 'bg-primary text-white font-bold shadow-sm' : 'text-text-muted hover:text-primary hover:bg-surface-hover/80'"
          class="w-full text-left px-4 py-3 rounded-xl transition-colors flex items-center gap-3 text-sm"
          @click="activeSubTab = 'adv'"
        >
          <Cpu :size="16" /> {{ t('ovr.tab_adv') }}
        </button>

        <!-- OVR Advanced Settings 鍒嗛殧绾?-->
        <div class="my-3 flex items-center gap-2">
          <div class="flex-1 border-primary" />
          <span class="text-[10px] font-bold text-primary uppercase tracking-wider whitespace-nowrap">{{ t('ovr.title_adv') }}</span>
          <div class="flex-1 border-primary" />
        </div>

        <button
          :class="activeSubTab === 'steamvr' ? 'bg-primary text-white font-bold shadow-sm' : 'text-text-muted hover:text-primary hover:bg-surface-hover/80'"
          class="w-full text-left px-4 py-3 rounded-xl transition-colors flex items-center gap-3 text-sm"
          @click="activeSubTab = 'steamvr'"
        >
          <Power :size="16" /> {{ t('ovr.tab_steamvr') }}
        </button>
        <button
          :class="activeSubTab === 'chaperone' ? 'bg-primary text-white font-bold shadow-sm' : 'text-text-muted hover:text-primary hover:bg-surface-hover/80'"
          class="w-full text-left px-4 py-3 rounded-xl transition-colors flex items-center gap-3 text-sm"
          @click="activeSubTab = 'chaperone'"
        >
          <Shield :size="16" /> {{ t('ovr.tab_chaperone') }}
        </button>
        <button
          :class="activeSubTab === 'playspace' ? 'bg-primary text-white font-bold shadow-sm' : 'text-text-muted hover:text-primary hover:bg-surface-hover/80'"
          class="w-full text-left px-4 py-3 rounded-xl transition-colors flex items-center gap-3 text-sm"
          @click="activeSubTab = 'playspace'"
        >
          <Move3d :size="16" /> {{ t('ovr.tab_playspace') }}
        </button>
        <button
          :class="activeSubTab === 'audio' ? 'bg-primary text-white font-bold shadow-sm' : 'text-text-muted hover:text-primary hover:bg-surface-hover/80'"
          class="w-full text-left px-4 py-3 rounded-xl transition-colors flex items-center gap-3 text-sm"
          @click="activeSubTab = 'audio'"
        >
          <Headphones :size="16" /> {{ t('ovr.tab_audio') }}
        </button>
        <button
          :class="activeSubTab === 'video' ? 'bg-primary text-white font-bold shadow-sm' : 'text-text-muted hover:text-primary hover:bg-surface-hover/80'"
          class="w-full text-left px-4 py-3 rounded-xl transition-colors flex items-center gap-3 text-sm"
          @click="activeSubTab = 'video'"
        >
          <Sun :size="16" /> {{ t('ovr.tab_video') }}
        </button>
        <button
          :class="activeSubTab === 'utilities' ? 'bg-primary text-white font-bold shadow-sm' : 'text-text-muted hover:text-primary hover:bg-surface-hover/80'"
          class="w-full text-left px-4 py-3 rounded-xl transition-colors flex items-center gap-3 text-sm"
          @click="activeSubTab = 'utilities'"
        >
          <Keyboard :size="16" /> {{ t('ovr.tab_utilities') }}
        </button>
        <button
          :class="activeSubTab === 'statistics' ? 'bg-primary text-white font-bold shadow-sm' : 'text-text-muted hover:text-primary hover:bg-surface-hover/80'"
          class="w-full text-left px-4 py-3 rounded-xl transition-colors flex items-center gap-3 text-sm"
          @click="activeSubTab = 'statistics'"
        >
          <BarChart3 :size="16" /> {{ t('ovr.tab_statistics') }}
        </button>
      </div>

      <!-- 右侧内容 -->
      <div class="flex-1 p-8 overflow-y-auto custom-scrollbar">
        <!-- 基础设置 -->
        <div
          v-if="activeSubTab === 'basic'"
          class="space-y-5 animate-fade-in"
        >
          <h2 class="text-xl font-extrabold text-primary mb-4 border-primary pb-2">
            {{ t('ovr.basic_title') }}
          </h2>
          
          <div class="flex items-center justify-between p-4 bg-surface rounded-2xl border-primary shadow-sm">
            <div>
              <h3 class="font-bold text-primary flex items-center gap-2">
                <PlaySquare class="w-4 h-4 text-primary" /> {{ t('ovr.basic_enable') }}
              </h3>
              <p class="text-xs text-primary mt-0.5">
                {{ t('ovr.basic_enable_desc') }}
              </p>
            </div>
            <label class="relative inline-flex items-center cursor-pointer">
              <input
                v-model="config.general.enabled"
                type="checkbox"
                class="sr-only peer"
              >
              <div class="w-11 h-6 bg-border-strong/35 border border-border-strong peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-white after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-primary peer-checked:border-primary shadow-inner" />
            </label>
          </div>

          <div class="flex items-center justify-between p-4 bg-surface rounded-2xl border-primary shadow-sm">
            <div>
              <h3 class="font-bold text-primary flex items-center gap-2">
                <Languages class="w-4 h-4 text-primary" /> {{ t('ovr.basic_dual') }}
              </h3>
              <p class="text-xs text-primary mt-0.5">
                {{ t('ovr.basic_dual_desc') }}
              </p>
            </div>
            <label class="relative inline-flex items-center cursor-pointer">
              <input
                v-model="config.general.dualDisplay"
                type="checkbox"
                class="sr-only peer"
              >
              <div class="w-11 h-6 bg-border-strong/35 border border-border-strong rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-white after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-primary peer-checked:border-primary shadow-inner" />
            </label>
          </div>
          
          <div class="flex items-center justify-between p-4 bg-surface rounded-2xl border-primary shadow-sm">
            <div>
              <h3 class="font-bold text-primary flex items-center gap-2">
                <Watch class="w-4 h-4 text-primary" /> {{ t('ovr.basic_wrist') }}
              </h3>
              <p class="text-xs text-primary mt-0.5">
                {{ t('ovr.basic_wrist_desc') }}
              </p>
            </div>
            <label class="relative inline-flex items-center cursor-pointer">
              <input
                v-model="config.general.wristMode"
                type="checkbox"
                class="sr-only peer"
              >
              <div class="w-11 h-6 bg-border-strong/35 border border-border-strong rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-white after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-primary peer-checked:border-primary shadow-inner" />
            </label>
          </div>

          <h3 class="text-sm font-extrabold text-primary mt-6 mb-2 flex items-center gap-2">
            <Gamepad2 class="w-4 h-4 text-primary" /> {{ t('ovr.controller_mapping') }}
          </h3>

          <div class="p-4 bg-surface rounded-2xl border-primary shadow-sm space-y-4">
            <div>
              <label class="block text-sm font-bold text-primary mb-1">{{ t('ovr.trigger_key') }}</label>
              <CustomSelect v-model="config.general.triggerKey" :options="[
                  { label: t('ovr.trigger_full'), value: 'trigger' },
                  { label: t('ovr.grip_full'), value: 'grip' },
                  { label: t('ovr.a_button'), value: 'a_button' },
                  { label: t('ovr.b_button'), value: 'b_button' }
                ]" />
            </div>
            <div>
              <label class="block text-sm font-bold text-primary mb-1">{{ t('ovr.clear_key') }}</label>
              <CustomSelect v-model="config.general.clearKey" :options="[
                  { label: t('ovr.left_stick_full'), value: 'left_stick' },
                  { label: t('ovr.right_stick_full'), value: 'right_stick' },
                  { label: t('ovr.a_button'), value: 'a_button' },
                  { label: t('ovr.b_button'), value: 'b_button' },
                  { label: t('ovr.grip_full'), value: 'grip' }
                ]" />
            </div>
            <button
              type="button"
              class="w-full px-3 py-2 rounded-lg bg-primary text-white font-bold text-sm hover:opacity-90 transition-opacity"
              :disabled="!ovrConnected"
              @click="toggleNativeVrMenu"
            >
              <ScanEye class="w-4 h-4 inline-block mr-2 align-[-2px]" />
              在头显中显示/隐藏 VR 菜单
            </button>
            <button
              type="button"
              class="w-full px-3 py-2 rounded-lg border border-primary/30 text-primary font-bold text-sm hover:bg-primary/10 transition-colors"
              :disabled="!ovrConnected"
              @click="openSteamVrBindings"
            >
              <Gamepad2 class="w-4 h-4 inline-block mr-2 align-[-2px]" />
              在 SteamVR 中编辑完整按键绑定
            </button>
          </div>
        </div>

        <!-- 桌面投屏截图翻译 -->
        <div
          v-else-if="activeSubTab === 'desktop'"
          class="space-y-5 animate-fade-in"
        >
          <h2 class="text-xl font-extrabold text-primary mb-4 border-primary pb-2">
            {{ t('ovr.desktop_title') }}
          </h2>

          <!-- 功能说明 -->
          <div class="p-4 bg-gradient-to-r from-primary/10 to-primary/5 rounded-2xl border border-primary/20 shadow-sm">
            <p class="text-sm text-primary leading-relaxed">
              {{ t('ovr.desktop_desc') }}
            </p>
          </div>

          <!-- 主开关-->
          <div class="flex items-center justify-between p-4 bg-surface rounded-2xl border-primary shadow-sm">
            <div>
              <h3 class="font-bold text-primary flex items-center gap-2">
                <MonitorSpeaker class="w-4 h-4 text-primary" /> {{ t('ovr.desktop_enable') }}
              </h3>
              <p class="text-xs text-primary mt-0.5">
                {{ t('ovr.desktop_enable_desc') }}
              </p>
            </div>
            <label class="relative inline-flex items-center cursor-pointer">
              <input
                v-model="config.general.desktopMode"
                type="checkbox"
                class="sr-only peer"
              >
              <div class="w-11 h-6 bg-border-strong/35 border border-border-strong rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-white after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-primary peer-checked:border-primary shadow-inner" />
            </label>
          </div>

          <!-- 手动扫描按钮 -->
          <div
            v-if="config.general.desktopMode"
            class="p-4 bg-surface rounded-2xl border-primary shadow-sm space-y-3"
          >
            <h3 class="text-sm font-extrabold text-primary flex items-center gap-2">
              <ScanEye class="w-4 h-4 text-primary" /> {{ t('ovr.desktop_scan_section') }}
            </h3>
            <div class="flex gap-3">
              <button
                class="flex-1 flex items-center justify-center gap-2 px-4 py-3 rounded-xl font-bold text-sm transition-all"
                :class="desktopScanLoading
                  ? 'bg-yellow-100 text-yellow-700 cursor-wait'
                  : 'bg-gradient-to-r from-primary to-primary-hover text-white hover:brightness-110 shadow-md hover:shadow-lg'"
                :disabled="desktopScanLoading"
                @click="triggerDesktopScan"
              >
                <Loader2
                  v-if="desktopScanLoading"
                  class="w-4 h-4 animate-spin"
                />
                <ScanEye
                  v-else
                  class="w-4 h-4"
                />
                {{ desktopScanLoading ? t('ovr.desktop_scanning') : t('ovr.desktop_scan_now') }}
              </button>
              <button
                class="flex items-center justify-center gap-2 px-4 py-3 rounded-xl font-bold text-sm transition-all"
                :class="autoScanRunning
                  ? 'bg-red-100 text-red-700 hover:bg-red-200 border-red-200'
                  : 'bg-green-100 text-green-700 hover:bg-green-200 border-green-200'"
                @click="toggleAutoScan"
              >
                <RefreshCw
                  class="w-4 h-4"
                  :class="autoScanRunning ? 'animate-spin' : ''"
                />
                {{ autoScanRunning ? t('ovr.desktop_auto_stop') : t('ovr.desktop_auto_start') }}
              </button>
            </div>
          </div>

          <!-- 自动扫描间隔 -->
          <div
            v-if="config.general.desktopMode"
            class="p-4 bg-surface rounded-2xl border-primary shadow-sm space-y-3"
          >
            <h3 class="text-sm font-extrabold text-primary flex items-center gap-2">
              <Timer class="w-4 h-4 text-primary" /> {{ t('ovr.desktop_interval') }}
            </h3>
            <div class="flex items-center gap-4">
              <input
                v-model.number="config.general.autoScanInterval"
                type="range"
                min="3"
                max="30"
                step="1"
                class="flex-1 accent-primary"
              >
              <span class="text-lg font-bold text-primary min-w-[4ch] text-center">{{ config.general.autoScanInterval }}s</span>
            </div>
            <p class="text-xs text-primary">
              {{ t('ovr.desktop_interval_desc') }}
            </p>
          </div>

          <!-- 输出开关-->
          <div
            v-if="config.general.desktopMode"
            class="space-y-3"
          >
            <div class="flex items-center justify-between p-4 bg-surface rounded-2xl border-primary shadow-sm">
              <div>
                <h3 class="font-bold text-primary flex items-center gap-2">
                  <Volume2 class="w-4 h-4 text-primary" /> {{ t('ovr.desktop_tts') }}
                </h3>
                <p class="text-xs text-primary mt-0.5">
                  {{ t('ovr.desktop_tts_desc') }}
                </p>
              </div>
              <label class="relative inline-flex items-center cursor-pointer">
                <input
                  v-model="config.general.ttsEnabled"
                  type="checkbox"
                  class="sr-only peer"
                >
                <div class="w-11 h-6 bg-border-strong/35 border border-border-strong rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-white after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-primary peer-checked:border-primary shadow-inner" />
              </label>
            </div>

            <div class="flex items-center justify-between p-4 bg-surface rounded-2xl border-primary shadow-sm">
              <div>
                <h3 class="font-bold text-primary flex items-center gap-2">
                  <MessageSquare class="w-4 h-4 text-primary" /> {{ t('ovr.desktop_osc') }}
                </h3>
                <p class="text-xs text-primary mt-0.5">
                  {{ t('ovr.desktop_osc_desc') }}
                </p>
              </div>
              <label class="relative inline-flex items-center cursor-pointer">
                <input
                  v-model="config.general.oscChatboxEnabled"
                  type="checkbox"
                  class="sr-only peer"
                >
                <div class="w-11 h-6 bg-border-strong/35 border border-border-strong rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-white after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-primary peer-checked:border-primary shadow-inner" />
              </label>
            </div>
          </div>

          <!-- 翻译结果实时显示 -->
          <div
            v-if="config.general.desktopMode && desktopTranslationResult"
            class="p-4 bg-surface rounded-2xl border-primary shadow-sm space-y-3"
          >
            <h3 class="text-sm font-extrabold text-primary flex items-center gap-2">
              <Languages class="w-4 h-4 text-primary" /> {{ t('ovr.desktop_result') }}
            </h3>
            <div
              v-if="desktopTranslationResult.original"
              class="p-3 bg-primary/10 rounded-xl"
            >
              <p class="text-xs font-bold text-primary mb-1">
                {{ t('ovr.desktop_original') }}
              </p>
              <p class="text-sm text-primary whitespace-pre-wrap break-words leading-relaxed max-h-24 overflow-y-auto custom-scrollbar">
                {{ desktopTranslationResult.original }}
              </p>
            </div>
            <div class="p-3 bg-primary/5 rounded-xl border border-primary/10">
              <p class="text-xs font-bold text-primary mb-1">
                {{ t('ovr.desktop_translated') }}
              </p>
              <p class="text-sm text-primary/90 whitespace-pre-wrap break-words leading-relaxed max-h-24 overflow-y-auto custom-scrollbar font-medium">
                {{ desktopTranslationResult.translated }}
              </p>
            </div>
          </div>
        </div>

        <!-- OCR设置 -->
        <div
          v-else-if="activeSubTab === 'ocr'"
          class="space-y-5 animate-fade-in"
        >
          <h2 class="text-xl font-extrabold text-primary mb-4 border-primary pb-2">
            {{ t('ovr.ocr_title') }}
          </h2>
          
          <div class="p-4 bg-surface rounded-2xl border-primary shadow-sm space-y-4">
            <div>
              <label class="block text-sm font-bold text-primary mb-1">{{ t('ovr.ocr_lang') }}</label>
              <CustomSelect v-model="config.ocr.model" :options="[
                  { label: t('ovr.ocr_lang_zhenja'), value: 'zh-en-ja' },
                  { label: t('ovr.ocr_lang_zhtw'), value: 'zh-tw' },
                  { label: t('ovr.ocr_lang_ko'), value: 'ko' },
                  { label: t('ovr.ocr_lang_latin'), value: 'latin' }
                ]" />
            </div>
            
            <div>
              <label class="block text-sm font-bold text-primary mb-1">{{ t('ovr.ocr_speed') }}</label>
              <CustomSelect v-model="config.ocr.speedMode" :options="[
                  { label: t('ovr.ocr_speed_fast'), value: 'fast' },
                  { label: t('ovr.ocr_speed_standard'), value: 'standard' },
                  { label: t('ovr.ocr_speed_accurate'), value: 'accurate' }
                ]" />
            </div>
          </div>
          
          <div class="p-4 bg-surface rounded-2xl border-primary shadow-sm space-y-4">
            <h3 class="font-bold text-primary">
              {{ t('ovr.ocr_enhance_title') }}
            </h3>
            <div class="grid grid-cols-2 gap-4">
              <label class="flex items-center gap-3 cursor-pointer">
                <input
                  v-model="config.ocr.enhanceContrast"
                  type="checkbox"
                  class="w-5 h-5 text-primary rounded focus:ring-indigo-500"
                >
                <span class="text-sm font-bold text-primary">{{ t('ovr.ocr_contrast') }}</span>
              </label>
              <label class="flex items-center gap-3 cursor-pointer">
                <input
                  v-model="config.general.ocrSharpen"
                  type="checkbox"
                  class="w-5 h-5 text-primary rounded focus:ring-indigo-500"
                >
                <span class="text-sm font-bold text-primary">{{ t('ovr.ocr_sharpen') }}</span>
              </label>
            </div>
          </div>
        </div>

        <!-- 翻译服务 -->
        <div
          v-else-if="activeSubTab === 'trans'"
          class="space-y-5 animate-fade-in"
        >
          <h2 class="text-xl font-extrabold text-primary mb-4 border-primary pb-2">
            {{ t('ovr.trans_title') }}
          </h2>
          
          <div class="p-4 bg-surface rounded-2xl border-primary shadow-sm space-y-4">
            <div class="grid grid-cols-2 gap-4">
              <div>
                <label class="block text-sm font-bold text-primary mb-1">{{ t('ovr.trans_source') }}</label>
                <CustomSelect v-model="config.general.transSourceLang" :options="[
                  { label: t('ovr.trans_auto'), value: 'auto' },
                  { label: 'English', value: 'en' },
                  { label: 'Japanese', value: 'ja' },
                  { label: 'Korean', value: 'ko' }
                ]" />
              </div>
              <div>
                <label class="block text-sm font-bold text-primary mb-1">{{ t('ovr.trans_target') }}</label>
                <CustomSelect v-model="config.general.transTargetLang" :options="[
                  { label: 'Chinese', value: 'zh' },
                  { label: 'English', value: 'en' }
                ]" />
              </div>
            </div>
          </div>
          
          <div class="p-4 bg-surface rounded-2xl border-primary shadow-sm space-y-4">
            <div>
              <label class="block text-sm font-bold text-primary mb-1">{{ t('ovr.trans_mode') }}</label>
              <CustomSelect v-model="config.general.transMode" :options="[
                  { label: t('ovr.trans_builtin'), value: 'builtin' },
                  { label: t('ovr.trans_custom'), value: 'custom' }
                ]" />
            </div>
            
            <div
              v-if="config.general.transMode === 'custom'"
              class="space-y-4 pt-4 border-primary"
            >
              <div>
                <label class="block text-sm font-bold text-primary mb-1">{{ t('ovr.trans_provider') }}</label>
                <CustomSelect v-model="config.general.transService" :options="translationProviderOptions" />
              </div>
              
              <div>
                <label class="block text-sm font-bold text-primary mb-1">{{ t('ovr.trans_apikey') }}</label>
                <div class="flex gap-2 items-center">
                  <input
                    v-model="config.general.transApiKey"
                    type="password"
                    placeholder="sk-..."
                    class="flex-1 bg-primary/10 border-primary rounded-xl px-4 py-2 text-primary font-medium"
                  >
                  <button
                    class="px-4 py-2 bg-primary text-white hover:bg-primary-hover font-bold rounded-xl transition-colors flex items-center justify-center min-w-[100px]"
                    :disabled="isTestingApi"
                    @click="testApiConnection"
                  >
                    <Loader2
                      v-if="isTestingApi"
                      class="w-4 h-4 animate-spin"
                    />
                    <span v-else>{{ t('ovr.api_test_btn') }}</span>
                  </button>
                </div>
                <!-- API Test Result Feedback -->
                <div
                  v-if="apiTestResult !== 'idle'"
                  class="mt-2 text-sm font-bold px-3 py-2 rounded-lg flex items-center gap-2"
                  :class="apiTestResult === 'success' ? 'bg-green-500/10 text-green-500 border border-green-500/30' : 'bg-red-500/10 text-red-500 border border-red-500/30'"
                >
                  <Check
                    v-if="apiTestResult === 'success'"
                    class="w-4 h-4"
                  />
                  <X
                    v-else
                    class="w-4 h-4"
                  />
                  {{ apiTestMsg }}
                </div>
              </div>
              
              <div v-if="llmPromptServices.includes(config.general.transService)">
                <label class="block text-sm font-bold text-primary mb-1">{{ t('ovr.trans_prompt') }}</label>
                <textarea
                  v-model="config.general.transLlmPrompt"
                  rows="3"
                  class="w-full bg-primary/10 border-primary rounded-xl px-4 py-2 text-primary font-medium"
                />
              </div>
            </div>
          </div>
        </div>

        <!-- 叠加层外观-->
        <div
          v-else-if="activeSubTab === 'overlay'"
          class="space-y-5 animate-fade-in"
        >
          <h2 class="text-xl font-extrabold text-primary mb-4 border-primary pb-2">
            {{ t('ovr.overlay_title') }}
          </h2>
          
          <div class="p-4 bg-surface rounded-2xl border-primary shadow-sm space-y-4">
            <h3 class="font-bold text-primary">
              {{ t('ovr.overlay_text_bg') }}
            </h3>
            <div class="grid grid-cols-2 gap-4">
              <div>
                <label class="block text-sm font-bold text-primary mb-1">{{ t('ovr.overlay_text_color') }}</label>
                <div class="flex items-center gap-2">
                  <input
                    v-model="config.general.overlayTextColor"
                    type="color"
                    class="w-10 h-10 p-1 bg-primary/10 border-primary rounded cursor-pointer"
                  >
                  <span class="text-xs font-bold text-primary uppercase">{{ config.general.overlayTextColor }}</span>
                </div>
              </div>
              <div>
                <label class="block text-sm font-bold text-primary mb-1">{{ t('ovr.overlay_bg_color') }}</label>
                <div class="flex items-center gap-2">
                  <input
                    v-model="config.general.overlayBgColor"
                    type="color"
                    class="w-10 h-10 p-1 bg-primary/10 border-primary rounded cursor-pointer"
                  >
                  <span class="text-xs font-bold text-primary uppercase">{{ config.general.overlayBgColor }}</span>
                </div>
              </div>
            </div>
            
            <div class="pt-2">
              <label class="block text-sm font-bold text-primary mb-2">{{ t('ovr.overlay_opacity') }}: {{ Math.round(config.general.overlayBgOpacity * 100) }}%</label>
              <input
                v-model="config.general.overlayBgOpacity"
                type="range"
                min="0"
                max="1"
                step="0.05"
                class="w-full h-2 bg-primary/10 rounded-lg appearance-none cursor-pointer accent-primary"
              >
            </div>

            <div class="grid grid-cols-2 gap-4 pt-2">
              <div>
                <label class="block text-sm font-bold text-primary mb-2">Panel: {{ config.general.transPanelMaxWidth }}px</label>
                <input
                  v-model.number="config.general.transPanelMaxWidth"
                  type="range"
                  min="320"
                  max="1024"
                  step="32"
                  class="w-full h-2 bg-primary/10 rounded-lg appearance-none cursor-pointer accent-primary"
                >
              </div>
              <div>
                <label class="block text-sm font-bold text-primary mb-2">Font: {{ config.general.overlayFontSize }}px</label>
                <input
                  v-model.number="config.general.overlayFontSize"
                  type="range"
                  min="18"
                  max="56"
                  step="1"
                  class="w-full h-2 bg-primary/10 rounded-lg appearance-none cursor-pointer accent-primary"
                >
              </div>
              <div>
                <label class="block text-sm font-bold text-primary mb-2">Border: {{ Math.round(config.general.overlayBorderOpacity * 100) }}%</label>
                <input
                  v-model.number="config.general.overlayBorderOpacity"
                  type="range"
                  min="0"
                  max="1"
                  step="0.05"
                  class="w-full h-2 bg-primary/10 rounded-lg appearance-none cursor-pointer accent-primary"
                >
              </div>
              <div>
                <label class="block text-sm font-bold text-primary mb-2">Radius: {{ config.general.overlayCornerRadius }}px</label>
                <input
                  v-model.number="config.general.overlayCornerRadius"
                  type="range"
                  min="4"
                  max="36"
                  step="1"
                  class="w-full h-2 bg-primary/10 rounded-lg appearance-none cursor-pointer accent-primary"
                >
              </div>
              <div class="col-span-2">
                <label class="block text-sm font-bold text-primary mb-2">Shadow: {{ Math.round(config.general.overlayShadowStrength * 100) }}%</label>
                <input
                  v-model.number="config.general.overlayShadowStrength"
                  type="range"
                  min="0"
                  max="1"
                  step="0.05"
                  class="w-full h-2 bg-primary/10 rounded-lg appearance-none cursor-pointer accent-primary"
                >
              </div>
            </div>
          </div>
          
          <div class="p-4 bg-surface rounded-2xl border-primary shadow-sm space-y-4">
            <div>
              <label class="block text-sm font-bold text-primary mb-1">{{ t('ovr.overlay_lock_mode') }}</label>
              <CustomSelect v-model="config.general.overlayLockMode" :options="[
                  { label: t('ovr.overlay_world_lock'), value: 'world' },
                  { label: t('ovr.overlay_head_lock'), value: 'head' }
                ]" />
            </div>
            
            <div class="pt-2">
              <label class="block text-sm font-bold text-primary mb-1">{{ t('ovr.overlay_status_color') }}</label>
              <div class="flex items-center gap-2">
                <input
                  v-model="config.general.overlayStatusColor"
                  type="color"
                  class="w-10 h-10 p-1 bg-primary/10 border-primary rounded cursor-pointer"
                >
                <span class="text-xs font-bold text-primary uppercase">{{ config.general.overlayStatusColor }}</span>
              </div>
            </div>
          </div>
        </div>

        <!-- 空间与运动(Playspace & Motion) -->
        <div
          v-else-if="activeSubTab === 'playspace'"
          class="space-y-5 animate-fade-in"
        >
          <h2 class="text-xl font-extrabold text-primary mb-4 border-primary pb-2">
            {{ t('ovr.playspace_title') }}
          </h2>
          
          <!-- 空间拖拽 -->
          <div class="p-4 bg-surface rounded-2xl border-primary shadow-sm space-y-4">
            <h3 class="font-bold text-primary flex items-center gap-2">
              <Move3d class="w-4 h-4 text-primary" /> {{ t('ovr.playspace_drag') }}
            </h3>
            <p class="text-xs text-primary mb-2">
              {{ t('ovr.playspace_drag_desc') }}
            </p>
            
            <div class="grid grid-cols-2 gap-4">
              <label class="flex items-center gap-3 cursor-pointer p-3 bg-primary text-white rounded-xl border border-primary hover:bg-primary-hover shadow-sm shadow-primary/20 transition-colors">
                <input
                  v-model="config.playspace.dragLeft"
                  type="checkbox"
                  class="w-5 h-5 text-primary rounded focus:ring-indigo-500"
                >
                <span class="text-sm font-bold text-white">{{ t('ovr.playspace_drag_left') }}</span>
              </label>
              <label class="flex items-center gap-3 cursor-pointer p-3 bg-primary text-white rounded-xl border border-primary hover:bg-primary-hover shadow-sm shadow-primary/20 transition-colors">
                <input
                  v-model="config.playspace.dragRight"
                  type="checkbox"
                  class="w-5 h-5 text-primary rounded focus:ring-indigo-500"
                >
                <span class="text-sm font-bold text-white">{{ t('ovr.playspace_drag_right') }}</span>
              </label>
            </div>
          </div>

          <!-- 高度调整 (拉高低) -->
          <div class="p-4 bg-surface rounded-2xl border-primary shadow-sm space-y-4">
            <div class="flex items-center justify-between mb-2">
              <div>
                <h3 class="font-bold text-primary flex items-center gap-2">
                  <ArrowUpDown class="w-4 h-4 text-primary" /> {{ t('ovr.playspace_height') }}
                </h3>
                <p class="text-xs text-primary mt-0.5">
                  {{ t('ovr.playspace_height_desc') }}
                </p>
              </div>
              <label class="relative inline-flex items-center cursor-pointer">
                <input
                  v-model="config.playspace.heightToggle"
                  type="checkbox"
                  class="sr-only peer"
                >
                <div class="w-11 h-6 bg-border-strong/35 border border-border-strong rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-white after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-primary peer-checked:border-primary shadow-inner" />
              </label>
            </div>

            <div
              v-if="config.playspace.heightToggle"
              class="space-y-2 pt-2 border-primary"
            >
              <label class="flex justify-between text-sm font-bold text-primary">
                <span>{{ t('ovr.playspace_height_offset') }}</span>
                <span class="text-primary">{{ config.playspace.heightOffset.toFixed(2) }}m</span>
              </label>
              <input
                v-model.number="config.playspace.heightOffset"
                type="range"
                min="-2.0"
                max="2.0"
                step="0.05"
                class="w-full accent-primary"
              >
            </div>
          </div>

          <!-- 空间偏移 (XYZ) -->
          <div class="p-4 bg-surface rounded-2xl border-primary shadow-sm space-y-4">
            <div>
              <h3 class="font-bold text-primary flex items-center gap-2">
                <Move class="w-4 h-4 text-primary" /> {{ t('ovr.playspace_offsets') }}
              </h3>
              <p class="text-xs text-primary mt-0.5">
                {{ t('ovr.playspace_offsets_desc') }}
              </p>
            </div>
            
            <div class="space-y-4 pt-2 border-primary">
              <div class="flex items-center gap-4">
                <span class="font-bold text-primary w-8">{{ t('ovr.axis_x') }}</span>
                <input
                  v-model.number="config.playspace.offsetX"
                  type="range"
                  min="-5.0"
                  max="5.0"
                  step="0.1"
                  class="flex-1 accent-primary"
                >
                <span class="text-sm font-mono text-primary w-12 text-right">{{ config.playspace.offsetX.toFixed(1) }}</span>
              </div>
              <div class="flex items-center gap-4">
                <span class="font-bold text-primary w-8">{{ t('ovr.axis_y') }}</span>
                <input
                  v-model.number="config.playspace.offsetY"
                  type="range"
                  min="-5.0"
                  max="5.0"
                  step="0.1"
                  class="flex-1 accent-primary"
                >
                <span class="text-sm font-mono text-primary w-12 text-right">{{ config.playspace.offsetY.toFixed(1) }}</span>
              </div>
              <div class="flex items-center gap-4">
                <span class="font-bold text-primary w-8">{{ t('ovr.axis_z') }}</span>
                <input
                  v-model.number="config.playspace.offsetZ"
                  type="range"
                  min="-5.0"
                  max="5.0"
                  step="0.1"
                  class="flex-1 accent-primary"
                >
                <span class="text-sm font-mono text-primary w-12 text-right">{{ config.playspace.offsetZ.toFixed(1) }}</span>
              </div>
            </div>
            
            <div class="flex justify-end pt-2">
              <button 
                class="px-4 py-2 bg-primary text-white rounded-lg text-sm font-bold hover:bg-primary-hover shadow-sm shadow-primary/20 transition-colors"
                @click="config.playspace.offsetX = 0; config.playspace.offsetY = 0; config.playspace.offsetZ = 0"
              >
                {{ t('ovr.playspace_reset') }}
              </button>
            </div>
          </div>
        </div>

        <!-- 护栏边界 (Chaperone) -->
        <div
          v-else-if="activeSubTab === 'chaperone'"
          class="space-y-5 animate-fade-in"
        >
          <h2 class="text-xl font-extrabold text-primary mb-4 border-primary pb-2">
            {{ t('ovr.chaperone_title') }}
          </h2>
          
          <div class="space-y-3">
            <div class="flex items-center justify-between p-4 bg-surface rounded-2xl border-primary shadow-sm">
              <div>
                <h3 class="font-bold text-primary flex items-center gap-2">
                  <Shield class="w-4 h-4 text-primary" /> {{ t('ovr.chaperone_force_bounds') }}
                </h3>
                <p class="text-xs text-primary mt-0.5">
                  {{ t('ovr.chaperone_force_bounds_desc') }}
                </p>
              </div>
              <label class="relative inline-flex items-center cursor-pointer">
                <input
                  v-model="config.chaperone.forceBounds"
                  type="checkbox"
                  class="sr-only peer"
                >
                <div class="w-11 h-6 bg-border-strong/35 border border-border-strong rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-white after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-primary peer-checked:border-primary shadow-inner" />
              </label>
            </div>
            
            <div class="flex items-center justify-between p-4 bg-surface rounded-2xl border-primary shadow-sm">
              <div>
                <h3 class="font-bold text-primary flex items-center gap-2">
                  <Activity class="w-4 h-4 text-primary" /> {{ t('ovr.chaperone_haptics') }}
                </h3>
                <p class="text-xs text-primary mt-0.5">
                  {{ t('ovr.chaperone_haptics_desc') }}
                </p>
              </div>
              <label class="relative inline-flex items-center cursor-pointer">
                <input
                  v-model="config.chaperone.hapticFeedback"
                  type="checkbox"
                  class="sr-only peer"
                >
                <div class="w-11 h-6 bg-border-strong/35 border border-border-strong rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-white after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-primary peer-checked:border-primary shadow-inner" />
              </label>
            </div>
            
            <div class="p-4 bg-surface rounded-2xl border-primary shadow-sm space-y-4">
              <div class="flex justify-between items-center">
                <div>
                  <h3 class="font-bold text-primary flex items-center gap-2">
                    <Eye class="w-4 h-4 text-primary" /> {{ t('ovr.chaperone_visibility') }}
                  </h3>
                  <p class="text-xs text-primary mt-0.5">
                    {{ t('ovr.chaperone_visibility_desc') }}
                  </p>
                </div>
                <span class="text-primary font-bold bg-primary/10 px-2 py-1 rounded">{{ config.chaperone.visibility }}%</span>
              </div>
              <input
                v-model.number="config.chaperone.visibility"
                type="range"
                min="0"
                max="100"
                class="w-full accent-primary"
              >
            </div>
          </div>
        </div>

        <!-- 视频画质 (Video) -->
        <div
          v-else-if="activeSubTab === 'video'"
          class="space-y-5 animate-fade-in"
        >
          <h2 class="text-xl font-extrabold text-primary mb-4 border-primary pb-2">
            {{ t('ovr.video_title') }}
          </h2>
          
          <div class="space-y-3">
            <div class="flex items-center justify-between p-4 bg-surface rounded-2xl border-primary shadow-sm">
              <div>
                <h3 class="font-bold text-primary flex items-center gap-2">
                  <Wind class="w-4 h-4 text-primary" /> {{ t('ovr.video_motion_smooth') }}
                </h3>
                <p class="text-xs text-primary mt-0.5">
                  {{ t('ovr.video_motion_smooth_desc') }}
                </p>
              </div>
              <label class="relative inline-flex items-center cursor-pointer">
                <input
                  v-model="config.video.motionSmooth"
                  type="checkbox"
                  class="sr-only peer"
                >
                <div class="w-11 h-6 bg-border-strong/35 border border-border-strong rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-white after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-primary peer-checked:border-primary shadow-inner" />
              </label>
            </div>
            
            <div class="p-4 bg-surface rounded-2xl border-primary shadow-sm space-y-4">
              <div class="flex justify-between items-center">
                <div>
                  <h3 class="font-bold text-primary flex items-center gap-2">
                    <Monitor class="w-4 h-4 text-primary" /> {{ t('ovr.video_supersampling') }}
                  </h3>
                  <p class="text-xs text-primary mt-0.5">
                    {{ t('ovr.video_supersampling_desc') }}
                  </p>
                </div>
                <span class="text-primary font-bold bg-primary/10 px-2 py-1 rounded">{{ (config.video.superSampling * 100).toFixed(0) }}%</span>
              </div>
              <input
                v-model.number="config.video.superSampling"
                type="range"
                min="0.1"
                max="2.5"
                step="0.05"
                class="w-full accent-primary"
              >
            </div>
          </div>
        </div>

        <!-- 实用工具 (Utilities) -->
        <div
          v-else-if="activeSubTab === 'utilities'"
          class="space-y-5 animate-fade-in"
        >
          <h2 class="text-xl font-extrabold text-primary mb-4 border-primary pb-2">
            {{ t('ovr.tab_utilities') }}
          </h2>
          
          <div class="space-y-3">
            <div class="flex items-center justify-between p-4 bg-surface rounded-2xl border-primary shadow-sm">
              <div>
                <h3 class="font-bold text-primary flex items-center gap-2">
                  <Keyboard class="w-4 h-4 text-primary" /> {{ t('ovr.util_media_keys') }}
                </h3>
                <p class="text-xs text-primary mt-0.5">
                  {{ t('ovr.util_media_keys_desc') }}
                </p>
              </div>
              <label class="relative inline-flex items-center cursor-pointer">
                <input
                  v-model="config.utilities.mediaKeys"
                  type="checkbox"
                  class="sr-only peer"
                >
                <div class="w-11 h-6 bg-border-strong/35 border border-border-strong rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-white after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-primary peer-checked:border-primary shadow-inner" />
              </label>
            </div>
          </div>
        </div>

        <!-- 高级性能 -->
        <div
          v-else-if="activeSubTab === 'adv'"
          class="space-y-5 animate-fade-in"
        >
          <h2 class="text-xl font-extrabold text-primary mb-4 border-primary pb-2">
            {{ t('ovr.adv_title') }}
          </h2>
          
          <div class="space-y-3">
            <div class="flex items-center justify-between p-4 bg-surface rounded-2xl border-primary shadow-sm">
              <div>
                <h3 class="font-bold text-primary">
                  {{ t('ovr.adv_cpu') }}
                </h3>
                <p class="text-xs text-primary mt-0.5">
                  {{ t('ovr.adv_cpu_desc') }}
                </p>
              </div>
              <label class="relative inline-flex items-center cursor-pointer">
                <input
                  v-model="config.general.advCpuAccel"
                  type="checkbox"
                  class="sr-only peer"
                >
                <div class="w-11 h-6 bg-border-strong/35 border border-border-strong peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-white after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-primary peer-checked:border-primary shadow-inner" />
              </label>
            </div>
            
            <div class="flex items-center justify-between p-4 bg-surface rounded-2xl border-primary shadow-sm">
              <div>
                <h3 class="font-bold text-primary">
                  {{ t('ovr.adv_gpu') }}
                </h3>
                <p class="text-xs text-primary mt-0.5">
                  {{ t('ovr.adv_gpu_desc') }}
                </p>
              </div>
              <label class="relative inline-flex items-center cursor-pointer">
                <input
                  v-model="config.general.advGpuAccel"
                  type="checkbox"
                  class="sr-only peer"
                >
                <div class="w-11 h-6 bg-border-strong/35 border border-border-strong peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-white after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-primary peer-checked:border-primary shadow-inner" />
              </label>
            </div>
            
            <div class="flex items-center justify-between p-4 bg-surface rounded-2xl border-primary shadow-sm">
              <div>
                <h3 class="font-bold text-primary text-primary">
                  {{ t('ovr.adv_debug') }}
                </h3>
                <p class="text-xs text-primary mt-0.5">
                  {{ t('ovr.adv_debug_desc') }}
                </p>
              </div>
              <label class="relative inline-flex items-center cursor-pointer">
                <input
                  v-model="config.general.advDebugMode"
                  type="checkbox"
                  class="sr-only peer"
                >
                <div class="w-11 h-6 bg-border-strong/35 border border-border-strong peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-white after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-primary peer-checked:border-primary shadow-inner" />
              </label>
            </div>

            <div class="flex items-center justify-between p-4 bg-surface rounded-2xl border-primary shadow-sm">
              <div>
                <h3 class="font-bold text-primary">
                  {{ t('ovr.adv_autostart') }}
                </h3>
                <p class="text-xs text-primary mt-0.5">
                  {{ t('ovr.adv_autostart_desc') }}
                </p>
              </div>
              <label class="relative inline-flex items-center cursor-pointer">
                <input
                  v-model="config.general.advAutoStart"
                  type="checkbox"
                  class="sr-only peer"
                >
                <div class="w-11 h-6 bg-border-strong/35 border border-border-strong peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-white after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-primary peer-checked:border-primary shadow-inner" />
              </label>
            </div>
          </div>
        </div>
        
        <OvrAdvPanels
          v-model:config="config"
          :active-sub-tab="activeSubTab"
        />
      </div>
    </div>

    <!-- ========== VR Environment Preview ========== -->
    <div class="mt-6 animate-fade-in">
      <div class="flex items-center justify-between mb-3">
        <h2 class="text-lg font-extrabold text-text flex items-center gap-2">
          <Eye
            :size="18"
            class="text-primary"
          />
          {{ t('ovr.preview_title') }}
        </h2>
        <div class="flex items-center gap-2">
          <button
            class="px-3 py-1.5 text-xs font-bold text-primary bg-surface-hover hover:bg-primary hover:text-white rounded-lg border border-primary/40 shadow-sm transition-colors flex items-center gap-1"
            @click="resetPosition"
          >
            <RotateCcw :size="12" /> {{ t('ovr.preview_reset') }}
          </button>
          <button
            class="px-3 py-1.5 text-xs font-bold rounded-lg transition-colors flex items-center gap-1"
            :class="showVrPreview ? 'text-white bg-primary hover:bg-primary-hover border-primary shadow-sm' : 'text-text-muted bg-surface-hover hover:text-primary hover:bg-surface border-border-soft'"
            @click="showVrPreview = !showVrPreview"
          >
            <component
              :is="showVrPreview ? Eye : EyeOff"
              :size="12"
            />
            {{ showVrPreview ? t('ovr.preview_hide') : t('ovr.preview_show') }}
          </button>
        </div>
      </div>

      <div
        v-if="showVrPreview"
        class="vr-viewport-wrapper flex-shrink-0 mb-8 rounded-2xl overflow-hidden shadow-xl"
      >
        <!-- VR 3D Scene -->
        <div
          class="vr-scene"
          @click="triggerScan"
        >
          <!-- Infinite grid floor -->
          <div class="vr-grid-floor" />

          <!-- Ambient floating particles -->
          <div class="vr-particles">
            <div
              v-for="(p, idx) in vrParticles"
              :key="idx"
              class="vr-particle"
              :style="p"
            />
          </div>

          <!-- Status indicator (top) -->
          <div
            class="vr-status-bar"
            :style="{ color: config.general.overlayStatusColor }"
          >
            <div
              class="vr-status-dot"
              :class="{ 'vr-status-scanning': vrIsScanning }"
              :style="{ backgroundColor: config.general.overlayStatusColor }"
            />
            <span>{{ vrStatusText }}</span>
          </div>

          <!-- ========== VrcDog Style VR Dashboard Menu Panel ========== -->
          <div
            class="vr-dashboard glass-panel"
            :class="{ 'vr-dashboard-collapsed': !vrDashboardOpen }"
            :style="{ background: overlayBgStyle }"
            @click.stop
          >
            <!-- Dashboard header -->
            <div
              class="vr-dash-header"
              @click="vrDashboardOpen = !vrDashboardOpen"
            >
              <div class="vr-dash-logo">
                <img
                  v-if="!vrLogoLoadFailed"
                  :src="currentTheme.logo"
                  class="w-6 h-6 rounded-full border-2 shadow-sm"
                  :style="{ borderColor: currentTheme.colors.borderStrong }"
                  alt=""
                  @error="vrLogoLoadFailed = true"
                >
                <Box
                  v-else
                  class="w-6 h-6 rounded-full p-1 shadow-sm bg-white/70"
                  :style="{ color: currentTheme.colors.textSoft }"
                />
                <span class="vr-dash-logo-text">{{ currentTheme.appTitle }}</span>
              </div>
              <div class="vr-dash-toggle">
                <ChevronDown v-if="vrDashboardOpen" class="w-4 h-4" />
                <ChevronRight v-else class="w-4 h-4" />
              </div>
            </div>

            <div
              v-if="vrDashboardOpen"
              class="vr-dash-body"
            >
              <!-- Sidebar tabs -->
              <div class="vr-dash-sidebar">
                 <!-- Theme Swatch Toggles in VR Sidebar -->
                 <div
                   class="flex justify-between items-center rounded-xl p-1 mb-2"
                   :style="{ border: `1px solid ${currentTheme.colors.borderSoft}`, backgroundColor: 'rgba(255,255,255,0.6)' }"
                 >
                   <button
                     v-for="theme in Object.values(themes)"
                     :key="theme.id"
                     class="flex-1 py-1 text-[10px] font-bold rounded-lg transition-colors flex items-center justify-center gap-1"
                     :style="currentTheme.id === theme.id ? { backgroundColor: theme.colors.activeBg, color: theme.colors.textStrong } : { color: currentTheme.colors.textSoft, opacity: 0.7 }"
                     :title="t(theme.name)"
                     @click="setTheme(theme.id as ThemeId)"
                   >
                     {{ t(theme.name).slice(0,2) }}
                   </button>
                 </div>

                <div class="vr-dash-sidebar-scroll">
                  <button
                    v-for="tab in vrDashboardTabs"
                    :key="tab.key"
                    class="vr-dash-tab btn-cute"
                    :class="{ 'vr-dash-tab-active': vrDashboardTab === tab.key }"
                    @click="vrDashboardTab = tab.key"
                  >
                    <component
                      :is="tab.icon"
                      class="w-[14px] h-[14px] vr-dash-tab-icon"
                    />
                    <span class="vr-dash-tab-label">{{ t(tab.label) }}</span>
                  </button>
                </div>
              </div>

              <!-- Tab content -->
              <div class="vr-dash-content">
                <!-- Basic -->
                <div
                  v-if="vrDashboardTab === 'basic'"
                  class="vr-dash-section"
                >
                  <div class="vr-dash-row">
                    <span>{{ t('ovr.basic_enable') }}</span>
                    <div
                      class="vr-dash-switch"
                      :class="{ 'on': config.general.enabled }"
                      @click="config.general.enabled = !config.general.enabled"
                    >
                      <div class="vr-dash-switch-knob" />
                    </div>
                  </div>
                  <div class="vr-dash-row">
                    <span>{{ t('ovr.basic_dual') }}</span>
                    <div
                      class="vr-dash-switch"
                      :class="{ 'on': config.general.dualDisplay }"
                      @click="config.general.dualDisplay = !config.general.dualDisplay"
                    >
                      <div class="vr-dash-switch-knob" />
                    </div>
                  </div>
                  <div class="vr-dash-row">
                    <span>{{ t('ovr.basic_wrist') }}</span>
                    <div
                      class="vr-dash-switch"
                      :class="{ 'on': config.general.wristMode }"
                      @click="config.general.wristMode = !config.general.wristMode"
                    >
                      <div class="vr-dash-switch-knob" />
                    </div>
                  </div>
                </div>

                <!-- OCR -->
                <div
                  v-else-if="vrDashboardTab === 'ocr'"
                  class="vr-dash-section"
                >
                  <div class="vr-dash-row">
                    <span>{{ t('ovr.ocr_lang') }}</span>
                    <span
                      class="vr-dash-value"
                      @click="config.ocr.model = config.ocr.model === 'zh-en-ja' ? 'zh-tw' : (config.ocr.model === 'zh-tw' ? 'ko' : 'zh-en-ja')"
                    >{{ config.ocr.model }}</span>
                  </div>
                  <div class="vr-dash-row">
                    <span>{{ t('ovr.ocr_speed') }}</span>
                    <span
                      class="vr-dash-value"
                      @click="config.ocr.speedMode = config.ocr.speedMode === 'fast' ? 'standard' : (config.ocr.speedMode === 'standard' ? 'accurate' : 'fast')"
                    >{{ config.ocr.speedMode }}</span>
                  </div>
                  <div class="vr-dash-row">
                    <span>{{ t('ovr.ocr_contrast') }}</span>
                    <div
                      class="vr-dash-switch"
                      :class="{ 'on': config.ocr.enhanceContrast }"
                      @click="config.ocr.enhanceContrast = !config.ocr.enhanceContrast"
                    >
                      <div class="vr-dash-switch-knob" />
                    </div>
                  </div>
                </div>

                <!-- Translation -->
                <div
                  v-else-if="vrDashboardTab === 'trans'"
                  class="vr-dash-section"
                >
                  <div class="vr-dash-row">
                    <span>{{ t('ovr.trans_source') }}</span>
                    <span
                      class="vr-dash-value"
                       @click="config.general.transSourceLang = config.general.transSourceLang === 'auto' ? 'en' : (config.general.transSourceLang === 'en' ? 'ja' : (config.general.transSourceLang === 'ja' ? 'ko' : 'auto'))"
                    >{{ config.general.transSourceLang === 'auto' ? t('ovr.dash_auto') : config.general.transSourceLang.toUpperCase() }}</span>
                  </div>
                  <div class="vr-dash-row">
                    <span>{{ t('ovr.trans_target') }}</span>
                    <span
                      class="vr-dash-value"
                       @click="config.general.transTargetLang = config.general.transTargetLang === 'zh' ? 'en' : 'zh'"
                    >{{ config.general.transTargetLang.toUpperCase() }}</span>
                  </div>
                  <div class="vr-dash-row">
                    <span>{{ t('ovr.trans_mode') }}</span>
                    <span
                      class="vr-dash-value"
                      @click="config.general.transMode = config.general.transMode === 'builtin' ? 'custom' : 'builtin'"
                    >{{ config.general.transMode === 'builtin' ? t('ovr.dash_builtin') : t('ovr.dash_cloud') }}</span>
                  </div>
                </div>

                <!-- Overlay -->
                <div
                  v-else-if="vrDashboardTab === 'overlay'"
                  class="vr-dash-section"
                >
                  <div class="vr-dash-row">
                    <span>{{ t('ovr.overlay_text_color') }}</span>
                    <div
                      class="vr-dash-color-btn"
                      :style="{ backgroundColor: config.general.overlayTextColor }"
                      @click="config.general.overlayTextColor = config.general.overlayTextColor === '#FFFFFF' ? '#FDE68A' : (config.general.overlayTextColor === '#FDE68A' ? '#A7F3D0' : '#FFFFFF')"
                    />
                  </div>
                  <div class="vr-dash-row">
                    <span>{{ t('ovr.overlay_bg_color') }}</span>
                    <div
                      class="vr-dash-color-btn"
                      :style="{ backgroundColor: config.general.overlayBgColor }"
                       @click="config.general.overlayBgColor = config.general.overlayBgColor === '#101826' ? '#1E3A8A' : (config.general.overlayBgColor === '#1E3A8A' ? '#831843' : '#101826')"
                    />
                  </div>
                  <div class="vr-dash-row">
                    <span>{{ t('ovr.overlay_opacity') }}</span>
                    <span
                      class="vr-dash-value"
                      @click="config.general.overlayBgOpacity = config.general.overlayBgOpacity >= 1.0 ? 0.2 : config.general.overlayBgOpacity + 0.2"
                    >{{ Math.round(config.general.overlayBgOpacity * 100) }}%</span>
                  </div>
                  <div class="vr-dash-row">
                    <span>{{ t('ovr.overlay_lock_mode') }}</span>
                    <span
                      class="vr-dash-value"
                      style="text-transform: capitalize;"
                      @click="config.general.overlayLockMode = config.general.overlayLockMode === 'world' ? 'head' : 'world'"
                    >{{ config.general.overlayLockMode }}</span>
                  </div>
                </div>

                <!-- Advanced -->
                <div
                  v-else-if="vrDashboardTab === 'adv'"
                  class="vr-dash-section"
                >
                  <div class="vr-dash-row">
                    <span>{{ t('ovr.adv_cpu') }}</span>
                    <div
                      class="vr-dash-switch"
                      :class="{ 'on': config.general.advCpuAccel }"
                      @click="config.general.advCpuAccel = !config.general.advCpuAccel"
                    >
                      <div class="vr-dash-switch-knob" />
                    </div>
                  </div>
                  <div class="vr-dash-row">
                    <span>{{ t('ovr.adv_gpu') }}</span>
                    <div
                      class="vr-dash-switch"
                      :class="{ 'on': config.general.advGpuAccel }"
                      @click="config.general.advGpuAccel = !config.general.advGpuAccel"
                    >
                      <div class="vr-dash-switch-knob" />
                    </div>
                  </div>
                  <div class="vr-dash-row">
                    <span>{{ t('ovr.adv_debug') }}</span>
                    <div
                      class="vr-dash-switch"
                      :class="{ 'on': config.general.advDebugMode }"
                      @click="config.general.advDebugMode = !config.general.advDebugMode"
                    >
                      <div class="vr-dash-switch-knob" />
                    </div>
                  </div>
                </div>
                
                <OvrAdvVrDashPanels 
                  v-if="['steamvr', 'chaperone', 'playspace', 'audio', 'video', 'utilities', 'statistics'].includes(vrDashboardTab)"
                  v-model:config="config" 
                  :vr-dashboard-tab="vrDashboardTab" 
                />
                
                <!-- Language -->
                <div
                  v-else-if="vrDashboardTab === 'lang'"
                  class="vr-dash-section"
                >
                  <div class="vr-dash-row">
                    <span>{{ t('ovr.dash_lang') }}</span>
                    <span class="vr-dash-value">{{ t('ovr.dash_sys') }}</span>
                  </div>
                </div>
                
                <!-- Help -->
                <div
                  v-else-if="vrDashboardTab === 'help'"
                  class="vr-dash-section"
                >
                  <div
                    class="vr-dash-row"
                    style="flex-direction: column; align-items: flex-start; gap: 8px;"
                  >
                    <span style="color: var(--color-primary); font-weight: bold; display: flex; align-items: center; gap: 4px;"><Gamepad2 class="w-4 h-4" /> {{ t('ovr.dash_guide') }}</span>
                    <span style="font-size: 11px; color: var(--color-text-muted);">{{ t('ovr.desc_trigger_trans').replace('{key}', getKeyDisplay(config.general.triggerKey)) }}</span>
                    <span style="font-size: 11px; color: var(--color-text-muted);">{{ t('ovr.desc_clear_trans').replace('{key}', getKeyDisplay(config.general.clearKey)) }}</span>
                    <span style="font-size: 11px; color: var(--color-text-muted);">{{ t('ovr.dash_guide_wrist') }}</span>
                  </div>
                </div>
                
                <!-- About -->
                <div
                  v-else-if="vrDashboardTab === 'about'"
                  class="vr-dash-section"
                >
                  <div
                    class="vr-dash-row"
                    style="flex-direction: column; align-items: center; gap: 8px; justify-content: center; height: 100%; padding-top: 40px;"
                  >
                    <img
                      v-if="!vrLogoLoadFailed"
                      :src="currentTheme.logo"
                      class="w-12 h-12 rounded-full border-2 shadow-md mb-2"
                      :style="{ borderColor: currentTheme.colors.borderStrong }"
                      alt=""
                      @error="vrLogoLoadFailed = true"
                    >
                    <Box
                      v-else
                      class="w-12 h-12 rounded-full p-3 bg-white/70 shadow-md mb-2"
                      :style="{ color: currentTheme.colors.textSoft }"
                    />
                    <span style="font-size: 20px; font-weight: 800; color: v-bind('currentTheme.colors.textStrong');">{{ currentTheme.appTitle }} OVR</span>
                    <span style="font-size: 12px; color: v-bind('currentTheme.colors.textSoft');">{{ t('ovr.dash_version') }} v1.0.0 Beta</span>
                  </div>
                </div>
              </div>
            </div>
          </div>

          <!-- Screenshot scan frame (green box) -->
          <div
            class="vr-scan-frame"
            :class="{ 'vr-scan-active': vrIsScanning }"
          >
            <div class="vr-scan-corner vr-scan-tl" />
            <div class="vr-scan-corner vr-scan-tr" />
            <div class="vr-scan-corner vr-scan-bl" />
            <div class="vr-scan-corner vr-scan-br" />
            <div
              v-if="vrIsScanning"
              class="vr-scan-line"
            />
          </div>

          <!-- Floating translation overlay panel (DRAGGABLE) -->
          <div
            class="vr-overlay-panel"
            :class="{ 'vr-lock-head': config.general.overlayLockMode === 'head', 'vr-lock-world': config.general.overlayLockMode === 'world' }"
            :style="{
              background: overlayBgStyle,
              color: config.general.overlayTextColor,
              maxWidth: (config.general.transPanelMaxWidth ?? 380) + 'px',
              transform: `translate(-50%, -50%) translate(${overlayPos.x}px, ${overlayPos.y}px)`
            }"
          >
            <!-- Drag handle -->
            <div
              class="vr-panel-drag"
              :title="t('ovr.preview_drag')"
              @mousedown="startDrag"
            >
              <GripVertical :size="14" />
            </div>
            <!-- Translation content -->
            <div
              class="vr-panel-content"
              :style="{ fontSize: (config.general.overlayFontSize ?? 14) + 'px' }"
              @click.stop="toggleOriginal"
            >
              <div
                v-if="vrShowOriginal && config.general.dualDisplay"
                class="vr-original-text"
              >
                <span
                   v-for="(line, idx) in currentSample.original.split('\n')"
                  :key="idx"
                >
                   {{ line }}<br v-if="idx < currentSample.original.split('\n').length - 1">
                </span>
              </div>
              <div class="vr-translated-text">
                <span
                   v-for="(line, idx) in (vrShowOriginal && config.general.dualDisplay ? currentSample.original : currentSample.translated).split('\n')"
                  :key="idx"
                >
                   {{ line }}<br v-if="idx < currentSample.translated.split('\n').length - 1">
                </span>
              </div>
            </div>
            <!-- Lock mode badge -->
            <div class="vr-lock-badge flex items-center gap-1">
              <Globe
                v-if="config.general.overlayLockMode === 'world'"
                class="w-3 h-3"
              />
              <User
                v-else
                class="w-3 h-3"
              />
              {{ config.general.overlayLockMode === 'world' ? 'World' : 'Head' }}
            </div>
          </div>

          <!-- Wrist display (when enabled) -->
          <div
            v-if="config.general.wristMode"
            class="vr-wrist-overlay"
            :style="{ background: overlayBgStyle, color: config.general.overlayTextColor, transform: `translate(${wristPos.x}px, ${wristPos.y}px)` }"
            @mousedown.stop="startWristDrag"
          >
            <div class="vr-wrist-label flex items-center gap-1">
              <Watch class="w-3 h-3" /> {{ t('ovr.basic_wrist') }}
            </div>
            <div class="vr-wrist-content">
               {{ currentSample.translated.split('\n')[0] }}...
            </div>
          </div>

          <!-- VR controller hints -->
          <div class="vr-hints">
            <div class="vr-hint flex items-center gap-1">
              <Gamepad2 class="w-3 h-3" /> {{ t('ovr.preview_click_scan') }}
            </div>
            <div
              v-if="config.general.dualDisplay"
              class="vr-hint flex items-center gap-1"
            >
              <RotateCcw class="w-3 h-3" /> {{ t('ovr.preview_click_toggle') }}
            </div>
            <div class="vr-hint flex items-center gap-1">
              <Hand class="w-3 h-3" /> {{ t('ovr.preview_drag') }}
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.animate-fade-in {
  animation: fadeIn 0.2s ease-out forwards;
}
@keyframes fadeIn {
  from { opacity: 0; transform: translateY(5px); }
  to { opacity: 1; transform: translateY(0); }
}

/* ========== VR Viewport ========== */
.vr-viewport-wrapper {
  position: relative;
}

.vr-scene {
  position: relative;
  height: 420px;
  background: radial-gradient(ellipse at 50% 40%, #1a1a3e 0%, #0d0d1f 50%, #050510 100%);
  overflow: hidden;
  cursor: crosshair;
  perspective: 800px;
  user-select: none;
}

/* Grid floor with 3D perspective */
.vr-grid-floor {
  position: absolute;
  bottom: 0;
  left: -50%;
  width: 200%;
  height: 55%;
  background-image:
    linear-gradient(rgba(99, 102, 241, 0.15) 1px, transparent 1px),
    linear-gradient(90deg, rgba(99, 102, 241, 0.15) 1px, transparent 1px);
  background-size: 40px 40px;
  transform: rotateX(65deg);
  transform-origin: bottom center;
  mask-image: linear-gradient(to top, rgba(0,0,0,0.6) 0%, transparent 100%);
  -webkit-mask-image: linear-gradient(to top, rgba(0,0,0,0.6) 0%, transparent 100%);
}

/* Floating particles */
.vr-particles { position: absolute; inset: 0; pointer-events: none; }
.vr-particle {
  position: absolute;
  width: 3px;
  height: 3px;
  background: rgba(129, 140, 248, 0.5);
  border-radius: 50%;
  animation: vrParticleFloat linear infinite;
}
@keyframes vrParticleFloat {
  0%   { transform: translateY(0) scale(1); opacity: 0; }
  20%  { opacity: 0.8; }
  80%  { opacity: 0.4; }
  100% { transform: translateY(-120px) scale(0.3); opacity: 0; }
}

/* Status bar */
.vr-status-bar {
  position: absolute;
  top: 16px;
  left: 50%;
  transform: translateX(-50%);
  display: flex;
  align-items: center;
  gap: 8px;
  font-family: 'Consolas', 'SF Mono', monospace;
  font-size: 12px;
  font-weight: 700;
  letter-spacing: 1px;
  text-transform: uppercase;
  text-shadow: 0 0 10px currentColor;
  z-index: 10;
}
.vr-status-dot {
  width: 8px; height: 8px; border-radius: 50%;
  box-shadow: 0 0 8px currentColor;
  transition: all 0.3s;
}
.vr-status-scanning {
  animation: vrStatusPulse 0.6s ease-in-out infinite;
}
@keyframes vrStatusPulse {
  0%, 100% { transform: scale(1); opacity: 1; }
  50% { transform: scale(1.5); opacity: 0.5; }
}

/* Scan frame */
.vr-scan-frame {
  position: absolute;
  top: 50%;
  left: 50%;
  width: 320px;
  height: 160px;
  transform: translate(-50%, -50%);
  pointer-events: none;
  z-index: 5;
}
.vr-scan-corner {
  position: absolute;
  width: 20px; height: 20px;
  border-color: rgba(74, 222, 128, 0.6);
  border-style: solid;
  border-width: 0;
  transition: border-color 0.3s;
}
.vr-scan-tl { top: 0; left: 0; border-top-width: 2px; border-left-width: 2px; }
.vr-scan-tr { top: 0; right: 0; border-top-width: 2px; border-right-width: 2px; }
.vr-scan-bl { bottom: 0; left: 0; border-bottom-width: 2px; border-left-width: 2px; }
.vr-scan-br { bottom: 0; right: 0; border-bottom-width: 2px; border-right-width: 2px; }
.vr-scan-active .vr-scan-corner {
  border-color: rgba(74, 222, 128, 1);
  box-shadow: 0 0 12px rgba(74, 222, 128, 0.4);
}
.vr-scan-line {
  position: absolute;
  top: 0; left: 0; right: 0;
  height: 2px;
  background: linear-gradient(90deg, transparent, #4ade80, transparent);
  animation: vrScanDown 1.2s ease-in-out;
  box-shadow: 0 0 15px #4ade80;
}
@keyframes vrScanDown {
  0%   { top: 0; }
  100% { top: 100%; }
}

/* Floating translation overlay */
.vr-overlay-panel {
  position: absolute;
  top: 50%;
  left: 50%;
  min-width: 280px;
  max-width: 380px;
  border-radius: 8px;
  backdrop-filter: blur(12px);
  box-shadow: 0 8px 40px rgba(0, 0, 0, 0.5), 0 0 0 1px rgba(255,255,255,0.12) inset;
  z-index: 20;
  transition: box-shadow 0.3s;
  display: flex;
  align-items: stretch;
  transform: translate(-50%, -50%);
}
.vr-overlay-panel:hover {
  box-shadow: 0 8px 40px rgba(0, 0, 0, 0.5), 0 0 20px rgba(99, 102, 241, 0.2);
}
.vr-lock-world { animation: vrFloatWorld 6s ease-in-out infinite; }
.vr-lock-head { animation: none; /* No float for head-lock */ }
@keyframes vrFloatWorld {
  0%, 100% { top: 50%; }
  50% { top: calc(50% - 5px); }
}

.vr-panel-drag {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  cursor: grab;
  opacity: 0.4;
  transition: opacity 0.2s;
  flex-shrink: 0;
  box-shadow: inset -1px 0 0 rgba(255,255,255,0.08);
}
.vr-panel-drag:hover { opacity: 0.9; }
.vr-panel-drag:active { cursor: grabbing; }

.vr-panel-content {
  flex: 1;
  padding: 14px 16px;
  cursor: pointer;
  line-height: 1.6;
  font-size: 14px;
  font-weight: 500;
  letter-spacing: 0.3px;
}
.vr-original-text {
  font-size: 11px;
  opacity: 0.5;
  margin-bottom: 6px;
  padding-bottom: 6px;
  border-bottom: 1px dashed rgba(255,255,255,0.15);
  font-style: italic;
}

.vr-lock-badge {
  position: absolute;
  top: -24px;
  right: 0;
  font-size: 10px;
  font-weight: 700;
  color: rgba(165, 180, 252, 0.7);
  letter-spacing: 0.5px;
}

/* Wrist overlay */
.vr-wrist-overlay {
  position: absolute;
  bottom: 60px;
  left: 40px;
  width: 180px;
  padding: 8px 12px;
  border-radius: 10px;
  backdrop-filter: blur(10px);
  box-shadow: 0 4px 20px rgba(0,0,0,0.5), 0 0 0 1px rgba(255,255,255,0.1) inset;
  z-index: 15;
  cursor: grab;
  font-size: 11px;
  transform-origin: bottom left;
  animation: vrWristBob 4s ease-in-out infinite;
}
@keyframes vrWristBob {
  0%, 100% { transform: rotate(-3deg); }
  50% { transform: rotate(-1deg); }
}
.vr-wrist-label {
  font-size: 9px;
  font-weight: 700;
  opacity: 0.5;
  text-transform: uppercase;
  letter-spacing: 1px;
  margin-bottom: 4px;
}
.vr-wrist-content {
  font-weight: 600;
  line-height: 1.4;
}

/* Hint bar */
.vr-hints {
  position: absolute;
  bottom: 12px;
  left: 50%;
  transform: translateX(-50%);
  display: flex;
  gap: 16px;
  z-index: 25;
}
.vr-hint {
  font-size: 10px;
  font-weight: 600;
  color: rgba(165, 180, 252, 0.6);
  background: rgba(15, 15, 40, 0.7);
  padding: 4px 10px;
  border-radius: 6px;
  border: 1px solid rgba(99, 102, 241, 0.2);
  white-space: nowrap;
}

/* ========== Dynamic Theme VR Dashboard Menu Panel ========== */
.vr-dashboard {
  position: absolute;
  top: 44px;
  left: 24px;
  width: min(540px, calc(100% - 48px));
  /* background opacity dynamically bound in template */
  backdrop-filter: blur(12px);
  border-radius: 18px;
  box-shadow: 0 22px 48px rgba(0, 0, 0, 0.28), 0 0 0 1px v-bind('currentTheme.colors.borderSoft') inset;
  z-index: 30;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  transition: all 0.3s cubic-bezier(0.34, 1.56, 0.64, 1);
  transform-origin: top left;
}
.vr-dashboard-collapsed {
  width: 220px;
}
.vr-dashboard-collapsed .vr-dash-body {
  height: 0;
  opacity: 0;
  pointer-events: none;
}
.vr-dash-header {
  padding: 12px 16px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  background: rgba(255, 255, 255, 0.5);
  cursor: pointer;
  user-select: none;
  box-shadow: inset 0 -1px 0 v-bind('currentTheme.colors.borderSoft');
}
.vr-dash-header:hover { background: rgba(255, 255, 255, 0.8); }
.vr-dash-logo {
  display: flex;
  align-items: center;
  gap: 10px;
}
.vr-dash-logo-text {
  color: v-bind('currentTheme.colors.textStrong');
  font-weight: 800;
  font-size: 15px;
  letter-spacing: 0.5px;
}
.vr-dash-toggle {
  color: v-bind('currentTheme.colors.textSoft');
  display: flex;
  align-items: center;
  justify-content: center;
}
.vr-dash-body {
  display: flex;
  height: 300px;
  transition: all 0.3s ease;
}
.vr-dash-sidebar {
  width: 158px;
  background: rgba(255, 255, 255, 0.4);
  display: flex;
  flex-direction: column;
  box-shadow: inset -1px 0 0 v-bind('currentTheme.colors.borderSoft');
  padding: 12px 8px;
  gap: 6px;
}
.vr-dash-sidebar-scroll {
  flex: 1;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding-right: 2px;
}
.vr-dash-sidebar-scroll::-webkit-scrollbar { width: 4px; }
.vr-dash-sidebar-scroll::-webkit-scrollbar-thumb { background: v-bind('currentTheme.colors.borderStrong'); border-radius: 4px; }
.vr-dash-tab {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 12px;
  color: v-bind('currentTheme.colors.textSoft');
  background: rgba(255, 255, 255, 0.42);
  transition: all 0.2s;
  font-size: 13px;
  font-weight: 600;
  text-align: left;
  border-radius: 8px;
  cursor: pointer;
  box-shadow: 0 0 0 1px v-bind('currentTheme.colors.borderSoft') inset;
}
.vr-dash-tab:hover { background: v-bind('currentTheme.colors.surfaceHover'); color: v-bind('currentTheme.colors.textStrong'); box-shadow: 0 0 0 1px v-bind('currentTheme.colors.borderStrong') inset; }
.vr-dash-tab-active { background: v-bind('currentTheme.colors.primaryBtnBg') !important; color: white !important; box-shadow: 0 8px 18px rgba(0,0,0,0.12), 0 0 0 1px v-bind('currentTheme.colors.primaryBtnBg') inset; }
.vr-dash-tab-icon { font-size: 14px; }

.vr-dash-content {
  flex: 1;
  padding: 20px 24px;
  overflow-y: auto;
}
.vr-dash-content::-webkit-scrollbar { width: 6px; }
.vr-dash-content::-webkit-scrollbar-thumb { background: v-bind('currentTheme.colors.borderStrong'); border-radius: 4px; }
.vr-dash-section {
  display: flex;
  flex-direction: column;
  gap: 16px;
  animation: fadeIn 0.2s;
}
.vr-dash-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  color: v-bind('currentTheme.colors.textStrong');
  font-size: 13px;
  font-weight: 600;
  padding-bottom: 8px;
  box-shadow: inset 0 -1px 0 v-bind('currentTheme.colors.borderSoft');
}
.vr-dash-value {
  color: v-bind('currentTheme.colors.textSoft');
  font-weight: 800;
  font-size: 12px;
  background: v-bind('currentTheme.colors.bgMain');
  padding: 4px 10px;
  border-radius: 6px;
  box-shadow: 0 0 0 1px v-bind('currentTheme.colors.borderStrong') inset;
}
.vr-dash-color-btn {
  width: 24px; height: 24px;
  border-radius: 50%;
  box-shadow: 0 2px 4px rgba(0,0,0,0.1), 0 0 0 2px rgba(255,255,255,0.75) inset;
  cursor: pointer;
}
.vr-dash-switch {
  width: 32px; height: 18px;
  background: v-bind('currentTheme.colors.borderStrong');
  border-radius: 10px;
  position: relative;
  cursor: pointer;
  transition: background 0.3s;
  box-shadow: inset 0 1px 3px rgba(0,0,0,0.1), 0 0 0 1px v-bind('currentTheme.colors.borderStrong') inset;
}
.vr-dash-switch.on { background: v-bind('currentTheme.colors.primaryBtnBg'); box-shadow: inset 0 1px 3px rgba(0,0,0,0.1), 0 0 0 1px v-bind('currentTheme.colors.primaryBtnBg') inset; }
.vr-dash-switch-knob {
  width: 14px; height: 14px;
  background: white;
  border-radius: 50%;
  position: absolute;
  top: 2px; left: 2px;
  transition: transform 0.3s cubic-bezier(0.34, 1.56, 0.64, 1);
  box-shadow: 0 1px 3px rgba(0,0,0,0.2);
}
.vr-dash-switch.on .vr-dash-switch-knob { transform: translateX(14px); }

/* Scanner Laser Animation */
.vr-scan-line {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 4px;
  background: linear-gradient(to right, transparent, v-bind('config.general.overlayStatusColor'), transparent);
  box-shadow: 0 0 10px v-bind('config.general.overlayStatusColor'), 0 0 20px v-bind('config.general.overlayStatusColor');
  opacity: 0.8;
  animation: scanLaser 1.2s cubic-bezier(0.4, 0, 0.2, 1) infinite;
  z-index: 10;
}

@keyframes scanLaser {
  0% { top: -10px; opacity: 0; }
  10% { opacity: 1; }
  90% { opacity: 1; }
  100% { top: 100%; opacity: 0; }
}

</style>

