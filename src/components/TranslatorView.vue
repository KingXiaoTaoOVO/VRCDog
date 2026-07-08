<script setup lang="ts">
import CustomSelect from './CustomSelect.vue';
import { ref, onMounted, onUnmounted } from 'vue';
import { SysApi, VrctApi } from "../api";
import { Mic, MicOff, Languages, Send, RefreshCw, Volume2, MonitorUp, Headphones, Ear, Lock, Settings } from 'lucide-vue-next';
import { WebviewWindow } from '@tauri-apps/api/webviewWindow';
import { emit, listen } from '@tauri-apps/api/event';
import { useI18n } from 'vue-i18n';

const { t } = useI18n();

const isRecording = ref(false);
const recognizedText = ref('');
const translatedText = ref('');
const isTranslating = ref(false);
const errorMsg = ref('');
const isOverlayOpen = ref(false);

const sourceLang = ref('zh-CN');
const targetLang = ref('en');

// Others translation refs
const otherEngine = ref('cloud');
const otherSourceLang = ref('en-US');
const otherTargetLang = ref('zh-CN');
const isOtherRecording = ref(false);
const pluginDownloadProgress = ref(0);

const autoSendOsc = ref(true);
const autoPlayTts = ref(true);
const showOriginalOsc = ref(true);

// TTS Configuration
const ttsEngine = ref<'system' | 'gpt_sovits'>('system');
const gptSovitsUrl = ref('http://127.0.0.1:9880');

// Speech Recognition setup
let recognition: any = null;

onMounted(() => {
  const SpeechRecognition = (window as any).SpeechRecognition || (window as any).webkitSpeechRecognition;
  if (SpeechRecognition) {
    recognition = new SpeechRecognition();
    recognition.continuous = true;
    recognition.interimResults = true;
    
    recognition.onresult = (event: any) => {
      let finalTranscript = '';
      let interimTranscript = '';

      for (let i = event.resultIndex; i < event.results.length; ++i) {
        if (event.results[i].isFinal) {
          finalTranscript += event.results[i][0].transcript;
        } else {
          interimTranscript += event.results[i][0].transcript;
        }
      }
      
      if (finalTranscript) {
        recognizedText.value = finalTranscript;
        translateText(finalTranscript);
      } else {
        recognizedText.value = interimTranscript;
      }
    };

    recognition.onerror = (event: any) => {
      console.error('Speech recognition error', event.error);
      if (event.error !== 'no-speech') {
        errorMsg.value = `Speech Recognition Error: ${event.error}`;
      }
      isRecording.value = false;
    };
    
    recognition.onend = () => {
      if (isRecording.value) {
        try {
          recognition.start(); // keep alive
        } catch (e) {}
      }
    };
  } else {
    errorMsg.value = t('auto_43b1967a');
  }

  // Listen to Python audio events
  listen('audio-capture-event', async (event: any) => {
    const payload = event.payload;
    if (payload.type === 'error') {
      errorMsg.value = `Audio Capture Error: ${payload.message}`;
    } else if (payload.type === 'status') {
      console.log('Audio Status:', payload.message, payload.device || '');
      if (payload.message === 'starting') {
         errorMsg.value = `Listening Ready: ${payload.device}`;
         setTimeout(() => errorMsg.value = '', 3000);
      }
    } else if (payload.type === 'result') {
      // 别人说话的结果处理
      const sourceText = payload.text;
      if (!sourceText?.trim()) return;
      const result = await handleVrctMessage(
        sourceText,
        'speaker',
        otherSourceLang.value,
        otherTargetLang.value,
        otherEngine.value,
        false,
      );
      if (isOverlayOpen.value) {
        emit('cmd-update-translation', {
          original: result.original,
          translated: result.translated,
          isSelf: false
        });
      }
        
        // 发送到覆盖层 (Overlay)
        
    }
  });
});

onUnmounted(async () => {
  if (recognition && isRecording.value) {
    recognition.stop();
  }
  if (isOtherRecording.value) {
    try {
      await SysApi.stopAudioCapture();
    } catch (e) {}
  }
});

const toggleRecording = () => {
  if (!recognition) return;
  
  errorMsg.value = '';
  if (isRecording.value) {
    recognition.stop();
    isRecording.value = false;
  } else {
    recognition.lang = sourceLang.value;
    try {
      recognition.start();
      isRecording.value = true;
    } catch (e) {
      console.error(e);
    }
  }
};

const toggleOtherRecording = async () => {
  errorMsg.value = '';
  
  if (isOtherRecording.value) {
    try {
      await SysApi.stopAudioCapture();
    } catch (e: any) {
      console.warn("Stop capture error", e);
    }
    isOtherRecording.value = false;
    pluginDownloadProgress.value = 0;
    return;
  }

  if (otherEngine.value === 'local') {
    isOtherRecording.value = true;
    try {
      await SysApi.startAudioCapture({ sourceLang: otherSourceLang.value, engine: 'local' });
      errorMsg.value = t('translator.capture_starting_local') || t('auto_a9f0e70f');
    } catch (e: any) {
      const errMsg = e.message || e;
      if (errMsg.includes('WASAPI') || errMsg.includes('loopback')) {
        errorMsg.value = t('translator.capture_error_wasapi') || t('auto_67019728');
      } else {
        errorMsg.value = t('translator.capture_error_local', { err: errMsg });
      }
      isOtherRecording.value = false;
    }
  } else {
    // 云端极速引擎
    try {
      await SysApi.startAudioCapture({ sourceLang: otherSourceLang.value, engine: 'cloud' });
      isOtherRecording.value = true;
    } catch (e: any) {
      const errMsg = e.message || e;
      if (errMsg.includes('WASAPI') || errMsg.includes('loopback')) {
        errorMsg.value = t('translator.capture_error_wasapi') || t('auto_67019728');
      } else {
        errorMsg.value = t('translator.capture_error_cloud', { err: errMsg });
      }
      isOtherRecording.value = false;
    }
  }
};
const playTts = async (text: string) => {
  if (ttsEngine.value === 'system') {
    if (!('speechSynthesis' in window)) return;
    const utterance = new SpeechSynthesisUtterance(text);
    utterance.lang = targetLang.value;
    utterance.rate = 1.0;
    window.speechSynthesis.speak(utterance);
  } else if (ttsEngine.value === 'gpt_sovits') {
    try {
      let lang = 'zh';
      if (targetLang.value.includes('en')) lang = 'en';
      if (targetLang.value.includes('ja')) lang = 'ja';
      if (targetLang.value.includes('ko')) lang = 'ko';

      const url = `${gptSovitsUrl.value.replace(/\/$/, '')}/?text=${encodeURIComponent(text)}&text_language=${lang}`;
      const audio = new Audio(url);
      await audio.play();
    } catch (e: any) {
      errorMsg.value = t('translator.tts_error', { err: e.message });
    }
  }
};

const sendToChatbox = async (text: string) => {
  try {
    await SysApi.sendOscChatbox({ text, complete: true });
  } catch (e: any) {
    console.error('OSC Error:', e);
    errorMsg.value = t('translator.osc_error', { err: e.message });
  }
};

const translateEngine = ref('google_free'); // 'google_free', 'deepl', 'baidu'...

const backendService = (engine: string) => engine; // 直接使用引擎标识，不再做映射

const engineOptions = [
  { label: '🌐 Google Translate (Free)', value: 'google_free' },
  { label: '🆓 DeepL Free', value: 'deepl_free' },
  { label: '💎 DeepL Pro', value: 'deepl' },
  { label: '📘 Microsoft / Bing', value: 'microsoft' },
  { label: '🐻 Baidu Translate', value: 'baidu' },
  { label: '🇰🇷 Papago (Naver)', value: 'papago' },
  { label: '🧠 Gemini (Google)', value: 'gemini' },
  { label: '🔵 Tencent Translate', value: 'tencent' },
  { label: '🤖 DeepSeek', value: 'deepseek' },
  { label: '🟢 OpenAI (GPT)', value: 'openai' },
  { label: '💜 SiliconFlow', value: 'siliconflow' },
  { label: '🌙 Moonshot', value: 'moonshot' },
  { label: '🟡 ZhiPu (GLM)', value: 'zhipu' },
  { label: '⚡ Groq', value: 'groq' },
  { label: '🔗 OpenRouter', value: 'openrouter' },
  { label: '🖥️ Ollama (Local)', value: 'ollama' },
  { label: '💻 LM Studio (Local)', value: 'lmstudio' },
  { label: '🔧 Custom LLM', value: 'custom_llm' },
];

const handleVrctMessage = async (
  text: string,
  source: 'chat' | 'mic' | 'speaker',
  sourceLanguage: string,
  targetLanguage: string,
  service: string,
  sendOsc: boolean,
) => {
  const result = await VrctApi.processMessage({
    req: {
      text,
      source,
      source_lang: sourceLanguage,
      target_lang: targetLanguage,
      service: backendService(service),
      send_osc: sendOsc,
      complete: true,
      notification: false,
      update_overlay: true,
      show_original_in_osc: showOriginalOsc.value,
    }
  });
  recognizedText.value = result.original || text;
  translatedText.value = result.translated || '';
  return result;
};

const translateText = async (text: string) => {
  if (!text.trim()) return;
  
  isTranslating.value = true;
  try {
    const result = await handleVrctMessage(
      text,
      'mic',
      sourceLang.value,
      targetLang.value,
      translateEngine.value,
      autoSendOsc.value,
    );

    if (result.translated) {
      if (autoPlayTts.value) {
        playTts(result.translated);
      }
      
      // Emit to overlay
      emit('translation-log', {
        type: 'self',
        text: result.original,
        translation: result.translated
      });
    }
  } catch (error) {
    console.error('Translation error:', error);
    errorMsg.value = t('translator.network_error') || t('auto_3d4b2cd9');
  } finally {
    isTranslating.value = false;
  }
};

let overlayWebview: any = null;

const toggleOverlay = async () => {
  if (isOverlayOpen.value) {
    try {
      await emit('cmd-close-overlay');
      
      // Fallback
      setTimeout(async () => {
        const { WebviewWindow } = await import('@tauri-apps/api/webviewWindow');
        const win = await WebviewWindow.getByLabel('translation-overlay');
        if (win) {
          try { await win.destroy(); } catch(e){}
        }
      }, 300);
      
    } catch (e: any) {
      console.warn('Destroy overlay error:', e);
    } finally {
      isOverlayOpen.value = false;
      overlayWebview = null;
    }
  } else {
    try {
      overlayWebview = new WebviewWindow('translation-overlay', {
        url: '/?mode=overlay',
        title: t('translator.overlay_title') || t('auto_e99d7346'),
        transparent: true,
        decorations: false,
        alwaysOnTop: true,
        width: 400,
        height: 300,
        x: 50,
        y: 50,
        skipTaskbar: true
      });
      
      overlayWebview.once('tauri://created', function () {
        isOverlayOpen.value = true;
      });
      
      overlayWebview.once('tauri://error', function (e: any) {
        console.warn('Webview creation error or already exists:', e);
        errorMsg.value = t('translator.overlay_fail', { error: JSON.stringify(e) });
        isOverlayOpen.value = false;
        overlayWebview = null;
      });
      
      overlayWebview.onCloseRequested(() => {
        isOverlayOpen.value = false;
        overlayWebview = null;
      });
    } catch (e: any) {
      console.warn(e);
      errorMsg.value = t('translator.exception', { error: e.message || JSON.stringify(e) });
      isOverlayOpen.value = false;
    }
  }
};

const manualSend = () => {
  if (translatedText.value) {
    const oscPayload = showOriginalOsc.value && recognizedText.value 
        ? `${translatedText.value} (${recognizedText.value})` 
        : translatedText.value;
    sendToChatbox(oscPayload);
  }
};
const manualPlay = () => {
  if (translatedText.value) {
    playTts(translatedText.value);
  }
};
</script>

<template>
  <div class="h-full flex flex-col p-6 bg-surface-hover rounded-3xl relative overflow-hidden">
    <!-- Subtle Background Glow -->
    <div class="absolute top-0 right-0 w-96 h-96 bg-primary/10 rounded-full blur-[100px] pointer-events-none -z-10" />
    <div class="absolute bottom-0 left-0 w-[500px] h-[500px] bg-blue-500/5 rounded-full blur-[120px] pointer-events-none -z-10" />

    <header class="flex items-center justify-between mb-6 shrink-0 z-10">
      <h2 class="text-3xl font-extrabold text-text flex items-center gap-3 tracking-tight">
        <span class="inline-flex items-center justify-center p-2 bg-primary/10 rounded-2xl shadow-sm border-primary">
          <Languages class="w-6 h-6 text-primary" />
        </span>
        {{ t('translator.title') }}
      </h2>
      <button
        :class="isOverlayOpen ? 'bg-red-500 hover:bg-red-600 shadow-red-500/30 border-red-500 text-white' : 'bg-surface hover:bg-surface-hover text-text-muted hover:text-primary border-border-soft shadow-sm'" 
        class="px-5 py-2.5 font-bold rounded-xl flex items-center gap-2 transition-all active:scale-95 text-sm"
        @click="toggleOverlay"
      >
        <MonitorUp :size="16" /> 
        {{ isOverlayOpen ? t('translator.overlay_close') : t('translator.overlay_open') }}
      </button>
    </header>

    <div class="flex-1 flex flex-col overflow-y-auto custom-scrollbar pr-2 z-10 relative">
      <!-- 功能开关与 TTS 配置 (置顶) -->
      <div class="flex flex-col gap-4 mb-5 bg-surface backdrop-blur-xl rounded-3xl p-5 border-border-strong shadow-sm shrink-0">
        <div class="flex flex-wrap items-center gap-6">
          <label class="flex items-center gap-2 cursor-pointer group">
            <input
              v-model="autoSendOsc"
              type="checkbox"
              class="w-4 h-4 text-primary rounded focus:ring-indigo-500 border-border-soft"
            >
            <span class="text-sm font-bold text-text-muted group-hover:text-primary transition-colors">{{ t('translator.auto_osc') }}</span>
          </label>
          <label class="flex items-center gap-2 cursor-pointer group">
            <input
              v-model="showOriginalOsc"
              type="checkbox"
              class="w-4 h-4 text-primary rounded focus:ring-indigo-500 border-border-soft"
            >
            <span class="text-sm font-bold text-text-muted group-hover:text-primary transition-colors">{{ t('translator.show_original') }}</span>
          </label>
          <label class="flex items-center gap-2 cursor-pointer group">
            <input
              v-model="autoPlayTts"
              type="checkbox"
              class="w-4 h-4 text-primary rounded focus:ring-indigo-500 border-border-soft"
            >
            <span class="text-sm font-bold text-text-muted group-hover:text-primary transition-colors">{{ t('translator.auto_tts') }}</span>
          </label>
        </div>

        <!-- TTS 高级配置 (当勾选自动语音播报或需要发声时可用) -->
        <div
          v-if="autoPlayTts"
          class="bg-surface-hover p-4 rounded-2xl border-border-soft flex flex-wrap gap-4 items-end animate-fade-in shadow-inner"
        >
          <div>
            <label class="block text-[11px] font-extrabold text-text-muted uppercase tracking-wider mb-1.5 flex items-center gap-1.5">
              <Settings :size="12" /> {{ t('translator.tts_engine_label') }}
            </label>
            <CustomSelect v-model="ttsEngine" :options="[
                  { label: t('translator.tts_system'), value: 'system' },
                  { label: t('translator.tts_gptsovits'), value: 'gpt_sovits' }
                ]" />
          </div>
          
          <div
            v-if="ttsEngine === 'gpt_sovits'"
            class="flex-1 min-w-[250px]"
          >
            <label class="block text-[11px] font-extrabold text-text-muted uppercase tracking-wider mb-1.5">{{ t('translator.gptsovits_url_label') }}</label>
            <input
              v-model="gptSovitsUrl"
              type="text"
              class="w-full px-4 py-2 bg-surface border-border-soft rounded-xl text-sm font-bold text-text outline-none  focus:ring-4 focus:ring-indigo-500/10 transition-all shadow-sm"
              :placeholder="t('translator.gptsovits_url_placeholder')"
            >
          </div>
          
          <div
            v-if="ttsEngine === 'gpt_sovits'"
            class="text-[11px] font-bold text-text-muted max-w-sm ml-auto bg-surface backdrop-blur p-2.5 rounded-xl border-border-soft shadow-sm"
          >
            {{ t('translator.gptsovits_help') }}
          </div>
        </div>
      </div>

      <div
        v-if="errorMsg"
        class="bg-red-50 border-red-200 text-red-600 px-4 py-3 rounded-xl text-sm font-bold flex items-center gap-2 mb-5 shrink-0 shadow-sm"
      >
        {{ errorMsg }}
      </div>

      <!-- 核心操作区: 左右双列布局 -->
      <div class="grid grid-cols-1 md:grid-cols-2 gap-5 flex-1 min-h-0">
        <!-- 左列: 我的语音 (发往 VRChat) -->
        <div class="flex flex-col gap-5 min-h-0">
          <!-- 语言设置 -->
          <div class="bg-surface backdrop-blur-md rounded-3xl p-6 border-border-soft shadow-sm shrink-0">
            <h3 class="font-extrabold text-text mb-4 flex items-center gap-2 text-lg">
              <Mic
                class="text-primary"
                :size="20"
              /> {{ t('translator.my_voice') }}
            </h3>
            <div class="mb-4">
              <label class="block text-[11px] font-extrabold text-text-muted uppercase tracking-wider mb-1.5">{{ t('translator.engine') }}</label>
              <CustomSelect v-model="translateEngine" :options="engineOptions" />
            </div>
            <div class="flex items-center gap-4">
              <div class="flex-1">
                <label class="block text-[11px] font-extrabold text-text-muted uppercase tracking-wider mb-1.5">{{ t('translator.source_lang') }}</label>
                <CustomSelect v-model="sourceLang" :options="[
                  { label: '🇨🇳 zh-CN', value: 'zh-CN' },
                  { label: '🇺🇸 en-US', value: 'en-US' },
                  { label: '🇯🇵 ja-JP', value: 'ja-JP' },
                  { label: '🇰🇷 ko-KR', value: 'ko-KR' }
                ]" />
              </div>
              <div class="pt-5 text-text-muted">
                <RefreshCw :size="16" />
              </div>
              <div class="flex-1">
                <label class="block text-[11px] font-extrabold text-text-muted uppercase tracking-wider mb-1.5">{{ t('translator.target_lang') }}</label>
                <CustomSelect v-model="targetLang" :options="[
                  { label: '🇺🇸 en-US', value: 'en' },
                  { label: '🇨🇳 zh-CN', value: 'zh-CN' },
                  { label: '🇯🇵 ja-JP', value: 'ja' },
                  { label: '🇰🇷 ko-KR', value: 'ko' }
                ]" />
              </div>
            </div>
          </div>

          <!-- 语音输入识别卡片 -->
          <div class="bg-surface backdrop-blur-md rounded-3xl p-6 border-border-soft shadow-sm relative overflow-hidden flex flex-col flex-1 min-h-[250px] group hover:shadow-lg transition-all ">
            <div class="absolute -right-4 -top-4 w-32 h-32 bg-primary/10 rounded-full blur-3xl opacity-50 group-hover:bg-primary/10 transition-colors" />
            <div class="flex justify-between items-center mb-4 relative z-10 shrink-0">
              <h3 class="font-extrabold text-text flex items-center gap-2 text-lg">
                <Mic
                  class="text-primary"
                  :size="20"
                /> {{ t('translator.voice_input') }}
              </h3>
              <button
                :class="isRecording ? 'bg-red-500 hover:bg-red-600 text-white shadow-red-500/30 shadow-lg border-red-500 animate-pulse' : 'bg-surface hover:bg-primary/10 text-text-muted hover:text-primary shadow-sm border-border-soft'" 
                class="px-4 py-2 rounded-xl font-extrabold text-xs flex items-center gap-2 transition-all active:scale-95"
                @click="toggleRecording"
              >
                <component
                  :is="isRecording ? MicOff : Mic"
                  :size="16"
                />
                {{ isRecording ? t('translator.stop_listen') : t('translator.start_listen') }}
              </button>
            </div>
            <div class="flex-1 bg-surface-hover rounded-2xl p-5 border-border-soft relative z-10 overflow-y-auto custom-scrollbar shadow-inner">
              <p
                v-if="!recognizedText && !isRecording"
                class="text-border-strong font-medium italic text-center mt-8 text-sm"
              >
                {{ t('translator.click_to_speak') }}
              </p>
              <p
                v-else-if="!recognizedText && isRecording"
                class="text-primary font-extrabold text-center mt-8 animate-pulse text-sm"
              >
                {{ t('translator.listening') }}
              </p>
              <p
                v-else
                class="text-text font-bold whitespace-pre-wrap leading-relaxed"
              >
                {{ recognizedText }}
              </p>
            </div>
          </div>
        </div>

        <!-- 右列: 他人的翻译 (系统内录) -->
        <div class="flex flex-col gap-5 min-h-0">
          <!-- 引擎与语言设置 -->
          <div class="bg-surface backdrop-blur-md rounded-3xl p-6 border-border-soft shadow-sm shrink-0">
            <div class="flex justify-between items-center mb-4">
              <h3 class="font-extrabold text-text flex items-center gap-2 text-lg">
                <Headphones
                  class="text-emerald-500"
                  :size="20"
                /> {{ t('translator.others_voice') }}
              </h3>
              <div class="flex items-center bg-surface-hover px-3 py-1.5 rounded-xl border-border-soft shadow-inner">
                <span class="text-[11px] font-extrabold text-text-muted uppercase tracking-wider mr-3 flex items-center gap-1.5">
                  <Ear
                    class="text-emerald-500"
                    :size="14"
                  /> {{ t('translator.listen_game') }}
                </span>
                <button
                  :class="isOtherRecording ? 'bg-emerald-500 shadow-md shadow-emerald-500/30' : 'bg-surface'" 
                  class="w-11 h-6 rounded-full relative transition-all duration-300"
                  @click="toggleOtherRecording"
                >
                  <div
                    :class="isOtherRecording ? 'translate-x-5' : 'translate-x-0.5'"
                    class="w-5 h-5 bg-surface rounded-full absolute left-0 top-[2px] transition-transform duration-300 shadow-sm"
                  />
                </button>
              </div>
            </div>
            
            <div class="flex items-center gap-2 mb-4">
              <div class="flex-1">
                <CustomSelect v-model="otherEngine" :options="[
                  { label: t('translator.engine_cloud'), value: 'cloud' },
                  { label: t('translator.engine_local'), value: 'local' }
                ]" />
              </div>
            </div>

            <div class="flex items-center gap-4">
              <div class="flex-1">
                <label class="block text-[11px] font-extrabold text-text-muted uppercase tracking-wider mb-1.5">{{ t('translator.source_lang') }}</label>
                <CustomSelect v-model="otherSourceLang" :options="[
                  { label: '🇺🇸 en-US', value: 'en-US' },
                  { label: '🇯🇵 ja-JP', value: 'ja-JP' },
                  { label: '🇰🇷 ko-KR', value: 'ko-KR' },
                  { label: '🇨🇳 zh-CN', value: 'zh-CN' }
                ]" />
              </div>
              <div class="pt-5 text-text-muted">
                <RefreshCw :size="16" />
              </div>
              <div class="flex-1">
                <label class="block text-[11px] font-extrabold text-text-muted uppercase tracking-wider mb-1.5">{{ t('translator.target_lang') }}</label>
                <CustomSelect v-model="otherTargetLang" :options="[
                  { label: '🇨🇳 zh-CN', value: 'zh-CN' },
                  { label: '🇺🇸 en-US', value: 'en' },
                  { label: '🇯🇵 ja-JP', value: 'ja' },
                  { label: '🇰🇷 ko-KR', value: 'ko' }
                ]" />
              </div>
            </div>
          </div>

          <!-- 翻译输出卡片 -->
          <div class="bg-surface backdrop-blur-md rounded-3xl p-6 border-border-soft shadow-sm relative overflow-hidden flex flex-col flex-1 min-h-[250px] group hover:shadow-lg transition-all ">
            <div class="absolute -right-4 -bottom-4 w-32 h-32 bg-emerald-500/10 rounded-full blur-3xl opacity-50 group-hover:bg-emerald-500/20 transition-colors" />
            
            <div class="flex justify-between items-center mb-4 relative z-10 shrink-0">
              <h3 class="font-extrabold text-text flex items-center gap-2 text-lg">
                <Volume2
                  class="text-emerald-500"
                  :size="20"
                /> {{ t('translator.machine_result') }}
              </h3>
              <span
                v-if="isTranslating"
                class="text-[11px] font-extrabold text-primary flex items-center gap-1.5 bg-primary/10 px-2 py-1 rounded-lg"
              >
                <RefreshCw
                  class="animate-spin"
                  :size="12"
                /> {{ t('translator.translating') }}
              </span>
            </div>
            
            <div class="flex-1 bg-surface-hover rounded-2xl p-5 border-border-soft relative z-10 overflow-y-auto custom-scrollbar shadow-inner">
              <p
                v-if="!translatedText"
                class="text-border-strong font-medium italic text-center mt-8 text-sm"
              >
                {{ t('translator.result_here') }}
              </p>
              <p
                v-else
                class="text-emerald-700 font-black text-lg whitespace-pre-wrap leading-relaxed"
              >
                {{ translatedText }}
              </p>
            </div>
            
            <div class="mt-4 flex gap-3 relative z-10 shrink-0">
              <button
                class="w-12 py-2.5 bg-surface border-border-soft hover:border-emerald-300 hover:text-emerald-600 text-text-muted rounded-xl flex items-center justify-center transition-all shadow-sm active:scale-95"
                @click="manualPlay"
              >
                <Volume2 :size="16" />
              </button>
              <button
                class="flex-1 py-2.5 bg-emerald-500 hover:bg-emerald-600 text-white font-extrabold text-sm rounded-xl flex items-center justify-center gap-2 transition-all shadow-md shadow-emerald-500/30 active:scale-95"
                @click="manualSend"
              >
                <Send :size="16" /> {{ t('translator.manual_send') }}
              </button>
            </div>
          </div>
        </div>
      </div>
      
      <p class="text-xs text-border-strong font-bold text-center mt-6">
        {{ t('translator.usage') }}
      </p>
    </div>
  </div>
</template>
