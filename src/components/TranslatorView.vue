<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue';
import { isTauri, invoke } from '@tauri-apps/api/core';
import { convertFileSrc } from '@tauri-apps/api/core';
import { emit, listen, type UnlistenFn } from '@tauri-apps/api/event';
import { WebviewWindow } from '@tauri-apps/api/webviewWindow';
import { open } from '@tauri-apps/plugin-dialog';
import { save } from '@tauri-apps/plugin-dialog';
import { useStorage } from '@vueuse/core';
import {
  Activity,
  ArrowLeftRight,
  Camera,
  CheckCircle2,
  Copy,
  Download,
  Ear,
  Eye,
  FileText,
  Gamepad2,
  Headphones,
  HelpCircle,
  Languages,
  Layers,
  MessageSquare,
  Mic,
  MicOff,
  MonitorUp,
  Plus,
  Radio,
  RefreshCw,
  RotateCcw,
  Send,
  Settings,
  Sliders,
  SlidersHorizontal,
  Square,
  Trash2,
  Tv,
  Upload,
  Volume2,
  VolumeX,
  X,
} from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';
import {
  GalleryApi,
  SysApi,
  VrctApi,
  VrpianoApi,
  type AudioDevice,
  type AudioSource,
  type AudioSessionInfo,
  type KeywordAction,
} from '../api';
import * as TranslationApis from '../api';
import { SerialTaskQueue } from '../utils/serialTaskQueue';
import { katakanaToHiragana, toRomaji } from '../utils/transliteration';
import CustomSelect from './CustomSelect.vue';

type MessageSource = 'chat' | 'mic' | 'speaker' | 'ocr';
type TtsEngine = 'system' | 'edge' | 'gpt_sovits' | 'server';
type TranslationRoute = 'manual' | 'mic' | 'speaker' | 'photo';

interface TranslationProfile {
  id: string;
  name: string;
  engine: string;
  apiKey: string;
  model: string;
  customApiUrl: string;
  prompt: string;
}

interface Option {
  label: string;
  value: string;
}

interface EngineOption extends Option {
  needsKey?: boolean;
  supportsLocal?: boolean;
  hint: string;
}

interface VrctMessageRecord {
  id: number;
  source: MessageSource;
  original: string;
  translated: string;
  source_lang: string;
  target_lang: string;
  translations?: Array<{ target_lang: string; translated: string }>;
  service: string;
  sent_osc: boolean;
  overlay_updated: boolean;
  timestamp: string;
}

interface GlossaryTerm {
  source: string;
  target: string;
  source_lang: string;
  target_lang: string;
  case_sensitive: boolean;
}

interface PresetConfig {
  sourceLang: string;
  target1: string;
  target2: string;
  target3: string;
  enableTarget2: boolean;
  enableTarget3: boolean;
  engine: string;
}

const { t } = useI18n();

const tt = (key: string, fallback: string) => {
  const value = t(key);
  return value === key ? fallback : value;
};

// ---------------------------------------------------------------------------
// Language & Engine Dictionaries
// ---------------------------------------------------------------------------
const languageOptions = computed<Option[]>(() => [
  { label: tt('translator.language_auto', 'Auto detect'), value: 'auto' },
  { label: `${tt('translator.language_chinese', 'Chinese')} (zh-CN)`, value: 'zh-CN' },
  { label: `${tt('translator.language_english', 'English')} (en-US)`, value: 'en-US' },
  { label: `${tt('translator.language_japanese', 'Japanese')} (ja-JP)`, value: 'ja-JP' },
  { label: `${tt('translator.language_korean', 'Korean')} (ko-KR)`, value: 'ko-KR' },
  { label: `${tt('translator.language_french', 'French')} (fr-FR)`, value: 'fr' },
  { label: `${tt('translator.language_german', 'German')} (de-DE)`, value: 'de' },
  { label: `${tt('translator.language_spanish', 'Spanish')} (es-ES)`, value: 'es' },
  { label: `${tt('translator.language_russian', 'Russian')} (ru-RU)`, value: 'ru' },
  { label: `${tt('translator.language_portuguese', 'Portuguese')} (pt-BR)`, value: 'pt-BR' },
  { label: `${tt('translator.language_thai', 'Thai')} (th-TH)`, value: 'th' },
  { label: `${tt('translator.language_vietnamese', 'Vietnamese')} (vi-VN)`, value: 'vi' },
]);

const speechLanguageOptions = computed(() => languageOptions.value.filter((opt) => opt.value !== 'auto'));

const engineOptions = computed<EngineOption[]>(() => [
  { label: 'Google Translate Free', value: 'google_free', hint: tt('translator.hint_no_api_key', 'No API key required') },
  { label: 'Google Cloud Translation', value: 'google_cloud', needsKey: true, hint: tt('translator.hint_google_cloud_key', 'Google Cloud API key') },
  { label: 'Microsoft Translator', value: 'microsoft', needsKey: true, hint: tt('translator.hint_azure_key', 'Azure Translator key') },
  { label: 'DeepL Free', value: 'deepl_free', needsKey: true, hint: tt('translator.hint_deepl_free_key', 'DeepL Free auth key') },
  { label: 'DeepL Pro', value: 'deepl', needsKey: true, hint: tt('translator.hint_deepl_pro_key', 'DeepL Pro auth key') },
  { label: 'CTranslate2 (NLLB Local)', value: 'ctranslate2', supportsLocal: true, hint: 'Offline CTranslate2 INT8 model' },
  { label: 'Papago', value: 'papago', needsKey: true, hint: 'ClientId:ClientSecret' },
  { label: 'OpenAI API (GPT-4o)', value: 'openai', needsKey: true, hint: tt('translator.hint_openai_key', 'OpenAI compatible API key') },
  { label: 'Gemini', value: 'gemini', needsKey: true, hint: tt('translator.hint_google_ai_key', 'Google AI Studio key') },
  { label: 'Groq API', value: 'groq', needsKey: true, hint: 'Groq API key' },
  { label: 'DeepSeek', value: 'deepseek', needsKey: true, hint: 'DeepSeek API key' },
  { label: 'OpenRouter', value: 'openrouter', needsKey: true, hint: 'OpenRouter API key' },
  { label: 'Plamo', value: 'plamo', needsKey: true, hint: 'PreferredAI Platform API key' },
  { label: 'Alibaba Qwen (通义千问)', value: 'qwen', needsKey: true, hint: tt('translator.hint_qwen_key', 'DashScope API Key') },
  { label: 'Tencent Translate', value: 'tencent', needsKey: true, hint: 'SecretId:SecretKey' },
  { label: 'Baidu Translate', value: 'baidu', needsKey: true, hint: 'AppID:SecretKey' },
  { label: tt('translator.engine_ollama_local', 'Ollama Local'), value: 'ollama', supportsLocal: true, hint: 'http://127.0.0.1:11434' },
  { label: tt('translator.engine_lmstudio_local', 'LM Studio Local'), value: 'lmstudio', supportsLocal: true, hint: 'http://127.0.0.1:1234' },
  { label: tt('translator.engine_custom_openai', 'Custom OpenAI API'), value: 'custom_llm', needsKey: true, hint: tt('translator.hint_openai_endpoint', 'OpenAI-compatible endpoint') },
]);

const sttEngineOptions = computed<Option[]>(() => [
  { label: tt('translator.engine_cloud', 'Cloud recognition (Google Web Speech)'), value: 'cloud' },
  { label: `${tt('translator.engine_local', 'Local Faster-Whisper')} (Offline)`, value: 'whisper' },
  { label: tt('translator.engine_groq_whisper', 'Groq Whisper (Ultra-fast Cloud)'), value: 'groq_whisper' },
  { label: tt('translator.engine_openai_whisper', 'OpenAI Whisper (Official Cloud)'), value: 'openai_whisper' },
  { label: tt('translator.engine_custom_whisper', 'Custom Whisper Server (OpenAI Compatible)'), value: 'custom_whisper' },
  { label: tt('translator.engine_deepgram', 'Deepgram Nova ASR'), value: 'deepgram' },
  { label: tt('translator.engine_sherpa', 'Sherpa-ONNX streaming'), value: 'sherpa' },
  { label: tt('translator.engine_tencent_realtime', 'Tencent Cloud realtime ASR'), value: 'tencent_realtime' },
  { label: tt('translator.engine_aliyun_realtime', 'Alibaba Cloud NLS realtime ASR'), value: 'aliyun_realtime' },
]);

// ---------------------------------------------------------------------------
// Presets System (VRCT Parity: Tab 1, Tab 2, Tab 3)
// ---------------------------------------------------------------------------
const activePresetTab = useStorage<'1' | '2' | '3'>('vrc_translator_preset_tab', '1');
const presets = useStorage<Record<'1' | '2' | '3', PresetConfig>>('vrc_translator_presets_v3', {
  '1': {
    sourceLang: 'zh-CN',
    target1: 'en-US',
    target2: 'ja-JP',
    target3: 'ko-KR',
    enableTarget2: false,
    enableTarget3: false,
    engine: 'google_free',
  },
  '2': {
    sourceLang: 'ja-JP',
    target1: 'zh-CN',
    target2: 'en-US',
    target3: 'ko-KR',
    enableTarget2: false,
    enableTarget3: false,
    engine: 'deepl_free',
  },
  '3': {
    sourceLang: 'en-US',
    target1: 'zh-CN',
    target2: 'ja-JP',
    target3: 'ko-KR',
    enableTarget2: false,
    enableTarget3: false,
    engine: 'openai',
  },
});

const activePreset = computed(() => presets.value[activePresetTab.value]);

const sourceLang = computed({
  get: () => activePreset.value.sourceLang,
  set: (val: string) => { presets.value[activePresetTab.value].sourceLang = val; },
});

const targetLang = computed({
  get: () => activePreset.value.target1,
  set: (val: string) => { presets.value[activePresetTab.value].target1 = val; },
});

const targetLang2 = computed({
  get: () => activePreset.value.target2,
  set: (val: string) => { presets.value[activePresetTab.value].target2 = val; },
});

const targetLang3 = computed({
  get: () => activePreset.value.target3,
  set: (val: string) => { presets.value[activePresetTab.value].target3 = val; },
});

const enableTarget2 = computed({
  get: () => activePreset.value.enableTarget2,
  set: (val: boolean) => { presets.value[activePresetTab.value].enableTarget2 = val; },
});

const enableTarget3 = computed({
  get: () => activePreset.value.enableTarget3,
  set: (val: boolean) => { presets.value[activePresetTab.value].enableTarget3 = val; },
});

const translateEngine = computed({
  get: () => activePreset.value.engine,
  set: (val: string) => { presets.value[activePresetTab.value].engine = val; },
});

const swapLanguages = () => {
  const currentSource = activePreset.value.sourceLang;
  const currentTarget = activePreset.value.target1;
  const newSource = currentTarget === 'auto' ? 'zh-CN' : currentTarget;
  const newTarget = currentSource === 'auto' ? 'en-US' : currentSource;
  presets.value[activePresetTab.value].sourceLang = newSource;
  presets.value[activePresetTab.value].target1 = newTarget;
  setStatus(tt('translator.swap_languages', '已对调源语言与目标语言'));
};

// ---------------------------------------------------------------------------
// 4 Master Switches (Top Bar)
// ---------------------------------------------------------------------------
const masterTranslation = useStorage('vrc_translator_master_translation', true);
const isRecording = ref(false);
const isOtherRecording = ref(false);
const isMicStarting = ref(false);
const isSpeakerStarting = ref(false);
const masterOcr = useStorage('vrc_translator_master_ocr', false);

// ---------------------------------------------------------------------------
// 8 Sub-navigation Tabs
// ---------------------------------------------------------------------------
type SubTabId = 'dashboard' | 'routing' | 'transcription' | 'translation' | 'osc' | 'ocr' | 'vr_obs' | 'preferences';
const activeSubTab = useStorage<SubTabId>('vrc_translator_active_subtab', 'dashboard');
const isSimpleMode = useStorage('vrc_translator_simple_mode', false);

// ---------------------------------------------------------------------------
// Speech & Audio Configuration
// ---------------------------------------------------------------------------
const micEngine = useStorage<'cloud' | 'local' | 'whisper' | 'sensevoice' | 'sherpa' | 'tencent_realtime' | 'aliyun_realtime' | 'groq_whisper' | 'openai_whisper' | 'custom_whisper' | 'deepgram'>('vrc_translator_mic_stt_engine', 'cloud');
const otherEngine = useStorage<'cloud' | 'local' | 'whisper' | 'sensevoice' | 'sherpa' | 'tencent_realtime' | 'aliyun_realtime' | 'groq_whisper' | 'openai_whisper' | 'custom_whisper' | 'deepgram'>('vrc_translator_stt_engine', 'cloud');

const whisperModel = useStorage('vrc_translator_whisper_model', 'tiny');
const whisperDevice = useStorage<'cpu' | 'cuda'>('vrc_translator_whisper_device', 'cpu');
const whisperComputeType = useStorage('vrc_translator_whisper_compute_type', 'int8');
const groqApiKey = useStorage('vrc_translator_groq_api_key', '');
const openaiApiKey = useStorage('vrc_translator_openai_whisper_key', '');
const customWhisperUrl = useStorage('vrc_translator_custom_whisper_url', 'http://127.0.0.1:8000/v1');
const customWhisperKey = useStorage('vrc_translator_custom_whisper_key', '');
const deepgramApiKey = useStorage('vrc_translator_deepgram_key', '');
const deepgramModel = useStorage('vrc_translator_deepgram_model', 'nova-2');

const micDeviceId = useStorage('vrc_translator_mic_device', '');
const speakerDeviceId = useStorage('vrc_translator_speaker_device', '');
const micEnergyThreshold = useStorage('vrc_translator_mic_energy_threshold', 0);
const speakerEnergyThreshold = useStorage('vrc_translator_speaker_energy_threshold', 0);
const micAutoThreshold = useStorage('vrc_translator_mic_auto_threshold', true);
const speakerAutoThreshold = useStorage('vrc_translator_speaker_auto_threshold', true);
const vadMode = useStorage<'webrtc' | 'silero' | 'rms'>('vrc_translator_vad_type', 'webrtc');
const vadAggressiveness = useStorage('vrc_translator_vad_aggressiveness', 2);
const denoiseStrength = useStorage('vrc_translator_denoise_strength', 0);
const phraseTimeLimit = useStorage('vrc_translator_phrase_time_limit', 10);
const silenceTimeout = useStorage('vrc_translator_silence_timeout', 0.6);
const speakerContextIdleSec = useStorage('vrc_translator_speaker_context_idle_sec', 60);
const micIdleDisconnectSec = useStorage('vrc_translator_mic_idle_disconnect_sec', 15);
const keepSpeakerContext = useStorage('vrc_translator_keep_speaker_context', true);
const captureMode = useStorage('vrc_translator_capture_mode', 'loopback');
const targetProcess = useStorage('vrc_translator_target_process', 'VRChat.exe');
const selfSuppress = useStorage('vrc_translator_self_suppress', false);
const selfSuppressSeconds = useStorage('vrc_translator_self_suppress_seconds', 0.8);
const correctionEnabled = useStorage('vrc_translator_correction_enabled', false);

// Word Filter Redaction List (VRCT MIC_WORD_FILTER)
const wordFilterList = useStorage<string[]>('vrc_translator_word_filter_list', []);
const newFilterWord = ref('');
const addFilterWord = () => {
  const word = newFilterWord.value.trim();
  if (word && !wordFilterList.value.includes(word)) {
    wordFilterList.value.push(word);
    newFilterWord.value = '';
    setStatus(tt('translator.word_filter_added', '已添加敏感词规则'));
  }
};
const removeFilterWord = (idx: number) => {
  wordFilterList.value.splice(idx, 1);
};

// ---------------------------------------------------------------------------
// Audio Routing (MIXLINE Visualizer)
// ---------------------------------------------------------------------------
const micMuted = ref(false);
const micVolume = useStorage('vrc_translator_mic_volume', 100);
const speakerMuted = ref(false);
const speakerVolume = useStorage('vrc_translator_speaker_volume', 100);
const vrchatStreamMuted = ref(false);
const vrchatStreamVolume = useStorage('vrc_translator_stream_volume', 100);
const headphoneMonitorEnabled = useStorage('vrc_translator_monitor_enabled', false);
const headphoneMonitorVolume = useStorage('vrc_translator_monitor_volume', 100);

// ---------------------------------------------------------------------------
// VRChat OSC Formatting & Chatbox
// ---------------------------------------------------------------------------
const autoSendOsc = useStorage('vrc_translator_auto_send_osc', true);
const showOriginalOsc = useStorage('vrc_translator_show_original', true);
const sendReceivedMessageToVrc = useStorage('vrc_translator_speaker2chatbox', false);
const sendOnlyTranslatedMessages = useStorage('vrc_translator_send_only_translated', false);
const notificationVrcSfx = useStorage('vrc_translator_notification_sfx', true);
const vrcMicMuteSync = useStorage('vrc_translator_mute_sync', false);
const autoClearMessageBox = useStorage('vrc_translator_auto_clear_msgbox', true);
const oscIpAddress = useStorage('vrc_translator_osc_ip', '127.0.0.1');
const oscPort = useStorage('vrc_translator_osc_port', 9000);
const avatarOscSync = useStorage('vrc_translator_avatar_osc_sync', true);
const avatarOscParam = useStorage('vrc_translator_avatar_osc_param', '/avatar/parameters/VRCT_IsTalking');

const sendMessageFormat = useStorage('vrc_translator_format_send', {
  prefix: '',
  suffix: '',
  separator: '\n',
  transPrefix: '',
  transSuffix: '',
  transFirst: false,
});

const receivedMessageFormat = useStorage('vrc_translator_format_receive', {
  prefix: '',
  suffix: '',
  separator: '\n',
  transPrefix: '',
  transSuffix: '',
  transFirst: false,
});

// ---------------------------------------------------------------------------
// Chat-Bubble OCR (VRCT OCR)
// ---------------------------------------------------------------------------
const ocrWindowTitle = useStorage('vrc_translator_ocr_window_title', 'VRChat');
const ocrSourceLang = useStorage('vrc_translator_ocr_source_lang', 'auto');
const ocrPollIntervalMs = useStorage('vrc_translator_ocr_poll_ms', 750);
const ocrMinConfidence = useStorage('vrc_translator_ocr_min_conf', 0.85);
const ocrBubbleMinLength = useStorage('vrc_translator_ocr_min_len', 2);
let ocrIntervalHandle: ReturnType<typeof setInterval> | null = null;

// ---------------------------------------------------------------------------
// VR Overlay & OBS Browser Source
// ---------------------------------------------------------------------------
const overlaySmallLog = useStorage('vrc_translator_vr_small_log', false);
const overlayLargeLog = useStorage('vrc_translator_vr_large_log', false);
const overlayTracker = useStorage<'HMD' | 'LeftHand' | 'RightHand' | 'Playspace'>('vrc_translator_vr_tracker', 'HMD');
const overlayBackgroundOpacity = useStorage('vrc_translation_overlay_opacity', 0.82);

const obsBrowserSource = useStorage('vrc_translator_obs_enabled', false);
const obsPort = useStorage('vrc_translator_obs_port', 2232);
const obsMaxMessages = useStorage('vrc_translator_obs_max_msg', 14);
const obsDisplayDuration = useStorage('vrc_translator_obs_duration', 60);
const obsFontSize = useStorage('vrc_translator_obs_font_size', 40);
const obsFontColor = useStorage('vrc_translator_obs_font_color', '#FFFFFF');
const obsOutlineThickness = useStorage('vrc_translator_obs_outline_thickness', 3);
const obsOutlineColor = useStorage('vrc_translator_obs_outline_color', '#000000');

const websocketServer = useStorage('vrc_translator_ws_enabled', false);
const websocketPort = useStorage('vrc_translator_ws_port', 2231);
const websocketAuthToken = useStorage('vrc_translator_ws_token', () => Math.random().toString(36).substring(2) + Math.random().toString(36).substring(2));

// ---------------------------------------------------------------------------
// Preferences & Transliteration
// ---------------------------------------------------------------------------
const romajiMode = useStorage('vrc_translator_romaji_mode', false);
const hiraganaMode = useStorage('vrc_translator_hiragana_mode', false);
const sendMessageButtonType = useStorage<'show' | 'hide' | 'show_and_disable_enter_key'>('vrc_translator_send_btn_type', 'show');
const showResendButton = useStorage('vrc_translator_show_resend_btn', true);
const textOnlyMode = useStorage('vrc_translator_text_only_mode', false);
const autoPlayTts = useStorage('vrc_translator_auto_tts', false);
const ttsEngine = useStorage<TtsEngine>('vrc_translator_tts_engine', 'system');
const ttsRate = useStorage('vrc_translator_tts_rate', 1);
const ttsVolume = useStorage('vrc_translator_tts_volume', 1);
const interruptTts = useStorage('vrc_translator_tts_interrupt', true);
const ttsDeviceId = useStorage('vrc_translator_tts_device', '');

// Hotkeys
const hotkeyToggleWindow = useStorage('vrc_translator_hotkey_window', 'Ctrl+Alt+V');
const hotkeyToggleTrans = useStorage('vrc_translator_hotkey_trans', 'Ctrl+Alt+T');
const hotkeyToggleMic = useStorage('vrc_translator_hotkey_mic', 'Ctrl+F8');
const hotkeyToggleSpeaker = useStorage('vrc_translator_hotkey_speaker', 'Ctrl+F9');

// ---------------------------------------------------------------------------
// Security & API Keys Storage
// ---------------------------------------------------------------------------
const SECURE_API_KEY_NAME = 'translator_api_key';
const WEB_API_KEY_STORAGE = 'vrc_translator_api_key';
const apiKey = ref<string>('');
const persistApiKey = async (val: string) => {
  if (isTauri()) {
    try {
      await invoke('sys_store_secure_string', { key: SECURE_API_KEY_NAME, value: val });
    } catch (e) {
      console.warn('[Translator] Secure storage error:', e);
    }
    return;
  }
  try {
    sessionStorage.setItem(WEB_API_KEY_STORAGE, val);
  } catch {
    // ignore
  }
};
watch(apiKey, (val) => { void persistApiKey(val); });

const model = useStorage('vrc_translator_model', '');
const customApiUrl = useStorage('vrc_translator_custom_api_url', '');
const prompt = useStorage('vrc_translator_prompt', '');
const retryCount = useStorage('vrc_translator_retry_count', 2);
const contextEnabled = useStorage('vrc_translator_context_enabled', true);
const glossary = useStorage<GlossaryTerm[]>('vrc_translator_glossary', []);
const translationProfiles = useStorage<TranslationProfile[]>('vrc_translator_profiles', []);
const activeProfileId = useStorage('vrc_translator_active_profile', '');
const routeProfileIds = useStorage<Record<TranslationRoute, string>>('vrc_translator_route_profiles', {
  manual: '',
  mic: '',
  speaker: '',
  photo: '',
});

// Live State
const manualText = ref('');
const recognizedText = ref('');
const translatedText = ref('');
const currentTranslations = ref<Array<{ target_lang: string; translated: string }>>([]);
const history = ref<VrctMessageRecord[]>([]);
const audioLevels = ref<Record<'mic' | 'speaker', number>>({ mic: 0, speaker: 0 });
const isTranslating = ref(false);
const isOverlayOpen = ref(false);
const errorMsg = ref('');
const statusMsg = ref('');
const audioDevices = ref<AudioDevice[]>([]);
const activeAudioSessions = ref<AudioSessionInfo[]>([]);
const isLoadingSessions = ref(false);
const showCableGuide = ref(false);
const photoBusy = ref(false);
const photoArmed = ref(false);
const photoPath = ref('');
const photoOriginal = ref('');
const photoTranslated = ref('');
const otherSourceLang = useStorage('vrc_translator_other_source_lang', 'en-US');
const otherTargetLang = useStorage('vrc_translator_other_target_lang', 'zh-CN');
const appMode = useStorage<'vrchat' | 'discord' | 'custom'>('vrc_translator_app_mode', 'vrchat');
const micPassthrough = useStorage('vrc_translator_mic_passthrough', false);
const qwenEndpoint = useStorage<'beijing' | 'intl'>('vrc_translator_qwen_endpoint', 'beijing');
const qwenWorkspaceId = useStorage('vrc_translator_qwen_workspace_id', '');
const gptSovitsUrl = useStorage('vrc_translator_gpt_sovits_url', 'http://127.0.0.1:9880');
const gptSovitsWeights = useStorage('vrc_translator_sovits_weights', '');
const gptWeights = useStorage('vrc_translator_gpt_weights', '');
const gptReferenceAudio = useStorage('vrc_translator_reference_audio', '');
const gptPromptText = useStorage('vrc_translator_prompt_text', '');
const gptPromptLanguage = useStorage('vrc_translator_prompt_language', 'zh');
const serverTtsProvider = useStorage('vrc_translator_server_tts_provider', 'edge');
const serverTtsBaseUrl = useStorage('vrc_translator_server_tts_url', '');
const serverTtsApiKey = useStorage('vrc_translator_server_tts_key', '');
const serverTtsVoice = useStorage('vrc_translator_server_tts_voice', '');
const ttsReferenceText = useStorage('vrc_translator_tts_reference_text', '');
const ttsPresets = ref<any[]>([]);
const activeTtsPresetId = useStorage('vrc_translator_tts_preset_id', '');
const runtimeVersion = ref('builtin');
const sherpaConfig = useStorage('vrc_translator_sherpa_config', { tokens: '', encoder: '', decoder: '', joiner: '' });
const realtimeAsrConfig = useStorage('vrc_translator_realtime_asr_config', {
  provider: 'tencent_realtime',
  appId: '',
  secretId: '',
  secretKey: '',
  appKey: '',
  accessKeyId: '',
  accessKeySecret: '',
  accessToken: '',
  model: '16k_zh_en',
});

let overlayWebview: WebviewWindow | null = null;
let unlistenAudio: UnlistenFn | null = null;
let unlistenVrct: UnlistenFn | null = null;
let unlistenTranslationHotkey: UnlistenFn | null = () => {};
let translatorDisposed = false;
let speakerSuppressedBySelf = false;
let selfSuppressTimer: ReturnType<typeof setTimeout> | null = null;

const currentEngine = computed(() => engineOptions.value.find((e) => e.value === translateEngine.value) ?? engineOptions.value[0]);
const needsApiKey = computed(() => Boolean(currentEngine.value.needsKey && !currentEngine.value.supportsLocal));
const canTranslate = computed(() => !isTranslating.value && Boolean(manualText.value.trim()));
const normalizedGlossary = computed(() => glossary.value.filter((term) => term.source.trim() && term.target.trim()).slice(0, 128));

const translationQueue = new SerialTaskQueue((pending) => {
  isTranslating.value = pending > 0;
});

const setStatus = (message: string) => {
  statusMsg.value = message;
  window.setTimeout(() => {
    if (statusMsg.value === message) statusMsg.value = '';
  }, 3500);
};

const errorText = (error: unknown) => {
  if (error instanceof Error) return error.message;
  if (typeof error === 'string') return error;
  return JSON.stringify(error);
};

const setAvatarTalking = async (talking: boolean) => {
  if (!avatarOscSync.value || !isTauri()) return;
  try {
    await SysApi.sendOscParam({
      address: avatarOscParam.value.trim() || '/avatar/parameters/VRCT_IsTalking',
      value: talking ? 1.0 : 0.0,
    });
  } catch (e) {
    console.warn('[Translator] Failed to sync avatar OSC param:', e);
  }
};

const routeConfig = (route: TranslationRoute) => {
  const profile = translationProfiles.value.find((item) => item.id === routeProfileIds.value[route]);
  const base = profile
    ? {
        engine: profile.engine,
        apiKey: profile.apiKey,
        model: profile.model,
        customApiUrl: profile.customApiUrl,
        prompt: profile.prompt,
      }
    : {
        engine: translateEngine.value,
        apiKey: apiKey.value,
        model: model.value,
        customApiUrl: customApiUrl.value,
        prompt: prompt.value,
      };

  if (base.engine === 'qwen') {
    let url = base.customApiUrl;
    if (!url) {
      url = qwenEndpoint.value === 'intl'
        ? 'https://dashscope-intl.aliyuncs.com/compatible-mode/v1/chat/completions'
        : 'https://dashscope.aliyuncs.com/compatible-mode/v1/chat/completions';
    }
    if (qwenWorkspaceId.value.trim()) {
      url += (url.includes('?') ? '&' : '?') + `workspace=${encodeURIComponent(qwenWorkspaceId.value.trim())}`;
    }
    base.customApiUrl = url;
  }
  return base;
};

// ---------------------------------------------------------------------------
// Processing & OSC Transmission
// ---------------------------------------------------------------------------
const buildTargetLanguages = (): string[] => {
  const list = [activePreset.value.target1];
  if (activePreset.value.enableTarget2 && activePreset.value.target2 && activePreset.value.target2 !== activePreset.value.target1) {
    list.push(activePreset.value.target2);
  }
  if (activePreset.value.enableTarget3 && activePreset.value.target3 && !list.includes(activePreset.value.target3)) {
    list.push(activePreset.value.target3);
  }
  return list;
};

const processMessageNow = async (
  text: string,
  source: MessageSource,
  sourceLanguage: string,
  targetLanguage: string,
  sendOsc: boolean,
) => {
  const trimmed = text.trim();
  if (!trimmed) return null;

  // Check word filter redaction
  const lower = trimmed.toLowerCase();
  if (wordFilterList.value.some((filter) => filter && lower.includes(filter.toLowerCase()))) {
    setStatus(tt('translator.word_filter_blocked', '消息触发敏感词过滤，已拦截发送'));
    return null;
  }

  // If master translation is off, pass text directly
  if (!masterTranslation.value) {
    const rawRecord: VrctMessageRecord = {
      id: Date.now(),
      source,
      original: trimmed,
      translated: trimmed,
      source_lang: sourceLanguage,
      target_lang: targetLanguage,
      translations: [{ target_lang: targetLanguage, translated: trimmed }],
      service: 'passthrough',
      sent_osc: sendOsc,
      overlay_updated: true,
      timestamp: new Date().toLocaleTimeString(),
    };
    applyRecord(rawRecord);
    if (sendOsc && autoSendOsc.value) {
      await SysApi.sendOscChatbox({ text: trimmed, complete: true }).catch(() => undefined);
    }
    return rawRecord;
  }

  const route: TranslationRoute = source === 'chat' ? 'manual' : source === 'mic' ? 'mic' : 'speaker';
  const selectedConfig = routeConfig(route);
  const selectedEngine = engineOptions.value.find((e) => e.value === selectedConfig.engine) ?? engineOptions.value[0];
  if (selectedEngine.needsKey && !selectedEngine.supportsLocal && !selectedConfig.apiKey.trim()) {
    errorMsg.value = `${selectedEngine.label}: ${selectedEngine.hint}`;
    return null;
  }

  errorMsg.value = '';
  try {
    const targetLangs = buildTargetLanguages();
    const fmt = source === 'speaker' ? receivedMessageFormat.value : sendMessageFormat.value;

    const record = await VrctApi.processMessage({
      req: {
        text: trimmed,
        source: source === 'ocr' ? 'ocr' : source,
        source_lang: sourceLanguage,
        target_lang: targetLanguage,
        target_langs: targetLangs.filter((l) => l !== targetLanguage),
        service: selectedConfig.engine,
        api_key: selectedConfig.apiKey.trim(),
        model: selectedConfig.model.trim(),
        prompt: selectedConfig.prompt.trim(),
        custom_api_url: selectedConfig.customApiUrl.trim(),
        send_osc: sendOsc && (source !== 'speaker' || sendReceivedMessageToVrc.value),
        send_typing: sendOsc,
        complete: true,
        notification: notificationVrcSfx.value,
        update_overlay: true,
        show_original_in_osc: showOriginalOsc.value && !sendOnlyTranslatedMessages.value,
        message_prefix: fmt.prefix,
        message_suffix: fmt.suffix,
        translation_prefix: fmt.transPrefix,
        translation_suffix: fmt.transSuffix,
        separator: fmt.separator,
        translation_first: fmt.transFirst,
        glossary: normalizedGlossary.value,
        context: contextEnabled.value ? history.value.slice(0, 6).map((item) => item.original) : [],
        retry_count: Math.min(3, Math.max(0, Number(retryCount.value) || 0)),
      },
    }) as VrctMessageRecord;

    applyRecord(record);
    await notifyOverlay(record);

    if (!textOnlyMode.value && autoPlayTts.value && record.translated) {
      await playTts(record.translated, targetLanguage);
    }

    return record;
  } catch (error) {
    errorMsg.value = `${tt('translator.network_error', 'Translation request failed.')} ${errorText(error)}`;
    return null;
  }
};

const queueMessage = (
  text: string,
  source: MessageSource,
  sourceLanguage: string,
  targetLanguage: string,
  sendOsc: boolean,
) => translationQueue.enqueue(() => processMessageNow(text, source, sourceLanguage, targetLanguage, sendOsc));

const translateManual = async () => {
  const text = manualText.value.trim();
  if (!text) return;
  const record = await queueMessage(text, 'chat', sourceLang.value, targetLang.value, autoSendOsc.value);
  if (record && autoClearMessageBox.value) {
    manualText.value = '';
  }
};

const translateMicText = (text: string) => queueMessage(text, 'mic', sourceLang.value, targetLang.value, autoSendOsc.value);
const translateSpeakerText = (text: string) => queueMessage(text, 'speaker', otherSourceLang.value, otherTargetLang.value, sendReceivedMessageToVrc.value);

const notifyOverlay = async (record: VrctMessageRecord) => {
  await emit('translation-log', {
    id: record.id,
    type: record.source === 'speaker' ? 'other' : 'self',
    text: record.original,
    translation: record.translated,
    translations: record.translations?.map((item) => ({
      lang: item.target_lang,
      text: item.translated,
    })),
  });
};

const applyRecord = (record: VrctMessageRecord) => {
  recognizedText.value = record.original;
  translatedText.value = record.translated;
  currentTranslations.value = record.translations?.length
    ? record.translations
    : [{ target_lang: record.target_lang, translated: record.translated }];
  if (!history.value.some((item) => item.id === record.id)) {
    history.value = [record, ...history.value].slice(0, 100);
  }
};

const playTts = async (text: string, lang = targetLang.value) => {
  if (!text.trim() || textOnlyMode.value) return;
  const pauseLoopback = isTauri() && isOtherRecording.value;
  try {
    void setAvatarTalking(true);
    if (pauseLoopback) {
      await SysApi.setAudioCapturePaused({ source: 'speaker', paused: true }).catch(() => undefined);
    }
    const targetOutputDevice = ttsDeviceId.value
      ? audioDevices.value.find((d) => d.id === ttsDeviceId.value && d.source === 'output')
      : undefined;
    const deviceIndex = targetOutputDevice?.index;

    if (ttsEngine.value === 'system') {
      if (!('speechSynthesis' in window)) throw new Error('Speech synthesis unsupported');
      if (interruptTts.value) window.speechSynthesis.cancel();
      const utterance = new SpeechSynthesisUtterance(text);
      utterance.lang = lang;
      utterance.rate = Math.min(2, Math.max(0.5, Number(ttsRate.value) || 1));
      utterance.volume = Math.min(1, Math.max(0, Number(ttsVolume.value) || 0));
      await new Promise<void>((resolve, reject) => {
        utterance.onend = () => resolve();
        utterance.onerror = (e) => reject(new Error(e.error || 'Speech synthesis failed'));
        window.speechSynthesis.speak(utterance);
      });
    } else if (ttsEngine.value === 'edge') {
      const result = await VrpianoApi.synthesizeSpeech({
        text,
        voice: lang.startsWith('ja') ? 'ja-JP-NanamiNeural' : lang.startsWith('ko') ? 'ko-KR-SunHiNeural' : lang.startsWith('en') ? 'en-US-AriaNeural' : 'zh-CN-XiaoxiaoNeural',
        rate: Number(ttsRate.value) || 1,
        volume: Number(ttsVolume.value) || 1,
      });
      if (deviceIndex !== undefined && SysApi.playAudioToDevice) {
        await SysApi.playAudioToDevice({
          filePath: result.output_path,
          deviceIndex,
          volume: Math.min(1, Math.max(0, Number(ttsVolume.value) || 1)),
        });
      } else {
        const audio = new Audio(convertFileSrc(result.output_path));
        audio.volume = Math.min(1, Math.max(0, Number(ttsVolume.value) || 0));
        await new Promise<void>((resolve, reject) => {
          audio.onended = () => resolve();
          audio.onerror = () => reject(new Error('Audio playback failed'));
          audio.play().catch(reject);
        });
      }
    }
  } catch (error) {
    errorMsg.value = `${tt('translator.tts_error', 'TTS failed')}: ${errorText(error)}`;
  } finally {
    void setAvatarTalking(false);
    if (pauseLoopback) {
      await SysApi.setAudioCapturePaused({ source: 'speaker', paused: false }).catch(() => undefined);
    }
  }
};

// ---------------------------------------------------------------------------
// Audio Capture Control
// ---------------------------------------------------------------------------
const selectedDeviceIndex = (source: AudioSource) => {
  const id = source === 'mic' ? micDeviceId.value : speakerDeviceId.value;
  return audioDevices.value.find((d) => d.id === id)?.index;
};

const startCapture = async (source: AudioSource) => {
  errorMsg.value = '';
  const isMic = source === 'mic';
  if (isMic) isMicStarting.value = true;
  else isSpeakerStarting.value = true;
  try {
    const selectedEng = isMic ? micEngine.value : otherEngine.value;
    await SysApi.startAudioCapture({
      source,
      sourceLang: isMic ? sourceLang.value : otherSourceLang.value,
      engine: selectedEng,
      realtimeProvider: realtimeAsrConfig.value.provider,
      realtimeConfig: realtimeAsrConfig.value,
      sherpaConfig: sherpaConfig.value,
      deviceIndex: selectedDeviceIndex(source),
      energyThreshold: isMic ? Number(micEnergyThreshold.value) : Number(speakerEnergyThreshold.value),
      dynamicEnergyThreshold: isMic ? micAutoThreshold.value : speakerAutoThreshold.value,
      phraseTimeLimit: Number(phraseTimeLimit.value),
      whisperModel: whisperModel.value,
      whisperDevice: whisperDevice.value,
      whisperComputeType: whisperComputeType.value,
      groqApiKey: groqApiKey.value,
      openaiApiKey: openaiApiKey.value,
      customWhisperUrl: customWhisperUrl.value,
      customWhisperKey: customWhisperKey.value,
      deepgramApiKey: deepgramApiKey.value,
      deepgramModel: deepgramModel.value,
      wordFilter: wordFilterList.value,
      vadType: vadMode.value,
      vadAggressiveness: Number(vadAggressiveness.value),
      denoiseStrength: Number(denoiseStrength.value),
      correctionEnabled: Boolean(correctionEnabled.value),
      captureMode: captureMode.value,
      targetProcess: targetProcess.value,
      selfSuppressSeconds: Number(selfSuppressSeconds.value),
      silenceTimeout: keepSpeakerContext.value ? Number(speakerContextIdleSec.value) : Number(silenceTimeout.value),
      speakerContextIdleSec: Number(speakerContextIdleSec.value),
      micIdleDisconnectSec: Number(micIdleDisconnectSec.value),
    });
    if (isMic) isRecording.value = true;
    else isOtherRecording.value = true;
    setStatus(t('translator.starting_audio_recognition_service'));
  } catch (error) {
    errorMsg.value = `${tt('translator.capture_error_cloud', 'Audio capture failed')}: ${errorText(error)}`;
    if (isMic) isRecording.value = false;
    else isOtherRecording.value = false;
  } finally {
    if (isMic) isMicStarting.value = false;
    else isSpeakerStarting.value = false;
  }
};

const stopCapture = async (source: AudioSource) => {
  await SysApi.stopAudioCapture({ source }).catch(() => undefined);
  if (source === 'mic') {
    isRecording.value = false;
    isMicStarting.value = false;
    void setAvatarTalking(false);
  } else {
    isOtherRecording.value = false;
    isSpeakerStarting.value = false;
  }
};

const toggleRecording = async () => {
  if (isRecording.value || isMicStarting.value) await stopCapture('mic');
  else await startCapture('mic');
};

const toggleOtherRecording = async () => {
  if (isOtherRecording.value || isSpeakerStarting.value) await stopCapture('speaker');
  else await startCapture('speaker');
};

const isTwoWayRunning = computed(() => isRecording.value && isOtherRecording.value);
const isTwoWayStarting = computed(() => isMicStarting.value || isSpeakerStarting.value);

const toggleTwoWay = async () => {
  if (isTwoWayRunning.value || isTwoWayStarting.value) {
    await Promise.all([stopCapture('mic'), stopCapture('speaker')]);
    setStatus(tt('translator.two_way_stopped', '已停止双向同传'));
  } else {
    selfSuppress.value = true;
    await Promise.all([startCapture('mic'), startCapture('speaker')]);
    setStatus(tt('translator.two_way_started', '已启动双向同传'));
  }
};

const loadAudioDevices = async () => {
  if (!isTauri()) return;
  try {
    audioDevices.value = await SysApi.getAudioDevices();
    const selectDefault = (source: AudioSource, current: string) => {
      const devices = audioDevices.value.filter((d) => d.source === source);
      if (devices.some((d) => d.id === current)) return current;
      return devices.find((d) => d.is_default)?.id ?? devices[0]?.id ?? '';
    };
    micDeviceId.value = selectDefault('mic', micDeviceId.value);
    speakerDeviceId.value = selectDefault('speaker', speakerDeviceId.value);
  } catch (error) {
    console.warn('Audio device load error:', error);
  }
};

const refreshAudioSessions = async () => {
  if (!isTauri() || !SysApi.getAudioSessions) return;
  isLoadingSessions.value = true;
  try {
    const sessions = await SysApi.getAudioSessions();
    activeAudioSessions.value = Array.isArray(sessions) ? sessions : [];
  } catch (e) {
    console.warn('Audio sessions error:', e);
  } finally {
    isLoadingSessions.value = false;
  }
};

const toggleOverlay = async () => {
  if (!isTauri()) return;
  if (isOverlayOpen.value) {
    try {
      await emit('cmd-close-overlay');
      window.setTimeout(async () => {
        const win = await WebviewWindow.getByLabel('translation-overlay');
        if (win) await win.destroy().catch(() => undefined);
      }, 300);
    } finally {
      isOverlayOpen.value = false;
      overlayWebview = null;
    }
    return;
  }
  try {
    overlayWebview = new WebviewWindow('translation-overlay', {
      url: '/?mode=overlay',
      title: tt('translator.overlay_title', 'Translation Overlay'),
      transparent: true,
      decorations: false,
      alwaysOnTop: true,
      width: 440,
      height: 320,
      x: 60,
      y: 60,
      skipTaskbar: true,
    });
    overlayWebview.once('tauri://created', () => { isOverlayOpen.value = true; });
    overlayWebview.onCloseRequested(() => { isOverlayOpen.value = false; overlayWebview = null; });
  } catch (err) {
    errorMsg.value = `Overlay error: ${errorText(err)}`;
  }
};

const copyText = async (text: string) => {
  try {
    await navigator.clipboard.writeText(text);
    setStatus(tt('translator.copied', '已复制到剪贴板'));
  } catch {
    // fallback
  }
};

const transliterateText = (text: string): string => {
  if (!text) return '';
  if (romajiMode.value) return toRomaji(text);
  if (hiraganaMode.value) return katakanaToHiragana(text);
  return '';
};

const clearHistory = async () => {
  try {
    await VrctApi.clearHistory();
    history.value = [];
  } catch {
    history.value = [];
  }
};

const saveTranslationProfile = () => {
  const profileKey = `vrc_translation_profile_${Date.now()}`;
  localStorage.setItem(profileKey, JSON.stringify({
    engine: translateEngine.value,
    apiKey: apiKey.value,
    customApiUrl: customApiUrl.value,
  }));
  setStatus('已保存当前配置为新档案');
};

const isWatchingScreenshots = ref(false);
const translatePhoto = async () => {
  try {
    const selected = await open({
      multiple: false,
      filters: [{ name: 'Images', extensions: ['png', 'jpg', 'jpeg', 'webp', 'bmp'] }],
    });
    if (typeof selected === 'string') {
      setStatus(`正在识别图片: ${selected}`);
      setStatus(`图片已载入: ${selected}`);
    }
  } catch (e) {
    console.warn('translatePhoto error:', e);
  }
};

const waitForPhoto = () => {
  isWatchingScreenshots.value = !isWatchingScreenshots.value;
  setStatus(isWatchingScreenshots.value ? '已开启截图目录自动监控' : '已停止截图监控');
};

// ---------------------------------------------------------------------------
// Lifecycle & Listeners
// ---------------------------------------------------------------------------
onMounted(async () => {
  translatorDisposed = false;
  if (isTauri()) {
    overlayWebview = await WebviewWindow.getByLabel('translation-overlay');
    isOverlayOpen.value = Boolean(overlayWebview);
    await loadAudioDevices();

    unlistenAudio = await listen('audio-capture-event', async (event: any) => {
      const payload = event.payload;
      const source = payload.source as AudioSource;
      if (payload.type === 'error') {
        errorMsg.value = `Audio capture error: ${payload.message}`;
        if (payload.fatal) {
          if (source === 'mic') isRecording.value = false;
          else isOtherRecording.value = false;
        }
        return;
      }
      if (payload.type === 'status') {
        if (payload.message === 'audio_level') {
          if (source === 'mic' || source === 'speaker') {
            audioLevels.value[source] = Math.min(10000, Math.max(0, Number(payload.level) || 0));
          }
        }
        if (payload.message === 'listening') {
          if (source === 'mic') isRecording.value = true;
          else isOtherRecording.value = true;
        }
        if (payload.message === 'stopped') {
          if (source === 'mic') isRecording.value = false;
          else isOtherRecording.value = false;
        }
        if (payload.message === 'word_filtered') {
          setStatus(`${tt('translator.word_filter', '敏感词拦截')}: ${payload.text}`);
        }
        return;
      }
      if (payload.type === 'result' && payload.text?.trim()) {
        recognizedText.value = payload.text.trim();
        if (source === 'mic') await translateMicText(payload.text);
        else await translateSpeakerText(payload.text);
      }
    });

    unlistenVrct = await listen('vrct_translation_event', (event: any) => {
      applyRecord(event.payload as VrctMessageRecord);
    });

    await refreshAudioSessions().catch(() => undefined);
  }

  try {
    const existing = await VrctApi.getHistory();
    if (Array.isArray(existing)) history.value = [...existing].reverse();
  } catch {
    history.value = [];
  }
});

onUnmounted(async () => {
  translatorDisposed = true;
  void setAvatarTalking(false);
  if (isTauri()) {
    await Promise.all([
      SysApi.stopAudioCapture({ source: 'mic' }).catch(() => undefined),
      SysApi.stopAudioCapture({ source: 'speaker' }).catch(() => undefined),
    ]);
  }
  unlistenAudio?.();
  unlistenVrct?.();
  unlistenTranslationHotkey?.();
});
</script>

<template>
  <div class="h-full min-h-0 flex flex-col p-5 bg-[var(--theme-bg-main)] text-[var(--theme-text)] rounded-2xl relative overflow-hidden select-none">
    <!-- Top Header & Master Switches -->
    <header class="flex items-center justify-between gap-3 mb-4 shrink-0 flex-wrap z-20">
      <div class="flex items-center gap-3">
        <div class="w-10 h-10 rounded-xl bg-[var(--theme-primary)]/10 border border-[var(--theme-primary)]/30 flex items-center justify-center text-[var(--theme-primary)] shadow-sm shadow-[var(--theme-primary)]/10">
          <Languages class="w-5 h-5" />
        </div>
        <div>
          <div class="flex items-center gap-2">
            <h2 class="text-xl font-black text-[var(--theme-text-strong)] tracking-tight">{{ tt('translator.title', '翻译官') }}</h2>
            <span class="px-2 py-0.5 rounded-full text-[10px] font-black bg-[var(--theme-primary)]/20 text-[var(--theme-primary)] border border-[var(--theme-primary)]/30">VRCT · Yakutan · Kikitan Enhanced</span>
          </div>
          <p class="text-xs text-[var(--theme-text-muted)]">实时同声传译 · 物理与虚拟声卡路由 · 多语言 OSC Chatbox</p>
        </div>
      </div>

      <!-- 4 Core Master Switches (VRCT Parity) -->
      <div class="flex items-center gap-2 flex-wrap">
        <!-- Master 1: Translation Toggle -->
        <button
          type="button"
          :class="masterTranslation ? 'bg-[var(--theme-primary)] text-white font-black shadow-lg shadow-[var(--theme-primary)]/20 border-[var(--theme-primary)]' : 'bg-[var(--theme-surface)] border-[var(--theme-border-soft)] text-[var(--theme-text-muted)] hover:border-[var(--theme-border-strong)]'"
          class="px-3 py-2 rounded-xl border text-xs flex items-center gap-2 transition-all active:scale-95"
          @click="masterTranslation = !masterTranslation"
        >
          <div :class="masterTranslation ? 'bg-[var(--theme-bg-main)]/60 text-[var(--theme-primary)]' : 'bg-[var(--theme-surface-hover)] text-[var(--theme-text-muted)]'" class="w-2 h-2 rounded-full" />
          <span>{{ tt('translator.master_translation', '主翻译') }}: {{ masterTranslation ? 'ON' : 'OFF' }}</span>
        </button>

        <!-- Master 2: Mic STT Toggle -->
        <button
          type="button"
          data-testid="start-microphone"
          :class="isRecording ? 'bg-emerald-500 text-white font-black shadow-lg shadow-emerald-500/20 border-emerald-400' : isMicStarting ? 'bg-[var(--theme-surface)] border-[var(--theme-primary)] text-[var(--theme-primary)] animate-pulse' : 'bg-[var(--theme-surface)] border-[var(--theme-border-soft)] text-[var(--theme-text-muted)] hover:border-[var(--theme-border-strong)]'"
          class="px-3 py-2 rounded-xl border text-xs flex items-center gap-2 transition-all active:scale-95"
          @click="toggleRecording"
        >
          <Mic class="w-3.5 h-3.5" :class="{ 'animate-bounce': isRecording }" />
          <span>{{ tt('translator.master_mic_stt', '麦克风 STT') }}: {{ isRecording ? 'ON' : 'OFF' }}</span>
        </button>

        <!-- Master 3: Speaker STT Toggle -->
        <button
          type="button"
          data-testid="start-speaker"
          :class="isOtherRecording ? 'bg-indigo-500 text-white font-black shadow-lg shadow-indigo-500/20 border-indigo-400' : isSpeakerStarting ? 'bg-[var(--theme-surface)] border-[var(--theme-primary)] text-[var(--theme-primary)] animate-pulse' : 'bg-[var(--theme-surface)] border-[var(--theme-border-soft)] text-[var(--theme-text-muted)] hover:border-[var(--theme-border-strong)]'"
          class="px-3 py-2 rounded-xl border text-xs flex items-center gap-2 transition-all active:scale-95"
          @click="toggleOtherRecording"
        >
          <Headphones class="w-3.5 h-3.5" :class="{ 'animate-pulse': isOtherRecording }" />
          <span>{{ tt('translator.master_speaker_stt', '扬声器 STT') }}: {{ isOtherRecording ? 'ON' : 'OFF' }}</span>
        </button>

        <!-- Master 4: OCR Toggle -->
        <button
          type="button"
          :class="masterOcr ? 'bg-cyan-500 text-white font-black shadow-lg shadow-cyan-500/20 border-cyan-400' : 'bg-[var(--theme-surface)] border-[var(--theme-border-soft)] text-[var(--theme-text-muted)] hover:border-[var(--theme-border-strong)]'"
          class="px-3 py-2 rounded-xl border text-xs flex items-center gap-2 transition-all active:scale-95"
          @click="masterOcr = !masterOcr"
        >
          <Camera class="w-3.5 h-3.5" />
          <span>{{ tt('translator.master_ocr', '气泡 OCR') }}: {{ masterOcr ? 'ON' : 'OFF' }}</span>
        </button>

        <!-- Two-Way Toggle -->
        <button
          type="button"
          :class="isTwoWayRunning ? 'bg-rose-500 text-white border-rose-400 shadow-rose-500/20' : 'bg-[var(--theme-surface)] border-[var(--theme-border-soft)] text-[var(--theme-text-muted)] hover:border-[var(--theme-border-strong)]'"
          class="px-3 py-2 rounded-xl border text-xs font-bold flex items-center gap-1.5 transition-all active:scale-95"
          @click="toggleTwoWay"
        >
          <Activity class="w-3.5 h-3.5" />
          <span>{{ isTwoWayRunning ? tt('translator.two_way_stop', '停止双向') : tt('translator.two_way_start', '双向同传') }}</span>
        </button>

        <!-- Overlay Toggle -->
        <button
          type="button"
          :class="isOverlayOpen ? 'bg-[var(--theme-primary)]/20 border-[var(--theme-primary)]/50 text-[var(--theme-primary)]' : 'bg-[var(--theme-surface)] border-[var(--theme-border-soft)] text-[var(--theme-text-muted)] hover:border-[var(--theme-border-strong)]'"
          class="px-3 py-2 rounded-xl border text-xs font-bold flex items-center gap-1.5 transition-all active:scale-95"
          @click="toggleOverlay"
        >
          <MonitorUp class="w-3.5 h-3.5" />
          <span>{{ isOverlayOpen ? tt('translator.overlay_close', '关闭悬浮窗') : tt('translator.overlay_open', '开启悬浮窗') }}</span>
        </button>

        <!-- Mode Switch: Simple / Advanced Tabs -->
        <button
          type="button"
          :class="isSimpleMode ? 'bg-[var(--theme-primary)] text-white font-black shadow-md border-[var(--theme-primary)]' : 'bg-[var(--theme-surface)] border-[var(--theme-border-soft)] text-[var(--theme-text)] hover:border-[var(--theme-border-strong)]'"
          class="px-3 py-2 rounded-xl border text-xs font-bold flex items-center gap-1.5 transition-all active:scale-95"
          @click="isSimpleMode = !isSimpleMode"
        >
          <SlidersHorizontal class="w-3.5 h-3.5" />
          <span>{{ isSimpleMode ? '极简模式 (Kikitan风格)' : '高级选项卡' }}</span>
        </button>
      </div>
    </header>

    <!-- Preset Selection Bar (Tab 1 / 2 / 3) & Language Selector with Swap -->
    <div v-if="!isSimpleMode" class="contents">
    <div class="mb-4 p-3 bg-[var(--theme-surface)] border border-[var(--theme-border-soft)] rounded-2xl flex items-center justify-between gap-4 flex-wrap z-10 shadow-sm">
      <!-- 3 Preset Tabs -->
      <div class="flex items-center gap-2">
        <span class="text-xs font-black text-[var(--theme-text-muted)] uppercase tracking-wider px-1">{{ tt('translator.preset_tab', '预设') }}:</span>
        <div class="inline-flex rounded-xl bg-[var(--theme-bg-main)]/60 p-1 border border-[var(--theme-border-soft)]">
          <button
            v-for="tabNo in (['1', '2', '3'] as const)"
            :key="tabNo"
            type="button"
            :class="activePresetTab === tabNo ? 'bg-[var(--theme-primary)] text-white font-black shadow-sm' : 'text-[var(--theme-text-muted)] hover:text-[var(--theme-text-strong)]'"
            class="px-3 py-1.5 rounded-lg text-xs font-bold transition-all"
            @click="activePresetTab = tabNo"
          >
            Tab {{ tabNo }}
          </button>
        </div>
      </div>

      <!-- Language Selector with Instant Swap -->
      <div class="flex items-center gap-2 flex-wrap min-w-0">
        <!-- Source Language (Your Language) -->
        <div class="min-w-[140px]">
          <CustomSelect v-model="sourceLang" :options="languageOptions" />
        </div>

        <!-- 1-Click Swap Button -->
        <button
          type="button"
          class="p-2 rounded-xl bg-[var(--theme-bg-main)]/60 border border-[var(--theme-border-soft)] text-[var(--theme-primary)] hover:border-[var(--theme-primary)]/50 hover:bg-[var(--theme-primary)]/10 transition-all active:scale-95"
          :title="tt('translator.swap_languages', '对调源语言与目标语言 1')"
          @click="swapLanguages"
        >
          <ArrowLeftRight class="w-4 h-4" />
        </button>

        <!-- Target 1 (Primary) -->
        <div class="min-w-[140px]">
          <CustomSelect v-model="targetLang" :options="speechLanguageOptions" />
        </div>

        <!-- Target 2 Toggle & Select -->
        <div class="flex items-center gap-1.5 px-2 py-1 bg-[var(--theme-bg-main)]/60 border border-[var(--theme-border-soft)] rounded-xl">
          <input v-model="enableTarget2" type="checkbox" class="w-3.5 h-3.5 rounded text-[var(--theme-primary)] accent-[var(--theme-primary)] cursor-pointer" :title="tt('translator.target_lang_2', '目标语言 2 开关')">
          <span class="text-xs font-bold text-[var(--theme-text-muted)]">T2</span>
          <div v-if="enableTarget2" class="w-28">
            <CustomSelect v-model="targetLang2" :options="speechLanguageOptions" />
          </div>
        </div>

        <!-- Target 3 Toggle & Select -->
        <div class="flex items-center gap-1.5 px-2 py-1 bg-[var(--theme-bg-main)]/60 border border-[var(--theme-border-soft)] rounded-xl">
          <input v-model="enableTarget3" type="checkbox" class="w-3.5 h-3.5 rounded text-[var(--theme-primary)] accent-[var(--theme-primary)] cursor-pointer" :title="tt('translator.target_lang_3', '目标语言 3 开关')">
          <span class="text-xs font-bold text-[var(--theme-text-muted)]">T3</span>
          <div v-if="enableTarget3" class="w-28">
            <CustomSelect v-model="targetLang3" :options="speechLanguageOptions" />
          </div>
        </div>

        <!-- Engine Select for active preset -->
        <div class="min-w-[160px]">
          <CustomSelect v-model="translateEngine" :options="engineOptions" />
        </div>
      </div>
    </div>

    <!-- 8 Sub-navigation Tabs (Pill Bar) -->
    <nav class="flex items-center gap-1.5 mb-4 overflow-x-auto pb-1 shrink-0 z-10 custom-scrollbar">
      <button
        v-for="subTab in ([
          { id: 'dashboard', label: tt('translator.tab_dashboard', '主控制台'), icon: MessageSquare },
          { id: 'routing', label: tt('translator.tab_routing', '声卡路由'), icon: Radio },
          { id: 'transcription', label: tt('translator.tab_transcription', '语音识别'), icon: Mic },
          { id: 'translation', label: tt('translator.tab_translation', '翻译引擎'), icon: Languages },
          { id: 'osc', label: tt('translator.tab_osc', 'VRChat OSC'), icon: Gamepad2 },
          { id: 'ocr', label: tt('translator.tab_ocr', '气泡 OCR'), icon: Camera },
          { id: 'vr_obs', label: tt('translator.tab_vr_obs', 'VR 与 OBS'), icon: Tv },
          { id: 'preferences', label: tt('translator.tab_preferences', '快捷键与偏好'), icon: Sliders },
        ] as const)"
        :key="subTab.id"
        type="button"
        :class="activeSubTab === subTab.id
          ? 'bg-[var(--theme-primary)]/15 border-[var(--theme-primary)]/50 text-[var(--theme-primary)] font-black shadow-sm'
          : 'bg-[var(--theme-surface)] border-[var(--theme-border-soft)] text-[var(--theme-text-muted)] hover:text-[var(--theme-text-strong)] hover:border-[var(--theme-border-strong)]'"
        class="px-3 py-2 rounded-xl border text-xs flex items-center gap-1.5 transition-all whitespace-nowrap active:scale-95"
        @click="activeSubTab = subTab.id"
      >
        <component :is="subTab.icon" class="w-3.5 h-3.5" />
        <span>{{ subTab.label }}</span>
      </button>
    </nav>
    </div>

    <!-- Main Dynamic Content Region -->
    <div class="flex-1 min-h-0 overflow-y-auto custom-scrollbar pr-1 z-10">
      <!-- SIMPLE MODE VIEW (Kikitan / Yakutan Minimalist View) -->
      <div v-if="isSimpleMode" class="h-full flex flex-col gap-4">
        <!-- Live Status Alerts -->
        <div v-if="statusMsg || errorMsg" class="p-3 rounded-xl border text-xs font-bold transition-all" :class="errorMsg ? 'bg-red-500/10 border-red-500/30 text-red-400' : 'bg-emerald-500/10 border-emerald-500/30 text-emerald-400'">
          {{ errorMsg || statusMsg }}
        </div>

        <!-- Master 1-Click Hero Card -->
        <div class="p-6 rounded-3xl bg-[var(--theme-surface)] border border-[var(--theme-border-soft)] shadow-md flex flex-col items-center justify-center text-center relative overflow-hidden shrink-0">
          <div class="flex items-center gap-2 mb-3">
            <span class="w-2.5 h-2.5 rounded-full" :class="isTwoWayRunning ? 'bg-emerald-500 animate-ping' : 'bg-[var(--theme-text-muted)]'" />
            <span class="text-xs font-black uppercase tracking-wider text-[var(--theme-text-soft)]">
              {{ isTwoWayRunning ? '双向同传进行中 · 物理麦克风与游戏内录正常运行' : '双向同传就绪 · 点击大按钮立即开启实时翻译' }}
            </span>
          </div>

          <!-- Giant Toggle Button -->
          <button
            type="button"
            :class="isTwoWayRunning ? 'bg-rose-500 hover:bg-rose-600 text-white shadow-xl shadow-rose-500/25 scale-105 border-rose-400' : 'bg-[var(--theme-primary)] hover:brightness-110 text-white shadow-xl shadow-[var(--theme-primary)]/25 border-transparent'"
            class="px-8 py-3.5 rounded-2xl text-base font-black border flex items-center gap-3 transition-all active:scale-95 mb-5 cursor-pointer"
            @click="toggleTwoWay"
          >
            <Activity class="w-5 h-5" :class="{ 'animate-pulse': isTwoWayRunning }" />
            <span>{{ isTwoWayRunning ? '停止双向同传' : '一键开启双向同传 (开始说话)' }}</span>
          </button>

          <!-- Core Quick Language & Engine Selectors -->
          <div class="flex items-center justify-center gap-3 flex-wrap max-w-2xl w-full">
            <div class="flex-1 min-w-[130px]">
              <label class="block text-[11px] font-bold text-[var(--theme-text-soft)] mb-1 text-left">我方语言 (你的母语)</label>
              <CustomSelect v-model="sourceLang" :options="languageOptions" />
            </div>

            <button
              type="button"
              class="mt-4 p-2.5 rounded-xl bg-[var(--theme-surface-hover)] border border-[var(--theme-border-soft)] text-[var(--theme-primary)] hover:scale-105 transition-all active:scale-95 cursor-pointer"
              title="对调我方与对方语言"
              @click="swapLanguages"
            >
              <ArrowLeftRight class="w-4 h-4" />
            </button>

            <div class="flex-1 min-w-[130px]">
              <label class="block text-[11px] font-bold text-[var(--theme-text-soft)] mb-1 text-left">对方语言 (翻译目标)</label>
              <CustomSelect v-model="targetLang" :options="speechLanguageOptions" />
            </div>

            <div class="flex-1 min-w-[150px]">
              <label class="block text-[11px] font-bold text-[var(--theme-text-soft)] mb-1 text-left">翻译服务</label>
              <CustomSelect v-model="translateEngine" :options="engineOptions" />
            </div>
          </div>

          <!-- Live Level Status -->
          <div class="mt-4 flex items-center justify-center gap-8 text-xs text-[var(--theme-text-soft)]">
            <div class="flex items-center gap-2">
              <Mic class="w-3.5 h-3.5" :class="isRecording ? 'text-emerald-500 animate-pulse' : 'text-[var(--theme-text-muted)]'" />
              <span>麦克风:</span>
              <span class="font-mono font-bold" :class="audioLevels.mic > 500 ? 'text-emerald-500' : ''">{{ Math.round(audioLevels.mic) }}</span>
            </div>
            <div class="flex items-center gap-2">
              <Headphones class="w-3.5 h-3.5" :class="isOtherRecording ? 'text-indigo-500 animate-pulse' : 'text-[var(--theme-text-muted)]'" />
              <span>游戏声音:</span>
              <span class="font-mono font-bold" :class="audioLevels.speaker > 500 ? 'text-indigo-500' : ''">{{ Math.round(audioLevels.speaker) }}</span>
            </div>
          </div>
        </div>

        <!-- Live Split Conversation Stream -->
        <div class="flex-1 min-h-[220px] bg-[var(--theme-surface)] border border-[var(--theme-border-soft)] rounded-3xl p-4 overflow-y-auto custom-scrollbar flex flex-col gap-3">
          <div v-if="!history.length" class="h-full flex flex-col items-center justify-center text-[var(--theme-text-soft)] text-xs py-8">
            <MessageSquare class="w-10 h-10 mb-2 opacity-30 text-[var(--theme-text-soft)]" />
            <p>说话或对方在 VRChat 说话后，双向翻译气泡将实时显示在这里...</p>
          </div>

          <div
            v-for="msg in history"
            :key="msg.id"
            class="p-3.5 rounded-2xl border transition-all max-w-[85%]"
            :class="msg.source === 'mic' ? 'self-end bg-emerald-500/10 border-emerald-500/30' : 'self-start bg-indigo-500/10 border-indigo-500/30'"
          >
            <div class="flex items-center justify-between gap-3 mb-1">
              <span class="text-[10px] font-black uppercase px-2 py-0.5 rounded-full" :class="msg.source === 'mic' ? 'bg-emerald-500/20 text-emerald-600 dark:text-emerald-400' : 'bg-indigo-500/20 text-indigo-600 dark:text-indigo-400'">
                {{ msg.source === 'mic' ? '我方' : '对方' }}
              </span>
              <span class="text-[10px] text-[var(--theme-text-muted)] font-mono">{{ msg.timestamp }}</span>
            </div>
            <p class="text-sm font-medium text-[var(--theme-text)] leading-relaxed">{{ msg.original }}</p>
            <p v-if="(romajiMode || hiraganaMode) && transliterateText(msg.original)" class="text-xs text-[var(--theme-primary)] font-mono mt-0.5">
              {{ transliterateText(msg.original) }}
            </p>
            <div class="mt-2 pt-1.5 border-t border-[var(--theme-border-soft)]">
              <p class="text-sm font-bold text-[var(--theme-primary)]">{{ msg.translated }}</p>
            </div>
          </div>
        </div>

        <!-- Quick Chatbox Input Bar -->
        <div class="p-3 rounded-2xl bg-[var(--theme-surface)] border border-[var(--theme-border-soft)] flex items-center gap-2 shrink-0">
          <input
            v-model="manualText"
            type="text"
            class="flex-1 bg-[var(--theme-surface-hover)] border border-[var(--theme-border-soft)] rounded-xl px-4 py-2 text-xs text-[var(--theme-text)] outline-none focus:ring-2 focus:ring-[var(--theme-primary)]/20"
            placeholder="输入文字按回车直接翻译并发送到 VRChat Chatbox..."
            @keydown.enter.prevent="translateManual"
          >
          <button
            type="button"
            :disabled="!canTranslate"
            class="px-4 py-2 rounded-xl bg-[var(--theme-primary)] hover:brightness-110 text-white font-bold text-xs flex items-center gap-1.5 transition-all disabled:opacity-50 active:scale-95 cursor-pointer"
            @click="translateManual"
          >
            <Send class="w-3.5 h-3.5" />
            <span>发送</span>
          </button>
        </div>
      </div>

      <!-- 1. DASHBOARD TAB -->
      <div v-if="activeSubTab === 'dashboard'" class="h-full flex flex-col gap-4">
        <!-- Live Status Alerts -->
        <div v-if="statusMsg || errorMsg" class="p-3 rounded-xl border text-xs font-bold transition-all" :class="errorMsg ? 'bg-red-500/10 border-red-500/30 text-red-400' : 'bg-emerald-500/10 border-emerald-500/30 text-emerald-400'">
          {{ errorMsg || statusMsg }}
        </div>

        <!-- Message History Log Stream -->
        <div class="flex-1 min-h-0 bg-[var(--theme-bg-main)]/60 border border-[var(--theme-border-soft)] rounded-2xl p-4 overflow-y-auto custom-scrollbar flex flex-col gap-3">
          <div v-if="!history.length" class="h-full flex flex-col items-center justify-center text-[var(--theme-text-muted)] text-xs">
            <MessageSquare class="w-8 h-8 mb-2 opacity-40 text-[var(--theme-text-muted)]" />
            <p>{{ tt('translator.placeholder', '翻译与识别记录将显示在这里...') }}</p>
          </div>

          <div
            v-for="msg in history"
            :key="msg.id"
            class="p-3.5 rounded-xl border transition-all"
            :class="msg.source === 'mic' ? 'bg-emerald-500/10 border-emerald-500/30' : msg.source === 'speaker' ? 'bg-indigo-500/10 border-indigo-500/30' : msg.source === 'ocr' ? 'bg-cyan-500/10 border-cyan-500/30' : 'bg-[var(--theme-surface)] border-[var(--theme-border-soft)]'"
          >
            <div class="flex items-center justify-between gap-2 mb-1.5">
              <div class="flex items-center gap-2">
                <span
                  class="px-2 py-0.5 rounded-full text-[10px] font-black uppercase tracking-wider"
                  :class="msg.source === 'mic' ? 'bg-emerald-500/20 text-emerald-400' : msg.source === 'speaker' ? 'bg-indigo-500/20 text-indigo-400' : msg.source === 'ocr' ? 'bg-cyan-500/20 text-cyan-400' : 'bg-[var(--theme-primary)]/20 text-[var(--theme-primary)]'"
                >
                  {{ msg.source }}
                </span>
                <span class="text-[10px] text-[var(--theme-text-muted)] font-mono">{{ msg.timestamp }}</span>
                <span v-if="msg.sent_osc" class="text-[10px] text-cyan-400 font-mono">OSC Chatbox</span>
              </div>
              <div class="flex items-center gap-1.5">
                <button type="button" class="p-1 rounded-lg hover:bg-[var(--theme-surface-hover)] text-[var(--theme-text-muted)] hover:text-[var(--theme-text-strong)]" title="Copy Translated" @click="copyText(msg.translated)">
                  <Copy class="w-3.5 h-3.5" />
                </button>
                <button type="button" class="p-1 rounded-lg hover:bg-[var(--theme-surface-hover)] text-[var(--theme-text-muted)] hover:text-[var(--theme-text-strong)]" title="Play TTS" @click="playTts(msg.translated, msg.target_lang)">
                  <Volume2 class="w-3.5 h-3.5" />
                </button>
                <button v-if="showResendButton" type="button" class="p-1 rounded-lg hover:bg-[var(--theme-surface-hover)] text-[var(--theme-text-muted)] hover:text-[var(--theme-primary)]" title="Resend to VRChat" @click="processMessageNow(msg.original, msg.source, msg.source_lang, msg.target_lang, true)">
                  <Send class="w-3.5 h-3.5" />
                </button>
              </div>
            </div>

            <!-- Original Text -->
            <p class="text-sm font-medium text-[var(--theme-text)] leading-relaxed">{{ msg.original }}</p>

            <!-- Romaji / Hiragana Transliteration if enabled -->
            <p v-if="(romajiMode || hiraganaMode) && transliterateText(msg.original)" class="text-xs text-[var(--theme-primary)]/80 font-mono mt-0.5">
              {{ transliterateText(msg.original) }}
            </p>

            <!-- Translated Text -->
            <div class="mt-2 pt-2 border-t border-[var(--theme-border-soft)]">
              <p class="text-sm font-bold text-[var(--theme-primary)] leading-relaxed">{{ msg.translated }}</p>
              <div v-if="msg.translations && msg.translations.length > 1" class="mt-1 flex flex-col gap-0.5">
                <div v-for="tr in msg.translations.slice(1)" :key="tr.target_lang" class="text-xs text-[var(--theme-text-muted)] font-medium">
                  <span class="text-[10px] font-mono text-[var(--theme-text-muted)] uppercase">[{{ tr.target_lang }}]</span> {{ tr.translated }}
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- Quick Input & Chatbox Bar -->
        <div class="p-3 bg-[var(--theme-surface)] border border-[var(--theme-border-soft)] rounded-2xl flex flex-col gap-2 shrink-0">
          <div class="flex items-center gap-2">
            <textarea
              v-model="manualText"
              rows="2"
              class="flex-1 bg-[var(--theme-bg-main)]/60 border border-[var(--theme-border-soft)] rounded-xl px-3 py-2 text-xs text-[var(--theme-text-strong)] placeholder-[var(--theme-text-muted)] outline-none focus:border-[var(--theme-primary)] resize-none"
              :placeholder="tt('translator.enter_text_to_translate_for_vrchat', '输入要翻译到 VRChat 的文字... (按 Enter 发送)')"
              @keydown.enter.exact.prevent="translateManual"
            />
            <button
              v-if="sendMessageButtonType !== 'hide'"
              type="button"
              :disabled="!canTranslate"
              class="px-4 h-[58px] rounded-xl bg-[var(--theme-primary)] hover:brightness-110 disabled:opacity-40 text-white font-black text-xs flex flex-col items-center justify-center gap-1 transition-all active:scale-95 shrink-0"
              @click="translateManual"
            >
              <Send class="w-4 h-4" />
              <span>{{ tt('translator.translate_and_send', '发送') }}</span>
            </button>
          </div>
          <div class="flex items-center justify-between text-[11px] text-[var(--theme-text-muted)] px-1">
            <div class="flex items-center gap-4">
              <label class="flex items-center gap-1.5 cursor-pointer">
                <input v-model="autoClearMessageBox" type="checkbox" class="w-3.5 h-3.5 rounded accent-[var(--theme-primary)]">
                <span>自动清空输入框</span>
              </label>
              <label class="flex items-center gap-1.5 cursor-pointer">
                <input v-model="autoSendOsc" type="checkbox" class="w-3.5 h-3.5 rounded accent-[var(--theme-primary)]">
                <span>同步发送至 Chatbox</span>
              </label>
            </div>
            <button type="button" class="text-[var(--theme-text-muted)] hover:text-red-400 transition-colors" @click="clearHistory">
              清空历史记录
            </button>
          </div>
        </div>
      </div>

      <!-- 2. AUDIO ROUTING (MIXLINE MATRIX) TAB -->
      <div v-else-if="activeSubTab === 'routing'" class="flex flex-col gap-5">
        <div class="p-4 bg-[var(--theme-surface)] border border-[var(--theme-border-soft)] rounded-2xl">
          <div class="flex items-center gap-2 mb-1">
            <Radio class="w-5 h-5 text-[var(--theme-primary)]" />
            <h3 class="text-sm font-black text-[var(--theme-text-strong)]">{{ tt('translator.routing_title', '声卡混音与物理路由矩阵 (MIXLINE 风格)') }}</h3>
          </div>
          <p class="text-xs text-[var(--theme-text-muted)] leading-relaxed">{{ tt('translator.routing_desc', '将物理麦克风、系统内录与虚拟音频通道无损串接并提供实时音量电平表与独立监听。') }}</p>
        </div>

        <!-- MIXLINE Matrix: Inputs -> Cables -> Outputs -->
        <div class="grid grid-cols-1 lg:grid-cols-3 gap-4 items-stretch relative">
          <!-- Column 1: INPUTS -->
          <div class="flex flex-col gap-4">
            <h4 class="text-xs font-black text-[var(--theme-text-muted)] uppercase tracking-wider px-1">{{ tt('translator.routing_inputs', '音频输入源 (Inputs)') }}</h4>

            <!-- Card: Microphone Input -->
            <div class="p-4 bg-[var(--theme-surface)] border border-[var(--theme-border-soft)] rounded-2xl flex flex-col gap-3 shadow-sm">
              <div class="flex items-center justify-between">
                <div class="flex items-center gap-2">
                  <div class="w-8 h-8 rounded-xl bg-emerald-500/10 border border-emerald-500/30 flex items-center justify-center text-emerald-400">
                    <Mic class="w-4 h-4" />
                  </div>
                  <div>
                    <h5 class="text-xs font-black text-[var(--theme-text-strong)]">{{ tt('translator.routing_mic', '物理麦克风') }}</h5>
                    <span class="text-[10px] text-[var(--theme-text-muted)]">{{ isRecording ? '采集中' : '就绪' }}</span>
                  </div>
                </div>
                <button
                  type="button"
                  :class="micMuted ? 'bg-red-500/20 text-red-400 border-red-500/40' : 'bg-[var(--theme-surface-hover)] text-[var(--theme-text)] hover:border-[var(--theme-border-strong)]'"
                  class="px-2.5 py-1 rounded-lg border text-[11px] font-bold flex items-center gap-1 transition-all"
                  @click="micMuted = !micMuted"
                >
                  <component :is="micMuted ? VolumeX : Volume2" class="w-3.5 h-3.5" />
                  <span>{{ micMuted ? tt('translator.routing_mute', '静音') : '正常' }}</span>
                </button>
              </div>

              <!-- Device Selector -->
              <CustomSelect v-model="micDeviceId" :options="audioDevices.filter(d => d.source === 'mic').map(d => ({ label: d.name, value: d.id }))" />

              <!-- Live VU Meter -->
              <div>
                <div class="flex justify-between text-[10px] text-[var(--theme-text-muted)] font-mono mb-1">
                  <span>实时电平 (VU Meter)</span>
                  <span>{{ Math.round(audioLevels.mic) }}</span>
                </div>
                <div class="w-full h-2.5 bg-[var(--theme-bg-main)]/60 rounded-full overflow-hidden p-0.5 border border-[var(--theme-border-soft)]">
                  <div
                    class="h-full rounded-full transition-all duration-75"
                    :class="audioLevels.mic > 500 ? 'bg-emerald-400 shadow-sm shadow-emerald-400/50' : 'bg-emerald-600'"
                    :style="{ width: `${Math.min(100, (audioLevels.mic / 1200) * 100)}%` }"
                  />
                </div>
              </div>

              <!-- Volume Slider -->
              <label class="flex items-center justify-between gap-3 text-xs">
                <span class="text-[var(--theme-text-muted)] font-bold">增益</span>
                <input v-model.number="micVolume" type="range" min="0" max="200" step="5" class="flex-1 accent-[var(--theme-primary)]">
                <span class="text-[var(--theme-text-strong)] font-mono font-bold w-10 text-right">{{ micVolume }}%</span>
              </label>
            </div>

            <!-- Card: Speaker / Loopback Input -->
            <div class="p-4 bg-[var(--theme-surface)] border border-[var(--theme-border-soft)] rounded-2xl flex flex-col gap-3 shadow-sm">
              <div class="flex items-center justify-between">
                <div class="flex items-center gap-2">
                  <div class="w-8 h-8 rounded-xl bg-indigo-500/10 border border-indigo-500/30 flex items-center justify-center text-indigo-400">
                    <Headphones class="w-4 h-4" />
                  </div>
                  <div>
                    <h5 class="text-xs font-black text-[var(--theme-text-strong)]">{{ tt('translator.routing_speaker', '系统声音 / 游戏内录 (Loopback)') }}</h5>
                    <span class="text-[10px] text-[var(--theme-text-muted)]">{{ isOtherRecording ? '内录中' : '就绪' }}</span>
                  </div>
                </div>
                <button
                  type="button"
                  :class="speakerMuted ? 'bg-red-500/20 text-red-400 border-red-500/40' : 'bg-[var(--theme-surface-hover)] text-[var(--theme-text)] hover:border-[var(--theme-border-strong)]'"
                  class="px-2.5 py-1 rounded-lg border text-[11px] font-bold flex items-center gap-1 transition-all"
                  @click="speakerMuted = !speakerMuted"
                >
                  <component :is="speakerMuted ? VolumeX : Volume2" class="w-3.5 h-3.5" />
                  <span>{{ speakerMuted ? tt('translator.routing_mute', '静音') : '正常' }}</span>
                </button>
              </div>

              <CustomSelect v-model="speakerDeviceId" :options="audioDevices.filter(d => d.source === 'speaker').map(d => ({ label: d.name, value: d.id }))" />

              <!-- VU Meter -->
              <div>
                <div class="flex justify-between text-[10px] text-[var(--theme-text-muted)] font-mono mb-1">
                  <span>实时电平 (VU Meter)</span>
                  <span>{{ Math.round(audioLevels.speaker) }}</span>
                </div>
                <div class="w-full h-2.5 bg-[var(--theme-bg-main)]/60 rounded-full overflow-hidden p-0.5 border border-[var(--theme-border-soft)]">
                  <div
                    class="h-full rounded-full transition-all duration-75"
                    :class="audioLevels.speaker > 500 ? 'bg-indigo-400 shadow-sm shadow-indigo-400/50' : 'bg-indigo-600'"
                    :style="{ width: `${Math.min(100, (audioLevels.speaker / 1200) * 100)}%` }"
                  />
                </div>
              </div>

              <label class="flex items-center justify-between gap-3 text-xs">
                <span class="text-[var(--theme-text-muted)] font-bold">增益</span>
                <input v-model.number="speakerVolume" type="range" min="0" max="200" step="5" class="flex-1 accent-indigo-500">
                <span class="text-[var(--theme-text-strong)] font-mono font-bold w-10 text-right">{{ speakerVolume }}%</span>
              </label>
            </div>
          </div>

          <!-- Column 2: SVG CONNECTION CABLES (MIXLINE WIRE VISUALIZER) -->
          <div class="hidden lg:flex flex-col items-center justify-center p-4 bg-[var(--theme-bg-main)]/60 border border-[var(--theme-border-soft)] rounded-2xl relative overflow-hidden">
            <svg class="w-full h-full min-h-[360px]" viewBox="0 0 200 400" fill="none">
              <!-- Wire 1: Mic -> VRChat Stream -->
              <path
                d="M 10 90 C 100 90, 100 90, 190 90"
                :stroke="isRecording ? '#10b981' : 'var(--theme-border-strong)'"
                stroke-width="3"
                stroke-dasharray="6 4"
                :class="{ 'animate-pulse': isRecording }"
              />
              <!-- Wire 2: Mic -> Headphone Monitor -->
              <path
                d="M 10 90 C 100 90, 100 310, 190 310"
                :stroke="headphoneMonitorEnabled ? 'var(--theme-primary)' : 'var(--theme-border-strong)'"
                stroke-width="2.5"
                stroke-dasharray="4 4"
              />
              <!-- Wire 3: Loopback -> Headphone Monitor -->
              <path
                d="M 10 270 C 100 270, 100 310, 190 310"
                :stroke="isOtherRecording ? '#6366f1' : 'var(--theme-border-strong)'"
                stroke-width="3"
                stroke-dasharray="6 4"
                :class="{ 'animate-pulse': isOtherRecording }"
              />
              <!-- Wire 4: Loopback -> VRChat Stream (if Speaker2Chatbox) -->
              <path
                d="M 10 270 C 100 270, 100 90, 190 90"
                :stroke="sendReceivedMessageToVrc ? '#06b6d4' : 'var(--theme-border-strong)'"
                stroke-width="2"
                stroke-dasharray="4 4"
              />
              <!-- Center Badge -->
              <circle cx="100" cy="190" r="28" fill="var(--theme-surface)" stroke="var(--theme-primary)" stroke-width="2" />
              <text x="100" y="186" text-anchor="middle" fill="var(--theme-primary)" font-size="10" font-weight="900">MIXLINE</text>
              <text x="100" y="200" text-anchor="middle" fill="var(--theme-text-muted)" font-size="8">ROUTING</text>
            </svg>
          </div>

          <!-- Column 3: OUTPUTS -->
          <div class="flex flex-col gap-4">
            <h4 class="text-xs font-black text-[var(--theme-text-muted)] uppercase tracking-wider px-1">{{ tt('translator.routing_outputs', '音频输出端 (Outputs)') }}</h4>

            <!-- Card: VRChat Stream -->
            <div class="p-4 bg-[var(--theme-surface)] border border-[var(--theme-border-soft)] rounded-2xl flex flex-col gap-3 shadow-sm">
              <div class="flex items-center justify-between">
                <div class="flex items-center gap-2">
                  <div class="w-8 h-8 rounded-xl bg-[var(--theme-primary)]/10 border border-[var(--theme-primary)]/30 flex items-center justify-center text-[var(--theme-primary)]">
                    <Gamepad2 class="w-4 h-4" />
                  </div>
                  <div>
                    <h5 class="text-xs font-black text-[var(--theme-text-strong)]">{{ tt('translator.routing_vrchat_stream', 'VRChat 虚拟麦克风通道') }}</h5>
                    <span class="text-[10px] text-[var(--theme-text-muted)]">OSC / Virtual Audio Cable</span>
                  </div>
                </div>
                <button
                  type="button"
                  :class="vrchatStreamMuted ? 'bg-red-500/20 text-red-400 border-red-500/40' : 'bg-[var(--theme-surface-hover)] text-[var(--theme-text)] hover:border-[var(--theme-border-strong)]'"
                  class="px-2.5 py-1 rounded-lg border text-[11px] font-bold flex items-center gap-1 transition-all"
                  @click="vrchatStreamMuted = !vrchatStreamMuted"
                >
                  <component :is="vrchatStreamMuted ? VolumeX : Volume2" class="w-3.5 h-3.5" />
                  <span>{{ vrchatStreamMuted ? tt('translator.routing_mute', '静音') : '正常' }}</span>
                </button>
              </div>

              <div class="p-2.5 bg-[var(--theme-bg-main)]/60 rounded-xl border border-[var(--theme-border-soft)] text-xs text-[var(--theme-text)] flex items-center justify-between">
                <span>直通真实物理声音</span>
                <input v-model="micPassthrough" type="checkbox" class="w-4 h-4 rounded accent-[var(--theme-primary)] cursor-pointer">
              </div>

              <label class="flex items-center justify-between gap-3 text-xs">
                <span class="text-[var(--theme-text-muted)] font-bold">音量</span>
                <input v-model.number="vrchatStreamVolume" type="range" min="0" max="200" step="5" class="flex-1 accent-[var(--theme-primary)]">
                <span class="text-[var(--theme-text-strong)] font-mono font-bold w-10 text-right">{{ vrchatStreamVolume }}%</span>
              </label>
            </div>

            <!-- Card: Headphone Monitor -->
            <div class="p-4 bg-[var(--theme-surface)] border border-[var(--theme-border-soft)] rounded-2xl flex flex-col gap-3 shadow-sm">
              <div class="flex items-center justify-between">
                <div class="flex items-center gap-2">
                  <div class="w-8 h-8 rounded-xl bg-cyan-500/10 border border-cyan-500/30 flex items-center justify-center text-cyan-400">
                    <Ear class="w-4 h-4" />
                  </div>
                  <div>
                    <h5 class="text-xs font-black text-[var(--theme-text-strong)]">{{ tt('translator.routing_monitor', '耳机监听通道 (Headphones)') }}</h5>
                    <span class="text-[10px] text-[var(--theme-text-muted)]">本地耳返与 TTS 音频</span>
                  </div>
                </div>
                <button
                  type="button"
                  :class="headphoneMonitorEnabled ? 'bg-cyan-500 text-white font-black border-cyan-400' : 'bg-[var(--theme-surface-hover)] text-[var(--theme-text)] hover:border-[var(--theme-border-strong)]'"
                  class="px-2.5 py-1 rounded-lg border text-[11px] font-bold flex items-center gap-1 transition-all"
                  @click="headphoneMonitorEnabled = !headphoneMonitorEnabled"
                >
                  <span>{{ headphoneMonitorEnabled ? '监听中' : '未开启' }}</span>
                </button>
              </div>

              <!-- Output Device -->
              <CustomSelect v-model="ttsDeviceId" :options="[{ label: '系统默认耳机/扬声器', value: '' }, ...audioDevices.filter(d => d.source === 'output').map(d => ({ label: d.name, value: d.id }))]" />

              <label class="flex items-center justify-between gap-3 text-xs">
                <span class="text-[var(--theme-text-muted)] font-bold">音量</span>
                <input v-model.number="headphoneMonitorVolume" type="range" min="0" max="200" step="5" class="flex-1 accent-cyan-500">
                <span class="text-[var(--theme-text-strong)] font-mono font-bold w-10 text-right">{{ headphoneMonitorVolume }}%</span>
              </label>
            </div>
          </div>
        </div>

        <!-- Audio Sessions List -->
        <div class="p-4 bg-[var(--theme-surface)] border border-[var(--theme-border-soft)] rounded-2xl flex flex-col gap-3">
          <div class="flex items-center justify-between">
            <h4 class="text-xs font-black text-[var(--theme-text-strong)] uppercase tracking-wider">活跃发声进程 (Audio Sessions)</h4>
            <button type="button" class="px-2.5 py-1 rounded-lg bg-[var(--theme-surface-hover)] hover:bg-[var(--theme-border-soft)] text-[var(--theme-text)] text-xs font-bold flex items-center gap-1" @click="refreshAudioSessions">
              <RefreshCw class="w-3 h-3" :class="{ 'animate-spin': isLoadingSessions }" />
              <span>刷新进程</span>
            </button>
          </div>
          <div class="flex flex-wrap gap-2">
            <div
              v-for="s in activeAudioSessions"
              :key="s.pid"
              class="px-3 py-1.5 rounded-xl bg-[var(--theme-bg-main)]/60 border border-[var(--theme-border-soft)] text-xs flex items-center gap-2"
            >
              <div class="w-2 h-2 rounded-full" :class="s.is_active ? 'bg-emerald-400' : 'bg-[var(--theme-border-strong)]'" />
              <span class="font-bold text-[var(--theme-text-strong)]">{{ s.name }}</span>
              <span class="text-[10px] text-[var(--theme-text-muted)] font-mono">PID {{ s.pid }}</span>
            </div>
            <span v-if="!activeAudioSessions.length" class="text-xs text-[var(--theme-text-muted)] py-1">未检测到任何正在发声的独立 Windows 进程。</span>
          </div>
        </div>
      </div>

      <!-- 3. TRANSCRIPTION STT TAB -->
      <div v-else-if="activeSubTab === 'transcription'" class="flex flex-col gap-5">
        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
          <!-- Engine Selector Card -->
          <div class="p-4 bg-[var(--theme-surface)] border border-[var(--theme-border-soft)] rounded-2xl flex flex-col gap-3">
            <h4 class="text-xs font-black text-[var(--theme-text-strong)] uppercase">麦克风识别引擎 (Mic STT Engine)</h4>
            <CustomSelect v-model="micEngine" :options="sttEngineOptions" />

            <h4 class="text-xs font-black text-[var(--theme-text-strong)] uppercase mt-2">扬声器识别引擎 (Speaker STT Engine)</h4>
            <CustomSelect v-model="otherEngine" :options="sttEngineOptions" />
          </div>

          <!-- Local Whisper Engine Settings -->
          <div class="p-4 bg-[var(--theme-surface)] border border-[var(--theme-border-soft)] rounded-2xl flex flex-col gap-3">
            <h4 class="text-xs font-black text-[var(--theme-text-strong)] uppercase">本地 Faster-Whisper 设置</h4>
            <div class="grid grid-cols-2 gap-2">
              <label class="text-xs">
                <span class="block text-[var(--theme-text-muted)] font-bold mb-1">模型尺寸 (Weights)</span>
                <select v-model="whisperModel" class="w-full bg-[var(--theme-bg-main)]/60 border border-[var(--theme-border-soft)] rounded-xl px-2.5 py-1.5 text-xs text-[var(--theme-text-strong)] outline-none">
                  <option value="tiny">tiny (快速/显存极小)</option>
                  <option value="base">base (轻量标准)</option>
                  <option value="small">small (高精推荐)</option>
                  <option value="medium">medium (专业)</option>
                  <option value="large-v3">large-v3 (旗舰)</option>
                  <option value="large-v3-turbo">large-v3-turbo (快速大模型)</option>
                </select>
              </label>

              <label class="text-xs">
                <span class="block text-[var(--theme-text-muted)] font-bold mb-1">{{ tt('translator.whisper_device', '运算设备') }}</span>
                <select v-model="whisperDevice" class="w-full bg-[var(--theme-bg-main)]/60 border border-[var(--theme-border-soft)] rounded-xl px-2.5 py-1.5 text-xs text-[var(--theme-text-strong)] outline-none">
                  <option value="cpu">CPU (兼容所有电脑)</option>
                  <option value="cuda">CUDA (NVIDIA 显卡加速)</option>
                </select>
              </label>
            </div>

            <label class="text-xs">
              <span class="block text-[var(--theme-text-muted)] font-bold mb-1">{{ tt('translator.whisper_compute_type', '计算精度 (Compute Type)') }}</span>
              <select v-model="whisperComputeType" class="w-full bg-[var(--theme-bg-main)]/60 border border-[var(--theme-border-soft)] rounded-xl px-2.5 py-1.5 text-xs text-[var(--theme-text-strong)] outline-none">
                <option value="int8">int8 (推荐，省内存)</option>
                <option value="float16">float16 (显卡半精度)</option>
                <option value="int8_float16">int8_float16 (混合精度)</option>
                <option value="float32">float32 (单精度)</option>
              </select>
            </label>
          </div>
        </div>

        <!-- Cloud STT API Keys (Groq / OpenAI / Custom / Deepgram) -->
        <div class="p-4 bg-[var(--theme-surface)] border border-[var(--theme-border-soft)] rounded-2xl flex flex-col gap-3">
          <h4 class="text-xs font-black text-[var(--theme-text-strong)] uppercase">云端 STT 密钥与服务地址 (Groq / OpenAI / Custom / Deepgram)</h4>
          <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
            <label class="text-xs">
              <span class="block text-[var(--theme-text-muted)] font-bold mb-1">Groq API Key (Groq Whisper)</span>
              <input v-model="groqApiKey" type="password" class="w-full bg-[var(--theme-bg-main)]/60 border border-[var(--theme-border-soft)] rounded-xl px-3 py-1.5 text-xs text-[var(--theme-text-strong)] outline-none focus:border-[var(--theme-primary)]" placeholder="gsk_...">
            </label>
            <label class="text-xs">
              <span class="block text-[var(--theme-text-muted)] font-bold mb-1">OpenAI API Key (Whisper-1)</span>
              <input v-model="openaiApiKey" type="password" class="w-full bg-[var(--theme-bg-main)]/60 border border-[var(--theme-border-soft)] rounded-xl px-3 py-1.5 text-xs text-[var(--theme-text-strong)] outline-none focus:border-[var(--theme-primary)]" placeholder="sk-...">
            </label>
            <label class="text-xs">
              <span class="block text-[var(--theme-text-muted)] font-bold mb-1">Custom Whisper URL</span>
              <input v-model="customWhisperUrl" type="text" class="w-full bg-[var(--theme-bg-main)]/60 border border-[var(--theme-border-soft)] rounded-xl px-3 py-1.5 text-xs text-[var(--theme-text-strong)] outline-none focus:border-[var(--theme-primary)]" placeholder="http://127.0.0.1:8000/v1">
            </label>
            <label class="text-xs">
              <span class="block text-[var(--theme-text-muted)] font-bold mb-1">Deepgram API Key (Nova-2 / Nova-3)</span>
              <input v-model="deepgramApiKey" type="password" class="w-full bg-[var(--theme-bg-main)]/60 border border-[var(--theme-border-soft)] rounded-xl px-3 py-1.5 text-xs text-[var(--theme-text-strong)] outline-none focus:border-[var(--theme-primary)]" placeholder="Token ...">
            </label>
          </div>
        </div>

        <!-- Word Filter (MIC_WORD_FILTER Redaction) -->
        <div class="p-4 bg-[var(--theme-surface)] border border-[var(--theme-border-soft)] rounded-2xl flex flex-col gap-3">
          <div class="flex items-center justify-between">
            <div>
              <h4 class="text-xs font-black text-[var(--theme-text-strong)] uppercase">{{ tt('translator.word_filter', '麦克风敏感词屏蔽列表 (Word Filter)') }}</h4>
              <p class="text-[11px] text-[var(--theme-text-muted)]">{{ tt('translator.word_filter_desc', '识别到列表内的敏感词时自动拦截，不发送至 VRChat Chatbox。') }}</p>
            </div>
          </div>
          <div class="flex items-center gap-2">
            <input
              v-model="newFilterWord"
              type="text"
              class="flex-1 bg-[var(--theme-bg-main)]/60 border border-[var(--theme-border-soft)] rounded-xl px-3 py-1.5 text-xs text-[var(--theme-text-strong)] outline-none focus:border-[var(--theme-primary)]"
              :placeholder="tt('translator.word_filter_placeholder', '输入要屏蔽的词并按回车或添加...')"
              @keydown.enter.prevent="addFilterWord"
            >
            <button type="button" class="px-3 py-1.5 rounded-xl bg-[var(--theme-primary)] hover:brightness-110 text-white font-bold text-xs" @click="addFilterWord">
              {{ tt('translator.word_filter_add', '添加') }}
            </button>
          </div>
          <div class="flex flex-wrap gap-2 mt-1">
            <div
              v-for="(w, idx) in wordFilterList"
              :key="idx"
              class="px-2.5 py-1 rounded-lg bg-[var(--theme-bg-main)]/60 border border-[var(--theme-border-soft)] text-xs font-bold text-[var(--theme-text)] flex items-center gap-1.5"
            >
              <span>{{ w }}</span>
              <button type="button" class="text-[var(--theme-text-muted)] hover:text-red-400" @click="removeFilterWord(idx)">×</button>
            </div>
            <span v-if="!wordFilterList.length" class="text-xs text-[var(--theme-text-muted)] py-1">{{ tt('translator.word_filter_empty', '当前未配置敏感词过滤规则') }}</span>
          </div>
        </div>

        <!-- VAD & Timeouts -->
        <div class="p-4 bg-[var(--theme-surface)] border border-[var(--theme-border-soft)] rounded-2xl flex flex-col gap-3">
          <h4 class="text-xs font-black text-[var(--theme-text-strong)] uppercase">发话检测与切分超时 (VAD & Timeouts)</h4>
          <div class="grid grid-cols-1 md:grid-cols-3 gap-3">
            <label class="text-xs">
              <span class="block text-[var(--theme-text-muted)] font-bold mb-1">VAD 模式</span>
              <select v-model="vadMode" class="w-full bg-[var(--theme-bg-main)]/60 border border-[var(--theme-border-soft)] rounded-xl px-2.5 py-1.5 text-xs text-[var(--theme-text-strong)] outline-none">
                <option value="webrtc">WebRTC VAD (标准实时)</option>
                <option value="silero">Silero VAD (神经网络)</option>
                <option value="rms">能量阈值 (传统模式)</option>
              </select>
            </label>
            <label class="text-xs">
              <span class="block text-[var(--theme-text-muted)] font-bold mb-1">断句静音超时 (秒)</span>
              <input v-model.number="silenceTimeout" type="number" step="0.1" min="0.2" max="3" class="w-full bg-[var(--theme-bg-main)]/60 border border-[var(--theme-border-soft)] rounded-xl px-2.5 py-1.5 text-xs text-[var(--theme-text-strong)] outline-none">
            </label>
            <label class="text-xs">
              <span class="block text-[var(--theme-text-muted)] font-bold mb-1">单句最长时间限制 (秒)</span>
              <input v-model.number="phraseTimeLimit" type="number" step="1" min="2" max="30" class="w-full bg-[var(--theme-bg-main)]/60 border border-[var(--theme-border-soft)] rounded-xl px-2.5 py-1.5 text-xs text-[var(--theme-text-strong)] outline-none">
            </label>
          </div>
        </div>
      </div>

      <!-- 4. TRANSLATION ENGINES TAB -->
      <div v-else-if="activeSubTab === 'translation'" class="flex flex-col gap-5">
        <div class="p-4 bg-[var(--theme-surface)] border border-[var(--theme-border-soft)] rounded-2xl flex flex-col gap-4">
          <div class="flex items-center justify-between">
            <div>
              <h4 class="text-xs font-black text-[var(--theme-text-strong)] uppercase">翻译引擎与 API 配置</h4>
              <p class="text-[11px] text-[var(--theme-text-muted)]">支持 20+ 款主流云端与本地大模型翻译提供商。</p>
            </div>
            <button type="button" class="px-3 py-1.5 rounded-xl bg-[var(--theme-primary)] hover:brightness-110 text-white font-bold text-xs" @click="saveTranslationProfile">
              保存为翻译档案
            </button>
          </div>

          <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
            <label class="text-xs">
              <span class="block text-[var(--theme-text-muted)] font-bold mb-1">当前翻译服务</span>
              <CustomSelect v-model="translateEngine" :options="engineOptions" />
            </label>
            <label v-if="needsApiKey" class="text-xs">
              <span class="block text-[var(--theme-text-muted)] font-bold mb-1">{{ currentEngine.hint }}</span>
              <input v-model="apiKey" type="password" class="w-full bg-[var(--theme-bg-main)]/60 border border-[var(--theme-border-soft)] rounded-xl px-3 py-1.5 text-xs text-[var(--theme-text-strong)] outline-none focus:border-[var(--theme-primary)]" placeholder="API Key / Auth Key">
            </label>
            <label class="text-xs">
              <span class="block text-[var(--theme-text-muted)] font-bold mb-1">模型名称 (Model)</span>
              <input v-model="model" type="text" class="w-full bg-[var(--theme-bg-main)]/60 border border-[var(--theme-border-soft)] rounded-xl px-3 py-1.5 text-xs text-[var(--theme-text-strong)] outline-none focus:border-[var(--theme-primary)]" placeholder="例如: gpt-4o, gemini-1.5-flash, qwen-mt-plus">
            </label>
            <label class="text-xs">
              <span class="block text-[var(--theme-text-muted)] font-bold mb-1">自定义 API 端点</span>
              <input v-model="customApiUrl" type="text" class="w-full bg-[var(--theme-bg-main)]/60 border border-[var(--theme-border-soft)] rounded-xl px-3 py-1.5 text-xs text-[var(--theme-text-strong)] outline-none focus:border-[var(--theme-primary)]" placeholder="https://api.openai.com/v1">
            </label>
          </div>

          <label class="text-xs">
            <span class="block text-[var(--theme-text-muted)] font-bold mb-1">系统提示词 (System Prompt)</span>
            <textarea v-model="prompt" rows="2" class="w-full bg-[var(--theme-bg-main)]/60 border border-[var(--theme-border-soft)] rounded-xl px-3 py-1.5 text-xs text-[var(--theme-text-strong)] outline-none focus:border-[var(--theme-primary)] resize-none" placeholder="为 LLM 翻译指定个性化风格，例如：保持口语化，适合 VRChat 聊天交友..." />
          </label>
        </div>

        <!-- Glossary Manager -->
        <div class="p-4 bg-[var(--theme-surface)] border border-[var(--theme-border-soft)] rounded-2xl flex flex-col gap-3">
          <div class="flex items-center justify-between">
            <div>
              <h4 class="text-xs font-black text-[var(--theme-text-strong)] uppercase">术语库与专有名词 (Glossary)</h4>
              <p class="text-[11px] text-[var(--theme-text-muted)]">强制替换 VRChat 常用黑话、玩家昵称或特殊词汇。</p>
            </div>
            <button type="button" class="px-2.5 py-1 rounded-lg bg-[var(--theme-surface-hover)] hover:bg-[var(--theme-border-soft)] text-[var(--theme-text)] text-xs font-bold" @click="glossary.push({ source: '', target: '', source_lang: 'any', target_lang: 'any', case_sensitive: false })">
              + 添加术语
            </button>
          </div>

          <div class="flex flex-col gap-2 max-h-48 overflow-y-auto custom-scrollbar">
            <div v-for="(item, idx) in glossary" :key="idx" class="flex items-center gap-2">
              <input v-model="item.source" type="text" class="flex-1 bg-[var(--theme-bg-main)]/60 border border-[var(--theme-border-soft)] rounded-lg px-2.5 py-1 text-xs text-[var(--theme-text-strong)] outline-none" placeholder="原词">
              <span class="text-[var(--theme-text-muted)]">→</span>
              <input v-model="item.target" type="text" class="flex-1 bg-[var(--theme-bg-main)]/60 border border-[var(--theme-border-soft)] rounded-lg px-2.5 py-1 text-xs text-[var(--theme-text-strong)] outline-none" placeholder="替换目标">
              <button type="button" class="text-[var(--theme-text-muted)] hover:text-red-400 px-1" @click="glossary.splice(idx, 1)">×</button>
            </div>
            <span v-if="!glossary.length" class="text-xs text-[var(--theme-text-muted)] py-1">当前没有专有术语。</span>
          </div>
        </div>
      </div>

      <!-- 5. VRCHAT & OSC TAB -->
      <div v-else-if="activeSubTab === 'osc'" class="flex flex-col gap-5">
        <div class="p-4 bg-[var(--theme-surface)] border border-[var(--theme-border-soft)] rounded-2xl flex flex-col gap-4">
          <h4 class="text-xs font-black text-[var(--theme-text-strong)] uppercase">VRChat OSC 传输开关</h4>
          <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
            <label class="flex items-center gap-2 p-3 bg-[var(--theme-bg-main)]/60 rounded-xl border border-[var(--theme-border-soft)] cursor-pointer">
              <input v-model="autoSendOsc" type="checkbox" class="w-4 h-4 rounded accent-[var(--theme-primary)]">
              <div>
                <span class="block text-xs font-bold text-[var(--theme-text-strong)]">自动发送至 Chatbox</span>
                <span class="text-[10px] text-[var(--theme-text-muted)]">说话或输入后通过 UDP OSC /chatbox/input 冒字</span>
              </div>
            </label>

            <label class="flex items-center gap-2 p-3 bg-[var(--theme-bg-main)]/60 rounded-xl border border-[var(--theme-border-soft)] cursor-pointer">
              <input v-model="sendReceivedMessageToVrc" type="checkbox" class="w-4 h-4 rounded accent-[var(--theme-primary)]">
              <div>
                <span class="block text-xs font-bold text-[var(--theme-text-strong)]">{{ tt('translator.osc_speaker2chatbox', '允许将听到的对方语音发送到 Chatbox') }}</span>
                <span class="text-[10px] text-[var(--theme-text-muted)]">将游戏内其他玩家说的话翻译后也发到自己的头顶</span>
              </div>
            </label>

            <label class="flex items-center gap-2 p-3 bg-[var(--theme-bg-main)]/60 rounded-xl border border-[var(--theme-border-soft)] cursor-pointer">
              <input v-model="notificationVrcSfx" type="checkbox" class="w-4 h-4 rounded accent-[var(--theme-primary)]">
              <div>
                <span class="block text-xs font-bold text-[var(--theme-text-strong)]">{{ tt('translator.osc_sfx_chime', '发送消息时播放 VRChat 提示音') }}</span>
                <span class="text-[10px] text-[var(--theme-text-muted)]">冒字时触发 VRChat 自带的叮咚音效</span>
              </div>
            </label>

            <label class="flex items-center gap-2 p-3 bg-[var(--theme-bg-main)]/60 rounded-xl border border-[var(--theme-border-soft)] cursor-pointer">
              <input v-model="vrcMicMuteSync" type="checkbox" class="w-4 h-4 rounded accent-[var(--theme-primary)]">
              <div>
                <span class="block text-xs font-bold text-[var(--theme-text-strong)]">{{ tt('translator.osc_mute_sync', '同步 VRChat 麦克风静音状态') }}</span>
                <span class="text-[10px] text-[var(--theme-text-muted)]">根据游戏内静音状态自动暂停/恢复识别</span>
              </div>
            </label>
          </div>
        </div>

        <!-- Formatting Templates (VRCT Parity) -->
        <div class="p-4 bg-[var(--theme-surface)] border border-[var(--theme-border-soft)] rounded-2xl flex flex-col gap-4">
          <h4 class="text-xs font-black text-[var(--theme-text-strong)] uppercase">{{ tt('translator.osc_format_send', '发送消息格式模版 (Send Format)') }}</h4>
          <div class="grid grid-cols-2 md:grid-cols-3 gap-3">
            <label class="text-xs">
              <span class="block text-[var(--theme-text-muted)] font-bold mb-1">{{ tt('translator.osc_prefix', '原文前缀') }}</span>
              <input v-model="sendMessageFormat.prefix" type="text" class="w-full bg-[var(--theme-bg-main)]/60 border border-[var(--theme-border-soft)] rounded-xl px-2.5 py-1 text-xs text-[var(--theme-text-strong)] outline-none">
            </label>
            <label class="text-xs">
              <span class="block text-[var(--theme-text-muted)] font-bold mb-1">{{ tt('translator.osc_suffix', '原文后缀') }}</span>
              <input v-model="sendMessageFormat.suffix" type="text" class="w-full bg-[var(--theme-bg-main)]/60 border border-[var(--theme-border-soft)] rounded-xl px-2.5 py-1 text-xs text-[var(--theme-text-strong)] outline-none">
            </label>
            <label class="text-xs">
              <span class="block text-[var(--theme-text-muted)] font-bold mb-1">{{ tt('translator.osc_separator', '分隔符') }}</span>
              <input v-model="sendMessageFormat.separator" type="text" class="w-full bg-[var(--theme-bg-main)]/60 border border-[var(--theme-border-soft)] rounded-xl px-2.5 py-1 text-xs text-[var(--theme-text-strong)] outline-none" placeholder="\n">
            </label>
            <label class="text-xs">
              <span class="block text-[var(--theme-text-muted)] font-bold mb-1">{{ tt('translator.osc_trans_prefix', '译文前缀') }}</span>
              <input v-model="sendMessageFormat.transPrefix" type="text" class="w-full bg-[var(--theme-bg-main)]/60 border border-[var(--theme-border-soft)] rounded-xl px-2.5 py-1 text-xs text-[var(--theme-text-strong)] outline-none">
            </label>
            <label class="text-xs">
              <span class="block text-[var(--theme-text-muted)] font-bold mb-1">{{ tt('translator.osc_trans_suffix', '译文后缀') }}</span>
              <input v-model="sendMessageFormat.transSuffix" type="text" class="w-full bg-[var(--theme-bg-main)]/60 border border-[var(--theme-border-soft)] rounded-xl px-2.5 py-1 text-xs text-[var(--theme-text-strong)] outline-none">
            </label>
            <label class="flex items-center gap-2 pt-5 cursor-pointer text-xs font-bold text-[var(--theme-text)]">
              <input v-model="sendMessageFormat.transFirst" type="checkbox" class="w-3.5 h-3.5 rounded accent-[var(--theme-primary)]">
              <span>{{ tt('translator.osc_trans_first', '优先显示译文 (译文在前)') }}</span>
            </label>
          </div>

          <!-- Preview Card -->
          <div class="p-3 bg-[var(--theme-bg-main)]/60 rounded-xl border border-[var(--theme-border-soft)] text-xs">
            <span class="text-[10px] text-[var(--theme-text-muted)] font-bold block mb-1">Chatbox 渲染预览 (Preview):</span>
            <div class="font-mono text-[var(--theme-primary)] whitespace-pre-wrap">
              {{ sendMessageFormat.transFirst
                ? `${sendMessageFormat.transPrefix}Hello world!${sendMessageFormat.transSuffix}${sendMessageFormat.separator}${sendMessageFormat.prefix}你好世界！${sendMessageFormat.suffix}`
                : `${sendMessageFormat.prefix}你好世界！${sendMessageFormat.suffix}${sendMessageFormat.separator}${sendMessageFormat.transPrefix}Hello world!${sendMessageFormat.transSuffix}`
              }}
            </div>
          </div>
        </div>
      </div>

      <!-- 6. CHAT-BUBBLE OCR TAB -->
      <div v-else-if="activeSubTab === 'ocr'" class="flex flex-col gap-5">
        <div class="p-4 bg-[var(--theme-surface)] border border-[var(--theme-border-soft)] rounded-2xl flex flex-col gap-4">
          <div class="flex items-center justify-between">
            <div>
              <h4 class="text-xs font-black text-[var(--theme-text-strong)] uppercase">{{ tt('translator.master_ocr', '气泡/屏幕 OCR 识别配置') }}</h4>
              <p class="text-[11px] text-[var(--theme-text-muted)]">自动截取 VRChat 窗口中其他玩家头顶的文字气泡并翻译为自己的语言。</p>
            </div>
            <button
              type="button"
              :class="masterOcr ? 'bg-cyan-500 text-white font-black' : 'bg-[var(--theme-surface-hover)] text-[var(--theme-text)]'"
              class="px-3 py-1.5 rounded-xl text-xs font-bold transition-all"
              @click="masterOcr = !masterOcr"
            >
              {{ masterOcr ? '气泡捕获已开启' : '开启自动捕获' }}
            </button>
          </div>

          <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
            <label class="text-xs">
              <span class="block text-[var(--theme-text-muted)] font-bold mb-1">{{ tt('translator.ocr_window_title', '目标捕获窗口标题') }}</span>
              <input v-model="ocrWindowTitle" type="text" class="w-full bg-[var(--theme-bg-main)]/60 border border-[var(--theme-border-soft)] rounded-xl px-3 py-1.5 text-xs text-[var(--theme-text-strong)] outline-none">
            </label>
            <label class="text-xs">
              <span class="block text-[var(--theme-text-muted)] font-bold mb-1">OCR 识别语言</span>
              <CustomSelect v-model="ocrSourceLang" :options="languageOptions" />
            </label>
            <label class="text-xs">
              <span class="block text-[var(--theme-text-muted)] font-bold mb-1">{{ tt('translator.ocr_poll_interval', '轮询检测间隔 (毫秒)') }} · {{ ocrPollIntervalMs }}ms</span>
              <input v-model.number="ocrPollIntervalMs" type="range" min="100" max="5000" step="50" class="w-full accent-cyan-500">
            </label>
            <label class="text-xs">
              <span class="block text-[var(--theme-text-muted)] font-bold mb-1">{{ tt('translator.ocr_min_confidence', '置信度过滤阈值') }} · {{ Number(ocrMinConfidence).toFixed(2) }}</span>
              <input v-model.number="ocrMinConfidence" type="range" min="0.1" max="0.99" step="0.05" class="w-full accent-cyan-500">
            </label>
          </div>

          <div class="flex items-center gap-2 pt-2 border-t border-[var(--theme-border-soft)]">
            <button type="button" class="px-3 py-2 rounded-xl bg-[var(--theme-surface-hover)] hover:bg-[var(--theme-border-soft)] text-[var(--theme-text-strong)] text-xs font-bold flex items-center gap-1.5" @click="translatePhoto">
              <Camera class="w-3.5 h-3.5" />
              <span>选择图片识别</span>
            </button>
            <button type="button" class="px-3 py-2 rounded-xl bg-[var(--theme-surface-hover)] hover:bg-[var(--theme-border-soft)] text-[var(--theme-text-strong)] text-xs font-bold flex items-center gap-1.5" @click="waitForPhoto">
              <Eye class="w-3.5 h-3.5" />
              <span>等待新截图自动翻译</span>
            </button>
          </div>
        </div>
      </div>

      <!-- 7. VR OVERLAY & OBS TAB -->
      <div v-else-if="activeSubTab === 'vr_obs'" class="flex flex-col gap-5">
        <!-- VR Subtitles -->
        <div class="p-4 bg-[var(--theme-surface)] border border-[var(--theme-border-soft)] rounded-2xl flex flex-col gap-3">
          <h4 class="text-xs font-black text-[var(--theme-text-strong)] uppercase">{{ tt('translator.vr_subtitles', 'SteamVR 空间字幕覆盖层') }}</h4>
          <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
            <label class="flex items-center gap-2 p-3 bg-[var(--theme-bg-main)]/60 rounded-xl border border-[var(--theme-border-soft)] cursor-pointer">
              <input v-model="overlaySmallLog" type="checkbox" class="w-4 h-4 rounded accent-[var(--theme-primary)]">
              <div>
                <span class="block text-xs font-bold text-[var(--theme-text-strong)]">{{ tt('translator.vr_small_log', '单行快捷字幕 (Small Log)') }}</span>
                <span class="text-[10px] text-[var(--theme-text-muted)]">紧凑视野悬浮，适合快速阅读最新一句话</span>
              </div>
            </label>
            <label class="flex items-center gap-2 p-3 bg-[var(--theme-bg-main)]/60 rounded-xl border border-[var(--theme-border-soft)] cursor-pointer">
              <input v-model="overlayLargeLog" type="checkbox" class="w-4 h-4 rounded accent-[var(--theme-primary)]">
              <div>
                <span class="block text-xs font-bold text-[var(--theme-text-strong)]">{{ tt('translator.vr_large_log', '多行历史字幕 (Large Log)') }}</span>
                <span class="text-[10px] text-[var(--theme-text-muted)]">完整历史窗口，支持手腕或视角空间固定</span>
              </div>
            </label>
          </div>

          <label class="text-xs mt-2">
            <span class="block text-[var(--theme-text-muted)] font-bold mb-1">{{ tt('translator.vr_tracker', '空间追踪绑定点') }}</span>
            <select v-model="overlayTracker" class="w-full bg-[var(--theme-bg-main)]/60 border border-[var(--theme-border-soft)] rounded-xl px-2.5 py-1.5 text-xs text-[var(--theme-text-strong)] outline-none">
              <option value="HMD">HMD 头显视线固定</option>
              <option value="LeftHand">左手手腕 (LeftHand)</option>
              <option value="RightHand">右手手腕 (RightHand)</option>
              <option value="Playspace">房间世界坐标固定 (Playspace)</option>
            </select>
          </label>
        </div>

        <!-- OBS Browser Source -->
        <div class="p-4 bg-[var(--theme-surface)] border border-[var(--theme-border-soft)] rounded-2xl flex flex-col gap-3">
          <div class="flex items-center justify-between">
            <div>
              <h4 class="text-xs font-black text-[var(--theme-text-strong)] uppercase">{{ tt('translator.obs_browser_source', 'OBS 浏览器源字幕服务') }}</h4>
              <p class="text-[11px] text-[var(--theme-text-muted)]">在 OBS 添加浏览器源以将实时双向翻译投射至直播画面。</p>
            </div>
            <button
              type="button"
              :class="obsBrowserSource ? 'bg-[var(--theme-primary)] text-white font-black' : 'bg-[var(--theme-surface-hover)] text-[var(--theme-text)]'"
              class="px-3 py-1.5 rounded-xl text-xs font-bold transition-all"
              @click="obsBrowserSource = !obsBrowserSource"
            >
              {{ obsBrowserSource ? 'OBS 服务已开启' : '开启 OBS 服务' }}
            </button>
          </div>

          <div class="p-3 bg-[var(--theme-bg-main)]/60 rounded-xl border border-[var(--theme-border-soft)] text-xs flex items-center justify-between">
            <span class="font-mono text-[var(--theme-primary)]">http://127.0.0.1:{{ obsPort }}/obs</span>
            <button type="button" class="px-2.5 py-1 rounded-lg bg-[var(--theme-surface-hover)] hover:bg-[var(--theme-border-soft)] text-[var(--theme-text-strong)] text-xs" @click="copyText(`http://127.0.0.1:${obsPort}/obs`)">
              复制链接
            </button>
          </div>

          <div class="grid grid-cols-2 md:grid-cols-4 gap-3 text-xs">
            <label>
              <span class="block text-[var(--theme-text-muted)] font-bold mb-1">字体大小</span>
              <input v-model.number="obsFontSize" type="number" class="w-full bg-[var(--theme-bg-main)]/60 border border-[var(--theme-border-soft)] rounded-xl px-2.5 py-1 text-xs text-[var(--theme-text-strong)]">
            </label>
            <label>
              <span class="block text-[var(--theme-text-muted)] font-bold mb-1">保留最大条数</span>
              <input v-model.number="obsMaxMessages" type="number" class="w-full bg-[var(--theme-bg-main)]/60 border border-[var(--theme-border-soft)] rounded-xl px-2.5 py-1 text-xs text-[var(--theme-text-strong)]">
            </label>
            <label>
              <span class="block text-[var(--theme-text-muted)] font-bold mb-1">显示时长 (秒)</span>
              <input v-model.number="obsDisplayDuration" type="number" class="w-full bg-[var(--theme-bg-main)]/60 border border-[var(--theme-border-soft)] rounded-xl px-2.5 py-1 text-xs text-[var(--theme-text-strong)]">
            </label>
            <label>
              <span class="block text-[var(--theme-text-muted)] font-bold mb-1">描边粗细 (px)</span>
              <input v-model.number="obsOutlineThickness" type="number" class="w-full bg-[var(--theme-bg-main)]/60 border border-[var(--theme-border-soft)] rounded-xl px-2.5 py-1 text-xs text-[var(--theme-text-strong)]">
            </label>
          </div>
        </div>

        <!-- WebSocket Broadcast Server -->
        <div class="p-4 bg-[var(--theme-surface)] border border-[var(--theme-border-soft)] rounded-2xl flex flex-col gap-3">
          <div class="flex items-center justify-between">
            <div>
              <h4 class="text-xs font-black text-[var(--theme-text-strong)] uppercase">{{ tt('translator.websocket_server', 'WebSocket 字幕广播服务器') }}</h4>
              <p class="text-[11px] text-[var(--theme-text-muted)]">供外部工具 (如 VRCT-TTS、VTuber 渲染器) 接入接收全量文字流。</p>
            </div>
            <button
              type="button"
              :class="websocketServer ? 'bg-cyan-500 text-white font-black' : 'bg-[var(--theme-surface-hover)] text-[var(--theme-text)]'"
              class="px-3 py-1.5 rounded-xl text-xs font-bold transition-all"
              @click="websocketServer = !websocketServer"
            >
              {{ websocketServer ? 'WS 广播中' : '开启 WS 服务器' }}
            </button>
          </div>

          <div class="flex items-center gap-3 text-xs">
            <span class="font-mono text-cyan-400">ws://127.0.0.1:{{ websocketPort }}</span>
            <div class="flex-1 bg-[var(--theme-bg-main)]/60 border border-[var(--theme-border-soft)] rounded-xl px-3 py-1 text-xs font-mono text-[var(--theme-text-muted)] truncate">
              Token: {{ websocketAuthToken }}
            </div>
            <button type="button" class="px-2.5 py-1 rounded-lg bg-[var(--theme-surface-hover)] hover:bg-[var(--theme-border-soft)] text-[var(--theme-text-strong)] text-xs" @click="copyText(websocketAuthToken)">
              复制 Token
            </button>
          </div>
        </div>
      </div>

      <!-- 8. PREFERENCES TAB -->
      <div v-else-if="activeSubTab === 'preferences'" class="flex flex-col gap-5">
        <!-- Hotkeys -->
        <div class="p-4 bg-[var(--theme-surface)] border border-[var(--theme-border-soft)] rounded-2xl flex flex-col gap-3">
          <h4 class="text-xs font-black text-[var(--theme-text-strong)] uppercase">全局快捷键配置 (Global Hotkeys)</h4>
          <div class="grid grid-cols-1 md:grid-cols-2 gap-3 text-xs">
            <label>
              <span class="block text-[var(--theme-text-muted)] font-bold mb-1">切换软件界面显示/隐藏</span>
              <input v-model="hotkeyToggleWindow" type="text" class="w-full bg-[var(--theme-bg-main)]/60 border border-[var(--theme-border-soft)] rounded-xl px-3 py-1.5 text-xs text-[var(--theme-text-strong)] outline-none">
            </label>
            <label>
              <span class="block text-[var(--theme-text-muted)] font-bold mb-1">切换主翻译开关</span>
              <input v-model="hotkeyToggleTrans" type="text" class="w-full bg-[var(--theme-bg-main)]/60 border border-[var(--theme-border-soft)] rounded-xl px-3 py-1.5 text-xs text-[var(--theme-text-strong)] outline-none">
            </label>
            <label>
              <span class="block text-[var(--theme-text-muted)] font-bold mb-1">切换麦克风识别 (Mic STT)</span>
              <input v-model="hotkeyToggleMic" type="text" class="w-full bg-[var(--theme-bg-main)]/60 border border-[var(--theme-border-soft)] rounded-xl px-3 py-1.5 text-xs text-[var(--theme-text-strong)] outline-none">
            </label>
            <label>
              <span class="block text-[var(--theme-text-muted)] font-bold mb-1">切换游戏扬声器识别 (Speaker STT)</span>
              <input v-model="hotkeyToggleSpeaker" type="text" class="w-full bg-[var(--theme-bg-main)]/60 border border-[var(--theme-border-soft)] rounded-xl px-3 py-1.5 text-xs text-[var(--theme-text-strong)] outline-none">
            </label>
          </div>
        </div>

        <!-- Transliteration & Send Behavior -->
        <div class="p-4 bg-[var(--theme-surface)] border border-[var(--theme-border-soft)] rounded-2xl flex flex-col gap-3">
          <h4 class="text-xs font-black text-[var(--theme-text-strong)] uppercase">日文注音与发送交互</h4>
          <div class="grid grid-cols-1 md:grid-cols-3 gap-3">
            <label class="flex items-center gap-2 p-3 bg-[var(--theme-bg-main)]/60 rounded-xl border border-[var(--theme-border-soft)] cursor-pointer text-xs font-bold text-[var(--theme-text)]">
              <input v-model="romajiMode" type="checkbox" class="w-4 h-4 rounded accent-[var(--theme-primary)]">
              <span>{{ tt('translator.romaji_mode', '日文罗马音 (Romaji)') }}</span>
            </label>
            <label class="flex items-center gap-2 p-3 bg-[var(--theme-bg-main)]/60 rounded-xl border border-[var(--theme-border-soft)] cursor-pointer text-xs font-bold text-[var(--theme-text)]">
              <input v-model="hiraganaMode" type="checkbox" class="w-4 h-4 rounded accent-[var(--theme-primary)]">
              <span>{{ tt('translator.hiragana_mode', '日文平假名 (Hiragana)') }}</span>
            </label>
            <label class="flex items-center gap-2 p-3 bg-[var(--theme-bg-main)]/60 rounded-xl border border-[var(--theme-border-soft)] cursor-pointer text-xs font-bold text-[var(--theme-text)]">
              <input v-model="showResendButton" type="checkbox" class="w-4 h-4 rounded accent-[var(--theme-primary)]">
              <span>显示消息重新发送按钮</span>
            </label>
          </div>
        </div>

        <!-- TTS Speech Output -->
        <div class="p-4 bg-[var(--theme-surface)] border border-[var(--theme-border-soft)] rounded-2xl flex flex-col gap-3">
          <h4 class="text-xs font-black text-[var(--theme-text-strong)] uppercase">语音合成播报 (TTS)</h4>
          <div class="grid grid-cols-1 md:grid-cols-3 gap-3 text-xs">
            <label>
              <span class="block text-[var(--theme-text-muted)] font-bold mb-1">TTS 播报引擎</span>
              <select v-model="ttsEngine" class="w-full bg-[var(--theme-bg-main)]/60 border border-[var(--theme-border-soft)] rounded-xl px-2.5 py-1.5 text-xs text-[var(--theme-text-strong)] outline-none">
                <option value="system">系统原生 (Web Speech)</option>
                <option value="edge">Edge TTS (高清微软多国音色)</option>
                <option value="gpt_sovits">GPT-SoVITS (本地克隆音色)</option>
              </select>
            </label>
            <label>
              <span class="block text-[var(--theme-text-muted)] font-bold mb-1">TTS 语速 · {{ Number(ttsRate).toFixed(1) }}</span>
              <input v-model.number="ttsRate" type="range" min="0.5" max="2" step="0.1" class="w-full accent-[var(--theme-primary)]">
            </label>
            <label>
              <span class="block text-[var(--theme-text-muted)] font-bold mb-1">TTS 音量 · {{ Math.round(Number(ttsVolume) * 100) }}%</span>
              <input v-model.number="ttsVolume" type="range" min="0" max="1" step="0.05" class="w-full accent-[var(--theme-primary)]">
            </label>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
