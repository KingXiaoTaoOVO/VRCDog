<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import {
  Search,
  Cloud,
  X,
  Play,
  Pause,
  Download,
  RotateCcw,
  SlidersHorizontal,
  ExternalLink,
  Volume2,
  Check,
  Loader2,
  Music,
  Mic,
  Sparkles,
  Gamepad2,
  Piano,
  Disc,
  Film,
  BookOpen,
  Plus,
  Trash2,
  Globe,
  Radio,
  Library,
  HelpCircle,
  AlertCircle,
  Info,
} from 'lucide-vue-next';
import {
  VrpianoApi,
  type MidiCloudTrack,
  type MidiCloudCategory,
  type MidiCloudSource,
  type VrpianoSong,
} from '../api';

const props = defineProps<{
  show: boolean;
  localSongs?: VrpianoSong[];
}>();

const emit = defineEmits<{
  (e: 'close'): void;
  (e: 'play-song', song: VrpianoSong): void;
  (e: 'song-downloaded', song: VrpianoSong): void;
  (e: 'preview-midi', data: { name: string; data: string }): void;
}>();

const { t } = useI18n();

// 搜索与筛选状态
const searchQuery = ref('');
const activeCategory = ref('all');
const activeSort = ref<'hot' | 'latest' | 'title' | 'notes'>('hot');
const activeSource = ref('all');
const currentPage = ref(1);
const pageSize = ref(25);
const totalTracks = ref(0);
const tracks = ref<MidiCloudTrack[]>([]);
const categories = ref<MidiCloudCategory[]>([]);
const sources = ref<MidiCloudSource[]>([]);
const loading = ref(false);
const errorMsg = ref('');

// 试听播放器状态
const currentPlayingTrack = ref<MidiCloudTrack | null>(null);
const isPlaying = ref(false);
const previewLoadingId = ref<string | null>(null);
const downloadLoadingId = ref<string | null>(null);
const downloadedIds = ref<Set<string>>(new Set());

// Toast 状态提示（取代原生系统弹窗）
const toastMessage = ref('');
const toastType = ref<'success' | 'error' | 'info'>('info');
let toastTimer: number | null = null;

const showToast = (msg: string, type: 'success' | 'error' | 'info' = 'info') => {
  toastMessage.value = msg;
  toastType.value = type;
  if (toastTimer !== null) {
    window.clearTimeout(toastTimer);
  }
  toastTimer = window.setTimeout(() => {
    toastMessage.value = '';
    toastTimer = null;
  }, 3200);
};

// 数据源管理弹窗
const showSourceManager = ref(false);
const newSourceName = ref('');
const newSourceUrl = ref('');
const newSourceDesc = ref('');
const addingSource = ref(false);
const sourceAddError = ref('');

// 热门搜索词
const hotKeywords = [
  '晴天',
  '周杰伦',
  '稻香',
  '千本樱',
  '原神',
  '晨曦酒庄',
  '卡农',
  'Bad Apple',
  'Faded',
  '极乐净土',
  '恋爱循环',
  '孤勇者',
  'Summer',
  '天空之城',
];

const categoryIconMap: Record<string, any> = {
  all: Music,
  mandopop: Mic,
  acg: Sparkles,
  game: Gamepad2,
  classical: Piano,
  western: Disc,
  ost: Film,
  practice: BookOpen,
};

const getCategoryIcon = (id: string) => {
  return categoryIconMap[id] || Music;
};

// 检查是否已经在本地曲库中
const isDownloaded = (track: MidiCloudTrack) => {
  if (downloadedIds.value.has(track.id)) return true;
  if (!props.localSongs) return false;
  const titleLower = track.title.toLowerCase();
  return props.localSongs.some((s) => s.name.toLowerCase().includes(titleLower));
};

// 获取分类与数据源
const loadCategoriesAndSources = async () => {
  try {
    const catRes = await VrpianoApi.cloudLibraryCategories();
    if (catRes && catRes.categories) {
      categories.value = catRes.categories;
    }
  } catch (e) {
    console.warn('获取云端分类失败:', e);
  }

  try {
    const srcRes = await VrpianoApi.cloudLibrarySources();
    if (srcRes && srcRes.sources) {
      sources.value = srcRes.sources;
    }
  } catch (e) {
    console.warn('获取云端数据源失败:', e);
  }
};

// 检索曲目
let searchDebounceTimer: number | null = null;

const searchTracks = async () => {
  loading.value = true;
  errorMsg.value = '';
  try {
    const res = await VrpianoApi.cloudLibrarySearch({
      keyword: searchQuery.value.trim() || undefined,
      category: activeCategory.value === 'all' ? undefined : activeCategory.value,
      source: activeSource.value === 'all' ? undefined : activeSource.value,
      sort: activeSort.value,
      page: currentPage.value,
      limit: pageSize.value,
    });
    if (res && res.items) {
      tracks.value = res.items;
      totalTracks.value = res.total;
    } else {
      tracks.value = [];
      totalTracks.value = 0;
    }
  } catch (e: any) {
    errorMsg.value = e.message || String(e);
    tracks.value = [];
  } finally {
    loading.value = false;
  }
};

const onSearchInput = () => {
  if (searchDebounceTimer !== null) {
    window.clearTimeout(searchDebounceTimer);
  }
  searchDebounceTimer = window.setTimeout(() => {
    currentPage.value = 1;
    searchTracks();
  }, 260);
};

// 切换分类
const selectCategory = (catId: string) => {
  activeCategory.value = catId;
  currentPage.value = 1;
  searchTracks();
};

// 应用热门关键词
const applyHotKeyword = (kw: string) => {
  searchQuery.value = kw;
  currentPage.value = 1;
  searchTracks();
};

// 试听曲目
const handlePreview = async (track: MidiCloudTrack) => {
  if (currentPlayingTrack.value?.id === track.id && isPlaying.value) {
    isPlaying.value = false;
    return;
  }

  previewLoadingId.value = track.id;
  try {
    const midiData = await VrpianoApi.midishowPreviewData({
      midiId: 999999,
      title: `${track.title} - ${track.artist}`,
      downloadUrl: track.download_url,
      source: 'vrcdog_cloud',
    });
    currentPlayingTrack.value = track;
    isPlaying.value = true;
    emit('preview-midi', midiData);
    showToast(`正在试听《${track.title}》`, 'info');
  } catch (e: any) {
    showToast(`试听解析失败: ${e.message || String(e)}`, 'error');
  } finally {
    previewLoadingId.value = null;
  }
};

// 一键下载到本地曲库
const handleDownload = async (track: MidiCloudTrack) => {
  downloadLoadingId.value = track.id;
  try {
    const downloaded = await VrpianoApi.downloadMidishow({
      midiId: 999999,
      title: `${track.title} - ${track.artist}`,
      downloadUrl: track.download_url,
      source: 'vrcdog_cloud',
    });
    if (downloaded) {
      downloadedIds.value.add(track.id);
      emit('song-downloaded', downloaded);
      showToast(`《${track.title}》已成功收录至本地钢琴曲库`, 'success');
    }
  } catch (e: any) {
    showToast(`下载失败: ${e.message || String(e)}`, 'error');
  } finally {
    downloadLoadingId.value = null;
  }
};

// 立即弹奏
const handlePlayInGame = async (track: MidiCloudTrack) => {
  if (!isDownloaded(track)) {
    downloadLoadingId.value = track.id;
    try {
      const downloaded = await VrpianoApi.downloadMidishow({
        midiId: 999999,
        title: `${track.title} - ${track.artist}`,
        downloadUrl: track.download_url,
        source: 'vrcdog_cloud',
      });
      if (downloaded) {
        downloadedIds.value.add(track.id);
        emit('song-downloaded', downloaded);
        emit('play-song', downloaded);
        emit('close');
      }
    } catch (e: any) {
      showToast(`准备弹奏失败: ${e.message || String(e)}`, 'error');
    } finally {
      downloadLoadingId.value = null;
    }
  } else {
    const found = props.localSongs?.find((s) => s.name.includes(track.title));
    if (found) {
      emit('play-song', found);
      emit('close');
    }
  }
};

// 格式化时间与大小
const formatDuration = (secs: number) => {
  const m = Math.floor(secs / 60);
  const s = secs % 60;
  return `${m.toString().padStart(2, '0')}:${s.toString().padStart(2, '0')}`;
};

// 切换数据源
const onSourceChange = () => {
  currentPage.value = 1;
  searchTracks();
};

// 保存自定义数据源
const addCustomSource = async () => {
  if (!newSourceName.value.trim() || !newSourceUrl.value.trim()) {
    sourceAddError.value = '请填写源名称与接口地址';
    return;
  }
  sourceAddError.value = '';
  addingSource.value = true;
  try {
    const existing = await VrpianoApi.cloudSourcesGet();
    const newSrc: MidiCloudSource = {
      id: `custom_${Date.now()}`,
      name: newSourceName.value.trim(),
      url: newSourceUrl.value.trim(),
      source_type: 'custom',
      description: newSourceDesc.value.trim() || '用户自定义源',
      enabled: true,
      icon: 'Globe',
    };
    const updated = [...existing, newSrc];
    await VrpianoApi.cloudSourcesSave(updated);
    sources.value = updated;
    newSourceName.value = '';
    newSourceUrl.value = '';
    newSourceDesc.value = '';
    showSourceManager.value = false;
    showToast('自定义数据源已添加', 'success');
  } catch (e: any) {
    sourceAddError.value = e.message || String(e);
  } finally {
    addingSource.value = false;
  }
};

// 删除自定义数据源
const removeCustomSource = async (id: string) => {
  try {
    const existing = await VrpianoApi.cloudSourcesGet();
    const updated = existing.filter((s) => s.id !== id);
    await VrpianoApi.cloudSourcesSave(updated);
    sources.value = updated;
    if (activeSource.value === id) {
      activeSource.value = 'all';
      searchTracks();
    }
    showToast('数据源已删除', 'info');
  } catch (e: any) {
    showToast(`删除失败: ${e.message || String(e)}`, 'error');
  }
};

onMounted(() => {
  if (props.show) {
    loadCategoriesAndSources();
    searchTracks();
  }
});

watch(
  () => props.show,
  (val) => {
    if (val) {
      loadCategoriesAndSources();
      searchTracks();
    }
  }
);
</script>

<template>
  <div v-if="show" class="midi-cloud-modal-backdrop" @click.self="emit('close')">
    <div class="midi-cloud-window">
      <!-- 浮动通知 Toast -->
      <transition name="toast-fade">
        <div v-if="toastMessage" class="cloud-toast" :class="toastType">
          <Check v-if="toastType === 'success'" :size="14" />
          <AlertCircle v-else-if="toastType === 'error'" :size="14" />
          <Info v-else :size="14" />
          <span>{{ toastMessage }}</span>
        </div>
      </transition>

      <!-- 顶部控制条 -->
      <div class="window-header">
        <div class="header-left">
          <div class="brand-badge">
            <Cloud :size="20" class="brand-icon" />
            <div class="brand-titles">
              <div class="brand-name">VRCDog 社区与云端曲库</div>
              <div class="brand-sub">海量精选曲目 · 官方服务器直连 · 支持自定义第三方音源</div>
            </div>
          </div>
        </div>

        <div class="header-actions">
          <div class="source-selector">
            <Globe :size="14" class="source-icon" />
            <select v-model="activeSource" @change="onSourceChange">
              <option value="all">全部分源</option>
              <option v-for="s in sources" :key="s.id" :value="s.id">
                {{ s.name }}
              </option>
            </select>
          </div>

          <button
            class="header-btn"
            title="管理数据源"
            @click="showSourceManager = true"
          >
            <SlidersHorizontal :size="14" />
            <span>数据源管理</span>
          </button>

          <button class="close-btn" @click="emit('close')" title="关闭">
            <X :size="16" />
          </button>
        </div>
      </div>

      <!-- 分类标签导航栏 -->
      <div class="category-navbar">
        <button
          v-for="cat in categories"
          :key="cat.id"
          class="cat-pill"
          :class="{ active: activeCategory === cat.id }"
          @click="selectCategory(cat.id)"
        >
          <component :is="getCategoryIcon(cat.id)" :size="13" class="cat-icon" />
          <span class="cat-name">{{ cat.name }}</span>
          <span v-if="cat.count > 0" class="cat-count">{{ cat.count }}</span>
        </button>
      </div>

      <!-- 搜索与筛选工具栏 -->
      <div class="search-toolbar">
        <div class="search-box">
          <Search :size="15" class="search-icon" />
          <input
            v-model="searchQuery"
            type="text"
            placeholder="搜索歌名、艺术家、原神、周杰伦、动漫或标签..."
            @input="onSearchInput"
            @keyup.enter="searchTracks"
          />
          <button v-if="searchQuery" class="clear-input-btn" @click="searchQuery = ''; searchTracks()">
            <X :size="13" />
          </button>
          <button class="search-submit-btn" :disabled="loading" @click="searchTracks">
            <Loader2 v-if="loading" :size="13" class="spin" />
            <span v-else>搜索</span>
          </button>
        </div>

        <!-- 热门搜索推荐词 -->
        <div class="hot-chips">
          <span class="hot-label">热门推荐:</span>
          <button
            v-for="kw in hotKeywords"
            :key="kw"
            class="hot-chip"
            @click="applyHotKeyword(kw)"
          >
            {{ kw }}
          </button>
        </div>

        <!-- 排序方式 -->
        <div class="sort-selector">
          <select v-model="activeSort" @change="searchTracks">
            <option value="hot">按热度排序</option>
            <option value="latest">按最新收录</option>
            <option value="title">按歌名拼音</option>
            <option value="notes">按音符数量</option>
          </select>
        </div>
      </div>

      <!-- 歌曲列表主内容区 -->
      <div class="tracks-content">
        <div v-if="loading && tracks.length === 0" class="state-container loading">
          <Loader2 :size="30" class="spin icon-center" />
          <p>正在检索高品质曲库音源...</p>
        </div>

        <div v-else-if="errorMsg" class="state-container error">
          <HelpCircle :size="32" class="icon-center" />
          <p>{{ errorMsg }}</p>
          <button class="retry-btn" @click="searchTracks">重试</button>
        </div>

        <div v-else-if="tracks.length === 0" class="state-container empty">
          <Music :size="34" class="icon-center" />
          <p>未找到符合条件的曲目</p>
          <div class="empty-hint">您可以尝试输入其他关键词，或在上方切换至其他数据源</div>
        </div>

        <!-- 歌曲表格 -->
        <div v-else class="tracks-table-wrapper">
          <table class="tracks-table">
            <thead>
              <tr>
                <th class="col-num">#</th>
                <th class="col-title">歌曲名称 / 标签</th>
                <th class="col-artist">歌手 / 艺术家</th>
                <th class="col-category">分类</th>
                <th class="col-duration">时长 / 音符</th>
                <th class="col-source">音源来源</th>
                <th class="col-actions">操作</th>
              </tr>
            </thead>
            <tbody>
              <tr
                v-for="(track, idx) in tracks"
                :key="track.id"
                class="track-row"
                :class="{ 'is-playing': currentPlayingTrack?.id === track.id }"
                @dblclick="handlePreview(track)"
              >
                <!-- 序号 / 试听播放按钮 -->
                <td class="col-num">
                  <button
                    class="play-audition-btn"
                    :class="{ 'is-active': currentPlayingTrack?.id === track.id && isPlaying }"
                    :disabled="previewLoadingId === track.id"
                    :title="currentPlayingTrack?.id === track.id && isPlaying ? '暂停试听' : '试听此曲'"
                    @click.stop="handlePreview(track)"
                  >
                    <Loader2 v-if="previewLoadingId === track.id" :size="13" class="spin" />
                    <Pause v-else-if="currentPlayingTrack?.id === track.id && isPlaying" :size="13" />
                    <Play v-else :size="13" />
                  </button>
                </td>

                <!-- 歌曲名称 -->
                <td class="col-title">
                  <div class="title-cell">
                    <span class="track-title-text">{{ track.title }}</span>
                    <span
                      class="difficulty-badge"
                      :class="track.difficulty === '简单' ? 'easy' : track.difficulty === '大师' ? 'master' : 'medium'"
                    >
                      {{ track.difficulty }}
                    </span>
                  </div>
                  <div class="tags-row" v-if="track.tags && track.tags.length > 0">
                    <span v-for="t in track.tags" :key="t" class="tag-pill">{{ t }}</span>
                  </div>
                </td>

                <!-- 歌手 -->
                <td class="col-artist">
                  <span class="artist-text">{{ track.artist }}</span>
                  <div v-if="track.album" class="album-sub">{{ track.album }}</div>
                </td>

                <!-- 分类 -->
                <td class="col-category">
                  <span class="category-tag">{{ track.category_name }}</span>
                </td>

                <!-- 时长 / 音符数 -->
                <td class="col-duration">
                  <div class="duration-text">{{ formatDuration(track.duration_secs) }}</div>
                  <div class="notes-sub">{{ track.notes_count }} 音符</div>
                </td>

                <!-- 音源标签 -->
                <td class="col-source">
                  <span class="source-badge" :class="track.source === 'official' ? 'official' : 'third-party'">
                    {{ track.source_name }}
                  </span>
                </td>

                <!-- 操作按钮组 -->
                <td class="col-actions">
                  <div class="action-buttons">
                    <!-- 下载按钮 -->
                    <button
                      class="action-btn download"
                      :class="{ downloaded: isDownloaded(track) }"
                      :disabled="downloadLoadingId === track.id"
                      :title="isDownloaded(track) ? '已收录在本地曲库' : '下载到本地曲库'"
                      @click.stop="handleDownload(track)"
                    >
                      <Loader2 v-if="downloadLoadingId === track.id" :size="12" class="spin" />
                      <Check v-else-if="isDownloaded(track)" :size="12" />
                      <Download v-else :size="12" />
                      <span>{{ isDownloaded(track) ? '已入库' : '下载' }}</span>
                    </button>

                    <!-- 立即弹奏按钮 -->
                    <button
                      class="action-btn play-game"
                      title="立即在 VRChat 中弹奏这首曲目"
                      @click.stop="handlePlayInGame(track)"
                    >
                      <Piano :size="12" />
                      <span>弹奏</span>
                    </button>
                  </div>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      <!-- 底部播放器控制条 -->
      <div v-if="currentPlayingTrack" class="bottom-player-bar">
        <div class="player-left">
          <div class="player-cover">
            <Music :size="16" />
          </div>
          <div class="player-info">
            <div class="player-title">{{ currentPlayingTrack.title }}</div>
            <div class="player-artist">{{ currentPlayingTrack.artist }} · {{ currentPlayingTrack.category_name }}</div>
          </div>
        </div>

        <div class="player-center">
          <button
            class="ctrl-btn play-pause-btn"
            @click="handlePreview(currentPlayingTrack)"
          >
            <Pause v-if="isPlaying" :size="15" />
            <Play v-else :size="15" />
          </button>
          <div class="playing-indicator" v-if="isPlaying">
            <span class="bar bar1"></span>
            <span class="bar bar2"></span>
            <span class="bar bar3"></span>
          </div>
          <span class="playing-state-text">{{ isPlaying ? '正在试听中' : '试听暂停' }}</span>
        </div>

        <div class="player-right">
          <button
            class="player-action-btn"
            :class="{ downloaded: isDownloaded(currentPlayingTrack) }"
            :disabled="downloadLoadingId === currentPlayingTrack.id"
            @click="handleDownload(currentPlayingTrack)"
          >
            <Loader2 v-if="downloadLoadingId === currentPlayingTrack.id" :size="12" class="spin" />
            <Check v-else-if="isDownloaded(currentPlayingTrack)" :size="12" />
            <Download v-else :size="12" />
            <span>{{ isDownloaded(currentPlayingTrack) ? '已在本地' : '下载此曲' }}</span>
          </button>
          <button
            class="player-action-btn play-direct"
            @click="handlePlayInGame(currentPlayingTrack)"
          >
            <Piano :size="12" />
            <span>游戏弹奏</span>
          </button>
        </div>
      </div>
    </div>

    <!-- 数据源管理弹窗 -->
    <div v-if="showSourceManager" class="source-modal-overlay" @click.self="showSourceManager = false">
      <div class="source-modal-card">
        <div class="source-modal-header">
          <div class="title">数据源与第三方接口配置</div>
          <button class="close-icon" @click="showSourceManager = false">
            <X :size="16" />
          </button>
        </div>

        <div class="source-modal-body">
          <div class="source-list-section">
            <div class="section-title">内置与已配置数据源</div>
            <div class="source-items">
              <div v-for="s in sources" :key="s.id" class="source-item">
                <div class="source-info">
                  <div class="name-row">
                    <span class="s-name">{{ s.name }}</span>
                    <span class="s-type" :class="s.source_type">{{ s.source_type }}</span>
                  </div>
                  <div class="s-url">{{ s.url }}</div>
                  <div class="s-desc">{{ s.description }}</div>
                </div>
                <div class="source-actions" v-if="s.source_type === 'custom'">
                  <button class="delete-btn" title="删除自定义源" @click="removeCustomSource(s.id)">
                    <Trash2 :size="14" />
                  </button>
                </div>
              </div>
            </div>
          </div>

          <!-- 添加自定义第三方源 -->
          <div class="add-source-section">
            <div class="section-title">添加自定义第三方源接口</div>
            <div class="form-row">
              <label>源名称</label>
              <input v-model="newSourceName" placeholder="例如：我的自建曲库 / 朋友的 MIDI 源" />
            </div>
            <div class="form-row">
              <label>接口 URL</label>
              <input v-model="newSourceUrl" placeholder="https://example.com/api/midi/search" />
            </div>
            <div class="form-row">
              <label>源描述 (可选)</label>
              <input v-model="newSourceDesc" placeholder="支持标准 JSON 格式或直链下载列表" />
            </div>
            <div v-if="sourceAddError" class="form-error">{{ sourceAddError }}</div>
            <div class="form-submit">
              <button class="add-btn" :disabled="addingSource" @click="addCustomSource">
                <Loader2 v-if="addingSource" :size="14" class="spin" />
                <Plus v-else :size="14" />
                <span>保存并添加数据源</span>
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
/* 模态遮罩层：适配动态毛玻璃风格 */
.midi-cloud-modal-backdrop {
  position: fixed;
  inset: 0;
  z-index: 50;
  background: rgba(36, 22, 8, 0.35);
  backdrop-filter: blur(10px);
  -webkit-backdrop-filter: blur(10px);
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 24px;
}

/* 主窗口：采用软件原生动态主题变量 (--vp-*)，杜绝死板 AI 模板感 */
.midi-cloud-window {
  --vp-surface: var(--theme-surface, rgba(255, 252, 240, 0.62));
  --vp-panel: var(--theme-surface-hover, rgba(255, 252, 240, 0.82));
  --vp-hover: var(--theme-active-bg, rgba(251, 191, 36, 0.2));
  --vp-border: var(--theme-border-soft, rgba(120, 53, 15, 0.12));
  --vp-border-strong: var(--theme-border-strong, rgba(120, 53, 15, 0.22));
  --vp-text: var(--theme-text-strong, #451a03);
  --vp-muted: var(--theme-text-soft, #76552d);
  --vp-dim: var(--theme-text-muted, #9a7b4f);
  --vp-primary: var(--theme-primary, #d97706);
  --vp-primary-hover: var(--theme-primary-hover, #b45309);

  position: relative;
  width: 100%;
  max-width: 1100px;
  height: 85vh;
  max-height: 820px;
  background: var(--vp-surface);
  backdrop-filter: var(--theme-glass-effect, blur(24px) saturate(180%));
  -webkit-backdrop-filter: var(--theme-glass-effect, blur(24px) saturate(180%));
  border: 1px solid var(--vp-border-strong);
  border-radius: 12px;
  box-shadow: 0 24px 60px rgba(0, 0, 0, 0.22);
  display: flex;
  flex-direction: column;
  overflow: hidden;
  color: var(--vp-text);
  font-family: inherit;
}

/* 顶部标题栏 */
.window-header {
  padding: 12px 18px;
  border-bottom: 1px solid var(--vp-border);
  display: flex;
  align-items: center;
  justify-content: space-between;
  background: var(--vp-panel);
}

.brand-badge {
  display: flex;
  align-items: center;
  gap: 10px;
}

.brand-icon {
  color: var(--vp-primary);
}

.brand-name {
  font-size: 15px;
  font-weight: 700;
  color: var(--vp-text);
  line-height: 1.2;
}

.brand-sub {
  font-size: 11px;
  color: var(--vp-muted);
  margin-top: 2px;
}

.header-actions {
  display: flex;
  align-items: center;
  gap: 10px;
}

.source-selector {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 0 8px;
  height: 30px;
  border-radius: 7px;
  border: 1px solid var(--vp-border);
  background: var(--vp-surface);
  color: var(--vp-muted);
}

.source-selector select {
  border: none;
  background: transparent;
  outline: none;
  color: var(--vp-text);
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  padding: 0;
}

.header-btn {
  height: 30px;
  padding: 0 12px;
  border-radius: 7px;
  border: 1px solid var(--vp-border);
  background: var(--vp-surface);
  color: var(--vp-text);
  font-size: 12px;
  font-weight: 600;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  cursor: pointer;
  transition: all 0.15s ease;
}

.header-btn:hover {
  background: var(--vp-hover);
  border-color: var(--vp-border-strong);
}

.close-btn {
  width: 30px;
  height: 30px;
  border-radius: 7px;
  border: 1px solid var(--vp-border);
  background: var(--vp-surface);
  color: var(--vp-muted);
  display: inline-flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: all 0.15s ease;
}

.close-btn:hover {
  background: var(--vp-hover);
  color: var(--vp-text);
}

/* 分类标签栏 */
.category-navbar {
  padding: 8px 18px;
  display: flex;
  align-items: center;
  gap: 8px;
  overflow-x: auto;
  background: color-mix(in srgb, var(--vp-panel) 50%, transparent);
  border-bottom: 1px solid var(--vp-border);
}

.cat-pill {
  height: 28px;
  padding: 0 12px;
  border-radius: 14px;
  border: 1px solid var(--vp-border);
  background: var(--vp-surface);
  color: var(--vp-muted);
  font-size: 12px;
  font-weight: 600;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  cursor: pointer;
  white-space: nowrap;
  transition: all 0.18s ease;
}

.cat-pill:hover {
  background: var(--vp-hover);
  color: var(--vp-text);
  border-color: var(--vp-border-strong);
}

.cat-pill.active {
  background: var(--vp-primary);
  color: #ffffff;
  border-color: var(--vp-primary);
  box-shadow: 0 2px 6px color-mix(in srgb, var(--vp-primary) 35%, transparent);
}

.cat-count {
  font-size: 10px;
  padding: 1px 6px;
  border-radius: 10px;
  background: color-mix(in srgb, var(--vp-primary) 14%, transparent);
  color: var(--vp-primary);
  font-weight: 700;
}

.cat-pill.active .cat-count {
  background: rgba(255, 255, 255, 0.25);
  color: #ffffff;
}

/* 搜索工具栏 */
.search-toolbar {
  padding: 10px 18px;
  display: flex;
  align-items: center;
  gap: 12px;
  border-bottom: 1px solid var(--vp-border);
  background: var(--vp-surface);
  flex-wrap: wrap;
}

.search-box {
  flex: 1;
  min-width: 260px;
  max-width: 480px;
  height: 32px;
  border-radius: 7px;
  border: 1px solid var(--vp-border-strong);
  background: var(--vp-panel);
  display: flex;
  align-items: center;
  padding: 0 4px 0 10px;
  gap: 8px;
}

.search-icon {
  color: var(--vp-dim);
}

.search-box input {
  flex: 1;
  border: none;
  background: transparent;
  outline: none;
  color: var(--vp-text);
  font-size: 12px;
}

.search-box input::placeholder {
  color: var(--vp-dim);
}

.clear-input-btn {
  border: none;
  background: transparent;
  color: var(--vp-dim);
  cursor: pointer;
  padding: 2px;
  display: flex;
  align-items: center;
}

.clear-input-btn:hover {
  color: var(--vp-text);
}

.search-submit-btn {
  height: 24px;
  padding: 0 12px;
  border-radius: 5px;
  border: none;
  background: var(--vp-primary);
  color: #ffffff;
  font-size: 11px;
  font-weight: 600;
  cursor: pointer;
  transition: background 0.15s ease;
  display: inline-flex;
  align-items: center;
  justify-content: center;
}

.search-submit-btn:hover:not(:disabled) {
  background: var(--vp-primary-hover);
}

.hot-chips {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-wrap: wrap;
}

.hot-label {
  font-size: 11px;
  color: var(--vp-dim);
}

.hot-chip {
  height: 22px;
  padding: 0 9px;
  border-radius: 11px;
  border: 1px solid var(--vp-border);
  background: var(--vp-panel);
  color: var(--vp-muted);
  font-size: 11px;
  cursor: pointer;
  transition: all 0.15s ease;
  white-space: nowrap;
}

.hot-chip:hover {
  background: var(--vp-hover);
  color: var(--vp-primary);
  border-color: var(--vp-primary);
}

.sort-selector select {
  height: 30px;
  border-radius: 6px;
  border: 1px solid var(--vp-border);
  background: var(--vp-surface);
  color: var(--vp-text);
  font-size: 11px;
  padding: 0 8px;
  outline: none;
  cursor: pointer;
}

/* 列表主内容区 */
.tracks-content {
  flex: 1;
  overflow-y: auto;
  position: relative;
}

.state-container {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100%;
  padding: 40px;
  color: var(--vp-muted);
  gap: 12px;
}

.icon-center {
  color: var(--vp-primary);
}

.retry-btn {
  background: var(--vp-primary);
  color: #fff;
  border: none;
  border-radius: 6px;
  padding: 6px 16px;
  font-size: 12px;
  cursor: pointer;
}

.tracks-table-wrapper {
  width: 100%;
}

.tracks-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 12px;
}

.tracks-table thead th {
  text-align: left;
  padding: 9px 14px;
  font-size: 11px;
  color: var(--vp-muted);
  border-bottom: 1px solid var(--vp-border-strong);
  background: var(--vp-panel);
  font-weight: 700;
  position: sticky;
  top: 0;
  z-index: 10;
}

.track-row {
  border-bottom: 1px solid var(--vp-border);
  transition: background 0.15s;
}

.track-row:hover {
  background: var(--vp-hover);
}

.track-row.is-playing {
  background: color-mix(in srgb, var(--vp-primary) 12%, var(--vp-surface));
  border-left: 3px solid var(--vp-primary);
}

.tracks-table td {
  padding: 8px 14px;
  vertical-align: middle;
}

.col-num {
  width: 44px;
  text-align: center;
}

/* 试听圆按钮 */
.play-audition-btn {
  width: 28px;
  height: 28px;
  min-width: 28px;
  border-radius: 50%;
  border: 1px solid var(--vp-border);
  background: var(--vp-panel);
  color: var(--vp-muted);
  display: inline-flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: all 0.18s ease;
}

.play-audition-btn:hover:not(:disabled) {
  background: var(--vp-hover);
  color: var(--vp-primary);
  border-color: var(--vp-primary);
  transform: scale(1.05);
}

.play-audition-btn.is-active {
  background: var(--vp-primary);
  color: #ffffff;
  border-color: var(--vp-primary);
}

.title-cell {
  display: flex;
  align-items: center;
  gap: 8px;
}

.track-title-text {
  font-size: 13px;
  font-weight: 600;
  color: var(--vp-text);
}

.difficulty-badge {
  font-size: 10px;
  padding: 1px 6px;
  border-radius: 4px;
  font-weight: 600;
}

.difficulty-badge.easy {
  background: color-mix(in srgb, #22c55e 14%, transparent);
  color: #16a34a;
  border: 1px solid color-mix(in srgb, #22c55e 30%, transparent);
}

.difficulty-badge.medium {
  background: color-mix(in srgb, #f59e0b 14%, transparent);
  color: #d97706;
  border: 1px solid color-mix(in srgb, #f59e0b 30%, transparent);
}

.difficulty-badge.master {
  background: color-mix(in srgb, #8b5cf6 14%, transparent);
  color: #7c3aed;
  border: 1px solid color-mix(in srgb, #8b5cf6 30%, transparent);
}

.tags-row {
  display: flex;
  gap: 4px;
  margin-top: 4px;
}

.tag-pill {
  font-size: 10px;
  background: color-mix(in srgb, var(--vp-primary) 8%, var(--vp-panel));
  color: var(--vp-muted);
  padding: 1px 5px;
  border-radius: 3px;
  border: 1px solid var(--vp-border);
}

.artist-text {
  font-size: 12px;
  color: var(--vp-text);
}

.album-sub {
  font-size: 10px;
  color: var(--vp-muted);
  margin-top: 2px;
}

.category-tag {
  font-size: 11px;
  color: var(--vp-muted);
}

.duration-text {
  font-size: 12px;
  color: var(--vp-text);
}

.notes-sub {
  font-size: 10px;
  color: var(--vp-muted);
  margin-top: 2px;
}

.source-badge {
  font-size: 11px;
  padding: 2px 8px;
  border-radius: 4px;
  border: 1px solid var(--vp-border);
  background: var(--vp-panel);
  color: var(--vp-muted);
}

.source-badge.official {
  background: color-mix(in srgb, var(--vp-primary) 10%, var(--vp-panel));
  color: var(--vp-primary);
  border-color: color-mix(in srgb, var(--vp-primary) 25%, transparent);
}

/* 操作按钮列：固定宽度与统一尺寸 */
.col-actions {
  width: 170px;
  min-width: 170px;
}

.action-buttons {
  display: flex;
  align-items: center;
  gap: 6px;
  justify-content: flex-end;
}

.action-btn {
  height: 28px;
  min-height: 28px;
  padding: 0 10px;
  border-radius: 6px;
  font-size: 12px;
  font-weight: 600;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 4px;
  cursor: pointer;
  transition: all 0.15s ease;
  box-sizing: border-box;
  white-space: nowrap;
}

.action-btn.download {
  border: 1px solid var(--vp-border);
  background: var(--vp-surface);
  color: var(--vp-text);
}

.action-btn.download:hover:not(:disabled) {
  background: var(--vp-hover);
  border-color: var(--vp-border-strong);
}

.action-btn.download.downloaded {
  border: 1px solid color-mix(in srgb, #22c55e 35%, transparent);
  background: color-mix(in srgb, #22c55e 12%, var(--vp-panel));
  color: #16a34a;
}

.action-btn.download.downloaded:hover:not(:disabled) {
  background: color-mix(in srgb, #22c55e 20%, var(--vp-panel));
}

.action-btn.play-game {
  border: 1px solid color-mix(in srgb, var(--vp-primary) 35%, transparent);
  background: color-mix(in srgb, var(--vp-primary) 12%, var(--vp-panel));
  color: var(--vp-primary);
}

.action-btn.play-game:hover:not(:disabled) {
  background: var(--vp-primary);
  color: #ffffff;
  border-color: var(--vp-primary);
}

/* 底部播放条 */
.bottom-player-bar {
  padding: 10px 18px;
  border-top: 1px solid var(--vp-border-strong);
  background: var(--vp-panel);
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.player-left {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 240px;
}

.player-cover {
  width: 32px;
  height: 32px;
  background: color-mix(in srgb, var(--vp-primary) 14%, var(--vp-surface));
  border: 1px solid color-mix(in srgb, var(--vp-primary) 28%, transparent);
  border-radius: 6px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--vp-primary);
}

.player-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--vp-text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 180px;
}

.player-artist {
  font-size: 11px;
  color: var(--vp-muted);
}

.player-center {
  display: flex;
  align-items: center;
  gap: 12px;
}

.ctrl-btn {
  width: 32px;
  height: 32px;
  border-radius: 50%;
  border: none;
  background: var(--vp-primary);
  color: #fff;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: all 0.15s ease;
}

.ctrl-btn:hover {
  background: var(--vp-primary-hover);
  transform: scale(1.06);
}

.playing-state-text {
  font-size: 12px;
  color: var(--vp-muted);
}

.playing-indicator {
  display: flex;
  align-items: flex-end;
  gap: 2px;
  height: 14px;
}

.playing-indicator .bar {
  width: 3px;
  background: var(--vp-primary);
  border-radius: 1px;
  animation: soundBars 0.8s ease-in-out infinite alternate;
}

.playing-indicator .bar1 { height: 8px; animation-delay: 0.1s; }
.playing-indicator .bar2 { height: 14px; animation-delay: 0.3s; }
.playing-indicator .bar3 { height: 6px; animation-delay: 0.2s; }

@keyframes soundBars {
  0% { height: 3px; }
  100% { height: 14px; }
}

.player-right {
  display: flex;
  align-items: center;
  gap: 8px;
}

.player-action-btn {
  height: 28px;
  padding: 0 10px;
  border-radius: 6px;
  border: 1px solid var(--vp-border);
  background: var(--vp-surface);
  color: var(--vp-text);
  font-size: 12px;
  font-weight: 600;
  display: inline-flex;
  align-items: center;
  gap: 5px;
  cursor: pointer;
  transition: all 0.15s ease;
}

.player-action-btn:hover:not(:disabled) {
  background: var(--vp-hover);
}

.player-action-btn.downloaded {
  color: #16a34a;
  border-color: color-mix(in srgb, #22c55e 35%, transparent);
  background: color-mix(in srgb, #22c55e 12%, var(--vp-panel));
}

.player-action-btn.play-direct {
  background: var(--vp-primary);
  color: #fff;
  border-color: var(--vp-primary);
}

.player-action-btn.play-direct:hover {
  background: var(--vp-primary-hover);
}

/* 浮动 Toast 样式 */
.cloud-toast {
  position: absolute;
  top: 14px;
  left: 50%;
  transform: translateX(-50%);
  z-index: 100;
  padding: 7px 16px;
  border-radius: 8px;
  font-size: 12px;
  font-weight: 600;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  box-shadow: 0 4px 18px rgba(0, 0, 0, 0.16);
  pointer-events: none;
}

.cloud-toast.success {
  background: #16a34a;
  color: #ffffff;
}

.cloud-toast.error {
  background: #dc2626;
  color: #ffffff;
}

.cloud-toast.info {
  background: var(--vp-panel);
  color: var(--vp-text);
  border: 1px solid var(--vp-border-strong);
}

.toast-fade-enter-active,
.toast-fade-leave-active {
  transition: opacity 0.2s ease, transform 0.2s ease;
}

.toast-fade-enter-from,
.toast-fade-leave-to {
  opacity: 0;
  transform: translate(-50%, -10px);
}

/* 数据源管理弹窗 */
.source-modal-overlay {
  position: fixed;
  inset: 0;
  z-index: 60;
  background: rgba(0, 0, 0, 0.45);
  backdrop-filter: blur(6px);
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 20px;
}

.source-modal-card {
  width: 100%;
  max-width: 600px;
  max-height: 85vh;
  background: var(--vp-surface);
  border: 1px solid var(--vp-border-strong);
  border-radius: 10px;
  box-shadow: 0 20px 50px rgba(0, 0, 0, 0.25);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.source-modal-header {
  padding: 12px 16px;
  border-bottom: 1px solid var(--vp-border);
  display: flex;
  align-items: center;
  justify-content: space-between;
  background: var(--vp-panel);
}

.source-modal-header .title {
  font-size: 14px;
  font-weight: 700;
  color: var(--vp-text);
}

.close-icon {
  background: none;
  border: none;
  color: var(--vp-muted);
  cursor: pointer;
  padding: 4px;
}

.close-icon:hover {
  color: var(--vp-text);
}

.source-modal-body {
  padding: 16px;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.section-title {
  font-size: 12px;
  font-weight: 700;
  color: var(--vp-text);
  margin-bottom: 8px;
}

.source-items {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.source-item {
  padding: 10px 12px;
  border-radius: 8px;
  border: 1px solid var(--vp-border);
  background: var(--vp-panel);
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.s-name {
  font-size: 13px;
  font-weight: 600;
  color: var(--vp-text);
}

.s-type {
  font-size: 10px;
  padding: 1px 6px;
  border-radius: 4px;
  margin-left: 6px;
  background: color-mix(in srgb, var(--vp-primary) 12%, transparent);
  color: var(--vp-primary);
}

.s-url {
  font-size: 11px;
  color: var(--vp-muted);
  margin-top: 2px;
  word-break: break-all;
}

.s-desc {
  font-size: 11px;
  color: var(--vp-dim);
  margin-top: 2px;
}

.delete-btn {
  background: none;
  border: none;
  color: #ef4444;
  cursor: pointer;
  padding: 4px;
}

.add-source-section {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding-top: 10px;
  border-top: 1px solid var(--vp-border);
}

.form-row {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.form-row label {
  font-size: 11px;
  font-weight: 600;
  color: var(--vp-muted);
}

.form-row input {
  height: 30px;
  padding: 0 10px;
  border-radius: 6px;
  border: 1px solid var(--vp-border);
  background: var(--vp-panel);
  color: var(--vp-text);
  font-size: 12px;
  outline: none;
}

.form-row input:focus {
  border-color: var(--vp-primary);
}

.form-error {
  color: #ef4444;
  font-size: 11px;
}

.add-btn {
  height: 32px;
  padding: 0 16px;
  border-radius: 6px;
  border: none;
  background: var(--vp-primary);
  color: #fff;
  font-size: 12px;
  font-weight: 600;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  cursor: pointer;
  margin-top: 6px;
}

.add-btn:hover:not(:disabled) {
  background: var(--vp-primary-hover);
}

.spin {
  animation: spin 1s linear infinite;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}
</style>