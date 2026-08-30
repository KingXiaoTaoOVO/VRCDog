<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue';
import { isTauri } from '@tauri-apps/api/core';
import { convertFileSrc } from '@tauri-apps/api/core';
import { emit, listen, type UnlistenFn } from '@tauri-apps/api/event';
import { WebviewWindow } from '@tauri-apps/api/webviewWindow';
import { open } from '@tauri-apps/plugin-dialog';
import { save } from '@tauri-apps/plugin-dialog';
import { useStorage } from '@vueuse/core';
import {
  CheckCircle2,
  Camera,
  ClipboardList,
  Ear,
  Headphones,
  Languages,
  Mic,
  MicOff,
  MonitorUp,
  Plus,
  RefreshCw,
  RotateCcw,
  Send,
  Settings,
  SlidersHorizontal,
  Square,
  Trash2,
  Volume2,
} from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';
import { GalleryApi, SysApi, VrctApi, VrpianoApi, type AudioDevice, type AudioSource, type KeywordAction } from '../api';
import * as TranslationApis from '../api';
import { SerialTaskQueue } from '../utils/serialTaskQueue';
import CustomSelect from './CustomSelect.vue';

type MessageSource = 'chat' | 'mic' | 'speaker';
type TtsEngine = 'system' | 'edge' | 'gpt_sovits' | 'server';

interface TranslationProfile {
  id: string;
  name: string;
  engine: string;
  apiKey: string;
  model: string;
  customApiUrl: string;
  prompt: string;
}

type TranslationRoute = 'manual' | 'mic' | 'speaker' | 'photo';

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

const { t } = useI18n();

const tt = (key: string, fallback: string) => {
  const value = t(key);
  return value === key ? fallback : value;
};

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

const speechLanguageOptions = computed(() => languageOptions.value.filter((option) => option.value !== 'auto'));

const engineOptions = computed<EngineOption[]>(() => [
  { label: 'Google Translate Free', value: 'google_free', hint: tt('translator.hint_no_api_key', 'No API key required') },
  { label: 'Google Cloud Translation', value: 'google_cloud', needsKey: true, hint: tt('translator.hint_google_cloud_key', 'Google Cloud API key') },
  { label: 'Microsoft Translator', value: 'microsoft', needsKey: true, hint: tt('translator.hint_azure_key', 'Azure Translator key') },
  { label: 'DeepL Free', value: 'deepl_free', needsKey: true, hint: tt('translator.hint_deepl_free_key', 'DeepL Free auth key') },
  { label: 'DeepL Pro', value: 'deepl', needsKey: true, hint: tt('translator.hint_deepl_pro_key', 'DeepL Pro auth key') },
  { label: 'Tencent Translate', value: 'tencent', needsKey: true, hint: 'SecretId:SecretKey' },
  { label: 'Baidu Translate', value: 'baidu', needsKey: true, hint: 'AppID:SecretKey' },
  { label: 'Papago', value: 'papago', needsKey: true, hint: 'ClientId:ClientSecret' },
  { label: 'Gemini', value: 'gemini', needsKey: true, hint: tt('translator.hint_google_ai_key', 'Google AI Studio key') },
  { label: 'OpenAI', value: 'openai', needsKey: true, hint: tt('translator.hint_openai_key', 'OpenAI compatible API key') },
  { label: 'DeepSeek', value: 'deepseek', needsKey: true, hint: 'DeepSeek API key' },
  { label: 'SiliconFlow', value: 'siliconflow', needsKey: true, hint: 'SiliconFlow API key' },
  { label: 'Moonshot', value: 'moonshot', needsKey: true, hint: 'Moonshot API key' },
  { label: 'ZhiPu GLM', value: 'zhipu', needsKey: true, hint: 'ZhiPu API key' },
  { label: 'Groq', value: 'groq', needsKey: true, hint: 'Groq API key' },
  { label: 'OpenRouter', value: 'openrouter', needsKey: true, hint: 'OpenRouter API key' },
  { label: 'Plamo', value: 'plamo', needsKey: true, hint: 'PreferredAI Platform API key' },
  { label: tt('translator.engine_ollama_local', 'Ollama Local'), value: 'ollama', supportsLocal: true, hint: 'http://127.0.0.1:11434' },
  { label: tt('translator.engine_lmstudio_local', 'LM Studio Local'), value: 'lmstudio', supportsLocal: true, hint: 'http://127.0.0.1:1234' },
  { label: tt('translator.engine_custom_openai', 'Custom OpenAI API'), value: 'custom_llm', needsKey: true, hint: tt('translator.hint_openai_endpoint', 'OpenAI-compatible endpoint') },
]);

const speakerEngineOptions = computed<Option[]>(() => [
  { label: tt('translator.engine_cloud', 'Cloud recognition (Google Web Speech)'), value: 'cloud' },
  { label: tt('translator.engine_local', 'Local Whisper / offline fallback'), value: 'local' },
  { label: tt('translator.engine_sherpa', 'Sherpa-ONNX streaming'), value: 'sherpa' },
  { label: tt('translator.engine_tencent_realtime', 'Tencent Cloud realtime ASR'), value: 'tencent_realtime' },
  { label: tt('translator.engine_aliyun_realtime', 'Alibaba Cloud NLS realtime ASR'), value: 'aliyun_realtime' },
]);

const sourceLang = useStorage('vrc_translator_source_lang', 'zh-CN');
const targetLang = useStorage('vrc_translator_target_lang', 'en-US');
const otherSourceLang = useStorage('vrc_translator_other_source_lang', 'en-US');
const otherTargetLang = useStorage('vrc_translator_other_target_lang', 'zh-CN');
const photoSourceLang = useStorage('vrc_translator_photo_source_lang', 'auto');
const photoTargetLang = useStorage('vrc_translator_photo_target_lang', 'zh-CN');
const photoOcrLang = useStorage('vrc_translator_photo_ocr_lang', 'auto');
const translateEngine = useStorage('vrc_translator_engine', 'google_free');
const micEngine = useStorage<'cloud' | 'local' | 'sherpa' | 'tencent_realtime' | 'aliyun_realtime'>('vrc_translator_mic_stt_engine', 'cloud');
const otherEngine = useStorage<'cloud' | 'local' | 'sherpa' | 'tencent_realtime' | 'aliyun_realtime'>('vrc_translator_stt_engine', 'cloud');
const apiKey = useStorage('vrc_translator_api_key', '');
const model = useStorage('vrc_translator_model', '');
const customApiUrl = useStorage('vrc_translator_custom_api_url', '');
const prompt = useStorage('vrc_translator_prompt', '');
const whisperModel = useStorage('vrc_translator_whisper_model', 'tiny');
const micDeviceId = useStorage('vrc_translator_mic_device', '');
const speakerDeviceId = useStorage('vrc_translator_speaker_device', '');
const micEnergyThreshold = useStorage('vrc_translator_mic_energy_threshold', 0);
const speakerEnergyThreshold = useStorage('vrc_translator_speaker_energy_threshold', 0);
const phraseTimeLimit = useStorage('vrc_translator_phrase_time_limit', 10);
const vadType = useStorage('vrc_translator_vad_type', 'webrtc');
const vadAggressiveness = useStorage('vrc_translator_vad_aggressiveness', 2);
const denoiseStrength = useStorage('vrc_translator_denoise_strength', 0);
const correctionEnabled = useStorage('vrc_translator_correction_enabled', false);
const minSegmentS = useStorage('vrc_translator_min_segment_s', 0.45);
const maxSegmentS = useStorage('vrc_translator_max_segment_s', 8.0);
const partialInterval = useStorage('vrc_translator_partial_interval', 1.2);
const captureMode = useStorage('vrc_translator_capture_mode', 'loopback');
const targetProcess = useStorage('vrc_translator_target_process', 'VRChat.exe');
const selfSuppress = useStorage('vrc_translator_self_suppress', false);
const selfSuppressSeconds = useStorage('vrc_translator_self_suppress_seconds', 0.8);
const glossary = useStorage<GlossaryTerm[]>('vrc_translator_glossary', []);
const contextEnabled = useStorage('vrc_translator_context_enabled', true);
const retryCount = useStorage('vrc_translator_retry_count', 2);
const translationProfiles = useStorage<TranslationProfile[]>('vrc_translator_profiles', []);
const activeProfileId = useStorage('vrc_translator_active_profile', '');
const routeProfileIds = useStorage<Record<TranslationRoute, string>>('vrc_translator_route_profiles', {
  manual: '',
  mic: '',
  speaker: '',
  photo: '',
});
const ttsRate = useStorage('vrc_translator_tts_rate', 1);
const ttsVolume = useStorage('vrc_translator_tts_volume', 1);
const interruptTts = useStorage('vrc_translator_tts_interrupt', true);
const keywordActions = useStorage<KeywordAction[]>('vrc_translator_keyword_actions', []);
const quickInputHotkey = useStorage('vrc_translator_hotkey_input', 'Ctrl+Alt+I');
const voiceToggleHotkey = useStorage('vrc_translator_hotkey_voice', 'Ctrl+F8');
const audioLevels = ref<Record<'mic' | 'speaker', number>>({ mic: 0, speaker: 0 });
const vadCalibration = ref<{ source: 'mic' | 'speaker'; levels: number[]; suggested?: number } | null>(null);
const vadCalibrationPhase = ref<'noise' | 'voice' | null>(null);
const realtimeAsrStatus = ref('');
const modelStatus = ref<{ installed: boolean; valid: boolean; size: number; path: string } | null>(null);
const modelBusy = ref(false);
const hotkeyConflicts = ref<Array<{ hotkey: string; reason: string }>>([]);
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

const manualText = ref('');
const recognizedText = ref('');
const translatedText = ref('');
const currentTranslations = ref<Array<{ target_lang: string; translated: string }>>([]);
const lastTargetLang = ref(targetLang.value);
const history = ref<VrctMessageRecord[]>([]);

const autoSendOsc = useStorage('vrc_translator_auto_send_osc', true);
const showOriginalOsc = useStorage('vrc_translator_show_original', true);
const autoPlayTts = useStorage('vrc_translator_auto_tts', false);
const multiLangEnabled = useStorage('vrc_translator_multi_lang', false);
const multiLangTargets = useStorage<string[]>('vrc_translator_multi_targets', []);
const ttsEngine = useStorage<TtsEngine>('vrc_translator_tts_engine', 'system');
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

const isRecording = ref(false);
const isOtherRecording = ref(false);
const isMicStarting = ref(false);
const isSpeakerStarting = ref(false);
const isTranslating = ref(false);
const isOverlayOpen = ref(false);
const overlayBackgroundOpacity = useStorage('vrc_translation_overlay_opacity', 0.82);
const errorMsg = ref('');
const statusMsg = ref('');
const audioDevices = ref<AudioDevice[]>([]);
const audioDeviceError = ref('');
const photoBusy = ref(false);
const photoArmed = ref(false);
const photoPath = ref('');
const photoOriginal = ref('');
const photoTranslated = ref('');

// 自声抑制（self-suppress）：自己用麦克风说话时，暂停"听别人"捕获，
// 避免把泄漏到自己游戏音频里的声音再次转写。由前端用已有的暂停/恢复接口协调。
let speakerSuppressedBySelf = false;
let selfSuppressTimer: ReturnType<typeof setTimeout> | null = null;

let overlayWebview: WebviewWindow | null = null;
let unlistenAudio: UnlistenFn | null = null;
let unlistenVrct: UnlistenFn | null = null;
let unlistenTranslationHotkey: UnlistenFn | null = null;

const currentEngine = computed(() => engineOptions.value.find((engine) => engine.value === translateEngine.value) ?? engineOptions.value[0]);
const needsApiKey = computed(() => Boolean(currentEngine.value.needsKey && !currentEngine.value.supportsLocal));
const showModelField = computed(() => ['openai', 'deepseek', 'siliconflow', 'moonshot', 'zhipu', 'groq', 'openrouter', 'plamo', 'ollama', 'lmstudio', 'custom_llm', 'gemini'].includes(translateEngine.value));
const canTranslate = computed(() => !isTranslating.value && Boolean(manualText.value.trim()));
const normalizedGlossary = computed(() => glossary.value.filter((term) => term.source.trim() && term.target.trim()).slice(0, 128));
const activeProfile = computed(() => translationProfiles.value.find((profile) => profile.id === activeProfileId.value));
const profileOptions = computed<Option[]>(() => [
  { label: tt('translator.profile_default', '跟随当前配置'), value: '' },
  ...translationProfiles.value.map((profile) => ({ label: profile.name, value: profile.id })),
]);

const routeConfig = (route: TranslationRoute) => {
  const profile = translationProfiles.value.find((item) => item.id === routeProfileIds.value[route]);
  return profile
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
};
const micDeviceOptions = computed<Option[]>(() => audioDevices.value
  .filter(device => device.source === 'mic')
  .map(device => ({ label: `${device.name}${device.is_default ? ` (${t('translator.default')}})` : ''}`, value: device.id })));
const speakerDeviceOptions = computed<Option[]>(() => audioDevices.value
  .filter(device => device.source === 'speaker')
  .map(device => ({ label: `${device.name}${device.is_default ? ` (${t('translator.default')}})` : ''}`, value: device.id })));

const translationQueue = new SerialTaskQueue((pending) => {
  isTranslating.value = pending > 0;
});

const setStatus = (message: string) => {
  statusMsg.value = message;
  window.setTimeout(() => {
    if (statusMsg.value === message) statusMsg.value = '';
  }, 3000);
};

const errorText = (error: unknown) => {
  if (error instanceof Error) return error.message;
  if (typeof error === 'string') return error;
  return JSON.stringify(error);
};

const overlayType = (source: MessageSource): 'self' | 'other' => (source === 'speaker' ? 'other' : 'self');

const addHistory = (record: VrctMessageRecord) => {
  const exists = history.value.some((item) => item.id === record.id);
  if (!exists) {
    history.value = [record, ...history.value].slice(0, 80);
  }
};

const applyRecord = (record: VrctMessageRecord) => {
  recognizedText.value = record.original;
  translatedText.value = record.translated;
  currentTranslations.value = record.translations?.length
    ? record.translations
    : [{ target_lang: record.target_lang, translated: record.translated }];
  lastTargetLang.value = record.target_lang || targetLang.value;
  addHistory(record);
};

const notifyOverlay = async (record: VrctMessageRecord) => {
  await emit('translation-log', {
    id: record.id,
    type: overlayType(record.source),
    text: record.original,
    translation: record.translated,
    translations: record.translations?.map(item => ({
      lang: item.target_lang,
      text: item.translated,
    })),
  });
};

const syncOverlaySettings = async () => {
  if (!isTauri()) return;
  await emit('translation-overlay-settings', {
    backgroundOpacity: Math.min(1, Math.max(0, Number(overlayBackgroundOpacity.value) || 0)),
  });
};

watch(overlayBackgroundOpacity, () => {
  syncOverlaySettings().catch(() => undefined);
});

const playTts = async (text: string, lang = lastTargetLang.value) => {
  if (!text.trim()) return;
  const pauseLoopback = isTauri() && isOtherRecording.value;
  try {
    if (pauseLoopback) {
      await SysApi.setAudioCapturePaused({ source: 'speaker', paused: true });
    }

    if (ttsEngine.value === 'system') {
      if (!('speechSynthesis' in window)) {
        throw new Error(tt('auto_43b1967a', 'This WebView does not support speech synthesis.'));
      }
      if (interruptTts.value) window.speechSynthesis.cancel();
      const utterance = new SpeechSynthesisUtterance(text);
      utterance.lang = lang;
      utterance.rate = Math.min(2, Math.max(0.5, Number(ttsRate.value) || 1));
      utterance.volume = Math.min(1, Math.max(0, Number(ttsVolume.value) || 0));
      await new Promise<void>((resolve, reject) => {
        utterance.onend = () => resolve();
        utterance.onerror = event => reject(new Error(event.error || 'Speech synthesis failed'));
        window.speechSynthesis.speak(utterance);
      });
    } else if (ttsEngine.value === 'edge') {
      const result = await VrpianoApi.synthesizeSpeech({
        text,
        voice: lang.startsWith('ja') ? 'ja-JP-NanamiNeural' : lang.startsWith('ko') ? 'ko-KR-SunHiNeural' : lang.startsWith('en') ? 'en-US-AriaNeural' : 'zh-CN-XiaoxiaoNeural',
        rate: Number(ttsRate.value) || 1,
        volume: Number(ttsVolume.value) || 1,
      });
      const audio = new Audio(convertFileSrc(result.output_path));
      audio.volume = Math.min(1, Math.max(0, Number(ttsVolume.value) || 0));
      await new Promise<void>((resolve, reject) => {
        audio.onended = () => resolve();
        audio.onerror = () => reject(new Error('Edge TTS audio playback failed'));
        audio.play().catch(reject);
      });
    } else if (ttsEngine.value === 'gpt_sovits') {
      const langCode = lang.startsWith('ja') ? 'ja' : lang.startsWith('ko') ? 'ko' : lang.startsWith('en') ? 'en' : 'zh';
      const audioUrl = await SysApi.synthesizeGptSovits({
        baseUrl: gptSovitsUrl.value,
        text,
        textLanguage: langCode,
        sovitsWeights: gptSovitsWeights.value.trim() || undefined,
        gptWeights: gptWeights.value.trim() || undefined,
        referenceAudio: gptReferenceAudio.value.trim() || undefined,
        promptText: gptPromptText.value.trim() || undefined,
        promptLanguage: gptPromptLanguage.value || langCode,
      });
      const audio = new Audio(audioUrl);
      audio.volume = 1;
      await new Promise<void>((resolve, reject) => {
        audio.onended = () => resolve();
        audio.onerror = () => reject(new Error('TTS audio playback failed'));
        audio.play().catch(reject);
      });
    } else {
      const result = await TranslationApis.TtsApi?.synthesize?.({
        provider: serverTtsProvider.value,
        baseUrl: serverTtsBaseUrl.value,
        apiKey: serverTtsApiKey.value,
        text,
        language: lang,
        voice: serverTtsVoice.value,
        speed: Number(ttsRate.value) || 1,
        volume: Number(ttsVolume.value) || 1,
        referenceAudio: gptReferenceAudio.value,
        referenceText: ttsReferenceText.value,
        instruct: gptPromptText.value,
      });
      if (!result) throw new Error('TTS provider is unavailable');
      const audio = new Audio(convertFileSrc(result.output_path));
      audio.volume = Math.min(1, Math.max(0, Number(ttsVolume.value) || 0));
      await new Promise<void>((resolve, reject) => { audio.onended = () => resolve(); audio.onerror = () => reject(new Error('Server TTS audio playback failed')); audio.play().catch(reject); });
    }
  } catch (error) {
    errorMsg.value = tt('translator.tts_error', 'TTS playback failed: {err}').replace('{err}', errorText(error));
  } finally {
    if (pauseLoopback) {
      await new Promise(resolve => setTimeout(resolve, 200));
      await SysApi.setAudioCapturePaused({ source: 'speaker', paused: false }).catch(() => undefined);
    }
  }
};

const saveTranslationProfile = () => {
  const id = activeProfileId.value || `profile-${Date.now()}`;
  const profile: TranslationProfile = {
    id,
    name: activeProfile.value?.name || `${currentEngine.value.label} ${translationProfiles.value.length + 1}`,
    engine: translateEngine.value,
    apiKey: apiKey.value,
    model: model.value,
    customApiUrl: customApiUrl.value,
    prompt: prompt.value,
  };
  translationProfiles.value = [...translationProfiles.value.filter((item) => item.id !== id), profile];
  activeProfileId.value = id;
  setStatus(tt('translator.profile_saved', '翻译档案已保存'));
};

const setRouteProfile = (route: TranslationRoute, profileId: string) => {
  routeProfileIds.value = { ...routeProfileIds.value, [route]: profileId };
};

const applyTranslationProfile = (profile: TranslationProfile) => {
  activeProfileId.value = profile.id;
  translateEngine.value = profile.engine;
  apiKey.value = profile.apiKey;
  model.value = profile.model;
  customApiUrl.value = profile.customApiUrl;
  prompt.value = profile.prompt;
};

const deleteTranslationProfile = (profileId: string) => {
  translationProfiles.value = translationProfiles.value.filter((profile) => profile.id !== profileId);
  if (activeProfileId.value === profileId) activeProfileId.value = '';
};

const addKeywordAction = () => {
  keywordActions.value = [...keywordActions.value, { keyword: '', address: '/avatar/parameters/VRCDogAction', host: '127.0.0.1', port: 9000, value: 1, valueType: 'float', enabled: true, cooldownMs: 1500 }];
};

const removeKeywordAction = (index: number) => {
  keywordActions.value = keywordActions.value.filter((_, actionIndex) => actionIndex !== index);
};

const checkTranslationHotkeys = async () => {
  try {
    hotkeyConflicts.value = await TranslationApis.TranslationHotkeyApi?.check?.([quickInputHotkey.value, voiceToggleHotkey.value]).catch(() => []) ?? [];
    if (isTauri()) await TranslationApis.TranslationHotkeyApi?.apply?.([{ id: 4101, hotkey: quickInputHotkey.value }, { id: 4102, hotkey: voiceToggleHotkey.value }]).catch(() => undefined);
  } catch {
    hotkeyConflicts.value = [];
  }
};

const refreshModelStatus = async () => {
  if (!isTauri()) return;
  try {
    modelStatus.value = await TranslationApis.ModelRuntimeApi?.getStatus?.().catch(() => null) ?? null;
  } catch {
    modelStatus.value = null;
  }
};

const updateTranslationRuntime = async () => {
  if (!isTauri() || !TranslationApis.TranslationRuntimeApi?.update) return;
  try {
    const runtime = await TranslationApis.TranslationRuntimeApi.update();
    if (runtime?.capabilities?.version) runtimeVersion.value = runtime.capabilities.version;
    setStatus(tt('translator.runtime_updated', '翻译运行时能力已更新'));
  } catch (error) {
    errorMsg.value = `${tt('translator.runtime_update_failed', '翻译运行时更新失败')}: ${errorText(error)}`;
  }
};

const downloadSilero = async () => {
  modelBusy.value = true;
  try {
    modelStatus.value = await TranslationApis.ModelRuntimeApi?.downloadSilero?.() ?? null;
    setStatus(tt('translator.model_ready', 'Silero VAD 模型已就绪'));
  } catch (error) {
    errorMsg.value = `${tt('translator.model_download_failed', '模型下载失败')}: ${errorText(error)}`;
  } finally {
    modelBusy.value = false;
  }
};

const startVadCalibration = (source: 'mic' | 'speaker') => {
  vadCalibration.value = { source, levels: [], suggested: undefined };
  vadCalibrationPhase.value = 'noise';
  setStatus(tt('translator.vad_calibrating', '正在采集环境噪声，请保持安静...'));
  window.setTimeout(() => {
    if (vadCalibration.value?.source === source) {
      vadCalibrationPhase.value = 'voice';
      setStatus(tt('translator.vad_speak_now', '请说一句完整句子，正在采集语音峰值...'));
    }
  }, 2000);
  window.setTimeout(async () => {
    const current = vadCalibration.value;
    if (!current || current.source !== source) return;
    const result = await TranslationApis.ModelRuntimeApi?.calibrateVad?.({ source, observedLevels: current.levels.length ? current.levels : [audioLevels.value[source]] }).catch(() => null);
    if (result) {
      current.suggested = result.suggestedThreshold;
      if (source === 'mic') micEnergyThreshold.value = result.suggestedThreshold;
      else speakerEnergyThreshold.value = result.suggestedThreshold;
      setStatus(`${tt('translator.vad_suggested', '建议阈值')}: ${result.suggestedThreshold}`);
    }
  }, 4000);
};

const stopVadCalibration = () => { vadCalibration.value = null; };

const validateRealtimeAsr = async () => {
  try {
    const result = await TranslationApis.TranslationRuntimeApi?.validateRealtimeAsr?.({
      provider: realtimeAsrConfig.value.provider,
      app_id: realtimeAsrConfig.value.appId,
      secret_id: realtimeAsrConfig.value.secretId,
      secret_key: realtimeAsrConfig.value.secretKey,
      app_key: realtimeAsrConfig.value.appKey,
      access_token: realtimeAsrConfig.value.accessToken,
      model: realtimeAsrConfig.value.model,
    });
    realtimeAsrStatus.value = result?.message || tt('translator.realtime_validation_failed', '实时 ASR 配置验证失败');
  } catch (error) {
    realtimeAsrStatus.value = errorText(error);
  }
};

const loadTtsPresets = async () => {
  try { ttsPresets.value = await TranslationApis.TtsApi?.listPresets?.() ?? []; } catch { ttsPresets.value = []; }
};

const applyTtsPreset = (id: string) => {
  const preset = ttsPresets.value.find((item) => item.id === id);
  if (!preset) return;
  activeTtsPresetId.value = id;
  serverTtsProvider.value = preset.provider || serverTtsProvider.value;
  serverTtsVoice.value = preset.voice || '';
  ttsReferenceText.value = preset.referenceText || '';
  gptReferenceAudio.value = preset.referenceAudio || '';
  gptPromptText.value = preset.instruct || '';
};

const saveTtsPreset = async () => {
  const presets = await TranslationApis.TtsApi?.savePreset?.({
    id: activeTtsPresetId.value,
    name: `${serverTtsProvider.value} ${serverTtsVoice.value || 'default'}`,
    provider: serverTtsProvider.value,
    voice: serverTtsVoice.value,
    language: gptPromptLanguage.value,
    referenceAudio: gptReferenceAudio.value,
    referenceText: ttsReferenceText.value,
    instruct: gptPromptText.value,
  }).catch(() => null);
  if (presets) { ttsPresets.value = presets; activeTtsPresetId.value = presets[presets.length - 1]?.id || activeTtsPresetId.value; setStatus(tt('translator.tts_preset_saved', 'TTS 预设已保存')); }
};

const deleteTtsPreset = async () => {
  if (!activeTtsPresetId.value) return;
  ttsPresets.value = await TranslationApis.TtsApi?.deletePreset?.(activeTtsPresetId.value).catch(() => ttsPresets.value) ?? ttsPresets.value;
  activeTtsPresetId.value = '';
};

const exportTtsPresets = async () => {
  const path = await save({ defaultPath: 'vrcdog-tts-presets.json', filters: [{ name: 'JSON', extensions: ['json'] }] });
  if (path) await TranslationApis.TtsApi?.exportPresets?.(path).catch(() => undefined);
};

const importTtsPresets = async () => {
  const path = await open({ multiple: false, filters: [{ name: 'JSON', extensions: ['json'] }] });
  if (typeof path === 'string') ttsPresets.value = await TranslationApis.TtsApi?.importPresets?.(path).catch(() => ttsPresets.value) ?? ttsPresets.value;
};

const updateAudioLevel = (source: AudioSource, level: unknown) => {
  const numeric = Number(level);
  if (!Number.isFinite(numeric)) return;
  audioLevels.value = { ...audioLevels.value, [source]: Math.min(10000, Math.max(0, numeric)) };
  if (vadCalibration.value?.source === source) vadCalibration.value.levels.push(numeric);
};

const translatePhoto = async () => {
  const selected = await open({ multiple: false, directory: false, title: tt('translator.photo_choose', '选择要翻译的图片') });
  if (!selected || typeof selected !== 'string') return;
  await translatePhotoPath(selected);
};

const translatePhotoPath = async (selected: string) => {
  photoBusy.value = true;
  photoPath.value = selected;
  photoOriginal.value = '';
  photoTranslated.value = '';
  try {
    const photoConfig = routeConfig('photo');
    const result = await VrctApi.translateImage({ request: {
      image_path: selected,
      source_lang: photoSourceLang.value,
      target_lang: photoTargetLang.value,
      ocr_lang: photoOcrLang.value,
      service: photoConfig.engine,
      api_key: photoConfig.apiKey.trim(),
      model: photoConfig.model.trim(),
      prompt: photoConfig.prompt.trim(),
      custom_api_url: photoConfig.customApiUrl.trim(),
      glossary: normalizedGlossary.value,
      retry_count: Math.min(3, Math.max(0, Number(retryCount.value) || 0)),
    } });
    photoOriginal.value = result.original;
    photoTranslated.value = result.translated;
    await emit('translation-log', { id: Date.now(), type: 'other', text: result.original, translation: result.translated });
    setStatus(tt('translator.photo_done', '拍照翻译完成'));
  } catch (error) {
    errorMsg.value = `${tt('translator.photo_failed', '拍照翻译失败')}: ${errorText(error)}`;
  } finally {
    photoBusy.value = false;
  }
};

const waitForPhoto = async () => {
  if (photoBusy.value || photoArmed.value) return;
  photoArmed.value = true;
  try {
    setStatus(tt('translator.photo_waiting', '正在等待下一张 VRChat 截图...'));
    const path = await GalleryApi.waitForNewImage({ timeoutSeconds: 300 });
    await translatePhotoPath(path);
  } catch (error) {
    errorMsg.value = `${tt('translator.photo_wait_failed', '等待截图失败')}: ${errorText(error)}`;
  } finally {
    photoArmed.value = false;
  }
};

const sendToChatbox = async (text: string) => {
  if (!text.trim()) return;
  try {
    await SysApi.sendOscChatbox({ text, complete: true });
    setStatus(t('translator.sent_to_vrchat_chatbox'));
  } catch (error) {
    errorMsg.value = tt('translator.osc_error', 'Unable to send to VRChat: {err}').replace('{err}', errorText(error));
  }
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

  const route: TranslationRoute = source === 'chat' ? 'manual' : source === 'mic' ? 'mic' : 'speaker';
  const selectedConfig = routeConfig(route);
  const selectedEngine = engineOptions.value.find((engine) => engine.value === selectedConfig.engine) ?? engineOptions.value[0];
  if (selectedEngine.needsKey && !selectedEngine.supportsLocal && !selectedConfig.apiKey.trim()) {
    errorMsg.value = `${selectedEngine.label}: ${selectedEngine.hint}`;
    return null;
  }

  errorMsg.value = '';
  try {
    const record = await VrctApi.processMessage({
      req: {
        text: trimmed,
        source,
        source_lang: sourceLanguage,
        target_lang: targetLanguage,
        target_langs: multiLangEnabled.value
          ? Array.from(
              new Set(
                multiLangTargets.value.filter(
                  (l) => l !== sourceLanguage && l !== targetLanguage,
                ),
              ),
            )
          : [],
        service: selectedConfig.engine,
        api_key: selectedConfig.apiKey.trim(),
        model: selectedConfig.model.trim(),
        prompt: selectedConfig.prompt.trim(),
        custom_api_url: selectedConfig.customApiUrl.trim(),
        send_osc: sendOsc,
        send_typing: sendOsc,
        complete: true,
        notification: false,
        update_overlay: true,
        show_original_in_osc: showOriginalOsc.value,
        glossary: normalizedGlossary.value,
        context: contextEnabled.value ? history.value.slice(0, 6).map((item) => item.original) : [],
        retry_count: Math.min(3, Math.max(0, Number(retryCount.value) || 0)),
      },
    }) as VrctMessageRecord;

    applyRecord(record);
    await notifyOverlay(record);

    if (autoPlayTts.value && record.translated) {
      await playTts(record.translated, targetLanguage);
    }

    return record;
  } catch (error) {
    errorMsg.value = `${tt('translator.network_error', 'Translation request failed. Please check your network and API settings')} ${errorText(error)}`;
    return null;
  }
};

const addGlossaryTerm = () => {
  glossary.value = [...glossary.value, {
    source: '',
    target: '',
    source_lang: 'any',
    target_lang: 'any',
    case_sensitive: false,
  }];
};

const removeGlossaryTerm = (index: number) => {
  glossary.value = glossary.value.filter((_, termIndex) => termIndex !== index);
};

const queueMessage = (
  text: string,
  source: MessageSource,
  sourceLanguage: string,
  targetLanguage: string,
  sendOsc: boolean,
) => translationQueue.enqueue(() => processMessageNow(text, source, sourceLanguage, targetLanguage, sendOsc));

const translateManual = async () => {
  const record = await queueMessage(manualText.value, 'chat', sourceLang.value, targetLang.value, autoSendOsc.value);
  if (record) manualText.value = '';
};

const translateMicText = (text: string) => queueMessage(text, 'mic', sourceLang.value, targetLang.value, autoSendOsc.value);

const translateSpeakerText = (text: string) => queueMessage(text, 'speaker', otherSourceLang.value, otherTargetLang.value, false);

const triggerKeywordActions = (text: string) => {
  if (!text.trim() || !keywordActions.value.some((action) => action.enabled && action.keyword.trim())) return;
  TranslationApis.KeywordActionApi?.trigger?.({ text, actions: keywordActions.value }).then((result) => {
    if (result.matched?.length) setStatus(`${tt('translator.keyword_triggered', '已触发动作')}: ${result.matched.join(', ')}`);
  }).catch((error) => {
    errorMsg.value = `${tt('translator.keyword_failed', '关键词动作失败')}: ${errorText(error)}`;
  });
};

const swapMyLanguages = () => {
  if (sourceLang.value === 'auto') return;
  [sourceLang.value, targetLang.value] = [targetLang.value, sourceLang.value];
};

const toggleMultiLangTarget = (lang: string) => {
  const idx = multiLangTargets.value.indexOf(lang);
  if (idx >= 0) {
    multiLangTargets.value = multiLangTargets.value.filter(l => l !== lang);
  } else {
    multiLangTargets.value = [...multiLangTargets.value, lang];
  }
};

const swapOtherLanguages = () => {
  [otherSourceLang.value, otherTargetLang.value] = [otherTargetLang.value, otherSourceLang.value];
};

const selectedDeviceIndex = (source: AudioSource) => {
  const id = source === 'mic' ? micDeviceId.value : speakerDeviceId.value;
  return audioDevices.value.find(device => device.id === id)?.index;
};

const startCapture = async (source: AudioSource) => {
  errorMsg.value = '';
  const isMic = source === 'mic';
  if (isMic) isMicStarting.value = true;
  else isSpeakerStarting.value = true;
  try {
    await SysApi.startAudioCapture({
      source,
      sourceLang: isMic ? sourceLang.value : otherSourceLang.value,
      engine: (isMic ? micEngine.value : otherEngine.value) as 'cloud' | 'local' | 'whisper' | 'sensevoice' | 'sherpa' | 'tencent_realtime' | 'aliyun_realtime',
      realtimeProvider: realtimeAsrConfig.value.provider,
      realtimeConfig: realtimeAsrConfig.value,
      sherpaConfig: sherpaConfig.value,
      deviceIndex: selectedDeviceIndex(source),
      energyThreshold: isMic ? Number(micEnergyThreshold.value) : Number(speakerEnergyThreshold.value),
      dynamicEnergyThreshold: true,
      phraseTimeLimit: Number(phraseTimeLimit.value),
      whisperModel: whisperModel.value,
      vadType: vadType.value,
      vadAggressiveness: Number(vadAggressiveness.value),
      denoiseStrength: Number(denoiseStrength.value),
      correctionEnabled: Boolean(correctionEnabled.value),
      minSegmentS: Number(minSegmentS.value),
      maxSegmentS: Number(maxSegmentS.value),
      partialInterval: Number(partialInterval.value),
      captureMode: captureMode.value,
      targetProcess: targetProcess.value,
      selfSuppressSeconds: Number(selfSuppressSeconds.value),
    });
    setStatus(t('translator.starting_audio_recognition_service'));
  } catch (error) {
    errorMsg.value = tt('translator.capture_error_cloud', 'Audio capture failed: {err}').replace('{err}', errorText(error));
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
  } else {
    isOtherRecording.value = false;
    isSpeakerStarting.value = false;
    speakerSuppressedBySelf = false;
    if (selfSuppressTimer) {
      clearTimeout(selfSuppressTimer);
      selfSuppressTimer = null;
    }
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

const manualSend = () => {
  const multilingual = currentTranslations.value.length > 1
    ? currentTranslations.value.map(item => `[${item.target_lang}] ${item.translated}`).join(' | ')
    : translatedText.value;
  const payload = showOriginalOsc.value && recognizedText.value
    ? `${multilingual} (${recognizedText.value})`
    : multilingual;
  sendToChatbox(payload);
};

const manualPlay = () => playTts(translatedText.value);

const useHistory = (record: VrctMessageRecord) => {
  recognizedText.value = record.original;
  translatedText.value = record.translated;
  currentTranslations.value = record.translations?.length
    ? record.translations
    : [{ target_lang: record.target_lang, translated: record.translated }];
  lastTargetLang.value = record.target_lang;
};

const clearHistory = async () => {
  try {
    await VrctApi.clearHistory();
  } finally {
    history.value = [];
  }
};

const toggleOverlay = async () => {
  if (!isTauri()) {
    setStatus(t('translator.the_tauri_overlay_is_unavailable_in_brow'));
    return;
  }

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
      width: 420,
      height: 320,
      x: 50,
      y: 50,
      skipTaskbar: true,
    });

    overlayWebview.once('tauri://created', () => {
      isOverlayOpen.value = true;
      syncOverlaySettings().catch(() => undefined);
    });
    overlayWebview.once('tauri://error', (event) => {
      errorMsg.value = tt('translator.overlay_fail', 'Overlay creation failed: {error}').replace('{error}', JSON.stringify(event));
      isOverlayOpen.value = false;
      overlayWebview = null;
    });
    overlayWebview.onCloseRequested(() => {
      isOverlayOpen.value = false;
      overlayWebview = null;
    });
  } catch (error) {
    errorMsg.value = tt('translator.exception', 'Unexpected error: {error}').replace('{error}', errorText(error));
    isOverlayOpen.value = false;
  }
};

const loadAudioDevices = async () => {
  if (!isTauri()) return;
  audioDeviceError.value = '';
  try {
    audioDevices.value = await SysApi.getAudioDevices();
    const selectDefault = (source: AudioSource, current: string) => {
      const devices = audioDevices.value.filter(device => device.source === source);
      if (devices.some(device => device.id === current)) return current;
      return devices.find(device => device.is_default)?.id ?? devices[0]?.id ?? '';
    };
    micDeviceId.value = selectDefault('mic', micDeviceId.value);
    speakerDeviceId.value = selectDefault('speaker', speakerDeviceId.value);
  } catch (error) {
    audioDeviceError.value = errorText(error);
  }
};

// 竞态防护：onMounted 里有多个 await，组件可能在监听器注册前被卸载
let translatorDisposed = false;

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
          if (source === 'mic') {
            isRecording.value = false;
            isMicStarting.value = false;
          } else {
            isOtherRecording.value = false;
            isSpeakerStarting.value = false;
          }
        }
        return;
      }
      if (payload.type === 'status') {
        if (payload.message === 'audio_level') updateAudioLevel(source, payload.level);
        if (payload.message === 'starting') setStatus(`${t('translator.listening_device')}: ${payload.device || 'Default'}`);
        if (payload.message === 'loading_model') setStatus(t('translator.loading_local_whisper_model'));
        if (payload.message === 'recognizing') setStatus(source === 'mic'
          ?t('translator.recognizing_microphone_audio')
          :t('translator.recognizing_game_audio'));
        if (payload.message === 'backlog_trimmed') setStatus(t('translator.audio_backlog_trimmed'));
        if (payload.message === 'listening') {
          if (source === 'mic') {
            isRecording.value = true;
            isMicStarting.value = false;
          } else {
            isOtherRecording.value = true;
            isSpeakerStarting.value = false;
          }
        }
        if (payload.message === 'stopped') {
          if (source === 'mic') {
            isRecording.value = false;
            isMicStarting.value = false;
          } else {
            isOtherRecording.value = false;
            isSpeakerStarting.value = false;
          }
          if (!payload.expected && payload.exit_code !== 0) {
            errorMsg.value = t('translator.audio_recognition_service_stopped_unexpe');
          }
        }

        // 自声抑制：自己用麦克风说话时暂停"听别人"捕获，避免回声重复转写
        if (source === 'mic' && selfSuppress.value && isOtherRecording.value) {
          if (payload.message === 'recording' && !speakerSuppressedBySelf) {
            speakerSuppressedBySelf = true;
            if (selfSuppressTimer) {
              clearTimeout(selfSuppressTimer);
              selfSuppressTimer = null;
            }
            await SysApi.setAudioCapturePaused({ source: 'speaker', paused: true }).catch(() => undefined);
          } else if (payload.message === 'listening' && speakerSuppressedBySelf) {
            if (selfSuppressTimer) clearTimeout(selfSuppressTimer);
            const tail = Math.max(0, Number(selfSuppressSeconds.value) || 0) * 1000;
            selfSuppressTimer = setTimeout(() => {
              speakerSuppressedBySelf = false;
              selfSuppressTimer = null;
              if (isOtherRecording.value && !isSpeakerStarting.value) {
                SysApi.setAudioCapturePaused({ source: 'speaker', paused: false }).catch(() => undefined);
              }
            }, tail);
          }
        }
        return;
      }
      if (payload.type === 'partial' && payload.text?.trim()) {
        // 实时预览识别中的文本（不触发翻译，最终结果覆盖并触发翻译）
        recognizedText.value = payload.text.trim();
        return;
      }
      if (payload.type === 'result' && payload.text?.trim()) {
        recognizedText.value = payload.text.trim();
        triggerKeywordActions(payload.text);
        if (source === 'mic') await translateMicText(payload.text);
        else await translateSpeakerText(payload.text);
      }
    });

    unlistenVrct = await listen('vrct_translation_event', (event: any) => {
      addHistory(event.payload as VrctMessageRecord);
    });

    unlistenTranslationHotkey = await listen('translation-hotkey', (event: any) => {
      const id = Number(event.payload?.id);
      if (id === 4101) {
        document.querySelector<HTMLTextAreaElement>('textarea')?.focus();
      } else if (id === 4102) {
        toggleRecording().catch(() => undefined);
      }
    });

    await refreshModelStatus();
    await loadTtsPresets();
    await checkTranslationHotkeys();
    let runtime: any = null;
    try { runtime = await TranslationApis.TranslationRuntimeApi?.get?.().catch(() => null); } catch { runtime = null; }
    if (runtime?.version) runtimeVersion.value = runtime.version;

    if (translatorDisposed) {
      // await 期间组件已被卸载：立即注销刚拿到的监听器，防止泄漏
      unlistenAudio?.();
      unlistenVrct?.();
      unlistenTranslationHotkey?.();
      unlistenAudio = null;
      unlistenVrct = null;
      unlistenTranslationHotkey = null;
    }
  }

  try {
    const existing = await VrctApi.getHistory();
    if (Array.isArray(existing)) {
      history.value = [...existing].reverse();
    }
  } catch {
    history.value = [];
  }
});

onUnmounted(async () => {
  translatorDisposed = true;
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
  <div class="h-full min-h-0 flex flex-col p-6 bg-surface-hover rounded-3xl relative overflow-hidden">
    <header class="flex items-center justify-between gap-4 mb-5 shrink-0 z-10 flex-wrap">
      <h2 class="text-3xl font-extrabold text-text flex items-center gap-3 tracking-tight min-w-0">
        <span class="inline-flex items-center justify-center p-2 bg-primary/10 rounded-2xl shadow-sm border-primary shrink-0">
          <Languages class="w-6 h-6 text-primary" />
        </span>
        <span class="truncate">{{ tt('translator.title', '翻译官') }}</span>
      </h2>
      <div class="flex items-center justify-end gap-3 flex-wrap">
        <label class="min-w-[220px] flex items-center gap-3 px-3 py-2 bg-surface border border-border-soft rounded-xl shadow-sm">
          <span class="text-xs font-bold text-text-muted whitespace-nowrap">
            {{ tt('translator.overlay_opacity', '背景不透明度') }}
          </span>
          <input
            v-model.number="overlayBackgroundOpacity"
            type="range"
            min="0"
            max="1"
            step="0.05"
            class="overlay-opacity-slider min-w-0 flex-1"
          >
          <output class="w-10 text-right text-xs font-extrabold text-primary tabular-nums">
            {{ Math.round(overlayBackgroundOpacity * 100) }}%
          </output>
        </label>
        <button
          :class="isOverlayOpen ? 'bg-red-500 hover:bg-red-600 shadow-red-500/30 border-red-500 text-white' : 'bg-surface hover:bg-surface-hover text-text-muted hover:text-primary border-border-soft shadow-sm'"
          class="px-4 py-2.5 font-bold rounded-xl flex items-center gap-2 transition-all active:scale-95 text-sm shrink-0"
          @click="toggleOverlay"
        >
          <MonitorUp :size="16" />
          <span>{{ isOverlayOpen ? tt('translator.overlay_close', '关闭悬浮窗') : tt('translator.overlay_open', '开启悬浮窗') }}</span>
        </button>
      </div>
    </header>

    <section class="bg-surface backdrop-blur-xl rounded-2xl p-4 border-border-strong shadow-sm mb-5">
      <div class="grid grid-cols-1 xl:grid-cols-[1.2fr_1fr] gap-4">
        <div class="min-w-0">
          <div class="flex items-center justify-between gap-3 mb-2">
            <div>
              <p class="text-xs font-black text-text">{{ tt('translator.profile_title', '翻译服务档案') }}</p>
              <p class="text-[10px] text-text-muted">{{ tt('translator.profile_desc', '保存 API、模型和提示词，一键切换麦克风、游戏语音与手动输入使用的翻译方案。') }}</p>
            </div>
            <button class="px-2.5 py-1.5 rounded-lg bg-primary text-white text-xs font-extrabold hover:brightness-110" @click="saveTranslationProfile">{{ tt('translator.profile_save', '保存当前') }}</button>
          </div>
          <div class="flex flex-wrap gap-2">
            <button v-for="profile in translationProfiles" :key="profile.id" class="group inline-flex items-center gap-2 px-3 py-2 rounded-xl border text-xs font-bold transition-colors" :class="activeProfileId === profile.id ? 'border-primary bg-primary/10 text-primary' : 'border-border-soft bg-surface-hover text-text-muted hover:border-primary'" @click="applyTranslationProfile(profile)">
              <span>{{ profile.name }}</span>
              <span class="hidden group-hover:inline text-red-500" @click.stop="deleteTranslationProfile(profile.id)">×</span>
            </button>
            <span v-if="!translationProfiles.length" class="text-[11px] text-text-muted py-2">{{ tt('translator.profile_empty', '还没有保存的翻译档案。') }}</span>
          </div>
          <div class="grid grid-cols-2 md:grid-cols-4 gap-2 mt-3">
            <label v-for="route in ([['manual', '手动输入'], ['mic', '麦克风'], ['speaker', '游戏语音'], ['photo', '拍照翻译']] as const)" :key="route[0]" class="min-w-0">
              <span class="block text-[10px] font-extrabold text-text-muted mb-1">{{ tt(`translator.route_${route[0]}`, route[1]) }}</span>
              <CustomSelect :model-value="routeProfileIds[route[0]]" :options="profileOptions" @update:model-value="setRouteProfile(route[0], $event)" />
            </label>
          </div>
        </div>
        <div class="grid grid-cols-1 md:grid-cols-3 gap-3">
          <label class="min-w-0">
            <span class="block text-[11px] font-extrabold text-text-muted uppercase mb-1.5">{{ tt('translator.tts_rate', 'TTS 语速') }} · {{ Number(ttsRate).toFixed(1) }}</span>
            <input v-model.number="ttsRate" type="range" min="0.5" max="2" step="0.1" class="w-full accent-primary">
          </label>
          <label class="min-w-0">
            <span class="block text-[11px] font-extrabold text-text-muted uppercase mb-1.5">{{ tt('translator.tts_volume', 'TTS 音量') }} · {{ Math.round(Number(ttsVolume) * 100) }}%</span>
            <input v-model.number="ttsVolume" type="range" min="0" max="1" step="0.05" class="w-full accent-primary">
          </label>
          <label class="flex items-center gap-2 px-3 py-2 bg-surface-hover rounded-xl border border-border-soft cursor-pointer text-xs font-bold text-text-muted">
            <input v-model="interruptTts" type="checkbox" class="w-4 h-4 text-primary rounded">
            {{ tt('translator.tts_interrupt', '新播报打断旧播报') }}
          </label>
        </div>
      </div>
    </section>

    <div class="flex-1 min-h-0 overflow-y-auto custom-scrollbar pr-2 z-10 relative">
      <section class="bg-surface backdrop-blur-xl rounded-2xl p-4 border-border-strong shadow-sm mb-5">
        <div class="grid grid-cols-1 xl:grid-cols-[1.4fr_1fr] gap-4">
          <div class="grid grid-cols-1 md:grid-cols-3 gap-3">
            <label class="flex items-center gap-2 cursor-pointer bg-surface-hover rounded-xl px-3 py-2 border-border-soft min-w-0">
              <input v-model="autoSendOsc" type="checkbox" class="w-4 h-4 text-primary rounded focus:ring-indigo-500 border-border-soft shrink-0">
              <span class="text-sm font-bold text-text-muted truncate">{{ tt('translator.auto_osc', '自动发送到 VRChat Chatbox') }}</span>
            </label>
            <label class="flex items-center gap-2 cursor-pointer bg-surface-hover rounded-xl px-3 py-2 border-border-soft min-w-0">
              <input v-model="showOriginalOsc" type="checkbox" class="w-4 h-4 text-primary rounded focus:ring-indigo-500 border-border-soft shrink-0">
              <span class="text-sm font-bold text-text-muted truncate">{{ tt('translator.show_original', '附加原文') }}</span>
            </label>
            <label class="flex items-center gap-2 cursor-pointer bg-surface-hover rounded-xl px-3 py-2 border-border-soft min-w-0">
              <input v-model="autoPlayTts" type="checkbox" class="w-4 h-4 text-primary rounded focus:ring-indigo-500 border-border-soft shrink-0">
              <span class="text-sm font-bold text-text-muted truncate">{{ tt('translator.auto_tts', '自动语音播报') }}</span>
            </label>
            <label class="flex items-center gap-2 cursor-pointer bg-surface-hover rounded-xl px-3 py-2 border-border-soft min-w-0">
              <input v-model="multiLangEnabled" type="checkbox" class="w-4 h-4 text-primary rounded focus:ring-indigo-500 border-border-soft shrink-0">
              <span class="text-sm font-bold text-text-muted truncate">{{t('translator.translate_to_multiple_languages') }}</span>
            </label>
          </div>

          <!-- Multi-language target selector -->
          <div v-if="multiLangEnabled" class="md:col-span-2 mt-2 p-3 bg-surface-hover rounded-xl border border-border-soft">
            <label class="block text-[11px] font-extrabold text-text-muted uppercase mb-2">{{t('translator.additional_target_languages_multi_langua') }}</label>
            <div class="flex flex-wrap gap-2">
              <label
                v-for="opt in languageOptions.filter(o => o.value !== 'auto' && o.value !== targetLang && o.value !== sourceLang)"
                :key="opt.value"
                class="flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg bg-surface border border-border-soft cursor-pointer hover:border-primary transition-all text-xs font-bold"
                :class="multiLangTargets.includes(opt.value) ? 'border-primary text-primary bg-primary/5' : 'text-text-muted'"
              >
                <input
                  :checked="multiLangTargets.includes(opt.value)"
                  type="checkbox"
                  class="w-3 h-3 text-primary rounded shrink-0"
                  @change="toggleMultiLangTarget(opt.value)"
                >
                {{ opt.label }}
              </label>
            </div>
            <p class="text-[10px] text-text-muted mt-2">{{t('translator.translations_are_sent_to_vrchat_chatbox_') }}</p>
          </div>

          <div class="md:col-span-2 grid grid-cols-1 xl:grid-cols-[1fr_auto] gap-3 p-3 bg-surface-hover rounded-xl border border-border-soft">
            <div class="min-w-0">
              <div class="flex items-center justify-between gap-3 mb-2">
                <div>
                  <p class="text-xs font-black text-text">{{ tt('translator.glossary_title', '术语库与专名保护') }}</p>
                  <p class="text-[10px] text-text-muted">{{ tt('translator.glossary_desc', '优先保护 VRChat 世界、用户、品牌和自定义词汇，避免机器翻译改名。') }}</p>
                </div>
                <button class="shrink-0 inline-flex items-center gap-1 px-2.5 py-1.5 rounded-lg bg-surface border border-border-soft text-xs font-extrabold text-primary hover:border-primary transition-colors" @click="addGlossaryTerm">
                  <Plus :size="13" /> {{ tt('translator.glossary_add', '添加术语') }}
                </button>
              </div>
              <div v-if="glossary.length" class="space-y-2 max-h-32 overflow-y-auto custom-scrollbar pr-1">
                <div v-for="(term, index) in glossary" :key="index" class="grid grid-cols-[1fr_1fr_auto] gap-2 items-center">
                  <input v-model="term.source" type="text" class="min-w-0 px-2.5 py-1.5 bg-surface border border-border-soft rounded-lg text-xs font-bold text-text outline-none focus:border-primary" :placeholder="tt('translator.glossary_source', '原文术语')">
                  <input v-model="term.target" type="text" class="min-w-0 px-2.5 py-1.5 bg-surface border border-border-soft rounded-lg text-xs font-bold text-text outline-none focus:border-primary" :placeholder="tt('translator.glossary_target', '固定译法')">
                  <button class="w-8 h-8 inline-flex items-center justify-center rounded-lg text-text-muted hover:text-red-500 hover:bg-red-500/10" :title="tt('translator.glossary_remove', '删除术语')" @click="removeGlossaryTerm(index)"><Trash2 :size="14" /></button>
                </div>
              </div>
              <p v-else class="text-[11px] text-text-muted py-1">{{ tt('translator.glossary_empty', '尚未添加术语；翻译会自动保持 URL、表情和换行。') }}</p>
            </div>
            <div class="flex xl:flex-col gap-2 xl:justify-center">
              <label class="flex items-center gap-2 px-2.5 py-1.5 rounded-lg bg-surface border border-border-soft cursor-pointer text-[11px] font-bold text-text-muted whitespace-nowrap">
                <input v-model="contextEnabled" type="checkbox" class="w-3.5 h-3.5 text-primary rounded">
                {{ tt('translator.context_enabled', '启用最近对话上下文') }}
              </label>
              <label class="flex items-center gap-2 px-2.5 py-1.5 rounded-lg bg-surface border border-border-soft text-[11px] font-bold text-text-muted whitespace-nowrap">
                <span>{{ tt('translator.retry_count', '失败重试') }}</span>
                <input v-model.number="retryCount" type="number" min="0" max="3" class="w-10 bg-transparent text-center text-text outline-none">
              </label>
            </div>
          </div>

          <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
            <div class="min-w-0">
              <label class="block text-[11px] font-extrabold text-text-muted uppercase mb-1.5 flex items-center gap-1.5">
                <Settings :size="12" /> {{ tt('translator.tts_engine_label', 'TTS 播报引擎') }}
              </label>
              <CustomSelect
                v-model="ttsEngine"
                :options="[
                  { label: tt('translator.tts_system', '系统原生 (Web Speech)'), value: 'system' },
                  { label: tt('translator.tts_edge', 'Edge-TTS'), value: 'edge' },
                  { label: tt('translator.tts_gptsovits', 'GPT-SoVITS API'), value: 'gpt_sovits' },
                  { label: tt('translator.tts_server', 'VRCLS Server TTS'), value: 'server' }
                ]"
              />
            </div>
            <div v-if="ttsEngine === 'server'" class="md:col-span-2 grid grid-cols-1 md:grid-cols-2 gap-3">
              <div><label class="block text-[11px] font-extrabold text-text-muted uppercase mb-1.5">{{ tt('translator.tts_provider', 'Provider') }}</label><CustomSelect v-model="serverTtsProvider" :options="[{ label: 'Edge-TTS', value: 'edge' }, { label: 'Qwen-TTS', value: 'qwen' }, { label: 'MOSS-TTS', value: 'moss' }, { label: 'OmniVoice', value: 'omnivoice' }]" /></div>
              <div><label class="block text-[11px] font-extrabold text-text-muted uppercase mb-1.5">{{ tt('translator.tts_voice', 'Voice') }}</label><input v-model="serverTtsVoice" class="w-full px-3 py-2 bg-surface-hover border border-border-soft rounded-xl text-sm font-bold text-text outline-none" placeholder="default" /></div>
              <div class="md:col-span-2"><label class="block text-[11px] font-extrabold text-text-muted uppercase mb-1.5">{{ tt('translator.tts_base_url', 'Server TTS URL') }}</label><input v-model="serverTtsBaseUrl" class="w-full px-3 py-2 bg-surface-hover border border-border-soft rounded-xl text-sm font-bold text-text outline-none" placeholder="https://your-server.example/api" /></div>
              <div class="md:col-span-2"><label class="block text-[11px] font-extrabold text-text-muted uppercase mb-1.5">{{ tt('translator.tts_clone_reference', '声音克隆参考音频') }}</label><input v-model="gptReferenceAudio" class="w-full px-3 py-2 bg-surface-hover border border-border-soft rounded-xl text-sm font-bold text-text outline-none" placeholder="D:\voices\reference.wav" /></div>
              <div class="md:col-span-2"><label class="block text-[11px] font-extrabold text-text-muted uppercase mb-1.5">{{ tt('translator.tts_clone_text', '参考音频文本 / Qwen 指令') }}</label><input v-model="ttsReferenceText" class="w-full px-3 py-2 bg-surface-hover border border-border-soft rounded-xl text-sm font-bold text-text outline-none" /></div>
              <div class="md:col-span-2 flex items-center gap-2">
                <CustomSelect v-model="activeTtsPresetId" :options="[{ label: tt('translator.tts_preset_default', '选择声音预设'), value: '' }, ...ttsPresets.map((preset) => ({ label: `${preset.name} · ${preset.provider}`, value: preset.id }))]" @update:model-value="applyTtsPreset" />
                <button class="px-2.5 py-2 rounded-xl bg-primary text-white text-xs font-extrabold whitespace-nowrap" @click="saveTtsPreset">{{ tt('translator.tts_preset_save', '保存预设') }}</button>
                <button class="px-2.5 py-2 rounded-xl bg-surface-hover border border-border-soft text-text-muted text-xs font-extrabold whitespace-nowrap" :disabled="!activeTtsPresetId" @click="deleteTtsPreset">{{ tt('translator.tts_preset_delete', '删除') }}</button>
                <button class="px-2.5 py-2 rounded-xl bg-surface-hover border border-border-soft text-text-muted text-xs font-extrabold whitespace-nowrap" @click="importTtsPresets">{{ tt('translator.tts_preset_import', '导入') }}</button>
                <button class="px-2.5 py-2 rounded-xl bg-surface-hover border border-border-soft text-text-muted text-xs font-extrabold whitespace-nowrap" @click="exportTtsPresets">{{ tt('translator.tts_preset_export', '导出') }}</button>
              </div>
            </div>
            <div v-if="ttsEngine === 'gpt_sovits'" class="md:col-span-2 grid grid-cols-1 md:grid-cols-2 gap-3">
              <div class="min-w-0">
                <label class="block text-[11px] font-extrabold text-text-muted uppercase mb-1.5">{{ tt('translator.gptsovits_url_label', 'GPT-SoVITS API URL') }}</label>
                <input
                  v-model="gptSovitsUrl"
                  type="text"
                  class="w-full px-3 py-2 bg-surface-hover border-border-soft rounded-xl text-sm font-bold text-text outline-none focus:ring-4 focus:ring-indigo-500/10 transition-all"
                  :placeholder="tt('translator.gptsovits_url_placeholder', 'http://127.0.0.1:9880')"
                >
              </div>
              <div class="min-w-0">
                <label class="block text-[11px] font-extrabold text-text-muted uppercase mb-1.5">{{ tt('translator.sovits_weights_label', 'SoVITS weights (.pth)') }}</label>
                <input v-model="gptSovitsWeights" type="text" class="w-full px-3 py-2 bg-surface-hover border-border-soft rounded-xl text-sm font-bold text-text outline-none focus:ring-4 focus:ring-indigo-500/10 transition-all" :placeholder="tt('translator.sovits_weights_placeholder', 'D:\\models\\voice.pth')">
              </div>
              <div class="min-w-0">
                <label class="block text-[11px] font-extrabold text-text-muted uppercase mb-1.5">{{ tt('translator.gpt_weights_label', 'GPT weights (.ckpt)') }}</label>
                <input v-model="gptWeights" type="text" class="w-full px-3 py-2 bg-surface-hover border-border-soft rounded-xl text-sm font-bold text-text outline-none focus:ring-4 focus:ring-indigo-500/10 transition-all" :placeholder="tt('translator.gpt_weights_placeholder', 'D:\\models\\voice.ckpt')">
              </div>
              <div class="min-w-0">
                <label class="block text-[11px] font-extrabold text-text-muted uppercase mb-1.5">{{ tt('translator.reference_audio_label', 'Reference audio') }}</label>
                <input v-model="gptReferenceAudio" type="text" class="w-full px-3 py-2 bg-surface-hover border-border-soft rounded-xl text-sm font-bold text-text outline-none focus:ring-4 focus:ring-indigo-500/10 transition-all" :placeholder="tt('translator.reference_audio_placeholder', 'D:\\voices\\reference.wav')">
              </div>
              <div class="min-w-0">
                <label class="block text-[11px] font-extrabold text-text-muted uppercase mb-1.5">{{ tt('translator.prompt_text_label', 'Reference transcript') }}</label>
                <input v-model="gptPromptText" type="text" class="w-full px-3 py-2 bg-surface-hover border-border-soft rounded-xl text-sm font-bold text-text outline-none focus:ring-4 focus:ring-indigo-500/10 transition-all">
              </div>
              <div class="min-w-0">
                <label class="block text-[11px] font-extrabold text-text-muted uppercase mb-1.5">{{ tt('translator.prompt_language_label', 'Reference language') }}</label>
                <CustomSelect
                  v-model="gptPromptLanguage"
                  :options="[
                    { label: tt('translator.language_chinese', 'Chinese'), value: 'zh' },
                    { label: tt('translator.language_english', 'English'), value: 'en' },
                    { label: tt('translator.language_japanese', 'Japanese'), value: 'ja' },
                    { label: tt('translator.language_korean', 'Korean'), value: 'ko' }
                  ]"
                />
              </div>
            </div>
          </div>
        </div>
      </section>

      <div v-if="errorMsg || audioDeviceError || statusMsg" class="mb-5 shrink-0">
        <div v-if="errorMsg" class="bg-red-50 border-red-200 text-red-600 px-4 py-3 rounded-xl text-sm font-bold shadow-sm">
          {{ errorMsg }}
        </div>
        <div v-else-if="audioDeviceError" class="bg-red-50 border-red-200 text-red-600 px-4 py-3 rounded-xl text-sm font-bold shadow-sm flex items-center justify-between gap-3">
          <span class="min-w-0 break-words">{{ audioDeviceError }}</span>
          <button class="w-8 h-8 shrink-0 inline-flex items-center justify-center rounded-lg bg-white/70" :title="t('translator.refresh_audio_devices')" @click="loadAudioDevices">
            <RefreshCw :size="14" />
          </button>
        </div>
        <div v-else class="bg-emerald-50 border-emerald-200 text-emerald-700 px-4 py-3 rounded-xl text-sm font-bold flex items-center gap-2 shadow-sm">
          <CheckCircle2 :size="16" /> {{ statusMsg }}
        </div>
      </div>

      <div class="grid grid-cols-1 xl:grid-cols-[minmax(0,1fr)_390px] gap-5 min-h-0">
        <main class="grid grid-cols-1 lg:grid-cols-2 gap-5 min-w-0">
          <section class="bg-surface backdrop-blur-md rounded-2xl p-5 border-border-soft shadow-sm min-w-0">
            <div class="flex items-center justify-between gap-3 mb-4">
              <h3 class="font-extrabold text-text flex items-center gap-2 text-lg min-w-0">
                <Mic class="text-primary shrink-0" :size="20" />
                <span class="truncate">{{ tt('translator.my_voice', '我说话 (发送到 VRC)') }}</span>
              </h3>
              <button
                data-testid="start-microphone"
                :class="isRecording ? 'bg-red-500 hover:bg-red-600 text-white shadow-red-500/30 border-red-500' : 'bg-surface-hover hover:bg-primary/10 text-text-muted hover:text-primary border-border-soft'"
                class="px-3 py-2 rounded-xl font-extrabold text-xs flex items-center gap-2 transition-all active:scale-95 shrink-0"
                @click="toggleRecording"
              >
                <component :is="isMicStarting ? RefreshCw : isRecording ? MicOff : Mic" :class="{ 'animate-spin': isMicStarting }" :size="16" />
                <span>{{ isRecording || isMicStarting ? tt('translator.stop_listen', '停止收音') : tt('translator.start_listen', '开始麦克风监听') }}</span>
              </button>
            </div>

            <div class="space-y-4">
              <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
                <div class="min-w-0">
                  <label class="block text-[11px] font-extrabold text-text-muted uppercase mb-1.5">{{t('translator.microphone_device') }}</label>
                  <CustomSelect v-model="micDeviceId" :options="micDeviceOptions" />
                </div>
                <div class="min-w-0">
                  <label class="block text-[11px] font-extrabold text-text-muted uppercase mb-1.5">{{ tt('translator.stt_engine_label', 'STT Engine') }}</label>
                  <CustomSelect v-model="micEngine" :options="speakerEngineOptions" />
                </div>
              </div>

              <div>
                <label class="block text-[11px] font-extrabold text-text-muted uppercase mb-1.5">{{ tt('translator.engine',t('translator.translation_engine')) }}</label>
                <CustomSelect v-model="translateEngine" :options="engineOptions" />
                <p class="mt-1.5 text-[11px] text-text-muted font-semibold truncate">{{ currentEngine.hint }}</p>
              </div>

              <div class="grid grid-cols-[1fr_auto_1fr] gap-3 items-end">
                <div class="min-w-0">
                  <label class="block text-[11px] font-extrabold text-text-muted uppercase mb-1.5">{{ tt('translator.source_lang', '我的语言 (Source)') }}</label>
                  <CustomSelect v-model="sourceLang" :options="speechLanguageOptions" />
                </div>
                <button class="mb-0.5 w-9 h-9 rounded-xl bg-surface-hover border-border-soft flex items-center justify-center text-text-muted hover:text-primary transition-colors" @click="swapMyLanguages">
                  <RefreshCw :size="16" />
                </button>
                <div class="min-w-0">
                  <label class="block text-[11px] font-extrabold text-text-muted uppercase mb-1.5">{{ tt('translator.target_lang', '翻译为 (Target)') }}</label>
                  <CustomSelect v-model="targetLang" :options="languageOptions.filter((option) => option.value !== 'auto')" />
                </div>
              </div>

              <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
                <div v-if="needsApiKey || apiKey" class="min-w-0">
                  <label class="block text-[11px] font-extrabold text-text-muted uppercase mb-1.5">{{ tt('translator.api_key_label', 'API Key') }}</label>
                  <input v-model="apiKey" type="password" class="w-full px-3 py-2 bg-surface-hover border-border-soft rounded-xl text-sm font-bold text-text outline-none focus:ring-4 focus:ring-indigo-500/10">
                </div>
                <div v-if="showModelField" class="min-w-0">
                  <label class="block text-[11px] font-extrabold text-text-muted uppercase mb-1.5">{{ tt('translator.model_label', 'Model') }}</label>
                  <input v-model="model" type="text" class="w-full px-3 py-2 bg-surface-hover border-border-soft rounded-xl text-sm font-bold text-text outline-none focus:ring-4 focus:ring-indigo-500/10" placeholder="default">
                </div>
                <div v-if="translateEngine === 'custom_llm'" class="md:col-span-2 min-w-0">
                  <label class="block text-[11px] font-extrabold text-text-muted uppercase mb-1.5">{{ tt('translator.custom_api_url_label', 'Custom API URL') }}</label>
                  <input v-model="customApiUrl" type="text" class="w-full px-3 py-2 bg-surface-hover border-border-soft rounded-xl text-sm font-bold text-text outline-none focus:ring-4 focus:ring-indigo-500/10" placeholder="https://example.com/v1/chat/completions">
                </div>
              </div>

              <div>
                <label class="block text-[11px] font-extrabold text-text-muted uppercase mb-1.5">{{ tt('translator.prompt_label', 'Prompt') }}</label>
                <textarea
                  v-model="prompt"
                  rows="2"
                  class="w-full resize-none px-3 py-2 bg-surface-hover border-border-soft rounded-xl text-sm font-bold text-text outline-none focus:ring-4 focus:ring-indigo-500/10"
                  placeholder="Return only the translated text."
                />
              </div>

              <div class="grid grid-cols-2 gap-3">
                <label class="min-w-0">
                  <span class="block text-[11px] font-extrabold text-text-muted uppercase mb-1.5">{{t('translator.energy_threshold') }}</span>
                  <input v-model.number="micEnergyThreshold" type="number" min="0" max="10000" step="50" class="w-full px-3 py-2 bg-surface-hover border-border-soft rounded-xl text-sm font-bold text-text outline-none">
                </label>
                <label v-if="micEngine === 'local'" class="min-w-0">
                  <span class="block text-[11px] font-extrabold text-text-muted uppercase mb-1.5">Whisper Model</span>
                  <CustomSelect v-model="whisperModel" :options="[{ label: 'Tiny', value: 'tiny' }, { label: 'Base', value: 'base' }, { label: 'Small', value: 'small' }]" />
                </label>
              </div>
            </div>
          </section>

          <section class="bg-surface backdrop-blur-md rounded-2xl p-5 border-border-soft shadow-sm min-w-0">
            <div class="flex items-center justify-between gap-3 mb-4">
              <h3 class="font-extrabold text-text flex items-center gap-2 text-lg min-w-0">
                <Headphones class="text-emerald-500 shrink-0" :size="20" />
                <span class="truncate">{{ tt('translator.others_voice', '别人说话 (系统内录)') }}</span>
              </h3>
              <button
                data-testid="start-speaker"
                :class="isOtherRecording ? 'bg-emerald-500 text-white border-emerald-500' : 'bg-surface-hover text-text-muted border-border-soft'"
                class="px-3 py-2 rounded-xl font-extrabold text-xs flex items-center gap-2 transition-all active:scale-95 shrink-0"
                @click="toggleOtherRecording"
              >
                <component :is="isSpeakerStarting ? RefreshCw : isOtherRecording ? Square : Ear" :class="{ 'animate-spin': isSpeakerStarting }" :size="15" />
                <span>{{ isOtherRecording || isSpeakerStarting ? tt('translator.stop_game_listen', '停止监听') : tt('translator.listen_game', '开启游戏语音监听') }}</span>
              </button>
            </div>

            <div class="space-y-4">
              <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
                <div class="min-w-0">
                  <label class="block text-[11px] font-extrabold text-text-muted uppercase mb-1.5">{{t('translator.playback_device') }}</label>
                  <CustomSelect v-model="speakerDeviceId" :options="speakerDeviceOptions" />
                </div>
                <div class="min-w-0">
                  <label class="block text-[11px] font-extrabold text-text-muted uppercase mb-1.5">{{ tt('translator.stt_engine_label', 'STT Engine') }}</label>
                  <CustomSelect v-model="otherEngine" :options="speakerEngineOptions" />
                </div>
              </div>

              <div class="grid grid-cols-[1fr_auto_1fr] gap-3 items-end">
                <div class="min-w-0">
                  <label class="block text-[11px] font-extrabold text-text-muted uppercase mb-1.5">{{ tt('translator.source_lang', '源语言') }}</label>
                  <CustomSelect v-model="otherSourceLang" :options="speechLanguageOptions" />
                </div>
                <button class="mb-0.5 w-9 h-9 rounded-xl bg-surface-hover border-border-soft flex items-center justify-center text-text-muted hover:text-primary transition-colors" @click="swapOtherLanguages">
                  <RefreshCw :size="16" />
                </button>
                <div class="min-w-0">
                  <label class="block text-[11px] font-extrabold text-text-muted uppercase mb-1.5">{{ tt('translator.target_lang', '目标语言') }}</label>
                  <CustomSelect v-model="otherTargetLang" :options="languageOptions.filter((option) => option.value !== 'auto')" />
                </div>
              </div>

              <div class="grid grid-cols-2 gap-3">
                <div class="min-w-0">
                  <label class="block text-[11px] font-extrabold text-text-muted uppercase mb-1.5">{{t('translator.capture_mode') }}</label>
                  <CustomSelect v-model="captureMode" :options="[{ label:t('translator.whole_speaker_loopback'), value: 'loopback' }, { label:t('translator.vrchat_process_only'), value: 'process' }]" />
                </div>
                <label class="min-w-0">
                  <span class="block text-[11px] font-extrabold text-text-muted uppercase mb-1.5">{{t('translator.target_process') }}</span>
                  <input v-model="targetProcess" type="text" class="w-full px-3 py-2 bg-surface-hover border-border-soft rounded-xl text-sm font-bold text-text outline-none" placeholder="VRChat.exe">
                </label>
                <label class="min-w-0">
                  <span class="block text-[11px] font-extrabold text-text-muted uppercase mb-1.5">{{t('translator.energy_threshold') }}</span>
                  <input v-model.number="speakerEnergyThreshold" type="number" min="0" max="10000" step="50" class="w-full px-3 py-2 bg-surface-hover border-border-soft rounded-xl text-sm font-bold text-text outline-none">
                </label>
                <label class="min-w-0">
                  <span class="block text-[11px] font-extrabold text-text-muted uppercase mb-1.5">{{t('translator.phrase_limit') }}</span>
                  <input v-model.number="phraseTimeLimit" type="number" min="2" max="30" step="1" class="w-full px-3 py-2 bg-surface-hover border-border-soft rounded-xl text-sm font-bold text-text outline-none">
                </label>
              </div>

              <div class="flex items-center justify-between gap-3 bg-surface-hover rounded-xl px-3 py-2.5 border-border-soft">
                <div class="min-w-0">
                  <span class="text-sm font-bold text-text block">{{t('translator.self_suppress') }}</span>
                  <span class="text-[11px] text-text-muted">{{t('translator.pause_listen_while_you_speak_to_avoid_ec') }}</span>
                </div>
                <button
                  type="button"
                  :class="selfSuppress ? 'bg-emerald-500 text-white' : 'bg-surface border-border-soft text-text-muted'"
                  class="relative w-11 h-6 rounded-full transition-colors shrink-0"
                  @click="selfSuppress = !selfSuppress"
                >
                  <span :class="selfSuppress ? 'translate-x-5' : 'translate-x-0.5'" class="absolute top-0.5 left-0 w-5 h-5 bg-white rounded-full transition-transform" />
                </button>
              </div>

              <div v-if="selfSuppress" class="min-w-0">
                <label class="block text-[11px] font-extrabold text-text-muted uppercase mb-1.5">{{t('translator.suppress_tail') }} (s)</label>
                <input v-model.number="selfSuppressSeconds" type="number" min="0" max="5" step="0.1" class="w-full px-3 py-2 bg-surface-hover border-border-soft rounded-xl text-sm font-bold text-text outline-none">
              </div>

              <div v-if="otherEngine === 'local'" class="min-w-0">
                <label class="block text-[11px] font-extrabold text-text-muted uppercase mb-1.5">Whisper Model</label>
                <CustomSelect v-model="whisperModel" :options="[{ label: 'Tiny', value: 'tiny' }, { label: 'Base', value: 'base' }, { label: 'Small', value: 'small' }]" />
              </div>

              <div class="bg-surface-hover rounded-xl p-4 border-border-soft min-h-[128px]">
                <p v-if="!recognizedText" class="text-border-strong font-medium italic text-center mt-8 text-sm">
                  {{ tt('translator.click_to_speak', '开始麦克风监听、游戏语音监听，或输入文本进行翻译') }}
                </p>
                <p v-else class="text-text font-bold whitespace-pre-wrap leading-relaxed break-words">
                  {{ recognizedText }}
                </p>
              </div>
            </div>
          </section>

          <section class="lg:col-span-2 bg-surface backdrop-blur-md rounded-2xl p-5 border-border-soft shadow-sm min-w-0">
            <div class="flex items-center justify-between gap-3 mb-4">
              <h3 class="font-extrabold text-text flex items-center gap-2 text-lg">
                <SlidersHorizontal class="text-primary" :size="20" />
                {{ tt('translator.advanced_recognition', '高级语音识别') }}
              </h3>
              <span class="text-[11px] font-extrabold text-text-muted bg-surface-hover px-2 py-1 rounded-lg">
                {{t('translator.shared_by_mic_game_audio') }}
              </span>
            </div>

            <div class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4">
              <div class="min-w-0">
                <label class="block text-[11px] font-extrabold text-text-muted uppercase mb-1.5">{{t('translator.voice_activity') }}</label>
                <CustomSelect v-model="vadType" :options="[{ label: 'Silero VAD (ONNX)', value: 'silero' }, { label: 'WebRTC VAD', value: 'webrtc' }, { label: 'RMS 能量', value: 'rms' }]" />
              </div>

              <div v-if="micEngine === 'sherpa' || otherEngine === 'sherpa'" class="md:col-span-2 xl:col-span-3 grid grid-cols-1 md:grid-cols-2 gap-2 rounded-xl border border-border-soft bg-surface-hover p-3">
                <p class="md:col-span-2 text-[10px] font-extrabold text-text-muted">{{ tt('translator.sherpa_model_paths', 'Sherpa-ONNX 流式模型路径') }}</p>
                <input v-model="sherpaConfig.tokens" class="px-3 py-2 bg-surface border border-border-soft rounded-xl text-xs font-bold text-text outline-none" placeholder="tokens.txt">
                <input v-model="sherpaConfig.encoder" class="px-3 py-2 bg-surface border border-border-soft rounded-xl text-xs font-bold text-text outline-none" placeholder="encoder.onnx">
                <input v-model="sherpaConfig.decoder" class="px-3 py-2 bg-surface border border-border-soft rounded-xl text-xs font-bold text-text outline-none" placeholder="decoder.onnx">
                <input v-model="sherpaConfig.joiner" class="px-3 py-2 bg-surface border border-border-soft rounded-xl text-xs font-bold text-text outline-none" placeholder="joiner.onnx">
              </div>

              <div v-if="micEngine.includes('realtime') || otherEngine.includes('realtime')" class="md:col-span-2 xl:col-span-3 grid grid-cols-1 md:grid-cols-2 gap-2 rounded-xl border border-border-soft bg-surface-hover p-3">
                <p class="md:col-span-2 text-[10px] font-extrabold text-text-muted">{{ tt('translator.realtime_asr_config', '实时云 ASR 配置') }}</p>
                <CustomSelect v-model="realtimeAsrConfig.provider" :options="[{ label: 'Tencent Cloud', value: 'tencent_realtime' }, { label: 'Alibaba Cloud NLS', value: 'aliyun_realtime' }]" />
                <input v-model="realtimeAsrConfig.model" class="px-3 py-2 bg-surface border border-border-soft rounded-xl text-xs font-bold text-text outline-none" placeholder="16k_zh_en">
                <template v-if="realtimeAsrConfig.provider === 'tencent_realtime'">
                  <input v-model="realtimeAsrConfig.appId" class="px-3 py-2 bg-surface border border-border-soft rounded-xl text-xs font-bold text-text outline-none" placeholder="Tencent AppID">
                  <input v-model="realtimeAsrConfig.secretId" class="px-3 py-2 bg-surface border border-border-soft rounded-xl text-xs font-bold text-text outline-none" placeholder="Tencent SecretId">
                  <input v-model="realtimeAsrConfig.secretKey" type="password" class="px-3 py-2 bg-surface border border-border-soft rounded-xl text-xs font-bold text-text outline-none" placeholder="Tencent SecretKey">
                </template>
                <template v-else>
                  <input v-model="realtimeAsrConfig.appKey" class="px-3 py-2 bg-surface border border-border-soft rounded-xl text-xs font-bold text-text outline-none" placeholder="Alibaba AppKey">
                  <input v-model="realtimeAsrConfig.accessToken" type="password" class="px-3 py-2 bg-surface border border-border-soft rounded-xl text-xs font-bold text-text outline-none" placeholder="NLS Token">
                </template>
              </div>

              <label class="min-w-0">
                <span class="block text-[11px] font-extrabold text-text-muted uppercase mb-1.5">{{t('translator.vad_aggressiveness') }} · {{ vadAggressiveness }}</span>
                <input v-model.number="vadAggressiveness" type="range" min="0" max="3" step="1" class="w-full accent-emerald-500">
              </label>

              <label class="min-w-0">
                <span class="block text-[11px] font-extrabold text-text-muted uppercase mb-1.5">{{t('translator.denoise') }} · {{ denoiseStrength }}</span>
                <input v-model.number="denoiseStrength" type="range" min="0" max="1" step="0.05" class="w-full accent-emerald-500">
              </label>

              <div class="min-w-0 flex items-center justify-between gap-3 bg-surface-hover rounded-xl px-3 py-2.5 border-border-soft">
                <span class="text-sm font-bold text-text">{{t('translator.asr_correction') }}</span>
                <button
                  type="button"
                  :class="correctionEnabled ? 'bg-emerald-500 text-white' : 'bg-surface border-border-soft text-text-muted'"
                  class="relative w-11 h-6 rounded-full transition-colors shrink-0"
                  @click="correctionEnabled = !correctionEnabled"
                >
                  <span :class="correctionEnabled ? 'translate-x-5' : 'translate-x-0.5'" class="absolute top-0.5 left-0 w-5 h-5 bg-white rounded-full transition-transform" />
                </button>
              </div>

              <label class="min-w-0">
                <span class="block text-[11px] font-extrabold text-text-muted uppercase mb-1.5">{{t('translator.min_segment') }} (s)</span>
                <input v-model.number="minSegmentS" type="number" min="0.1" max="5" step="0.05" class="w-full px-3 py-2 bg-surface-hover border-border-soft rounded-xl text-sm font-bold text-text outline-none">
              </label>

              <label class="min-w-0">
                <span class="block text-[11px] font-extrabold text-text-muted uppercase mb-1.5">{{t('translator.max_segment') }} (s)</span>
                <input v-model.number="maxSegmentS" type="number" min="1" max="30" step="0.5" class="w-full px-3 py-2 bg-surface-hover border-border-soft rounded-xl text-sm font-bold text-text outline-none">
              </label>

              <label class="min-w-0">
                <span class="block text-[11px] font-extrabold text-text-muted uppercase mb-1.5">{{t('translator.partial_interval') }} (s)</span>
                <input v-model.number="partialInterval" type="number" min="0" max="5" step="0.2" class="w-full px-3 py-2 bg-surface-hover border-border-soft rounded-xl text-sm font-bold text-text outline-none">
              </label>
            </div>

            <p class="mt-3 text-[11px] text-text-muted leading-relaxed">
              {{t('translator.built_in_asr_correction_dictionaries_por') }}
            </p>
            <div class="mt-4 grid grid-cols-1 md:grid-cols-2 gap-3">
              <div class="rounded-xl border border-border-soft bg-surface-hover p-3">
                <div class="flex items-center justify-between gap-3">
                  <div><p class="text-xs font-black text-text">{{ tt('translator.vad_calibration', 'VAD 环境校准') }}</p><p class="text-[10px] text-text-muted">{{ tt('translator.vad_calibration_desc', '先开始监听，再采样环境噪声与语音峰值。') }}</p></div>
                  <button v-if="!vadCalibration" class="px-2.5 py-1.5 rounded-lg bg-primary text-white text-[11px] font-extrabold" @click="startVadCalibration(isRecording ? 'mic' : 'speaker')">{{ tt('translator.calibrate', '开始校准') }}</button>
                  <button v-else class="px-2.5 py-1.5 rounded-lg bg-red-500 text-white text-[11px] font-extrabold" @click="stopVadCalibration">{{ tt('translator.cancel', '取消') }}</button>
                </div>
                <div class="mt-3 h-2 rounded-full bg-surface overflow-hidden"><div class="h-full rounded-full bg-primary transition-all" :style="{ width: `${Math.min(100, (audioLevels[vadCalibration?.source || 'mic'] / 1200) * 100)}%` }" /></div>
                <p v-if="vadCalibration?.suggested" class="mt-2 text-[11px] text-emerald-600 font-bold">{{ tt('translator.vad_suggested', '建议阈值') }}: {{ vadCalibration.suggested }}</p>
              </div>
              <div class="rounded-xl border border-border-soft bg-surface-hover p-3">
                <div class="flex items-center justify-between gap-3"><div><p class="text-xs font-black text-text">{{ tt('translator.silero_model', 'Silero VAD 模型') }}</p><p class="text-[10px] text-text-muted truncate max-w-[24ch]">{{ modelStatus?.path || tt('translator.model_not_checked', '尚未检查模型') }}</p></div><button class="px-2.5 py-1.5 rounded-lg bg-surface border border-border-soft text-[11px] font-extrabold text-primary disabled:opacity-50" :disabled="modelBusy || Boolean(modelStatus?.valid)" @click="downloadSilero">{{ modelBusy ? tt('translator.downloading', '下载中...') : modelStatus?.valid ? tt('translator.model_ready', '已就绪') : tt('translator.download_model', '下载模型') }}</button></div>
                <p class="mt-2 text-[10px] font-bold" :class="modelStatus?.valid ? 'text-emerald-600' : 'text-text-muted'">{{ modelStatus?.valid ? `${(modelStatus.size / 1024 / 1024).toFixed(1)} MiB · SHA-256 OK` : tt('translator.model_required_for_silero', '选择 Silero VAD 前请安装并校验模型') }}</p>
              </div>
            </div>
          </section>

          <section class="lg:col-span-2 bg-surface backdrop-blur-md rounded-2xl p-5 border-border-soft shadow-sm min-w-0">
            <div class="flex items-center justify-between gap-3 mb-4"><div><h3 class="font-extrabold text-text flex items-center gap-2 text-lg"><SlidersHorizontal class="text-primary" :size="20" />{{ tt('translator.automation_title', '翻译自动化') }}</h3><p class="text-[11px] text-text-muted mt-1">{{ tt('translator.automation_desc', '将识别文本映射到 VRChat Avatar 参数或任意 OSC 地址，并对快捷键冲突给出提示。') }}</p></div><button class="px-2.5 py-1.5 rounded-lg bg-surface border border-border-soft text-xs font-extrabold text-primary" @click="addKeywordAction"><Plus :size="13" />{{ tt('translator.add_action', '添加动作') }}</button></div>
            <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
              <label class="min-w-0"><span class="block text-[10px] font-extrabold text-text-muted mb-1">{{ tt('translator.input_hotkey', '快捷输入快捷键') }}</span><input v-model="quickInputHotkey" class="w-full px-3 py-2 bg-surface-hover border border-border-soft rounded-xl text-sm font-bold text-text outline-none" @change="checkTranslationHotkeys"></label>
              <label class="min-w-0"><span class="block text-[10px] font-extrabold text-text-muted mb-1">{{ tt('translator.voice_hotkey', '语音开关快捷键') }}</span><input v-model="voiceToggleHotkey" class="w-full px-3 py-2 bg-surface-hover border border-border-soft rounded-xl text-sm font-bold text-text outline-none" @change="checkTranslationHotkeys"></label>
            </div>
            <div v-if="hotkeyConflicts.length" class="mt-3 p-3 rounded-xl bg-red-500/10 border border-red-500/20 text-[11px] text-red-500 font-bold">{{ hotkeyConflicts.map((item) => `${item.hotkey}: ${item.reason}`).join('；') }}</div>
            <div v-if="keywordActions.length" class="mt-3 space-y-2 max-h-48 overflow-y-auto custom-scrollbar pr-1"><div v-for="(action, index) in keywordActions" :key="index" class="grid grid-cols-[1fr_1fr_80px_80px_100px_34px] gap-2 items-center"><input v-model="action.keyword" :placeholder="tt('translator.keyword', '关键词')" class="min-w-0 px-2.5 py-1.5 bg-surface-hover border border-border-soft rounded-lg text-xs font-bold text-text outline-none"><input v-model="action.address" placeholder="/avatar/parameters/Name" class="min-w-0 px-2.5 py-1.5 bg-surface-hover border border-border-soft rounded-lg text-xs font-bold text-text outline-none"><CustomSelect v-model="action.valueType" :options="[{ label: 'Float', value: 'float' }, { label: 'Int', value: 'int' }, { label: 'Bool', value: 'bool' }, { label: 'Double', value: 'double' }]" /><input v-model.number="action.value" type="number" step="0.1" placeholder="值" class="min-w-0 px-2.5 py-1.5 bg-surface-hover border border-border-soft rounded-lg text-xs font-bold text-text outline-none"><input v-model.number="action.cooldownMs" type="number" min="0" step="100" placeholder="冷却 ms" class="min-w-0 px-2.5 py-1.5 bg-surface-hover border border-border-soft rounded-lg text-xs font-bold text-text outline-none"><button class="w-8 h-8 rounded-lg text-text-muted hover:text-red-500" @click="removeKeywordAction(index)"><Trash2 :size="14" /></button></div></div>
            <p class="mt-3 text-[10px] text-text-muted">{{ tt('translator.runtime_version', '翻译能力清单') }}: {{ runtimeVersion }} <button class="ml-2 text-primary font-bold hover:underline" @click="updateTranslationRuntime">{{ tt('translator.update_now', '立即检查更新') }}</button></p>
          </section>

          <section class="lg:col-span-2 bg-surface backdrop-blur-md rounded-2xl p-5 border-border-soft shadow-sm min-w-0">
            <div class="flex items-center justify-between gap-3 mb-3">
              <div>
                <h3 class="font-extrabold text-text flex items-center gap-2 text-lg"><Camera class="text-primary" :size="20" /> {{ tt('translator.photo_title', '拍照翻译') }}</h3>
                <p class="text-[11px] text-text-muted mt-1">{{ tt('translator.photo_desc', '选择 VRChat 截图或本地图片，使用 Windows OCR 识别后复用当前翻译服务。') }}</p>
              </div>
              <div class="flex items-center gap-2">
                <button class="px-3 py-2 rounded-xl bg-surface-hover hover:bg-surface border border-border-soft text-text-muted text-xs font-extrabold flex items-center gap-2 disabled:opacity-50" :disabled="photoBusy || photoArmed" @click="translatePhoto"><Camera :size="14" />{{ tt('translator.photo_choose', '选择图片') }}</button>
                <button class="px-3 py-2 rounded-xl bg-primary hover:bg-primary-hover text-white text-xs font-extrabold flex items-center gap-2 disabled:opacity-50" :disabled="photoBusy || photoArmed" @click="waitForPhoto"><RefreshCw v-if="photoArmed" class="animate-spin" :size="14" /><Camera v-else :size="14" />{{ photoArmed ? tt('translator.photo_waiting', '等待截图...') : tt('translator.photo_wait_next', '等待下一张截图') }}</button>
              </div>
            </div>
            <div class="grid grid-cols-1 md:grid-cols-3 gap-3 mb-3">
              <div><label class="block text-[10px] font-extrabold text-text-muted mb-1">{{ tt('translator.photo_ocr_language', 'OCR 语言') }}</label><CustomSelect v-model="photoOcrLang" :options="languageOptions" /></div>
              <div><label class="block text-[10px] font-extrabold text-text-muted mb-1">{{ tt('translator.photo_source_language', '图片源语言') }}</label><CustomSelect v-model="photoSourceLang" :options="languageOptions" /></div>
              <div><label class="block text-[10px] font-extrabold text-text-muted mb-1">{{ tt('translator.photo_target_language', '图片目标语言') }}</label><CustomSelect v-model="photoTargetLang" :options="languageOptions.filter((option) => option.value !== 'auto')" /></div>
            </div>
            <div v-if="photoOriginal || photoTranslated" class="grid grid-cols-1 md:grid-cols-2 gap-3">
              <div class="p-3 rounded-xl bg-surface-hover border border-border-soft"><p class="text-[10px] font-black text-text-muted uppercase mb-1">{{ tt('translator.photo_original', '识别原文') }}</p><p class="text-sm text-text whitespace-pre-wrap break-words">{{ photoOriginal }}</p></div>
              <div class="p-3 rounded-xl bg-primary/5 border border-primary/20"><p class="text-[10px] font-black text-primary uppercase mb-1">{{ tt('translator.photo_translation', '图片译文') }}</p><p class="text-sm text-text whitespace-pre-wrap break-words">{{ photoTranslated }}</p></div>
            </div>
            <p v-else class="text-[11px] text-text-muted">{{ tt('translator.photo_empty', '还没有图片翻译结果。处理过程不会保存额外的截图副本。') }}</p>
          </section>

          <section class="lg:col-span-2 bg-surface backdrop-blur-md rounded-2xl p-5 border-border-soft shadow-sm min-w-0">
            <div class="flex items-center justify-between gap-3 mb-4">
              <h3 class="font-extrabold text-text flex items-center gap-2 text-lg">
                <ClipboardList class="text-primary" :size="20" />
                {{ tt('translator.text_translation', '文本翻译') }}
              </h3>
              <button
                :disabled="!canTranslate"
                class="px-4 py-2 rounded-xl bg-primary hover:bg-primary-hover disabled:opacity-50 disabled:cursor-not-allowed text-white font-extrabold text-sm flex items-center gap-2 transition-all active:scale-95"
                @click="translateManual"
              >
                <component :is="isTranslating ? RefreshCw : Send" :class="{ 'animate-spin': isTranslating }" :size="16" />
                {{ tt('translator.translate_and_send', '翻译并发送') }}
              </button>
            </div>
            <textarea
              v-model="manualText"
              rows="4"
              class="w-full resize-none px-4 py-3 bg-surface-hover border-border-soft rounded-xl text-sm font-bold text-text outline-none focus:ring-4 focus:ring-indigo-500/10"
              :placeholder="t('translator.enter_text_to_translate_for_vrchat')"
              @keydown.ctrl.enter.prevent="translateManual"
            />
          </section>

          <section class="lg:col-span-2 bg-surface backdrop-blur-md rounded-2xl p-5 border-border-soft shadow-sm min-w-0">
            <div class="flex justify-between items-center gap-3 mb-4">
              <h3 class="font-extrabold text-text flex items-center gap-2 text-lg min-w-0">
                <Volume2 class="text-emerald-500 shrink-0" :size="20" />
                <span class="truncate">{{ tt('translator.machine_result', '机器翻译结果') }}</span>
              </h3>
              <span v-if="isTranslating" class="text-[11px] font-extrabold text-primary flex items-center gap-1.5 bg-primary/10 px-2 py-1 rounded-lg shrink-0">
                <RefreshCw class="animate-spin" :size="12" /> {{ tt('translator.translating', '翻译中...') }}
              </span>
            </div>

            <div class="bg-surface-hover rounded-xl p-5 border-border-soft min-h-[150px] overflow-y-auto custom-scrollbar">
              <p v-if="!translatedText" class="text-border-strong font-medium italic text-center mt-8 text-sm">
                {{ tt('translator.result_here', '翻译结果将显示在这里...') }}
              </p>
              <div v-else class="space-y-2">
                <p
                  v-for="item in currentTranslations"
                  :key="item.target_lang"
                  class="text-emerald-700 font-black text-lg leading-relaxed break-words"
                >
                  <span v-if="currentTranslations.length > 1" class="mr-2 text-xs text-text-muted font-bold">{{ item.target_lang }}</span>
                  {{ item.translated }}
                </p>
              </div>
            </div>

            <div class="mt-4 flex gap-3 shrink-0">
              <button
                class="w-12 py-2.5 bg-surface border-border-soft hover:border-emerald-300 hover:text-emerald-600 text-text-muted rounded-xl flex items-center justify-center transition-all shadow-sm active:scale-95"
                :title="'TTS'"
                @click="manualPlay"
              >
                <Volume2 :size="16" />
              </button>
              <button
                class="flex-1 py-2.5 bg-emerald-500 hover:bg-emerald-600 text-white font-extrabold text-sm rounded-xl flex items-center justify-center gap-2 transition-all shadow-md shadow-emerald-500/30 active:scale-95 min-w-0"
                @click="manualSend"
              >
                <Send :size="16" />
                <span class="truncate">{{ tt('translator.manual_send', '手动发送至 VRChat') }}</span>
              </button>
            </div>
          </section>
        </main>

        <aside class="bg-surface backdrop-blur-md rounded-2xl p-5 border-border-soft shadow-sm min-w-0 h-fit max-h-[720px] overflow-hidden flex flex-col">
          <div class="flex items-center justify-between gap-3 mb-4">
            <h3 class="font-extrabold text-text flex items-center gap-2 text-lg">
              <ClipboardList class="text-primary" :size="20" />
              {{t('translator.translation_history') }}
            </h3>
            <button class="w-9 h-9 rounded-xl bg-surface-hover border-border-soft flex items-center justify-center text-text-muted hover:text-red-500 transition-colors" @click="clearHistory">
              <RotateCcw :size="15" />
            </button>
          </div>

          <div class="flex-1 min-h-0 overflow-y-auto custom-scrollbar space-y-3 pr-1">
            <button
              v-for="record in history"
              :key="record.id"
              class="w-full text-left bg-surface-hover hover:bg-surface-active rounded-xl p-3 border-border-soft transition-colors"
              @click="useHistory(record)"
            >
              <div class="flex items-center justify-between gap-2 mb-1">
                <span class="text-[11px] uppercase font-black text-primary">{{ record.source }}</span>
                <span class="text-[11px] font-bold text-text-muted truncate">{{ record.service }}</span>
              </div>
              <p class="text-sm font-bold text-text line-clamp-2 break-words">{{ record.original }}</p>
              <div class="mt-1 space-y-0.5">
                <p
                  v-for="item in (record.translations?.length ? record.translations : [{ target_lang: record.target_lang, translated: record.translated }])"
                  :key="item.target_lang"
                  class="text-sm font-black text-emerald-700 line-clamp-2 break-words"
                >
                  <span v-if="record.translations && record.translations.length > 1" class="text-[10px] text-text-muted mr-1">{{ item.target_lang }}</span>
                  {{ item.translated }}
                </p>
              </div>
              <div class="flex items-center gap-2 mt-2 text-[11px] font-bold text-text-muted">
                <span>{{ record.source_lang }} -> {{ record.target_lang }}</span>
                <span v-if="record.sent_osc" class="text-emerald-600">OSC</span>
              </div>
            </button>
            <p v-if="history.length === 0" class="text-sm text-border-strong font-bold text-center py-10">
              {{t('translator.no_translation_history_yet') }}
            </p>
          </div>
        </aside>
      </div>
    </div>
  </div>
</template>

<style scoped>
.overlay-opacity-slider {
  height: 4px;
  border-radius: 999px;
  accent-color: var(--theme-primary);
  cursor: pointer;
}
</style>
