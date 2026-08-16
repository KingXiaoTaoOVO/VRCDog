<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue';
import { isTauri } from '@tauri-apps/api/core';
import { emit, listen, type UnlistenFn } from '@tauri-apps/api/event';
import { WebviewWindow } from '@tauri-apps/api/webviewWindow';
import { useStorage } from '@vueuse/core';
import {
  CheckCircle2,
  ClipboardList,
  Ear,
  Headphones,
  Languages,
  Mic,
  MicOff,
  MonitorUp,
  RefreshCw,
  RotateCcw,
  Send,
  Settings,
  Square,
  Volume2,
} from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';
import { SysApi, VrctApi, type AudioDevice, type AudioSource } from '../api';
import { SerialTaskQueue } from '../utils/serialTaskQueue';
import CustomSelect from './CustomSelect.vue';

type MessageSource = 'chat' | 'mic' | 'speaker';
type TtsEngine = 'system' | 'gpt_sovits';

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

const { t, locale } = useI18n();
const l = (zh: string, en: string) => locale.value.startsWith('zh') ? zh : en;

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
]);

const sourceLang = useStorage('vrc_translator_source_lang', 'zh-CN');
const targetLang = useStorage('vrc_translator_target_lang', 'en-US');
const otherSourceLang = useStorage('vrc_translator_other_source_lang', 'en-US');
const otherTargetLang = useStorage('vrc_translator_other_target_lang', 'zh-CN');
const translateEngine = useStorage('vrc_translator_engine', 'google_free');
const micEngine = useStorage<'cloud' | 'local'>('vrc_translator_mic_stt_engine', 'cloud');
const otherEngine = useStorage('vrc_translator_stt_engine', 'cloud');
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

let overlayWebview: WebviewWindow | null = null;
let unlistenAudio: UnlistenFn | null = null;
let unlistenVrct: UnlistenFn | null = null;

const currentEngine = computed(() => engineOptions.value.find((engine) => engine.value === translateEngine.value) ?? engineOptions.value[0]);
const needsApiKey = computed(() => Boolean(currentEngine.value.needsKey && !currentEngine.value.supportsLocal));
const showModelField = computed(() => ['openai', 'deepseek', 'siliconflow', 'moonshot', 'zhipu', 'groq', 'openrouter', 'plamo', 'ollama', 'lmstudio', 'custom_llm', 'gemini'].includes(translateEngine.value));
const canTranslate = computed(() => !isTranslating.value && Boolean(manualText.value.trim()));
const micDeviceOptions = computed<Option[]>(() => audioDevices.value
  .filter(device => device.source === 'mic')
  .map(device => ({ label: `${device.name}${device.is_default ? ` (${l('默认', 'Default')})` : ''}`, value: device.id })));
const speakerDeviceOptions = computed<Option[]>(() => audioDevices.value
  .filter(device => device.source === 'speaker')
  .map(device => ({ label: `${device.name}${device.is_default ? ` (${l('默认', 'Default')})` : ''}`, value: device.id })));

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
      window.speechSynthesis.cancel();
      const utterance = new SpeechSynthesisUtterance(text);
      utterance.lang = lang;
      utterance.rate = 1.0;
      await new Promise<void>((resolve, reject) => {
        utterance.onend = () => resolve();
        utterance.onerror = event => reject(new Error(event.error || 'Speech synthesis failed'));
        window.speechSynthesis.speak(utterance);
      });
    } else {
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

const sendToChatbox = async (text: string) => {
  if (!text.trim()) return;
  try {
    await SysApi.sendOscChatbox({ text, complete: true });
    setStatus(l('已发送到 VRChat Chatbox', 'Sent to VRChat Chatbox'));
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

  if (needsApiKey.value && !apiKey.value.trim()) {
    errorMsg.value = `${currentEngine.value.label}: ${currentEngine.value.hint}`;
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
        service: translateEngine.value,
        api_key: apiKey.value.trim(),
        model: model.value.trim(),
        prompt: prompt.value.trim(),
        custom_api_url: customApiUrl.value.trim(),
        send_osc: sendOsc,
        send_typing: sendOsc,
        complete: true,
        notification: false,
        update_overlay: true,
        show_original_in_osc: showOriginalOsc.value,
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
      engine: (isMic ? micEngine.value : otherEngine.value) as 'cloud' | 'local',
      deviceIndex: selectedDeviceIndex(source),
      energyThreshold: isMic ? Number(micEnergyThreshold.value) : Number(speakerEnergyThreshold.value),
      dynamicEnergyThreshold: true,
      phraseTimeLimit: Number(phraseTimeLimit.value),
      whisperModel: whisperModel.value,
    });
    setStatus(l('音频识别服务启动中', 'Starting audio recognition service'));
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
    setStatus(l('浏览器预览中不创建 Tauri 悬浮窗', 'The Tauri overlay is unavailable in browser preview'));
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

onMounted(async () => {
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
        if (payload.message === 'starting') setStatus(`${l('监听设备', 'Listening device')}: ${payload.device || 'Default'}`);
        if (payload.message === 'loading_model') setStatus(l('首次加载本地 Whisper 模型', 'Loading local Whisper model'));
        if (payload.message === 'recognizing') setStatus(source === 'mic'
          ? l('正在识别麦克风语音', 'Recognizing microphone audio')
          : l('正在识别游戏语音', 'Recognizing game audio'));
        if (payload.message === 'backlog_trimmed') setStatus(l('语音过快，已跳过最旧片段', 'Audio backlog trimmed'));
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
            errorMsg.value = l('音频识别服务异常退出', 'Audio recognition service stopped unexpectedly');
          }
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
      addHistory(event.payload as VrctMessageRecord);
    });
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
  if (isTauri()) {
    await Promise.all([
      SysApi.stopAudioCapture({ source: 'mic' }).catch(() => undefined),
      SysApi.stopAudioCapture({ source: 'speaker' }).catch(() => undefined),
    ]);
  }
  unlistenAudio?.();
  unlistenVrct?.();
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
              <span class="text-sm font-bold text-text-muted truncate">{{ l('多语言同时翻译', 'Translate to multiple languages') }}</span>
            </label>
          </div>

          <!-- Multi-language target selector -->
          <div v-if="multiLangEnabled" class="md:col-span-2 mt-2 p-3 bg-surface-hover rounded-xl border border-border-soft">
            <label class="block text-[11px] font-extrabold text-text-muted uppercase mb-2">{{ l('额外目标语言 (多语言 OSC)', 'Additional target languages (multi-language OSC)') }}</label>
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
            <p class="text-[10px] text-text-muted mt-2">{{ l('翻译结果将依次发送到 VRChat ChatBox（每条间隔约1.2秒）', 'Translations are sent to VRChat Chatbox in sequence, about 1.2 seconds apart.') }}</p>
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
                  { label: tt('translator.tts_gptsovits', 'GPT-SoVITS API'), value: 'gpt_sovits' }
                ]"
              />
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
          <button class="w-8 h-8 shrink-0 inline-flex items-center justify-center rounded-lg bg-white/70" :title="l('刷新音频设备', 'Refresh audio devices')" @click="loadAudioDevices">
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
                  <label class="block text-[11px] font-extrabold text-text-muted uppercase mb-1.5">{{ l('麦克风设备', 'Microphone device') }}</label>
                  <CustomSelect v-model="micDeviceId" :options="micDeviceOptions" />
                </div>
                <div class="min-w-0">
                  <label class="block text-[11px] font-extrabold text-text-muted uppercase mb-1.5">{{ tt('translator.stt_engine_label', 'STT Engine') }}</label>
                  <CustomSelect v-model="micEngine" :options="speakerEngineOptions" />
                </div>
              </div>

              <div>
                <label class="block text-[11px] font-extrabold text-text-muted uppercase mb-1.5">{{ tt('translator.engine', l('翻译引擎', 'Translation engine')) }}</label>
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
                  <span class="block text-[11px] font-extrabold text-text-muted uppercase mb-1.5">{{ l('能量阈值', 'Energy threshold') }}</span>
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
                  <label class="block text-[11px] font-extrabold text-text-muted uppercase mb-1.5">{{ l('输出设备', 'Playback device') }}</label>
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
                <label class="min-w-0">
                  <span class="block text-[11px] font-extrabold text-text-muted uppercase mb-1.5">{{ l('能量阈值', 'Energy threshold') }}</span>
                  <input v-model.number="speakerEnergyThreshold" type="number" min="0" max="10000" step="50" class="w-full px-3 py-2 bg-surface-hover border-border-soft rounded-xl text-sm font-bold text-text outline-none">
                </label>
                <label class="min-w-0">
                  <span class="block text-[11px] font-extrabold text-text-muted uppercase mb-1.5">{{ l('最长分段', 'Phrase limit') }}</span>
                  <input v-model.number="phraseTimeLimit" type="number" min="2" max="30" step="1" class="w-full px-3 py-2 bg-surface-hover border-border-soft rounded-xl text-sm font-bold text-text outline-none">
                </label>
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
              :placeholder="l('输入要翻译到 VRChat 的文字...', 'Enter text to translate for VRChat...')"
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
              {{ l('翻译历史', 'Translation history') }}
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
              {{ l('暂无历史记录', 'No translation history yet') }}
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
