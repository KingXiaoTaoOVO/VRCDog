<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { Download, Search, Film, QrCode, LogOut, CheckCircle, Loader2, FolderOpen, Trash2, Image, Link, Copy } from 'lucide-vue-next';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { DbApi } from '../api/index';

const { t } = useI18n();

const bvidUrl = ref('');
const videoInfo = ref<any>(null);
const isLoading = ref(false);
const errorMsg = ref('');

// Login State
const isLoggedIn = ref(false);
const showQrModal = ref(false);
const qrCodeUrl = ref('');
const qrKey = ref('');
const qrStatusText = ref('');
let qrPollTimer: any = null;

// Platform Tabs
const activeTab = ref<'bilibili'|'xhs'>('bilibili');

// XHS State
const xhsUrl = ref('');
const xhsInfo = ref<any>(null);

// Download state
const isDownloading = ref(false);
const downloadProgress = ref(0);
const downloadDetail = ref('');

// Toast
const toastMessage = ref('');
const toastType = ref<'success' | 'error' | 'info'>('info');
const showToast = (msg: string, type: 'success' | 'error' | 'info' = 'info') => {
    toastMessage.value = msg;
    toastType.value = type;
    setTimeout(() => toastMessage.value = '', 3000);
};

const tasks = ref<any[]>([]);

const sessdata = ref('');
let unlistenProgress: any = null;

onMounted(async () => {
    // try to load sessdata
    try {
        const stored = await DbApi.getSetting({ key: 'bili_sessdata' });
        if (stored) {
            sessdata.value = stored;
            // verify
            const valid = await invoke<boolean>('bili_check_login', { sessdata: stored });
            isLoggedIn.value = valid;
            if (!valid) {
                sessdata.value = '';
                await DbApi.saveSetting({ key: 'bili_sessdata', value: '' });
            }
        }
    } catch {}
    
    await loadTasks();
    
    unlistenProgress = await listen('bili_task_progress', (event: any) => {
        const payload = event.payload;
        // Update task list
        const idx = tasks.value.findIndex(t => t.id === payload.id);
        if (idx !== -1) {
            tasks.value[idx].status = payload.status;
            tasks.value[idx].progress = payload.progress;
            tasks.value[idx].detail = payload.detail;
        }
        
        // Update current download card if it matches
        if (videoInfo.value && videoInfo.value.bvid === payload.bvid) {
            isDownloading.value = payload.status === 'running';
            downloadProgress.value = payload.progress;
            downloadDetail.value = payload.detail;
            
            if (payload.status === 'done' || payload.status === 'error') {
                setTimeout(() => { isDownloading.value = false; }, 2000);
            }
        }
    });
});

onUnmounted(() => {
    if (qrPollTimer) clearInterval(qrPollTimer);
    if (unlistenProgress) unlistenProgress();
});

const loadTasks = async () => {
    try {
        const res: any = await invoke('db_bili_get_tasks');
        tasks.value = res.map((t: any) => ({ ...t, progress: t.status === 'done' ? 100 : 0, detail: t.status }));
    } catch (e) {
        console.error(e);
    }
};

const openFolder = async (folder: string) => {
    try {
        await invoke('sys_open_dir', { target: folder });
    } catch (e) {
        showToast(t('bilidown.open_folder_fail') + e, 'error');
    }
};

const copyToClipboard = async (text: string, type: string) => {
    try {
        await navigator.clipboard.writeText(text);
        showToast(t('bilidown.copied') + type + t('bilidown.link_to_clipboard') + `${text.substring(0, 50)}...`, 'success');
    } catch (e) {
        showToast(t('bilidown.copy_fail') + e, 'error');
    }
};

const copyStreamUrl = async () => {
    try {
        const res: any = await invoke('bili_get_mp4_play_info', {
            bvid: videoInfo.value.bvid,
            cid: videoInfo.value.cid,
            sessdata: sessdata.value
        });
        
        let videoUrl = '';
        if (res?.data?.durl?.[0]?.url) {
            videoUrl = res.data.durl[0].url;
        } else if (res?.data?.dash?.video?.[0]?.baseUrl) {
            videoUrl = res.data.dash.video[0].baseUrl;
        }
        
        if (videoUrl) {
            await navigator.clipboard.writeText(videoUrl);
            showToast(t('bilidown.copy_mp4_success'), 'success');
        } else {
            showToast(t('bilidown.extract_fail'), 'error');
        }
    } catch (e) {
        showToast(t('bilidown.stream_fail') + e, 'error');
    }
};

const deleteTask = async (id: number) => {
    try {
        await invoke('db_bili_delete_task', { id });
        await loadTasks();
        showToast(t('bilidown.record_deleted'), 'success');
    } catch (e) {
        showToast(t('bilidown.delete_fail') + e, 'error');
    }
};

const generateQrUrl = (url: string) => {
    // Simple google charts qr code generator for now
    return `https://api.qrserver.com/v1/create-qr-code/?size=200x200&data=${encodeURIComponent(url)}`;
};

const openLogin = async () => {
    showQrModal.value = true;
    qrStatusText.value = t('bilidown.qr_fetching');
    try {
        const res: any = await invoke('bili_new_qr');
        if (res.code === 0 && res.data) {
            qrCodeUrl.value = generateQrUrl(res.data.url);
            qrKey.value = res.data.qrcode_key;
            qrStatusText.value = t('bilidown.qr_scan_prompt');
            
            // start polling
            qrPollTimer = setInterval(async () => {
                const pollRes: any = await invoke('bili_get_qr_status', { qrKey: qrKey.value });
                if (pollRes.data) {
                    if (pollRes.data.code === 0) {
                        // success
                        clearInterval(qrPollTimer);
                        qrStatusText.value = t('bilidown.login_success');
                        if (pollRes.sessdata_extracted) {
                            sessdata.value = pollRes.sessdata_extracted;
                            await DbApi.saveSetting({ key: 'bili_sessdata', value: sessdata.value });
                            isLoggedIn.value = true;
                        }
                        setTimeout(() => { showQrModal.value = false; }, 1000);
                    } else if (pollRes.data.code === 86038) {
                        qrStatusText.value = t('bilidown.qr_expired');
                        clearInterval(qrPollTimer);
                    } else if (pollRes.data.code === 86090) {
                        qrStatusText.value = t('bilidown.qr_scanned');
                    }
                }
            }, 3000);
        } else {
            qrStatusText.value = t('bilidown.qr_fail') + res.message;
        }
    } catch (e: any) {
        qrStatusText.value = t('bilidown.error_prefix') + e.toString();
    }
};

const closeLogin = () => {
    showQrModal.value = false;
    if (qrPollTimer) clearInterval(qrPollTimer);
};

const handleLogout = async () => {
    sessdata.value = '';
    isLoggedIn.value = false;
    await DbApi.saveSetting({ key: 'bili_sessdata', value: '' });
};

const extractBvid = (input: string) => {
    const match = input.match(/(BV[1-9A-HJ-NP-Za-km-z]+)/);
    return match ? match[1] : input.trim();
};

const searchVideo = async () => {
    if (!bvidUrl.value) return;
    const bvid = extractBvid(bvidUrl.value);
    isLoading.value = true;
    errorMsg.value = '';
    videoInfo.value = null;
    try {
        const res: any = await invoke('bili_get_video_info', { bvid, sessdata: sessdata.value });
        if (res.code === 0 && res.data) {
            videoInfo.value = res.data;
        } else {
            errorMsg.value = res.message || t('bilidown.fetch_fail');
        }
    } catch (e: any) {
        errorMsg.value = e.toString();
    } finally {
        isLoading.value = false;
    }
};

const searchXhs = async () => {
    if (!xhsUrl.value) return;
    isLoading.value = true;
    errorMsg.value = '';
    xhsInfo.value = null;
    try {
        const res: any = await invoke('xhs_parse_url', { url: xhsUrl.value });
        xhsInfo.value = res;
    } catch (e: any) {
        errorMsg.value = e.toString();
    } finally {
        isLoading.value = false;
    }
};

const openXhsMedia = async (url: string) => {
    try {
        await invoke('sys_open_url', { url });
    } catch (e) {
        showToast(t('bilidown.open_fail') + e, 'error');
    }
};

const downloadVideo = async () => {
    if (!videoInfo.value) return;
    
    isDownloading.value = true;
    downloadProgress.value = 0;
    downloadDetail.value = t('bilidown.prepare_download');
    
    try {
        await invoke('bili_download_video', { 
            bvid: videoInfo.value.bvid, 
            cid: videoInfo.value.cid, 
            title: videoInfo.value.title,
            owner: videoInfo.value.owner.name,
            cover: videoInfo.value.pic,
            sessdata: sessdata.value 
        });
        await loadTasks();
    } catch (e: any) {
        errorMsg.value = t('bilidown.download_error') + e.toString();
        isDownloading.value = false;
    }
};
</script>

<template>
  <div class="h-full flex flex-col p-6 bg-slate-50/50 rounded-3xl relative overflow-hidden">
    <!-- Subtle Background Glow -->
    <div class="absolute top-0 right-0 w-96 h-96 bg-indigo-500/10 rounded-full blur-[100px] pointer-events-none -z-10" />
    <div class="absolute bottom-0 left-0 w-[500px] h-[500px] bg-blue-500/5 rounded-full blur-[120px] pointer-events-none -z-10" />

    <header class="mb-8 flex justify-between items-end shrink-0 z-10">
      <div>
        <h1 class="text-3xl font-extrabold text-slate-900 tracking-tight flex items-center gap-3">
          <span class="inline-flex items-center justify-center p-2 bg-indigo-100 rounded-2xl shadow-sm border border-indigo-200/50">
            <Download class="w-6 h-6 text-indigo-600" />
          </span>
          {{ t('bilidown.tab_title') }}
        </h1>
        <div class="mt-4 flex items-center gap-2 bg-slate-100/50 p-1.5 rounded-xl w-max border border-slate-200/50 shadow-sm">
          <button
            :class="activeTab === 'bilibili' ? 'bg-white shadow-sm text-indigo-600 border border-slate-200' : 'text-slate-500 hover:text-slate-700'"
            class="px-6 py-2 rounded-lg font-bold text-sm transition-all"
            @click="activeTab = 'bilibili'"
          >
            哔哩哔哩
          </button>
          <button
            :class="activeTab === 'xhs' ? 'bg-white shadow-sm text-rose-600 border border-slate-200' : 'text-slate-500 hover:text-slate-700'"
            class="px-6 py-2 rounded-lg font-bold text-sm transition-all"
            @click="activeTab = 'xhs'"
          >
            小红书
          </button>
        </div>
      </div>
      
      <div class="flex items-center gap-4">
        <template v-if="isLoggedIn">
          <div class="flex items-center gap-2 px-4 py-2 bg-green-50 text-green-700 rounded-full border border-green-200">
            <CheckCircle class="w-4 h-4" />
            <span class="text-sm font-bold">{{ t('bilidown.logged_in') }}</span>
          </div>
          <button
            class="p-2 text-red-500 hover:bg-red-50 rounded-full transition-colors"
            @click="handleLogout"
          >
            <LogOut class="w-5 h-5" />
          </button>
        </template>
        <template v-else>
          <button
            class="flex items-center gap-2 px-6 py-2.5 bg-indigo-500 hover:bg-indigo-600 text-white rounded-xl font-bold transition-all shadow-md hover:shadow-lg active:scale-95"
            @click="openLogin"
          >
            <QrCode class="w-5 h-5" />
            扫码登录
          </button>
        </template>
      </div>
    </header>

    <div class="flex-1 overflow-y-auto pr-2 pb-8">
      <div class="bg-white/80 backdrop-blur-md rounded-3xl p-6 border border-slate-200 shadow-sm mb-8 transition-all relative overflow-hidden group">
        <div class="absolute -right-4 -top-4 w-32 h-32 bg-indigo-500/10 rounded-full blur-3xl opacity-50 group-hover:bg-indigo-500/20 transition-colors pointer-events-none" />
        <h2 class="text-lg font-bold text-slate-900 mb-4 flex items-center gap-2">
          <Search
            :class="activeTab === 'xhs' ? 'text-red-500' : 'text-indigo-500'"
            class="w-5 h-5"
          />
          {{ activeTab === 'xhs' ? t('bilidown.parse_xhs') : t('bilidown.parse_bili') }}
        </h2>
        
        <div
          v-if="activeTab === 'bilibili'"
          class="flex gap-4"
        >
          <input 
            v-model="bvidUrl"
            type="text"
            class="flex-1 bg-slate-50 border border-slate-200 rounded-xl px-6 py-4 text-slate-800 font-bold focus:outline-none focus:border-indigo-400 focus:ring-4 focus:ring-indigo-500/10 transition-all text-lg shadow-sm" 
            :placeholder="t('bilidown.placeholder_bili')"
            @keyup.enter="searchVideo"
          >
          <button 
            :disabled="isLoading || !bvidUrl"
            class="bg-indigo-500 hover:bg-indigo-600 disabled:bg-slate-300 disabled:cursor-not-allowed text-white px-8 rounded-2xl font-bold flex items-center justify-center gap-2 transition-all shadow-md active:scale-95 min-w-[140px]"
            @click="searchVideo"
          >
            <Loader2
              v-if="isLoading"
              class="w-6 h-6 animate-spin"
            />
            <template v-else>
              解析获取
            </template>
          </button>
        </div>
        
        <div
          v-else
          class="flex gap-4"
        >
          <input 
            v-model="xhsUrl"
            type="text"
            class="flex-1 bg-slate-50 border border-slate-200 rounded-xl px-6 py-4 text-slate-800 font-bold focus:outline-none focus:border-rose-400 focus:ring-4 focus:ring-rose-500/10 transition-all text-lg shadow-sm" 
            :placeholder="t('bilidown.placeholder_xhs')"
            @keyup.enter="searchXhs"
          >
          <button 
            :disabled="isLoading || !xhsUrl"
            class="bg-red-500 hover:bg-red-600 disabled:bg-slate-300 disabled:cursor-not-allowed text-white px-8 rounded-2xl font-bold flex items-center justify-center gap-2 transition-all shadow-md active:scale-95 min-w-[140px]"
            @click="searchXhs"
          >
            <Loader2
              v-if="isLoading"
              class="w-6 h-6 animate-spin"
            />
            <template v-else>
              解析获取
            </template>
          </button>
        </div>
        
        <div
          v-if="errorMsg"
          class="mt-4 p-4 bg-red-50 text-red-600 rounded-xl border border-red-200 text-sm font-bold"
        >
          {{ errorMsg }}
        </div>
      </div>

      <!-- Result Card -->
      <transition
        enter-active-class="transition-all duration-500 ease-out"
        enter-from-class="opacity-0 translate-y-8"
        enter-to-class="opacity-100 translate-y-0"
      >
        <div
          v-if="activeTab === 'bilibili' && videoInfo"
          class="bg-white rounded-3xl p-8 shadow-xl shadow-slate-900/5 border border-slate-200 flex gap-8"
        >
          <div class="w-1/3 aspect-video rounded-2xl overflow-hidden bg-slate-100 border border-slate-200 relative group flex-shrink-0">
            <img
              :src="videoInfo.pic"
              class="w-full h-full object-cover transition-transform duration-500 group-hover:scale-105"
              referrerpolicy="no-referrer"
            >
            <div class="absolute bottom-2 right-2 bg-black/70 text-white text-xs px-2 py-1 rounded-lg font-mono font-bold backdrop-blur-md">
              {{ Math.floor(videoInfo.duration / 60) }}:{{ (videoInfo.duration % 60).toString().padStart(2, '0') }}
            </div>
          </div>
              
          <div class="flex-1 flex flex-col">
            <h3 class="text-2xl font-bold text-slate-900 leading-snug mb-2 line-clamp-2">
              {{ videoInfo.title }}
            </h3>
                  
            <div class="flex items-center gap-3 mb-6">
              <img
                :src="videoInfo.owner.face"
                class="w-8 h-8 rounded-full border border-slate-200"
                referrerpolicy="no-referrer"
              >
              <span class="font-bold text-slate-700">{{ videoInfo.owner.name }}</span>
              <span class="text-sm text-slate-400 font-medium ml-auto">BVID: {{ videoInfo.bvid }}</span>
            </div>
                  
            <div class="grid grid-cols-4 gap-4 mb-auto">
              <div class="bg-slate-50 rounded-xl p-3 flex flex-col items-center justify-center border border-slate-100">
                <span class="text-slate-400 text-xs font-bold mb-1">{{ t('bilidown.stat_views') }}</span>
                <span class="text-slate-800 font-mono font-bold">{{ videoInfo.stat.view > 10000 ? (videoInfo.stat.view/10000).toFixed(1) + t('bilidown.ten_thousand') : videoInfo.stat.view }}</span>
              </div>
              <div class="bg-slate-50 rounded-xl p-3 flex flex-col items-center justify-center border border-slate-100">
                <span class="text-slate-400 text-xs font-bold mb-1">{{ t('bilidown.stat_likes') }}</span>
                <span class="text-slate-800 font-mono font-bold">{{ videoInfo.stat.like > 10000 ? (videoInfo.stat.like/10000).toFixed(1) + t('bilidown.ten_thousand') : videoInfo.stat.like }}</span>
              </div>
              <div class="bg-slate-50 rounded-xl p-3 flex flex-col items-center justify-center border border-slate-100">
                <span class="text-slate-400 text-xs font-bold mb-1">{{ t('bilidown.stat_coins') }}</span>
                <span class="text-slate-800 font-mono font-bold">{{ videoInfo.stat.coin > 10000 ? (videoInfo.stat.coin/10000).toFixed(1) + t('bilidown.ten_thousand') : videoInfo.stat.coin }}</span>
              </div>
              <div class="bg-slate-50 rounded-xl p-3 flex flex-col items-center justify-center border border-slate-100">
                <span class="text-slate-400 text-xs font-bold mb-1">{{ t('bilidown.stat_favs') }}</span>
                <span class="text-slate-800 font-mono font-bold">{{ videoInfo.stat.favorite > 10000 ? (videoInfo.stat.favorite/10000).toFixed(1) + t('bilidown.ten_thousand') : videoInfo.stat.favorite }}</span>
              </div>
            </div>
                  
            <div class="mt-6 flex items-center gap-4">
              <button 
                :disabled="isDownloading"
                class="flex-1 bg-indigo-600 hover:bg-indigo-700 text-white rounded-2xl py-4 font-bold flex items-center justify-center gap-2 transition-all shadow-md hover:shadow-xl active:scale-95 disabled:bg-slate-300 disabled:cursor-not-allowed disabled:shadow-none"
                @click="downloadVideo"
              >
                <Download class="w-5 h-5" />
                {{ isDownloading ? t('bilidown.downloading') : t('bilidown.download_best') }}
              </button>
            </div>
                  
            <div class="mt-4 flex flex-wrap items-center gap-3">
              <button
                class="flex-1 py-2.5 bg-slate-50 hover:bg-slate-100 text-slate-700 border border-slate-200 rounded-xl font-bold flex items-center justify-center gap-2 transition-all active:scale-95 text-sm"
                @click="copyToClipboard(videoInfo.pic, t('bilidown.cover'))"
              >
                <Image class="w-4 h-4" /> {{ t('bilidown.copy_cover') }}
              </button>
              <button
                class="flex-1 py-2.5 bg-blue-50 hover:bg-blue-100 text-blue-700 border border-blue-200 rounded-xl font-bold flex items-center justify-center gap-2 transition-all active:scale-95 text-sm"
                @click="copyStreamUrl"
              >
                <Link class="w-4 h-4" /> {{ t('bilidown.copy_video') }}
              </button>
            </div>
                  
            <div
              v-if="isDownloading"
              class="mt-4"
            >
              <div class="flex justify-between text-xs font-bold text-slate-500 mb-2">
                <span>{{ downloadDetail }}</span>
                <span>{{ Math.floor(downloadProgress) }}%</span>
              </div>
              <div class="w-full bg-slate-100 rounded-full h-2 overflow-hidden">
                <div
                  class="bg-indigo-500 h-2 rounded-full transition-all duration-300 ease-out"
                  :style="{ width: downloadProgress + '%' }"
                />
              </div>
            </div>
          </div>
        </div>
      </transition>

      <transition
        enter-active-class="transition-all duration-500 ease-out"
        enter-from-class="opacity-0 translate-y-8"
        enter-to-class="opacity-100 translate-y-0"
      >
        <div
          v-if="activeTab === 'xhs' && xhsInfo"
          class="bg-white rounded-3xl p-8 shadow-xl shadow-red-900/5 border border-red-100 flex gap-8"
        >
          <div class="w-1/3 aspect-auto rounded-2xl overflow-hidden bg-slate-100 border border-slate-200 relative group flex-shrink-0">
            <img
              :src="xhsInfo.cover"
              class="w-full h-auto object-cover transition-transform duration-500 group-hover:scale-105"
              referrerpolicy="no-referrer"
            >
            <div class="absolute top-2 right-2 bg-red-500 text-white text-xs px-2 py-1 rounded-lg font-bold backdrop-blur-md">
              {{ xhsInfo.type_name === 'video' ? t('bilidown.type_video') : t('bilidown.type_image') }}
            </div>
          </div>
              
          <div class="flex-1 flex flex-col">
            <h3 class="text-2xl font-bold text-slate-900 leading-snug mb-2 line-clamp-3">
              {{ xhsInfo.title }}
            </h3>
                  
            <div class="flex items-center gap-3 mb-6">
              <span class="font-bold text-slate-700">{{ t('bilidown.author') }}{{ xhsInfo.owner }}</span>
              <span class="text-sm text-slate-400 font-medium ml-auto">ID: {{ xhsInfo.id }}</span>
            </div>
                  
            <div class="grid grid-cols-2 gap-4 mb-auto">
              <template
                v-for="(media, index) in xhsInfo.media_list"
                :key="index"
              >
                <button
                  class="flex items-center justify-between p-3 rounded-xl border border-slate-200 hover:border-red-400 hover:bg-red-50 transition-all group"
                  @click="openXhsMedia(media.url)"
                >
                  <div class="flex items-center gap-3">
                    <Image
                      v-if="media.format === 'image'"
                      class="w-5 h-5 text-slate-400 group-hover:text-red-500"
                    />
                    <Film
                      v-else
                      class="w-5 h-5 text-slate-400 group-hover:text-red-500"
                    />
                    <span class="font-bold text-slate-700 text-sm">{{ t('bilidown.media') }}{{ Number(index) + 1 }} ({{ media.format === 'image' ? t('bilidown.type_image') : t('bilidown.type_video') }})</span>
                  </div>
                  <Download class="w-4 h-4 text-slate-400 group-hover:text-red-500" />
                </button>
              </template>
            </div>
                  
            <div class="mt-6 flex flex-wrap items-center gap-3">
              <button
                class="flex-1 py-3 bg-red-50 hover:bg-red-100 text-red-700 border border-red-200 rounded-xl font-bold flex items-center justify-center gap-2 transition-all active:scale-95 text-sm"
                @click="copyToClipboard(xhsInfo.cover, t('bilidown.xhs_cover'))"
              >
                <Copy class="w-4 h-4" /> {{ t('bilidown.copy_cover') }}
              </button>
            </div>
          </div>
        </div>
      </transition>
      
      <!-- Tasks List -->
      <div
        v-if="activeTab === 'bilibili' && tasks.length > 0"
        class="mt-8 bg-white rounded-3xl p-8 shadow-xl shadow-slate-900/5 border border-slate-200"
      >
        <h2 class="text-lg font-bold text-slate-900 mb-4 flex items-center gap-2">
          <Film class="w-5 h-5 text-indigo-500" />
          {{ t('bilidown.tasks_title') }}
        </h2>
        
        <div class="flex flex-col gap-3">
          <div
            v-for="task in tasks"
            :key="task.id"
            class="flex items-center gap-4 p-4 rounded-2xl border border-slate-100 bg-slate-50 hover:bg-white transition-colors group"
          >
            <img
              :src="task.cover"
              class="w-24 h-16 object-cover rounded-xl border border-slate-200 flex-shrink-0"
              referrerpolicy="no-referrer"
            >
                
            <div class="flex-1 min-w-0">
              <h4 class="font-bold text-slate-900 truncate mb-1">
                {{ task.title }}
              </h4>
              <div class="flex items-center gap-3 text-xs text-slate-500 font-medium">
                <span class="text-indigo-600 bg-indigo-50 px-2 py-0.5 rounded">{{ task.bvid }}</span>
                <span>UP: {{ task.owner }}</span>
                        
                <span
                  v-if="task.status === 'done'"
                  class="text-green-600 bg-green-50 px-2 py-0.5 rounded"
                >{{ t('bilidown.task_done') }}</span>
                <span
                  v-else-if="task.status === 'error'"
                  class="text-red-600 bg-red-50 px-2 py-0.5 rounded"
                >{{ t('bilidown.task_failed') }}</span>
                <span
                  v-else
                  class="text-indigo-600 bg-slate-50 px-2 py-0.5 rounded"
                >{{ task.detail || t('bilidown.task_processing') }}</span>
              </div>
                    
              <div
                v-if="task.status === 'running' || task.status === 'waiting'"
                class="mt-2 w-full bg-slate-200 rounded-full h-1.5 overflow-hidden"
              >
                <div
                  class="bg-indigo-500 h-1.5 rounded-full transition-all duration-300 ease-out"
                  :style="{ width: task.progress + '%' }"
                />
              </div>
            </div>
                
            <div class="flex gap-2 opacity-0 group-hover:opacity-100 transition-opacity">
              <button
                v-if="task.status === 'done'"
                class="p-2 text-indigo-600 hover:bg-indigo-50 rounded-xl"
                :title="t('bilidown.open_folder')"
                @click="openFolder(task.folder)"
              >
                <FolderOpen class="w-5 h-5" />
              </button>
              <button
                class="p-2 text-red-500 hover:bg-red-50 rounded-xl"
                :title="t('bilidown.delete_record')"
                @click="deleteTask(task.id)"
              >
                <Trash2 class="w-5 h-5" />
              </button>
            </div>
          </div>
        </div>
      </div>
      
      <!-- Empty State -->
      <div
        v-else-if="activeTab === 'bilibili' && !videoInfo"
        class="mt-12 flex flex-col items-center justify-center text-slate-400 opacity-60"
      >
        <div class="w-32 h-32 mb-4 relative">
          <div class="absolute inset-0 bg-slate-100 rounded-full animate-pulse" />
          <Film class="w-16 h-16 absolute top-1/2 left-1/2 transform -translate-x-1/2 -translate-y-1/2 text-slate-300" />
        </div>
        <p class="font-bold text-lg">
          {{ t('bilidown.no_records') }}
        </p>
        <p class="text-sm mt-1">
          {{ t('bilidown.hint_bili') }}
        </p>
      </div>

      <!-- XHS Empty State / Instructions -->
      <div
        v-else-if="activeTab === 'xhs' && !xhsInfo"
        class="mt-12 flex flex-col items-center justify-center text-slate-500 max-w-2xl mx-auto"
      >
        <div class="w-24 h-24 mb-6 relative opacity-50">
          <div class="absolute inset-0 bg-red-50 rounded-full animate-pulse" />
          <Search class="w-10 h-10 absolute top-1/2 left-1/2 transform -translate-x-1/2 -translate-y-1/2 text-red-400" />
        </div>
          
        <h3 class="font-bold text-xl text-slate-700 mb-6">
          {{ t('bilidown.xhs_guide_title') }}
        </h3>
        <div class="bg-slate-50 border border-slate-100 rounded-2xl p-6 w-full shadow-sm space-y-6 text-sm">
          <div class="flex items-start gap-4">
            <div class="w-8 h-8 rounded-full bg-white text-red-500 font-bold flex items-center justify-center border border-red-100 shadow-sm flex-shrink-0">
              1
            </div>
            <div>
              <h4 class="font-bold text-slate-800 text-base mb-1">
                {{ t('bilidown.step_copy') }}
              </h4>
              <p class="text-slate-500 leading-relaxed">
                打开小红书 App，找到想要下载的视频、图片或实况Live笔记，点击分享按钮，{{ t('bilidown.step_copy') }}。
              </p>
            </div>
          </div>
          <div class="flex items-start gap-4">
            <div class="w-8 h-8 rounded-full bg-white text-red-500 font-bold flex items-center justify-center border border-red-100 shadow-sm flex-shrink-0">
              2
            </div>
            <div>
              <h4 class="font-bold text-slate-800 text-base mb-1">
                {{ t('bilidown.step_paste') }}
              </h4>
              <p class="text-slate-500 leading-relaxed">
                {{ t('bilidown.step_paste_desc') }}
              </p>
            </div>
          </div>
          <div class="flex items-start gap-4">
            <div class="w-8 h-8 rounded-full bg-white text-red-500 font-bold flex items-center justify-center border border-red-100 shadow-sm flex-shrink-0">
              3
            </div>
            <div>
              <h4 class="font-bold text-slate-800 text-base mb-1">
                {{ t('bilidown.step_parse') }}
              </h4>
              <p class="text-slate-500 leading-relaxed">
                {{ t('bilidown.step_parse_desc') }}
              </p>
            </div>
          </div>
        </div>
      </div>
    </div>
    
    <!-- Login Modal -->
    <div
      v-if="showQrModal"
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/40 backdrop-blur-sm"
    >
      <div class="bg-white rounded-[32px] p-10 max-w-md w-full mx-4 shadow-2xl flex flex-col items-center text-center">
        <h2 class="text-2xl font-bold text-slate-900 mb-2">
          {{ t('bilidown.login_title') }}
        </h2>
        <p class="text-slate-500 mb-8 text-sm">
          {{ t('bilidown.login_desc') }}
        </p>
            
        <div class="w-48 h-48 bg-slate-50 border-2 border-slate-100 rounded-2xl p-2 mb-6 relative flex items-center justify-center">
          <Loader2
            v-if="!qrCodeUrl"
            class="w-8 h-8 animate-spin text-indigo-500"
          />
          <img
            v-else
            :src="qrCodeUrl"
            class="w-full h-full rounded-xl mix-blend-multiply"
          >
        </div>
            
        <div class="text-indigo-600 font-bold bg-indigo-50 px-6 py-3 rounded-xl w-full mb-6 flex items-center justify-center gap-2">
          <Loader2
            v-if="qrStatusText.includes(t('bilidown.status_fetching')) || qrStatusText.includes(t('bilidown.status_confirm'))"
            class="w-4 h-4 animate-spin"
          />
          {{ qrStatusText }}
        </div>
            
        <button
          class="text-slate-400 hover:text-slate-600 font-bold text-sm"
          @click="closeLogin"
        >
          {{ t('bilidown.cancel_login') }}
        </button>
      </div>
    </div>
    
    <!-- Toast Notification -->
    <div
      v-if="toastMessage"
      class="fixed top-5 left-1/2 transform -translate-x-1/2 px-6 py-3 rounded-2xl shadow-xl z-[1000] text-sm font-bold transition-all animate-bounce flex items-center gap-2"
      :class="{
        'bg-emerald-500 text-white shadow-emerald-500/20': toastType === 'success',
        'bg-red-500 text-white shadow-red-500/20': toastType === 'error',
        'bg-blue-500 text-white shadow-blue-500/20': toastType === 'info'
      }"
    >
      <CheckCircle
        v-if="toastType === 'success'"
        class="w-4 h-4"
      />
      <span class="whitespace-pre-wrap max-w-md break-all">{{ toastMessage }}</span>
    </div>
  </div>
</template>
