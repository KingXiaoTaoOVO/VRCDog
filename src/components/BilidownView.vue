<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
import { Download, Search, Film, QrCode, LogOut, CheckCircle, Loader2, FolderOpen, Trash2, Image, Link, Copy } from 'lucide-vue-next';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { DbApi } from '../api/index';

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
        showToast('打开文件夹失败: ' + e, 'error');
    }
};

const copyToClipboard = async (text: string, type: string) => {
    try {
        await navigator.clipboard.writeText(text);
        showToast(`已复制${type}链接到剪贴板！\n${text.substring(0, 50)}...`, 'success');
    } catch (e) {
        showToast('复制失败: ' + e, 'error');
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
            showToast('已成功复制原生MP4直链到剪贴板！可直接粘贴至浏览器播放(包含原声)。', 'success');
        } else {
            showToast('未能提取到视频底层直链', 'error');
        }
    } catch (e) {
        showToast('获取视频流失败: ' + e, 'error');
    }
};

const deleteTask = async (id: number) => {
    try {
        await invoke('db_bili_delete_task', { id });
        await loadTasks();
        showToast('记录已删除', 'success');
    } catch (e) {
        showToast('删除记录失败: ' + e, 'error');
    }
};

const generateQrUrl = (url: string) => {
    // Simple google charts qr code generator for now
    return `https://api.qrserver.com/v1/create-qr-code/?size=200x200&data=${encodeURIComponent(url)}`;
};

const openLogin = async () => {
    showQrModal.value = true;
    qrStatusText.value = '正在获取二维码...';
    try {
        const res: any = await invoke('bili_new_qr');
        if (res.code === 0 && res.data) {
            qrCodeUrl.value = generateQrUrl(res.data.url);
            qrKey.value = res.data.qrcode_key;
            qrStatusText.value = '请使用哔哩哔哩客户端扫码登录';
            
            // start polling
            qrPollTimer = setInterval(async () => {
                const pollRes: any = await invoke('bili_get_qr_status', { qrKey: qrKey.value });
                if (pollRes.data) {
                    if (pollRes.data.code === 0) {
                        // success
                        clearInterval(qrPollTimer);
                        qrStatusText.value = '登录成功！';
                        if (pollRes.sessdata_extracted) {
                            sessdata.value = pollRes.sessdata_extracted;
                            await DbApi.saveSetting({ key: 'bili_sessdata', value: sessdata.value });
                            isLoggedIn.value = true;
                        }
                        setTimeout(() => { showQrModal.value = false; }, 1000);
                    } else if (pollRes.data.code === 86038) {
                        qrStatusText.value = '二维码已失效，请重新获取';
                        clearInterval(qrPollTimer);
                    } else if (pollRes.data.code === 86090) {
                        qrStatusText.value = '扫码成功，请在手机上确认';
                    }
                }
            }, 3000);
        } else {
            qrStatusText.value = '获取二维码失败: ' + res.message;
        }
    } catch (e: any) {
        qrStatusText.value = '错误: ' + e.toString();
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
            errorMsg.value = res.message || '获取失败';
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
        showToast('打开失败: ' + e, 'error');
    }
};

const downloadVideo = async () => {
    if (!videoInfo.value) return;
    
    isDownloading.value = true;
    downloadProgress.value = 0;
    downloadDetail.value = '准备下载...';
    
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
        errorMsg.value = '下载出错: ' + e.toString();
        isDownloading.value = false;
    }
};
</script>

<template>
  <div class="p-8 max-w-5xl mx-auto h-full flex flex-col overflow-hidden">
    <div class="flex items-center justify-between mb-8 flex-shrink-0">
      <div>
        <h1 class="text-3xl font-extrabold text-[#451a03] tracking-tight flex items-center gap-3">
          <Download class="w-8 h-8 text-amber-500" />
          全平台媒体解析
        </h1>
        <div class="mt-3 flex items-center gap-2 bg-gray-100 p-1 rounded-xl w-max">
            <button @click="activeTab = 'bilibili'" :class="activeTab === 'bilibili' ? 'bg-white shadow text-amber-600' : 'text-gray-500 hover:text-gray-700'" class="px-6 py-2 rounded-lg font-bold text-sm transition-all">
                哔哩哔哩
            </button>
            <button @click="activeTab = 'xhs'" :class="activeTab === 'xhs' ? 'bg-white shadow text-red-600' : 'text-gray-500 hover:text-gray-700'" class="px-6 py-2 rounded-lg font-bold text-sm transition-all">
                小红书
            </button>
        </div>
      </div>
      
      <div class="flex items-center gap-4">
        <template v-if="isLoggedIn">
            <div class="flex items-center gap-2 px-4 py-2 bg-green-50 text-green-700 rounded-full border border-green-200">
                <CheckCircle class="w-4 h-4" />
                <span class="text-sm font-bold">已登录大会员/普通账号</span>
            </div>
            <button @click="handleLogout" class="p-2 text-red-500 hover:bg-red-50 rounded-full transition-colors">
                <LogOut class="w-5 h-5" />
            </button>
        </template>
        <template v-else>
            <button @click="openLogin" class="flex items-center gap-2 px-6 py-2.5 bg-amber-500 hover:bg-amber-600 text-white rounded-xl font-bold transition-all shadow-md hover:shadow-lg active:scale-95">
                <QrCode class="w-5 h-5" />
                扫码登录
            </button>
        </template>
      </div>
    </div>

    <div class="flex-1 overflow-y-auto pr-2 pb-8">
      <div class="bg-white rounded-3xl p-8 shadow-xl shadow-amber-900/5 border border-amber-100 mb-8 transition-all">
        <h2 class="text-lg font-bold text-[#451a03] mb-4 flex items-center gap-2">
          <Search :class="activeTab === 'xhs' ? 'text-red-500' : 'text-amber-500'" class="w-5 h-5" />
          {{ activeTab === 'xhs' ? '解析小红书作品' : '解析哔哩哔哩视频' }}
        </h2>
        
        <div v-if="activeTab === 'bilibili'" class="flex gap-4">
            <input 
                v-model="bvidUrl"
                @keyup.enter="searchVideo"
                type="text" 
                class="flex-1 bg-gray-50 border-2 border-gray-100 rounded-2xl px-6 py-4 text-gray-800 font-medium focus:outline-none focus:border-amber-300 focus:bg-white transition-all text-lg"
                placeholder="请输入视频链接或 BVID"
            >
            <button 
                @click="searchVideo"
                :disabled="isLoading || !bvidUrl"
                class="bg-amber-500 hover:bg-amber-600 disabled:bg-gray-300 disabled:cursor-not-allowed text-white px-8 rounded-2xl font-bold flex items-center justify-center gap-2 transition-all shadow-md active:scale-95 min-w-[140px]"
            >
                <Loader2 v-if="isLoading" class="w-6 h-6 animate-spin" />
                <template v-else>解析获取</template>
            </button>
        </div>
        
        <div v-else class="flex gap-4">
            <input 
                v-model="xhsUrl"
                @keyup.enter="searchXhs"
                type="text" 
                class="flex-1 bg-gray-50 border-2 border-gray-100 rounded-2xl px-6 py-4 text-gray-800 font-medium focus:outline-none focus:border-red-300 focus:bg-white transition-all text-lg"
                placeholder="请输入小红书分享链接 (例如: http://xhslink.com/...)"
            >
            <button 
                @click="searchXhs"
                :disabled="isLoading || !xhsUrl"
                class="bg-red-500 hover:bg-red-600 disabled:bg-gray-300 disabled:cursor-not-allowed text-white px-8 rounded-2xl font-bold flex items-center justify-center gap-2 transition-all shadow-md active:scale-95 min-w-[140px]"
            >
                <Loader2 v-if="isLoading" class="w-6 h-6 animate-spin" />
                <template v-else>解析获取</template>
            </button>
        </div>
        
        <div v-if="errorMsg" class="mt-4 p-4 bg-red-50 text-red-600 rounded-xl border border-red-200 text-sm font-bold">
            {{ errorMsg }}
        </div>
      </div>

      <!-- Result Card -->
      <transition enter-active-class="transition-all duration-500 ease-out" enter-from-class="opacity-0 translate-y-8" enter-to-class="opacity-100 translate-y-0">
          <div v-if="activeTab === 'bilibili' && videoInfo" class="bg-white rounded-3xl p-8 shadow-xl shadow-amber-900/5 border border-amber-100 flex gap-8">
              <div class="w-1/3 aspect-video rounded-2xl overflow-hidden bg-gray-100 border border-gray-200 relative group flex-shrink-0">
                  <img :src="videoInfo.pic" class="w-full h-full object-cover transition-transform duration-500 group-hover:scale-105" referrerpolicy="no-referrer">
                  <div class="absolute bottom-2 right-2 bg-black/70 text-white text-xs px-2 py-1 rounded-lg font-mono font-bold backdrop-blur-md">
                      {{ Math.floor(videoInfo.duration / 60) }}:{{ (videoInfo.duration % 60).toString().padStart(2, '0') }}
                  </div>
              </div>
              
              <div class="flex-1 flex flex-col">
                  <h3 class="text-2xl font-bold text-gray-900 leading-snug mb-2 line-clamp-2">{{ videoInfo.title }}</h3>
                  
                  <div class="flex items-center gap-3 mb-6">
                      <img :src="videoInfo.owner.face" class="w-8 h-8 rounded-full border border-gray-200" referrerpolicy="no-referrer">
                      <span class="font-bold text-gray-700">{{ videoInfo.owner.name }}</span>
                      <span class="text-sm text-gray-400 font-medium ml-auto">BVID: {{ videoInfo.bvid }}</span>
                  </div>
                  
                  <div class="grid grid-cols-4 gap-4 mb-auto">
                      <div class="bg-gray-50 rounded-xl p-3 flex flex-col items-center justify-center border border-gray-100">
                          <span class="text-gray-400 text-xs font-bold mb-1">播放</span>
                          <span class="text-gray-800 font-mono font-bold">{{ videoInfo.stat.view > 10000 ? (videoInfo.stat.view/10000).toFixed(1) + '万' : videoInfo.stat.view }}</span>
                      </div>
                      <div class="bg-gray-50 rounded-xl p-3 flex flex-col items-center justify-center border border-gray-100">
                          <span class="text-gray-400 text-xs font-bold mb-1">点赞</span>
                          <span class="text-gray-800 font-mono font-bold">{{ videoInfo.stat.like > 10000 ? (videoInfo.stat.like/10000).toFixed(1) + '万' : videoInfo.stat.like }}</span>
                      </div>
                      <div class="bg-gray-50 rounded-xl p-3 flex flex-col items-center justify-center border border-gray-100">
                          <span class="text-gray-400 text-xs font-bold mb-1">投币</span>
                          <span class="text-gray-800 font-mono font-bold">{{ videoInfo.stat.coin > 10000 ? (videoInfo.stat.coin/10000).toFixed(1) + '万' : videoInfo.stat.coin }}</span>
                      </div>
                      <div class="bg-gray-50 rounded-xl p-3 flex flex-col items-center justify-center border border-gray-100">
                          <span class="text-gray-400 text-xs font-bold mb-1">收藏</span>
                          <span class="text-gray-800 font-mono font-bold">{{ videoInfo.stat.favorite > 10000 ? (videoInfo.stat.favorite/10000).toFixed(1) + '万' : videoInfo.stat.favorite }}</span>
                      </div>
                  </div>
                  
                  <div class="mt-6 flex items-center gap-4">
                      <button 
                          @click="downloadVideo"
                          :disabled="isDownloading"
                          class="flex-1 bg-indigo-600 hover:bg-indigo-700 text-white rounded-2xl py-4 font-bold flex items-center justify-center gap-2 transition-all shadow-md hover:shadow-xl active:scale-95 disabled:bg-gray-300 disabled:cursor-not-allowed disabled:shadow-none"
                      >
                          <Download class="w-5 h-5" />
                          {{ isDownloading ? '下载合并中...' : '下载最高画质' }}
                      </button>
                  </div>
                  
                  <div class="mt-4 flex flex-wrap items-center gap-3">
                      <button @click="copyToClipboard(videoInfo.pic, '封面')" class="flex-1 py-2.5 bg-gray-50 hover:bg-gray-100 text-gray-700 border border-gray-200 rounded-xl font-bold flex items-center justify-center gap-2 transition-all active:scale-95 text-sm">
                          <Image class="w-4 h-4" /> 复制封面链接
                      </button>
                      <button @click="copyStreamUrl" class="flex-1 py-2.5 bg-blue-50 hover:bg-blue-100 text-blue-700 border border-blue-200 rounded-xl font-bold flex items-center justify-center gap-2 transition-all active:scale-95 text-sm">
                          <Link class="w-4 h-4" /> 复制视频链接
                      </button>
                  </div>
                  
                  <div v-if="isDownloading" class="mt-4">
                      <div class="flex justify-between text-xs font-bold text-gray-500 mb-2">
                          <span>{{ downloadDetail }}</span>
                          <span>{{ Math.floor(downloadProgress) }}%</span>
                      </div>
                      <div class="w-full bg-gray-100 rounded-full h-2 overflow-hidden">
                          <div class="bg-indigo-500 h-2 rounded-full transition-all duration-300 ease-out" :style="{ width: downloadProgress + '%' }"></div>
                      </div>
                  </div>
              </div>
          </div>
      </transition>

      <transition enter-active-class="transition-all duration-500 ease-out" enter-from-class="opacity-0 translate-y-8" enter-to-class="opacity-100 translate-y-0">
          <div v-if="activeTab === 'xhs' && xhsInfo" class="bg-white rounded-3xl p-8 shadow-xl shadow-red-900/5 border border-red-100 flex gap-8">
              <div class="w-1/3 aspect-auto rounded-2xl overflow-hidden bg-gray-100 border border-gray-200 relative group flex-shrink-0">
                  <img :src="xhsInfo.cover" class="w-full h-auto object-cover transition-transform duration-500 group-hover:scale-105" referrerpolicy="no-referrer">
                  <div class="absolute top-2 right-2 bg-red-500 text-white text-xs px-2 py-1 rounded-lg font-bold backdrop-blur-md">
                      {{ xhsInfo.type_name === 'video' ? '视频' : '图文' }}
                  </div>
              </div>
              
              <div class="flex-1 flex flex-col">
                  <h3 class="text-2xl font-bold text-gray-900 leading-snug mb-2 line-clamp-3">{{ xhsInfo.title }}</h3>
                  
                  <div class="flex items-center gap-3 mb-6">
                      <span class="font-bold text-gray-700">作者: {{ xhsInfo.owner }}</span>
                      <span class="text-sm text-gray-400 font-medium ml-auto">ID: {{ xhsInfo.id }}</span>
                  </div>
                  
                  <div class="grid grid-cols-2 gap-4 mb-auto">
                      <template v-for="(media, index) in xhsInfo.media_list" :key="index">
                          <button @click="openXhsMedia(media.url)" class="flex items-center justify-between p-3 rounded-xl border border-gray-200 hover:border-red-400 hover:bg-red-50 transition-all group">
                              <div class="flex items-center gap-3">
                                  <Image v-if="media.format === 'image'" class="w-5 h-5 text-gray-400 group-hover:text-red-500" />
                                  <Film v-else class="w-5 h-5 text-gray-400 group-hover:text-red-500" />
                                  <span class="font-bold text-gray-700 text-sm">媒体 {{ index + 1 }} ({{ media.format === 'image' ? '图片' : '视频' }})</span>
                              </div>
                              <Download class="w-4 h-4 text-gray-400 group-hover:text-red-500" />
                          </button>
                      </template>
                  </div>
                  
                  <div class="mt-6 flex flex-wrap items-center gap-3">
                      <button @click="copyToClipboard(xhsInfo.cover, '小红书封面')" class="flex-1 py-3 bg-red-50 hover:bg-red-100 text-red-700 border border-red-200 rounded-xl font-bold flex items-center justify-center gap-2 transition-all active:scale-95 text-sm">
                          <Copy class="w-4 h-4" /> 复制封面链接
                      </button>
                  </div>
              </div>
          </div>
      </transition>
      
      <!-- Tasks List -->
      <div v-if="activeTab === 'bilibili' && tasks.length > 0" class="mt-8 bg-white rounded-3xl p-8 shadow-xl shadow-amber-900/5 border border-amber-100">
        <h2 class="text-lg font-bold text-[#451a03] mb-4 flex items-center gap-2">
          <Film class="w-5 h-5 text-amber-500" />
          下载任务
        </h2>
        
        <div class="flex flex-col gap-3">
            <div v-for="task in tasks" :key="task.id" class="flex items-center gap-4 p-4 rounded-2xl border border-gray-100 bg-gray-50 hover:bg-white transition-colors group">
                <img :src="task.cover" class="w-24 h-16 object-cover rounded-xl border border-gray-200 flex-shrink-0" referrerpolicy="no-referrer">
                
                <div class="flex-1 min-w-0">
                    <h4 class="font-bold text-gray-900 truncate mb-1">{{ task.title }}</h4>
                    <div class="flex items-center gap-3 text-xs text-gray-500 font-medium">
                        <span class="text-indigo-600 bg-indigo-50 px-2 py-0.5 rounded">{{ task.bvid }}</span>
                        <span>UP: {{ task.owner }}</span>
                        
                        <span v-if="task.status === 'done'" class="text-green-600 bg-green-50 px-2 py-0.5 rounded">已完成</span>
                        <span v-else-if="task.status === 'error'" class="text-red-600 bg-red-50 px-2 py-0.5 rounded">下载失败</span>
                        <span v-else class="text-amber-600 bg-amber-50 px-2 py-0.5 rounded">{{ task.detail || '处理中...' }}</span>
                    </div>
                    
                    <div v-if="task.status === 'running' || task.status === 'waiting'" class="mt-2 w-full bg-gray-200 rounded-full h-1.5 overflow-hidden">
                        <div class="bg-amber-500 h-1.5 rounded-full transition-all duration-300 ease-out" :style="{ width: task.progress + '%' }"></div>
                    </div>
                </div>
                
                <div class="flex gap-2 opacity-0 group-hover:opacity-100 transition-opacity">
                    <button v-if="task.status === 'done'" @click="openFolder(task.folder)" class="p-2 text-indigo-600 hover:bg-indigo-50 rounded-xl" title="打开文件夹">
                        <FolderOpen class="w-5 h-5" />
                    </button>
                    <button @click="deleteTask(task.id)" class="p-2 text-red-500 hover:bg-red-50 rounded-xl" title="删除记录">
                        <Trash2 class="w-5 h-5" />
                    </button>
                </div>
            </div>
        </div>
      </div>
      
      <!-- Empty State -->
      <div v-else-if="activeTab === 'bilibili' && !videoInfo" class="mt-12 flex flex-col items-center justify-center text-gray-400 opacity-60">
        <div class="w-32 h-32 mb-4 relative">
            <div class="absolute inset-0 bg-gray-100 rounded-full animate-pulse"></div>
            <Film class="w-16 h-16 absolute top-1/2 left-1/2 transform -translate-x-1/2 -translate-y-1/2 text-gray-300" />
        </div>
        <p class="font-bold text-lg">暂无解析或下载记录</p>
        <p class="text-sm mt-1">在上方输入哔哩哔哩视频链接或 BVID 开始</p>
      </div>

      <!-- XHS Empty State / Instructions -->
      <div v-else-if="activeTab === 'xhs' && !xhsInfo" class="mt-12 flex flex-col items-center justify-center text-gray-500 max-w-2xl mx-auto">
          <div class="w-24 h-24 mb-6 relative opacity-50">
              <div class="absolute inset-0 bg-red-50 rounded-full animate-pulse"></div>
              <Search class="w-10 h-10 absolute top-1/2 left-1/2 transform -translate-x-1/2 -translate-y-1/2 text-red-400" />
          </div>
          
          <h3 class="font-bold text-xl text-gray-700 mb-6">如何使用小红书解析？</h3>
          <div class="bg-gray-50 border border-gray-100 rounded-2xl p-6 w-full shadow-sm space-y-6 text-sm">
              <div class="flex items-start gap-4">
                  <div class="w-8 h-8 rounded-full bg-white text-red-500 font-bold flex items-center justify-center border border-red-100 shadow-sm flex-shrink-0">1</div>
                  <div>
                      <h4 class="font-bold text-gray-800 text-base mb-1">复制链接</h4>
                      <p class="text-gray-500 leading-relaxed">打开小红书 App，找到想要下载的视频、图片或实况Live笔记，点击分享按钮，复制链接。</p>
                  </div>
              </div>
              <div class="flex items-start gap-4">
                  <div class="w-8 h-8 rounded-full bg-white text-red-500 font-bold flex items-center justify-center border border-red-100 shadow-sm flex-shrink-0">2</div>
                  <div>
                      <h4 class="font-bold text-gray-800 text-base mb-1">粘贴链接</h4>
                      <p class="text-gray-500 leading-relaxed">将复制的短链接 (例如 http://xhslink.com/...) 粘贴到上方的输入框中。</p>
                  </div>
              </div>
              <div class="flex items-start gap-4">
                  <div class="w-8 h-8 rounded-full bg-white text-red-500 font-bold flex items-center justify-center border border-red-100 shadow-sm flex-shrink-0">3</div>
                  <div>
                      <h4 class="font-bold text-gray-800 text-base mb-1">解析并下载</h4>
                      <p class="text-gray-500 leading-relaxed">点击"解析获取"按钮，等待解析完成后即可无水印下载高清视频或图片媒体文件。</p>
                  </div>
              </div>
          </div>
      </div>
    </div>
    
    <!-- Login Modal -->
    <div v-if="showQrModal" class="fixed inset-0 z-50 flex items-center justify-center bg-black/40 backdrop-blur-sm">
        <div class="bg-white rounded-[32px] p-10 max-w-md w-full mx-4 shadow-2xl flex flex-col items-center text-center">
            <h2 class="text-2xl font-bold text-gray-900 mb-2">哔哩哔哩登录</h2>
            <p class="text-gray-500 mb-8 text-sm">扫码登录以获取最高清晰度的视频解析权限</p>
            
            <div class="w-48 h-48 bg-gray-50 border-2 border-gray-100 rounded-2xl p-2 mb-6 relative flex items-center justify-center">
                <Loader2 v-if="!qrCodeUrl" class="w-8 h-8 animate-spin text-amber-500" />
                <img v-else :src="qrCodeUrl" class="w-full h-full rounded-xl mix-blend-multiply">
            </div>
            
            <div class="text-indigo-600 font-bold bg-indigo-50 px-6 py-3 rounded-xl w-full mb-6 flex items-center justify-center gap-2">
                <Loader2 v-if="qrStatusText.includes('正在获取') || qrStatusText.includes('手机上确认')" class="w-4 h-4 animate-spin" />
                {{ qrStatusText }}
            </div>
            
            <button @click="closeLogin" class="text-gray-400 hover:text-gray-600 font-bold text-sm">取消登录</button>
        </div>
    </div>
    
    <!-- Toast Notification -->
    <div v-if="toastMessage" class="fixed top-5 left-1/2 transform -translate-x-1/2 px-6 py-3 rounded-2xl shadow-xl z-[1000] text-sm font-bold transition-all animate-bounce flex items-center gap-2"
        :class="{
            'bg-emerald-500 text-white shadow-emerald-500/20': toastType === 'success',
            'bg-red-500 text-white shadow-red-500/20': toastType === 'error',
            'bg-blue-500 text-white shadow-blue-500/20': toastType === 'info'
        }">
        <CheckCircle v-if="toastType === 'success'" class="w-4 h-4" />
        <span class="whitespace-pre-wrap max-w-md break-all">{{ toastMessage }}</span>
    </div>

  </div>
</template>
