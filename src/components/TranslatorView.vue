<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
import { SysApi } from "../api";
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
        errorMsg.value = `语音识别错误: ${event.error}`;
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
    errorMsg.value = '您的系统/浏览器不支持 Web Speech API。';
  }

  // Listen to Python audio events
  listen('audio-capture-event', async (event: any) => {
    const payload = event.payload;
    if (payload.type === 'error') {
      errorMsg.value = `音频截获错误: ${payload.message}`;
    } else if (payload.type === 'status') {
      console.log('Audio Status:', payload.message, payload.device || '');
      if (payload.message === 'starting') {
         errorMsg.value = `监听已就绪: ${payload.device}`;
         setTimeout(() => errorMsg.value = '', 3000);
      }
    } else if (payload.type === 'result') {
      // 别人说话的结果处理
      const sourceText = payload.text;
      recognizedText.value = sourceText;
      
      try {
        const res = await fetch(`https://translate.googleapis.com/translate_a/single?client=gtx&sl=${otherSourceLang.value}&tl=${otherTargetLang.value}&dt=t&q=${encodeURIComponent(sourceText)}`);
        const data = await res.json();
        const translated = data[0].map((item: any) => item[0]).join('');
        translatedText.value = translated;
        
        // 发送到覆盖层 (Overlay)
        if (isOverlayOpen.value) {
          emit('cmd-update-translation', {
            original: sourceText,
            translated: translated,
            isSelf: false
          });
        }
        
      } catch (e: any) {
        console.error('Translation failed', e);
      }
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
      errorMsg.value = t('translator.capture_starting_local') || '提示: 本地离线引擎启动中...';
    } catch (e: any) {
      const errMsg = e.message || e;
      if (errMsg.includes('WASAPI') || errMsg.includes('loopback')) {
        errorMsg.value = t('translator.capture_error_wasapi') || `音频截获失败: 系统当前未播放任何声音，请随便播放一段声音(如BGM)后再重试。`;
      } else {
        errorMsg.value = t('translator.capture_error_local', { err: errMsg }) || `本地引擎启动失败: ${errMsg}`;
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
        errorMsg.value = t('translator.capture_error_wasapi') || `音频截获失败: 系统当前未播放任何声音，请随便播放一段声音(如BGM)后再重试。`;
      } else {
        errorMsg.value = t('translator.capture_error_cloud', { err: errMsg }) || `启动云端引擎失败: ${errMsg}`;
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
      errorMsg.value = t('translator.tts_error', { err: e.message }) || `GPT-SoVITS 语音播放失败: ${e.message}`;
    }
  }
};

const sendToChatbox = async (text: string) => {
  try {
    await SysApi.sendOscChatbox({ text, complete: true });
  } catch (e: any) {
    console.error('OSC Error:', e);
    errorMsg.value = t('translator.osc_error', { err: e.message }) || `无法发送到 VRChat: ${e.message}`;
  }
};

const translateEngine = ref('google'); // 'google', 'deepl', 'baidu'

const translateText = async (text: string) => {
  if (!text.trim()) return;
  
  isTranslating.value = true;
  try {
    let result = '';
    if (translateEngine.value === 'google') {
      const url = `https://translate.googleapis.com/translate_a/single?client=gtx&sl=${sourceLang.value.split('-')[0]}&tl=${targetLang.value.split('-')[0]}&dt=t&q=${encodeURIComponent(text)}`;
      const response = await fetch(url);
      const data = await response.json();
      if (data && data[0]) {
        result = data[0].map((item: any) => item[0]).join('');
      }
    } else {
      // Placeholder for DeepL or other engines
      result = `[${translateEngine.value} 模拟] ` + text;
    }

    if (result) {
      translatedText.value = result;
      
      if (autoSendOsc.value) {
        const oscPayload = showOriginalOsc.value ? `${result} (${text})` : result;
        await sendToChatbox(oscPayload);
      }
      if (autoPlayTts.value) {
        playTts(result);
      }
      
      // Emit to overlay
      emit('translation-log', {
        type: 'self',
        text: text,
        translation: result
      });
    }
  } catch (error) {
    console.error('Translation error:', error);
    errorMsg.value = t('translator.network_error') || '翻译请求失败。请检查网络。';
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
        title: t('translator.overlay_title') || '翻译悬浮窗',
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
        errorMsg.value = `悬浮窗创建失败: ${JSON.stringify(e)}`;
        isOverlayOpen.value = false;
        overlayWebview = null;
      });
      
      overlayWebview.onCloseRequested(() => {
        isOverlayOpen.value = false;
        overlayWebview = null;
      });
    } catch (e: any) {
      console.warn(e);
      errorMsg.value = `出现异常: ${e.message || JSON.stringify(e)}`;
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
  <div class="space-y-6 max-w-4xl mx-auto">
    <div class="flex items-center justify-between mb-6">
      <h2 class="text-2xl font-extrabold text-[#451a03] flex items-center gap-2">
        <Languages
          class="text-amber-500"
          :size="24"
        /> {{ t('translator.title') }}
      </h2>
      <button
        :class="isOverlayOpen ? 'bg-red-500 hover:bg-red-600 shadow-red-500/20' : 'bg-amber-500 hover:bg-amber-600 shadow-amber-500/20'" 
        class="px-4 py-2 text-white font-bold rounded-xl flex items-center gap-2 shadow-md transition-all"
        @click="toggleOverlay"
      >
        <MonitorUp :size="16" /> 
        {{ isOverlayOpen ? t('translator.overlay_close') : t('translator.overlay_open') }}
      </button>
    </div>

    <!-- 功能开关与 TTS 配置 (置顶) -->
    <div class="flex flex-col gap-4 mb-4 bg-white/60 backdrop-blur rounded-2xl p-4 border border-amber-100 shadow-sm">
      <div class="flex flex-wrap items-center gap-6">
        <label class="flex items-center gap-2 cursor-pointer">
          <input
            v-model="autoSendOsc"
            type="checkbox"
            class="w-4 h-4 text-amber-600 rounded focus:ring-amber-500 border-gray-300"
          >
          <span class="text-sm font-bold text-amber-900">{{ t('translator.auto_osc') }}</span>
        </label>
        <label class="flex items-center gap-2 cursor-pointer">
          <input
            v-model="showOriginalOsc"
            type="checkbox"
            class="w-4 h-4 text-amber-600 rounded focus:ring-amber-500 border-gray-300"
          >
          <span class="text-sm font-bold text-amber-900">{{ t('translator.show_original') }}</span>
        </label>
        <label class="flex items-center gap-2 cursor-pointer">
          <input
            v-model="autoPlayTts"
            type="checkbox"
            class="w-4 h-4 text-amber-600 rounded focus:ring-amber-500 border-gray-300"
          >
          <span class="text-sm font-bold text-amber-900">{{ t('translator.auto_tts') }}</span>
        </label>
      </div>

      <!-- TTS 高级配置 (当勾选自动语音播报或需要发声时可用) -->
      <div
        v-if="autoPlayTts"
        class="bg-amber-50/80 p-4 rounded-xl border border-amber-200 flex flex-wrap gap-4 items-end animate-fade-in"
      >
        <div>
          <label class="block text-xs font-bold text-amber-800 mb-1 flex items-center gap-1">
            <Settings :size="12" /> {{ t('translator.tts_engine_label') }}
          </label>
          <select
            v-model="ttsEngine"
            class="w-48 px-3 py-1.5 bg-white border border-amber-200 rounded-lg text-sm font-bold outline-none focus:border-amber-400"
          >
            <option value="system">
              {{ t('translator.tts_system') }}
            </option>
            <option value="gpt_sovits">
              {{ t('translator.tts_gptsovits') }}
            </option>
          </select>
        </div>
        
        <div
          v-if="ttsEngine === 'gpt_sovits'"
          class="flex-1 min-w-[250px]"
        >
          <label class="block text-xs font-bold text-amber-800 mb-1">{{ t('translator.gptsovits_url_label') }}</label>
          <input
            v-model="gptSovitsUrl"
            type="text"
            class="w-full px-3 py-1.5 bg-white border border-amber-200 rounded-lg text-sm font-medium outline-none focus:border-amber-400"
            :placeholder="t('translator.gptsovits_url_placeholder')"
          >
        </div>
        
        <div
          v-if="ttsEngine === 'gpt_sovits'"
          class="text-xs text-amber-700 max-w-sm ml-auto bg-amber-100/50 p-2 rounded-lg border border-amber-100"
        >
          {{ t('translator.gptsovits_help') }}
        </div>
      </div>
    </div>

    <div
      v-if="errorMsg"
      class="bg-red-50 border border-red-200 text-red-600 px-4 py-3 rounded-xl text-sm font-bold flex items-center gap-2 mb-4"
    >
      {{ errorMsg }}
    </div>

    <!-- 核心操作区: 左右双列布局 -->
    <div class="grid grid-cols-1 md:grid-cols-2 gap-4 flex-1 min-h-0">
      <!-- 左列: 我的语音 (发往 VRChat) -->
      <div class="flex flex-col gap-4 min-h-0">
        <!-- 语言设置 -->
        <div class="bg-white/90 backdrop-blur rounded-2xl p-5 border-2 border-amber-200 shadow-sm shrink-0">
          <h3 class="font-extrabold text-amber-900 mb-3 flex items-center gap-2">
            <Mic
              class="text-green-500"
              :size="18"
            /> {{ t('translator.my_voice') }}
          </h3>
          <div class="mb-4">
            <label class="block text-[10px] font-bold text-amber-800 mb-1">{{ t('translator.engine') || '翻译引擎' }}</label>
            <select
              v-model="translateEngine"
              class="w-full px-2 py-1.5 bg-amber-50/50 border-2 border-amber-100 rounded-lg text-sm font-bold outline-none focus:border-amber-400"
            >
              <option value="google">Google 翻译</option>
              <option value="deepl">DeepL 翻译</option>
              <option value="baidu">百度翻译</option>
            </select>
          </div>
          <div class="flex items-center gap-4">
            <div class="flex-1">
              <label class="block text-[10px] font-bold text-amber-800 mb-1">{{ t('translator.source_lang') }}</label>
              <select
                v-model="sourceLang"
                class="w-full px-2 py-1.5 bg-amber-50/50 border-2 border-amber-100 rounded-lg text-sm font-bold outline-none focus:border-amber-400"
              >
                <option value="zh-CN">
                  🇨🇳 中文
                </option>
                <option value="en-US">
                  🇺🇸 英文
                </option>
                <option value="ja-JP">
                  🇯🇵 日文
                </option>
                <option value="ko-KR">
                  🇰🇷 韩文
                </option>
              </select>
            </div>
            <div class="pt-4 text-amber-300">
              <RefreshCw :size="16" />
            </div>
            <div class="flex-1">
              <label class="block text-[10px] font-bold text-amber-800 mb-1">{{ t('translator.target_lang') }}</label>
              <select
                v-model="targetLang"
                class="w-full px-2 py-1.5 bg-amber-50/50 border-2 border-amber-100 rounded-lg text-sm font-bold outline-none focus:border-amber-400"
              >
                <option value="en">
                  🇺🇸 英文
                </option>
                <option value="zh-CN">
                  🇨🇳 中文
                </option>
                <option value="ja">
                  🇯🇵 日文
                </option>
                <option value="ko">
                  🇰🇷 韩文
                </option>
              </select>
            </div>
          </div>
        </div>

        <!-- 语音输入识别卡片 -->
        <div class="bg-white/90 backdrop-blur rounded-2xl p-5 border-2 border-amber-200 shadow-sm relative overflow-hidden flex flex-col flex-1 min-h-0">
          <div class="absolute -right-4 -top-4 w-32 h-32 bg-amber-100 rounded-full blur-3xl opacity-50" />
          <div class="flex justify-between items-center mb-3 relative z-10 shrink-0">
            <h3 class="font-bold text-[15px] text-amber-900 flex items-center gap-2">
              <Mic
                class="text-amber-500"
                :size="18"
              /> {{ t('translator.voice_input') }}
            </h3>
            <button
              :class="isRecording ? 'bg-red-500 hover:bg-red-600 text-white shadow-red-500/20 animate-pulse' : 'bg-amber-500 hover:bg-amber-600 text-white shadow-amber-500/20'" 
              class="px-3 py-1.5 rounded-full font-bold text-xs flex items-center gap-1.5 shadow-md transition-all"
              @click="toggleRecording"
            >
              <component
                :is="isRecording ? MicOff : Mic"
                :size="14"
              />
              {{ isRecording ? t('translator.stop_listen') : t('translator.start_listen') }}
            </button>
          </div>
          <div class="flex-1 bg-amber-50/50 rounded-xl p-4 border border-amber-100 relative z-10 overflow-y-auto">
            <p
              v-if="!recognizedText && !isRecording"
              class="text-amber-400/50 text-sm font-medium italic text-center mt-6"
            >
              {{ t('translator.click_to_speak') }}
            </p>
            <p
              v-else-if="!recognizedText && isRecording"
              class="text-amber-500 text-sm font-bold text-center mt-6 animate-pulse"
            >
              {{ t('translator.listening') }}
            </p>
            <p
              v-else
              class="text-amber-900 font-medium whitespace-pre-wrap"
            >
              {{ recognizedText }}
            </p>
          </div>
        </div>
      </div>

      <!-- 右列: 他人的翻译 (系统内录) -->
      <div class="flex flex-col gap-4 min-h-0">
        <!-- 引擎与语言设置 -->
        <div class="bg-white/90 backdrop-blur rounded-2xl p-5 border-2 border-blue-200 shadow-sm shrink-0">
          <div class="flex justify-between items-center mb-3">
            <h3 class="font-extrabold text-blue-900 flex items-center gap-2">
              <Headphones
                class="text-blue-500"
                :size="18"
              /> {{ t('translator.others_voice') }}
            </h3>
            <div class="flex items-center bg-blue-50/50 px-2 py-1 rounded-xl border border-blue-100">
              <span class="text-[10px] font-bold text-blue-800 mr-2 flex items-center gap-1">
                <Ear
                  class="text-blue-500"
                  :size="12"
                /> {{ t('translator.listen_game') }}
              </span>
              <button
                :class="isOtherRecording ? 'bg-blue-500 shadow-lg shadow-blue-500/30' : 'bg-blue-200'" 
                class="w-10 h-5 rounded-full relative transition-all duration-300"
                @click="toggleOtherRecording"
              >
                <div
                  :class="isOtherRecording ? 'translate-x-5' : 'translate-x-0'"
                  class="w-3.5 h-3.5 bg-white rounded-full absolute left-1 top-[3px] transition-transform duration-300"
                />
              </button>
            </div>
          </div>
          
          <div class="flex items-center gap-2 mb-3">
            <div class="flex-1">
              <select
                v-model="otherEngine"
                class="w-full px-2 py-1.5 bg-blue-50/50 border-2 border-blue-100 rounded-lg text-xs font-bold outline-none focus:border-blue-400"
              >
                <option value="cloud">
                  {{ t('translator.engine_cloud') }}
                </option>
                <option value="local">
                  {{ t('translator.engine_local') }}
                </option>
              </select>
            </div>
          </div>

          <div class="flex items-center gap-4">
            <div class="flex-1">
              <label class="block text-[10px] font-bold text-blue-800 mb-1">{{ t('translator.source_lang') }}</label>
              <select
                v-model="otherSourceLang"
                class="w-full px-2 py-1.5 bg-blue-50/50 border-2 border-blue-100 rounded-lg text-sm font-bold outline-none focus:border-blue-400"
              >
                <option value="en-US">
                  🇺🇸 英文
                </option>
                <option value="ja-JP">
                  🇯🇵 日文
                </option>
                <option value="ko-KR">
                  🇰🇷 韩文
                </option>
                <option value="zh-CN">
                  🇨🇳 中文
                </option>
              </select>
            </div>
            <div class="pt-4 text-blue-300">
              <RefreshCw :size="16" />
            </div>
            <div class="flex-1">
              <label class="block text-[10px] font-bold text-blue-800 mb-1">{{ t('translator.target_lang') }}</label>
              <select
                v-model="otherTargetLang"
                class="w-full px-2 py-1.5 bg-blue-50/50 border-2 border-blue-100 rounded-lg text-sm font-bold outline-none focus:border-blue-400"
              >
                <option value="zh-CN">
                  🇨🇳 中文
                </option>
                <option value="en">
                  🇺🇸 英文
                </option>
                <option value="ja">
                  🇯🇵 日文
                </option>
                <option value="ko">
                  🇰🇷 韩文
                </option>
              </select>
            </div>
          </div>
        </div>

        <!-- 翻译输出卡片 -->
        <div class="bg-white/90 backdrop-blur rounded-2xl p-5 border-2 border-amber-200 shadow-sm relative overflow-hidden flex flex-col flex-1 min-h-0 group">
          <div class="absolute -right-4 -bottom-4 w-32 h-32 bg-green-100 rounded-full blur-3xl opacity-50 group-hover:bg-green-200 transition-colors" />
          
          <div class="flex justify-between items-center mb-3 relative z-10 shrink-0">
            <h3 class="font-bold text-[15px] text-amber-900 flex items-center gap-2">
              <Volume2
                class="text-green-500"
                :size="18"
              /> {{ t('translator.machine_result') }}
            </h3>
            <span
              v-if="isTranslating"
              class="text-[10px] font-bold text-amber-500 flex items-center gap-1"
            >
              <RefreshCw
                class="animate-spin"
                :size="12"
              /> {{ t('translator.translating') }}
            </span>
          </div>
          
          <div class="flex-1 bg-amber-50/50 rounded-xl p-4 border border-amber-100 relative z-10 overflow-y-auto">
            <p
              v-if="!translatedText"
              class="text-amber-400/50 text-sm font-medium italic text-center mt-6"
            >
              {{ t('translator.result_here') }}
            </p>
            <p
              v-else
              class="text-green-700 font-bold text-lg whitespace-pre-wrap"
            >
              {{ translatedText }}
            </p>
          </div>
          
          <div class="mt-3 flex gap-2 relative z-10 shrink-0">
            <button
              class="w-10 py-1.5 bg-amber-100 hover:bg-amber-200 text-amber-700 rounded-xl flex items-center justify-center transition-colors"
              @click="manualPlay"
            >
              <Volume2 :size="14" />
            </button>
            <button
              class="flex-1 py-1.5 bg-green-500 hover:bg-green-600 text-white font-bold text-sm rounded-xl flex items-center justify-center gap-2 transition-colors shadow-md shadow-green-500/20"
              @click="manualSend"
            >
              <Send :size="14" /> {{ t('translator.manual_send') }}
            </button>
          </div>
        </div>
      </div>
    </div>
    
    <p class="text-xs text-amber-600/70 font-medium text-center">
      {{ t('translator.usage') }}
    </p>
  </div>
</template>
