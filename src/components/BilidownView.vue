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
const parsedCollection = ref<any>(null);
const parsedItems = ref<any[]>([]);
const selectedParsedIndex = ref(0);
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

const searchVideo = async () => {
    if (!bvidUrl.value) return;
    isLoading.value = true;
    errorMsg.value = '';
    videoInfo.value = null;
    parsedCollection.value = null;
    parsedItems.value = [];
    try {
        const parsed: any = await invoke('bili_parse_url', { url: bvidUrl.value, sessdata: sessdata.value });
        parsedCollection.value = parsed;
        parsedItems.value = Array.isArray(parsed?.items) ? parsed.items : [];
        if (parsedItems.value.length === 0) {
            errorMsg.value = t('bilidown.fetch_fail');
            return;
        }

        await selectParsedItem(parsedItems.value[0], 0);
    } catch (e: any) {
        errorMsg.value = e.toString();
    } finally {
        isLoading.value = false;
    }
};

const selectParsedItem = async (item: any, index: number) => {
    selectedParsedIndex.value = index;
    const fallbackInfo = {
        bvid: item.bvid,
        cid: item.cid || 0,
        title: item.title || item.bvid,
        pic: item.cover || '',
        duration: item.duration || 0,
        owner: { name: item.owner || '', face: '' },
        stat: { view: 0, like: 0, coin: 0, favorite: 0 }
    };
    videoInfo.value = fallbackInfo;

    try {
        const res: any = await invoke('bili_get_video_info', { bvid: item.bvid, sessdata: sessdata.value });
        if (res.code === 0 && res.data) {
            videoInfo.value = res.data;
            videoInfo.value.cid = item.cid || videoInfo.value.cid;
            videoInfo.value.title = item.title || videoInfo.value.title;
        }
    } catch (e: any) {
        console.warn('Failed to enrich Bilibili item', e);
    }
};

const downloadParsedItem = async (item: any) => {
    await invoke('bili_download_video', {
        bvid: item.bvid,
        cid: item.cid || 0,
        title: item.title,
        owner: item.owner || '',
        cover: item.cover || '',
        sessdata: sessdata.value
    });
};

const downloadAllParsed = async () => {
    if (parsedItems.value.length === 0) return;
    isDownloading.value = true;
    downloadProgress.value = 0;
    downloadDetail.value = t('bilidown.prepare_download');

    try {
        for (const item of parsedItems.value) {
            await downloadParsedItem(item);
        }
        await loadTasks();
        showToast(`${parsedItems.value.length} task(s) added`, 'success');
    } catch (e: any) {
        errorMsg.value = t('bilidown.download_error') + e.toString();
    } finally {
        isDownloading.value = false;
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
        const selectedItem = parsedItems.value[selectedParsedIndex.value];
        const item = (selectedItem?.bvid === videoInfo.value.bvid ? selectedItem : null)
            || parsedItems.value.find((entry) => entry.bvid === videoInfo.value.bvid && entry.cid === videoInfo.value.cid)
            || {
            bvid: videoInfo.value.bvid,
            cid: videoInfo.value.cid,
            title: videoInfo.value.title,
            owner: videoInfo.value.owner?.name || '',
            cover: videoInfo.value.pic
        };
        await downloadParsedItem(item);
        await loadTasks();
    } catch (e: any) {
        errorMsg.value = t('bilidown.download_error') + e.toString();
        isDownloading.value = false;
    }
};
</script>

<template>
  <div class="h-full flex flex-col p-6 bg-surface-hover rounded-3xl relative overflow-hidden">
    <!-- Subtle Background Glow -->
    <div class="absolute top-0 right-0 w-96 h-96 bg-primary/10 rounded-full blur-[100px] pointer-events-none -z-10" />
    <div class="absolute bottom-0 left-0 w-[500px] h-[500px] bg-primary/5 rounded-full blur-[120px] pointer-events-none -z-10" />

    <header class="mb-8 flex justify-between items-end shrink-0 z-10">
      <div>
        <h1 class="text-3xl font-extrabold text-text tracking-tight flex items-center gap-3">
          <span class="inline-flex items-center justify-center p-2 bg-primary/10 rounded-2xl shadow-sm border-primary">
            <Download class="w-6 h-6 text-primary" />
          </span>
          {{ t('bilidown.tab_title') }}
        </h1>
        <div class="mt-4 flex items-center gap-2 bg-surface p-1.5 rounded-xl w-max border-border-soft/50 shadow-sm">
          <button
            :class="activeTab === 'bilibili' ? 'bg-primary text-white shadow-sm border-primary' : 'text-text-muted hover:text-primary'"
            class="px-6 py-2 rounded-lg font-bold text-sm transition-all"
            @click="activeTab = 'bilibili'"
          >
            Bilibili
          </button>
          <button
            :class="activeTab === 'xhs' ? 'bg-primary text-white shadow-sm border-primary' : 'text-text-muted hover:text-primary'"
            class="px-6 py-2 rounded-lg font-bold text-sm transition-all"
            @click="activeTab = 'xhs'"
          >
            Xiaohongshu
          </button>
        </div>
      </div>
      
      <div class="flex items-center gap-4">
        <template v-if="isLoggedIn">
          <div class="flex items-center gap-2 px-4 py-2 bg-green-50 text-green-700 rounded-full border-green-200">
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
            class="flex items-center gap-2 px-6 py-2.5 bg-primary text-white hover:brightness-110 rounded-xl font-bold transition-all shadow-md hover:shadow-lg active:scale-95"
            @click="openLogin"
          >
            <QrCode class="w-5 h-5" />
            {{ t('bilidown.scan_login') }}
          </button>
        </template>
      </div>
    </header>

    <div class="flex-1 overflow-y-auto pr-2 pb-8">
      <div class="bg-surface backdrop-blur-md rounded-3xl p-6 border-border-soft shadow-sm mb-8 transition-all relative overflow-hidden group">
        <div class="absolute -right-4 -top-4 w-32 h-32 bg-primary/10 rounded-full blur-3xl opacity-50 group-hover:bg-primary-hover transition-colors pointer-events-none" />
        <h2 class="text-lg font-bold text-text mb-4 flex items-center gap-2">
          <Search
            class="text-primary w-5 h-5"
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
            class="flex-1 bg-surface-hover border-border-soft rounded-xl px-6 py-4 text-text font-bold focus:outline-none focus:border-primary focus:ring-4 focus:ring-primary/10 transition-all text-lg shadow-sm" 
            :placeholder="t('bilidown.placeholder_bili')"
            @keyup.enter="searchVideo"
          >
          <button 
            :disabled="isLoading || !bvidUrl"
            class="disabled-readable-btn bg-primary hover:bg-primary-hover disabled:bg-surface disabled:text-text-muted disabled:border disabled:border-border-soft disabled:cursor-not-allowed text-white px-8 rounded-2xl font-bold flex items-center justify-center gap-2 transition-all shadow-md disabled:shadow-none active:scale-95 min-w-[140px]"
            @click="searchVideo"
          >
            <Loader2
              v-if="isLoading"
              class="w-6 h-6 animate-spin"
            />
            <template v-else>
              {{ t('bilidown.parse_btn') }}
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
            class="flex-1 bg-surface-hover border-border-soft rounded-xl px-6 py-4 text-text font-bold focus:outline-none focus:border-primary focus:ring-4 focus:ring-primary/10 transition-all text-lg shadow-sm" 
            :placeholder="t('bilidown.placeholder_xhs')"
            @keyup.enter="searchXhs"
          >
          <button 
            :disabled="isLoading || !xhsUrl"
            class="disabled-readable-btn bg-primary hover:bg-primary-hover disabled:bg-surface disabled:text-text-muted disabled:border disabled:border-border-soft disabled:cursor-not-allowed text-white px-8 rounded-2xl font-bold flex items-center justify-center gap-2 transition-all shadow-md disabled:shadow-none active:scale-95 min-w-[140px]"
            @click="searchXhs"
          >
            <Loader2
              v-if="isLoading"
              class="w-6 h-6 animate-spin"
            />
            <template v-else>
              {{ t('bilidown.parse_btn') }}
            </template>
          </button>
        </div>
        
        <div
          v-if="errorMsg"
          class="mt-4 p-4 bg-red-50 text-red-600 rounded-xl border-red-200 text-sm font-bold"
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
          class="bg-surface rounded-3xl p-8 shadow-xl shadow-slate-900/5 border-border-soft flex gap-8"
        >
          <div class="w-1/3 aspect-video rounded-2xl overflow-hidden bg-surface border-border-soft relative group flex-shrink-0">
            <img
              :src="videoInfo.pic"
              class="w-full h-full object-cover transition-transform duration-500 group-hover:scale-105"
              referrerpolicy="no-referrer"
            >
            <div class="absolute bottom-2 right-2 bg-background/80 backdrop-blur-md/70 text-text-inverse text-xs px-2 py-1 rounded-lg font-mono font-bold backdrop-blur-md">
              {{ Math.floor(videoInfo.duration / 60) }}:{{ (videoInfo.duration % 60).toString().padStart(2, '0') }}
            </div>
          </div>
              
          <div class="flex-1 flex flex-col">
            <h3 class="text-2xl font-bold text-text leading-snug mb-2 line-clamp-2">
              {{ videoInfo.title }}
            </h3>
                  
            <div class="flex items-center gap-3 mb-6">
              <img
                :src="videoInfo.owner.face"
                class="w-8 h-8 rounded-full border-border-soft"
                referrerpolicy="no-referrer"
              >
              <span class="font-bold text-text-muted">{{ videoInfo.owner.name }}</span>
              <span class="text-sm text-border-strong font-medium ml-auto">BVID: {{ videoInfo.bvid }}</span>
            </div>
                  
            <div class="grid grid-cols-4 gap-4 mb-auto">
              <div class="bg-surface-hover rounded-xl p-3 flex flex-col items-center justify-center border-border-soft">
                <span class="text-border-strong text-xs font-bold mb-1">{{ t('bilidown.stat_views') }}</span>
                <span class="text-text font-mono font-bold">{{ videoInfo.stat.view > 10000 ? (videoInfo.stat.view/10000).toFixed(1) + t('bilidown.ten_thousand') : videoInfo.stat.view }}</span>
              </div>
              <div class="bg-surface-hover rounded-xl p-3 flex flex-col items-center justify-center border-border-soft">
                <span class="text-border-strong text-xs font-bold mb-1">{{ t('bilidown.stat_likes') }}</span>
                <span class="text-text font-mono font-bold">{{ videoInfo.stat.like > 10000 ? (videoInfo.stat.like/10000).toFixed(1) + t('bilidown.ten_thousand') : videoInfo.stat.like }}</span>
              </div>
              <div class="bg-surface-hover rounded-xl p-3 flex flex-col items-center justify-center border-border-soft">
                <span class="text-border-strong text-xs font-bold mb-1">{{ t('bilidown.stat_coins') }}</span>
                <span class="text-text font-mono font-bold">{{ videoInfo.stat.coin > 10000 ? (videoInfo.stat.coin/10000).toFixed(1) + t('bilidown.ten_thousand') : videoInfo.stat.coin }}</span>
              </div>
              <div class="bg-surface-hover rounded-xl p-3 flex flex-col items-center justify-center border-border-soft">
                <span class="text-border-strong text-xs font-bold mb-1">{{ t('bilidown.stat_favs') }}</span>
                <span class="text-text font-mono font-bold">{{ videoInfo.stat.favorite > 10000 ? (videoInfo.stat.favorite/10000).toFixed(1) + t('bilidown.ten_thousand') : videoInfo.stat.favorite }}</span>
              </div>
            </div>
                  
            <div class="mt-6 flex items-center gap-4">
              <button 
                :disabled="isDownloading"
                class="disabled-readable-btn flex-1 bg-primary text-white hover:brightness-110 rounded-2xl py-4 font-bold flex items-center justify-center gap-2 transition-all shadow-md hover:shadow-xl active:scale-95 disabled:bg-surface disabled:cursor-not-allowed disabled:shadow-none"
                @click="downloadVideo"
              >
                <Download class="w-5 h-5" />
                {{ isDownloading ? t('bilidown.downloading') : t('bilidown.download_best') }}
              </button>
            </div>

            <div
              v-if="parsedItems.length > 1"
              class="mt-4 rounded-2xl border-border-soft bg-surface-hover p-3"
            >
              <div class="flex items-center justify-between gap-3 mb-2">
                <div class="min-w-0">
                  <p class="text-xs font-bold text-text-muted uppercase">
                    {{ parsedCollection?.collection_name || 'Bilibili list' }}
                  </p>
                  <p class="text-[11px] text-border-strong">
                    {{ parsedItems.length }} items
                  </p>
                </div>
                <button
                  :disabled="isDownloading"
                  class="disabled-readable-btn px-3 py-1.5 rounded-lg bg-primary text-white text-xs font-bold flex items-center gap-1.5 disabled:bg-surface disabled:cursor-not-allowed"
                  @click="downloadAllParsed"
                >
                  <Download class="w-3.5 h-3.5" />
                  Download all
                </button>
              </div>

              <div class="max-h-48 overflow-y-auto custom-scrollbar space-y-1">
                <div
                  v-for="(item, index) in parsedItems"
                  :key="`${item.bvid}-${item.cid || index}`"
                  class="flex items-center gap-2"
                >
                  <button
                    class="flex-1 min-w-0 text-left px-3 py-2 rounded-xl border transition-all"
                    :class="selectedParsedIndex === index ? 'bg-primary/10 border-primary/40 text-primary' : 'bg-surface border-border-soft text-text-muted hover:text-text hover:border-primary/30'"
                    @click="selectParsedItem(item, index)"
                  >
                    <span class="block text-xs font-bold truncate">{{ item.title || item.bvid }}</span>
                    <span class="block text-[10px] opacity-70 truncate">{{ item.bvid }}{{ item.cid ? ` - cid ${item.cid}` : '' }}</span>
                  </button>
                  <button
                    :disabled="isDownloading"
                    class="p-2 rounded-lg bg-surface border-border-soft text-text-muted hover:text-primary disabled:opacity-50"
                    @click="downloadParsedItem(item).then(loadTasks)"
                  >
                    <Download class="w-4 h-4" />
                  </button>
                </div>
              </div>
            </div>

            <div class="mt-4 flex flex-wrap items-center gap-3">
              <button
                class="flex-1 py-2.5 bg-surface-hover hover:bg-surface text-text-muted border-border-soft rounded-xl font-bold flex items-center justify-center gap-2 transition-all active:scale-95 text-sm"
                @click="copyToClipboard(videoInfo.pic, t('bilidown.cover'))"
              >
                <Image class="w-4 h-4" /> {{ t('bilidown.copy_cover') }}
              </button>
              <button
                class="flex-1 py-2.5 bg-primary/10 hover:bg-primary/15 text-primary border-primary/20 rounded-xl font-bold flex items-center justify-center gap-2 transition-all active:scale-95 text-sm"
                @click="copyStreamUrl"
              >
                <Link class="w-4 h-4" /> {{ t('bilidown.copy_video') }}
              </button>
            </div>
                  
            <div
              v-if="isDownloading"
              class="mt-4"
            >
              <div class="flex justify-between text-xs font-bold text-text-muted mb-2">
                <span>{{ downloadDetail }}</span>
                <span>{{ Math.floor(downloadProgress) }}%</span>
              </div>
              <div class="w-full bg-surface rounded-full h-2 overflow-hidden">
                <div
                  class="bg-primary/10 h-2 rounded-full transition-all duration-300 ease-out"
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
          class="bg-surface rounded-3xl p-8 shadow-xl shadow-red-900/5 border-red-100 flex gap-8"
        >
          <div class="w-1/3 aspect-auto rounded-2xl overflow-hidden bg-surface border-border-soft relative group flex-shrink-0">
            <img
              :src="xhsInfo.cover"
              class="w-full h-auto object-cover transition-transform duration-500 group-hover:scale-105"
              referrerpolicy="no-referrer"
            >
            <div class="absolute top-2 right-2 bg-red-500 text-text-inverse text-xs px-2 py-1 rounded-lg font-bold backdrop-blur-md">
              {{ xhsInfo.type_name === 'video' ? t('bilidown.type_video') : t('bilidown.type_image') }}
            </div>
          </div>
              
          <div class="flex-1 flex flex-col">
            <h3 class="text-2xl font-bold text-text leading-snug mb-2 line-clamp-3">
              {{ xhsInfo.title }}
            </h3>
                  
            <div class="flex items-center gap-3 mb-6">
              <span class="font-bold text-text-muted">{{ t('bilidown.author') }}{{ xhsInfo.owner }}</span>
              <span class="text-sm text-border-strong font-medium ml-auto">ID: {{ xhsInfo.id }}</span>
            </div>
                  
            <div class="grid grid-cols-2 gap-4 mb-auto">
              <template
                v-for="(media, index) in xhsInfo.media_list"
                :key="index"
              >
                <button
                  class="flex items-center justify-between p-3 rounded-xl border-border-soft hover:border-red-400 hover:bg-red-50 transition-all group"
                  @click="openXhsMedia(media.url)"
                >
                  <div class="flex items-center gap-3">
                    <Image
                      v-if="media.format === 'image'"
                      class="w-5 h-5 text-border-strong group-hover:text-red-500"
                    />
                    <Film
                      v-else
                      class="w-5 h-5 text-border-strong group-hover:text-red-500"
                    />
                    <span class="font-bold text-text-muted text-sm">{{ t('bilidown.media') }}{{ Number(index) + 1 }} ({{ media.format === 'image' ? t('bilidown.type_image') : t('bilidown.type_video') }})</span>
                  </div>
                  <Download class="w-4 h-4 text-border-strong group-hover:text-red-500" />
                </button>
              </template>
            </div>
                  
            <div class="mt-6 flex flex-wrap items-center gap-3">
              <button
                class="flex-1 py-3 bg-red-50 hover:bg-red-100 text-red-700 border-red-200 rounded-xl font-bold flex items-center justify-center gap-2 transition-all active:scale-95 text-sm"
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
        class="mt-8 bg-surface rounded-3xl p-8 shadow-xl shadow-slate-900/5 border-border-soft"
      >
        <h2 class="text-lg font-bold text-text mb-4 flex items-center gap-2">
          <Film class="w-5 h-5 text-primary" />
          {{ t('bilidown.tasks_title') }}
        </h2>
        
        <div class="flex flex-col gap-3">
          <div
            v-for="task in tasks"
            :key="task.id"
            class="flex items-center gap-4 p-4 rounded-2xl border-border-soft bg-surface-hover hover:bg-surface transition-colors group"
          >
            <img
              :src="task.cover"
              class="w-24 h-16 object-cover rounded-xl border-border-soft flex-shrink-0"
              referrerpolicy="no-referrer"
            >
                
            <div class="flex-1 min-w-0">
              <h4 class="font-bold text-text truncate mb-1">
                {{ task.title }}
              </h4>
              <div class="flex items-center gap-3 text-xs text-text-muted font-medium">
                <span class="text-primary bg-primary/10 px-2 py-0.5 rounded">{{ task.bvid }}</span>
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
                  class="text-primary bg-surface-hover px-2 py-0.5 rounded"
                >{{ task.detail || t('bilidown.task_processing') }}</span>
              </div>
                    
              <div
                v-if="task.status === 'running' || task.status === 'waiting'"
                class="mt-2 w-full bg-background/20 rounded-full h-1.5 overflow-hidden"
              >
                <div
                  class="bg-primary/10 h-1.5 rounded-full transition-all duration-300 ease-out"
                  :style="{ width: task.progress + '%' }"
                />
              </div>
            </div>
                
            <div class="flex gap-2 opacity-0 group-hover:opacity-100 transition-opacity">
              <button
                v-if="task.status === 'done'"
                class="p-2 text-primary hover:bg-primary-hover rounded-xl"
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
        class="mt-12 flex flex-col items-center justify-center text-border-strong opacity-60"
      >
        <div class="w-32 h-32 mb-4 relative">
          <div class="absolute inset-0 bg-surface rounded-full animate-pulse" />
          <Film class="w-16 h-16 absolute top-1/2 left-1/2 transform -translate-x-1/2 -translate-y-1/2 text-text-muted" />
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
        class="mt-12 flex flex-col items-center justify-center text-text-muted max-w-2xl mx-auto"
      >
        <div class="w-24 h-24 mb-6 relative opacity-50">
          <div class="absolute inset-0 bg-red-50 rounded-full animate-pulse" />
          <Search class="w-10 h-10 absolute top-1/2 left-1/2 transform -translate-x-1/2 -translate-y-1/2 text-red-400" />
        </div>
          
        <h3 class="font-bold text-xl text-text-muted mb-6">
          {{ t('bilidown.xhs_guide_title') }}
        </h3>
        <div class="bg-surface-hover border-border-soft rounded-2xl p-6 w-full shadow-sm space-y-6 text-sm">
          <div class="flex items-start gap-4">
            <div class="w-8 h-8 rounded-full bg-surface text-red-500 font-bold flex items-center justify-center border-red-100 shadow-sm flex-shrink-0">
              1
            </div>
            <div>
              <h4 class="font-bold text-text text-base mb-1">
                {{ t('bilidown.step_copy') }}
              </h4>
              <p class="text-text-muted leading-relaxed">
                {{ $t('bilidown.step_desc') }}{{ t('bilidown.step_copy') }}。
              </p>
            </div>
          </div>
          <div class="flex items-start gap-4">
            <div class="w-8 h-8 rounded-full bg-surface text-red-500 font-bold flex items-center justify-center border-red-100 shadow-sm flex-shrink-0">
              2
            </div>
            <div>
              <h4 class="font-bold text-text text-base mb-1">
                {{ t('bilidown.step_paste') }}
              </h4>
              <p class="text-text-muted leading-relaxed">
                {{ t('bilidown.step_paste_desc') }}
              </p>
            </div>
          </div>
          <div class="flex items-start gap-4">
            <div class="w-8 h-8 rounded-full bg-surface text-red-500 font-bold flex items-center justify-center border-red-100 shadow-sm flex-shrink-0">
              3
            </div>
            <div>
              <h4 class="font-bold text-text text-base mb-1">
                {{ t('bilidown.step_parse') }}
              </h4>
              <p class="text-text-muted leading-relaxed">
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
      class="fixed inset-0 z-50 flex items-center justify-center bg-surface backdrop-blur-md backdrop-blur-sm"
    >
      <div class="bg-surface rounded-[32px] p-10 max-w-md w-full mx-4 shadow-2xl flex flex-col items-center text-center">
        <h2 class="text-2xl font-bold text-text mb-2">
          {{ t('bilidown.login_title') }}
        </h2>
        <p class="text-text-muted mb-8 text-sm">
          {{ t('bilidown.login_desc') }}
        </p>
            
        <div class="w-48 h-48 bg-surface-hover border-2 border-border-soft rounded-2xl p-2 mb-6 relative flex items-center justify-center">
          <Loader2
            v-if="!qrCodeUrl"
            class="w-8 h-8 animate-spin text-primary"
          />
          <img
            v-else
            :src="qrCodeUrl"
            class="w-full h-full rounded-xl mix-blend-multiply"
          >
        </div>
            
        <div class="text-primary font-bold bg-primary/10 px-6 py-3 rounded-xl w-full mb-6 flex items-center justify-center gap-2">
          <Loader2
            v-if="qrStatusText.includes(t('bilidown.status_fetching')) || qrStatusText.includes(t('bilidown.status_confirm'))"
            class="w-4 h-4 animate-spin"
          />
          {{ qrStatusText }}
        </div>
            
        <button
          class="text-border-strong hover:text-text-muted font-bold text-sm"
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
        'bg-primary text-white shadow-primary/20': toastType === 'info'
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
