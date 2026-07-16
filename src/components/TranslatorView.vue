<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue';
import { isTauri } from '@tauri-apps/api/core';
import { emit, listen, type UnlistenFn } from '@tauri-apps/api/event';
import { WebviewWindow } from '@tauri-apps/api/webviewWindow';
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
import { SysApi, VrctApi } from '../api';
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
  service: string;
  sent_osc: boolean;
  overlay_updated: boolean;
  timestamp: string;
}

const { t } = useI18n();

const tt = (key: string, fallback: string) => {
  const value = t(key);
  return value === key ? fallback : value;
};

const languageOptions: Option[] = [
  { label: 'Auto detect', value: 'auto' },
  { label: 'Chinese (zh-CN)', value: 'zh-CN' },
  { label: 'English (en-US)', value: 'en-US' },
  { label: 'Japanese (ja-JP)', value: 'ja-JP' },
  { label: 'Korean (ko-KR)', value: 'ko-KR' },
  { label: 'French (fr-FR)', value: 'fr' },
  { label: 'German (de-DE)', value: 'de' },
  { label: 'Spanish (es-ES)', value: 'es' },
  { label: 'Russian (ru-RU)', value: 'ru' },
  { label: 'Portuguese (pt-BR)', value: 'pt-BR' },
  { label: 'Thai (th-TH)', value: 'th' },
  { label: 'Vietnamese (vi-VN)', value: 'vi' },
];

const speechLanguageOptions = languageOptions.filter((option) => option.value !== 'auto');

const engineOptions: EngineOption[] = [
  { label: 'Google Translate Free', value: 'google_free', hint: 'No API key required' },
  { label: 'Microsoft Translator', value: 'microsoft', needsKey: true, hint: 'Azure Translator key' },
  { label: 'DeepL Free', value: 'deepl_free', needsKey: true, hint: 'DeepL Free auth key' },
  { label: 'DeepL Pro', value: 'deepl', needsKey: true, hint: 'DeepL Pro auth key' },
  { label: 'Tencent Translate', value: 'tencent', needsKey: true, hint: 'SecretId:SecretKey' },
  { label: 'Baidu Translate', value: 'baidu', needsKey: true, hint: 'AppID:SecretKey' },
  { label: 'Papago', value: 'papago', needsKey: true, hint: 'ClientId:ClientSecret' },
  { label: 'Gemini', value: 'gemini', needsKey: true, hint: 'Google AI Studio key' },
  { label: 'OpenAI', value: 'openai', needsKey: true, hint: 'OpenAI compatible API key' },
  { label: 'DeepSeek', value: 'deepseek', needsKey: true, hint: 'DeepSeek API key' },
  { label: 'SiliconFlow', value: 'siliconflow', needsKey: true, hint: 'SiliconFlow API key' },
  { label: 'Moonshot', value: 'moonshot', needsKey: true, hint: 'Moonshot API key' },
  { label: 'ZhiPu GLM', value: 'zhipu', needsKey: true, hint: 'ZhiPu API key' },
  { label: 'Groq', value: 'groq', needsKey: true, hint: 'Groq API key' },
  { label: 'OpenRouter', value: 'openrouter', needsKey: true, hint: 'OpenRouter API key' },
  { label: 'Plamo', value: 'plamo', needsKey: true, hint: 'PreferredAI Platform API key' },
  { label: 'Ollama Local', value: 'ollama', supportsLocal: true, hint: 'http://127.0.0.1:11434' },
  { label: 'LM Studio Local', value: 'lmstudio', supportsLocal: true, hint: 'http://127.0.0.1:1234' },
  { label: 'Custom OpenAI API', value: 'custom_llm', needsKey: true, hint: 'OpenAI-compatible endpoint' },
];

const speakerEngineOptions: Option[] = [
  { label: tt('translator.engine_cloud', 'Cloud recognition (Google Web Speech)'), value: 'cloud' },
  { label: tt('translator.engine_local', 'Local Whisper / offline fallback'), value: 'local' },
];

const sourceLang = ref('zh-CN');
const targetLang = ref('en-US');
const otherSourceLang = ref('en-US');
const otherTargetLang = ref('zh-CN');
const translateEngine = ref('google_free');
const otherEngine = ref('cloud');
const apiKey = ref('');
const model = ref('');
const customApiUrl = ref('');
const prompt = ref('');

const manualText = ref('');
const recognizedText = ref('');
const translatedText = ref('');
const lastTargetLang = ref(targetLang.value);
const history = ref<VrctMessageRecord[]>([]);

const autoSendOsc = ref(true);
const showOriginalOsc = ref(true);
const autoPlayTts = ref(false);
const ttsEngine = ref<TtsEngine>('system');
const gptSovitsUrl = ref('http://127.0.0.1:9880');

const isRecording = ref(false);
const isOtherRecording = ref(false);
const isTranslating = ref(false);
const isOverlayOpen = ref(false);
const errorMsg = ref('');
const statusMsg = ref('');

let recognition: any = null;
let overlayWebview: WebviewWindow | null = null;
let unlistenAudio: UnlistenFn | null = null;
let unlistenVrct: UnlistenFn | null = null;

const currentEngine = computed(() => engineOptions.find((engine) => engine.value === translateEngine.value) ?? engineOptions[0]);
const needsApiKey = computed(() => Boolean(currentEngine.value.needsKey && !currentEngine.value.supportsLocal));
const showModelField = computed(() => ['openai', 'deepseek', 'siliconflow', 'moonshot', 'zhipu', 'groq', 'openrouter', 'plamo', 'ollama', 'lmstudio', 'custom_llm', 'gemini'].includes(translateEngine.value));
const canTranslate = computed(() => !isTranslating.value && Boolean(manualText.value.trim()));

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
  lastTargetLang.value = record.target_lang || targetLang.value;
  addHistory(record);
};

const notifyOverlay = async (record: VrctMessageRecord) => {
  await emit('translation-log', {
    id: record.id,
    type: overlayType(record.source),
    text: record.original,
    translation: record.translated,
  });
};

const playTts = async (text: string, lang = lastTargetLang.value) => {
  if (!text.trim()) return;

  if (ttsEngine.value === 'system') {
    if (!('speechSynthesis' in window)) {
      errorMsg.value = tt('auto_43b1967a', 'This WebView does not support Web Speech API.');
      return;
    }
    window.speechSynthesis.cancel();
    const utterance = new SpeechSynthesisUtterance(text);
    utterance.lang = lang;
    utterance.rate = 1.0;
    window.speechSynthesis.speak(utterance);
    return;
  }

  try {
    const langCode = lang.startsWith('ja') ? 'ja' : lang.startsWith('ko') ? 'ko' : lang.startsWith('en') ? 'en' : 'zh';
    const url = `${gptSovitsUrl.value.replace(/\/$/, '')}/?text=${encodeURIComponent(text)}&text_language=${langCode}`;
    await new Audio(url).play();
  } catch (error) {
    errorMsg.value = tt('translator.tts_error', 'TTS playback failed: {err}').replace('{err}', errorText(error));
  }
};

const sendToChatbox = async (text: string) => {
  if (!text.trim()) return;
  try {
    await SysApi.sendOscChatbox({ text, complete: true });
    setStatus('已发送到 VRChat Chatbox');
  } catch (error) {
    errorMsg.value = tt('translator.osc_error', 'Unable to send to VRChat: {err}').replace('{err}', errorText(error));
  }
};

const processMessage = async (
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
  isTranslating.value = true;
  try {
    const record = await VrctApi.processMessage({
      req: {
        text: trimmed,
        source,
        source_lang: sourceLanguage,
        target_lang: targetLanguage,
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
  } finally {
    isTranslating.value = false;
  }
};

const translateManual = async () => {
  const record = await processMessage(manualText.value, 'chat', sourceLang.value, targetLang.value, autoSendOsc.value);
  if (record) manualText.value = '';
};

const translateMicText = (text: string) => processMessage(text, 'mic', sourceLang.value, targetLang.value, autoSendOsc.value);

const translateSpeakerText = (text: string) => processMessage(text, 'speaker', otherSourceLang.value, otherTargetLang.value, false);

const swapMyLanguages = () => {
  if (sourceLang.value === 'auto') return;
  [sourceLang.value, targetLang.value] = [targetLang.value, sourceLang.value];
};

const swapOtherLanguages = () => {
  [otherSourceLang.value, otherTargetLang.value] = [otherTargetLang.value, otherSourceLang.value];
};

const toggleRecording = () => {
  if (!recognition) {
    errorMsg.value = tt('auto_43b1967a', 'This WebView does not support Web Speech API.');
    return;
  }

  errorMsg.value = '';
  if (isRecording.value) {
    isRecording.value = false;
    recognition.stop();
    return;
  }

  try {
    recognition.lang = sourceLang.value === 'auto' ? 'en-US' : sourceLang.value;
    recognition.start();
    isRecording.value = true;
  } catch (error) {
    errorMsg.value = `Speech recognition failed: ${errorText(error)}`;
    isRecording.value = false;
  }
};

const toggleOtherRecording = async () => {
  errorMsg.value = '';
  if (isOtherRecording.value) {
    try {
      await SysApi.stopAudioCapture();
    } finally {
      isOtherRecording.value = false;
      setStatus('已停止游戏语音监听');
    }
    return;
  }

  try {
    await SysApi.startAudioCapture({ sourceLang: otherSourceLang.value, engine: otherEngine.value });
    isOtherRecording.value = true;
    setStatus(otherEngine.value === 'local' ? '本地语音识别启动中' : '游戏语音监听已开启');
  } catch (error) {
    const message = errorText(error);
    if (message.includes('WASAPI') || message.includes('loopback')) {
      errorMsg.value = tt('translator.capture_error_wasapi', 'Audio capture failed. Play any system sound once and retry.');
    } else {
      errorMsg.value = `${tt('translator.capture_error_cloud', 'Audio capture failed: {err}').replace('{err}', message)}`;
    }
    isOtherRecording.value = false;
  }
};

const manualSend = () => {
  const payload = showOriginalOsc.value && recognizedText.value
    ? `${translatedText.value} (${recognizedText.value})`
    : translatedText.value;
  sendToChatbox(payload);
};

const manualPlay = () => playTts(translatedText.value);

const useHistory = (record: VrctMessageRecord) => {
  recognizedText.value = record.original;
  translatedText.value = record.translated;
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
    setStatus('浏览器预览中不创建 Tauri 悬浮窗');
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

const setupSpeechRecognition = () => {
  const SpeechRecognition = (window as any).SpeechRecognition || (window as any).webkitSpeechRecognition;
  if (!SpeechRecognition) return;

  recognition = new SpeechRecognition();
  recognition.continuous = true;
  recognition.interimResults = true;

  recognition.onresult = (event: any) => {
    let finalTranscript = '';
    let interimTranscript = '';

    for (let i = event.resultIndex; i < event.results.length; i += 1) {
      const transcript = event.results[i][0].transcript;
      if (event.results[i].isFinal) finalTranscript += transcript;
      else interimTranscript += transcript;
    }

    if (finalTranscript.trim()) {
      recognizedText.value = finalTranscript.trim();
      void translateMicText(finalTranscript);
    } else {
      recognizedText.value = interimTranscript.trim();
    }
  };

  recognition.onerror = (event: any) => {
    if (event.error !== 'no-speech') {
      errorMsg.value = `Speech recognition error: ${event.error}`;
    }
    isRecording.value = false;
  };

  recognition.onend = () => {
    if (isRecording.value) {
      try {
        recognition.start();
      } catch {
        isRecording.value = false;
      }
    }
  };
};

onMounted(async () => {
  setupSpeechRecognition();

  if (isTauri()) {
    unlistenAudio = await listen('audio-capture-event', async (event: any) => {
      const payload = event.payload;
      if (payload.type === 'error') {
        errorMsg.value = `Audio capture error: ${payload.message}`;
        isOtherRecording.value = false;
        return;
      }
      if (payload.type === 'status') {
        if (payload.message === 'starting') setStatus(`监听设备: ${payload.device || 'Default'}`);
        if (payload.message === 'recognizing') setStatus('正在识别游戏语音');
        return;
      }
      if (payload.type === 'result' && payload.text?.trim()) {
        await translateSpeakerText(payload.text);
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
  if (recognition && isRecording.value) recognition.stop();
  if (isOtherRecording.value) await SysApi.stopAudioCapture().catch(() => undefined);
  unlistenAudio?.();
  unlistenVrct?.();
});
</script>

<template>
  <div class="h-full min-h-0 flex flex-col p-6 bg-surface-hover rounded-3xl relative overflow-hidden">
    <header class="flex items-center justify-between gap-4 mb-5 shrink-0 z-10">
      <h2 class="text-3xl font-extrabold text-text flex items-center gap-3 tracking-tight min-w-0">
        <span class="inline-flex items-center justify-center p-2 bg-primary/10 rounded-2xl shadow-sm border-primary shrink-0">
          <Languages class="w-6 h-6 text-primary" />
        </span>
        <span class="truncate">{{ tt('translator.title', '翻译官') }}</span>
      </h2>
      <button
        :class="isOverlayOpen ? 'bg-red-500 hover:bg-red-600 shadow-red-500/30 border-red-500 text-white' : 'bg-surface hover:bg-surface-hover text-text-muted hover:text-primary border-border-soft shadow-sm'"
        class="px-4 py-2.5 font-bold rounded-xl flex items-center gap-2 transition-all active:scale-95 text-sm shrink-0"
        @click="toggleOverlay"
      >
        <MonitorUp :size="16" />
        <span>{{ isOverlayOpen ? tt('translator.overlay_close', '关闭悬浮窗') : tt('translator.overlay_open', '开启悬浮窗') }}</span>
      </button>
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
            <div v-if="ttsEngine === 'gpt_sovits'" class="min-w-0">
              <label class="block text-[11px] font-extrabold text-text-muted uppercase mb-1.5">{{ tt('translator.gptsovits_url_label', 'GPT-SoVITS API URL') }}</label>
              <input
                v-model="gptSovitsUrl"
                type="text"
                class="w-full px-3 py-2 bg-surface-hover border-border-soft rounded-xl text-sm font-bold text-text outline-none focus:ring-4 focus:ring-indigo-500/10 transition-all"
                :placeholder="tt('translator.gptsovits_url_placeholder', 'http://127.0.0.1:9880')"
              >
            </div>
          </div>
        </div>
      </section>

      <div v-if="errorMsg || statusMsg" class="mb-5 shrink-0">
        <div v-if="errorMsg" class="bg-red-50 border-red-200 text-red-600 px-4 py-3 rounded-xl text-sm font-bold shadow-sm">
          {{ errorMsg }}
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
                <component :is="isRecording ? MicOff : Mic" :size="16" />
                <span>{{ isRecording ? tt('translator.stop_listen', '停止收音') : tt('translator.start_listen', '开始麦克风监听') }}</span>
              </button>
            </div>

            <div class="space-y-4">
              <div>
                <label class="block text-[11px] font-extrabold text-text-muted uppercase mb-1.5">{{ tt('translator.engine', '翻译引擎') }}</label>
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
                  <label class="block text-[11px] font-extrabold text-text-muted uppercase mb-1.5">API Key</label>
                  <input v-model="apiKey" type="password" class="w-full px-3 py-2 bg-surface-hover border-border-soft rounded-xl text-sm font-bold text-text outline-none focus:ring-4 focus:ring-indigo-500/10">
                </div>
                <div v-if="showModelField" class="min-w-0">
                  <label class="block text-[11px] font-extrabold text-text-muted uppercase mb-1.5">Model</label>
                  <input v-model="model" type="text" class="w-full px-3 py-2 bg-surface-hover border-border-soft rounded-xl text-sm font-bold text-text outline-none focus:ring-4 focus:ring-indigo-500/10" placeholder="default">
                </div>
                <div v-if="translateEngine === 'custom_llm'" class="md:col-span-2 min-w-0">
                  <label class="block text-[11px] font-extrabold text-text-muted uppercase mb-1.5">Custom API URL</label>
                  <input v-model="customApiUrl" type="text" class="w-full px-3 py-2 bg-surface-hover border-border-soft rounded-xl text-sm font-bold text-text outline-none focus:ring-4 focus:ring-indigo-500/10" placeholder="https://example.com/v1/chat/completions">
                </div>
              </div>

              <div>
                <label class="block text-[11px] font-extrabold text-text-muted uppercase mb-1.5">Prompt</label>
                <textarea
                  v-model="prompt"
                  rows="2"
                  class="w-full resize-none px-3 py-2 bg-surface-hover border-border-soft rounded-xl text-sm font-bold text-text outline-none focus:ring-4 focus:ring-indigo-500/10"
                  placeholder="Return only the translated text."
                />
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
                <component :is="isOtherRecording ? Square : Ear" :size="15" />
                <span>{{ isOtherRecording ? '停止监听' : tt('translator.listen_game', '开启游戏语音监听') }}</span>
              </button>
            </div>

            <div class="space-y-4">
              <div>
                <label class="block text-[11px] font-extrabold text-text-muted uppercase mb-1.5">STT Engine</label>
                <CustomSelect v-model="otherEngine" :options="speakerEngineOptions" />
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
                文本翻译
              </h3>
              <button
                :disabled="!canTranslate"
                class="px-4 py-2 rounded-xl bg-primary hover:bg-primary-hover disabled:opacity-50 disabled:cursor-not-allowed text-white font-extrabold text-sm flex items-center gap-2 transition-all active:scale-95"
                @click="translateManual"
              >
                <component :is="isTranslating ? RefreshCw : Send" :class="{ 'animate-spin': isTranslating }" :size="16" />
                翻译并发送
              </button>
            </div>
            <textarea
              v-model="manualText"
              rows="4"
              class="w-full resize-none px-4 py-3 bg-surface-hover border-border-soft rounded-xl text-sm font-bold text-text outline-none focus:ring-4 focus:ring-indigo-500/10"
              placeholder="输入要翻译到 VRChat 的文字..."
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
              <p v-else class="text-emerald-700 font-black text-lg whitespace-pre-wrap leading-relaxed break-words">
                {{ translatedText }}
              </p>
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
              翻译历史
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
              <p class="text-sm font-black text-emerald-700 line-clamp-2 break-words mt-1">{{ record.translated }}</p>
              <div class="flex items-center gap-2 mt-2 text-[11px] font-bold text-text-muted">
                <span>{{ record.source_lang }} -> {{ record.target_lang }}</span>
                <span v-if="record.sent_osc" class="text-emerald-600">OSC</span>
              </div>
            </button>
            <p v-if="history.length === 0" class="text-sm text-border-strong font-bold text-center py-10">
              暂无历史记录
            </p>
          </div>
        </aside>
      </div>
    </div>
  </div>
</template>
