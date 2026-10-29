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
const pageSize = ref(20);
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

// 加载分类列表与数据源
const loadCategoriesAndSources = async () => {
  try {
    const catRes = await VrpianoApi.cloudLibraryCategories();
    if (catRes && catRes.categories) {
      categories.value = catRes.categories;
    }
  } catch (e: any) {
    console.warn('获取分类失败，使用默认分类:', e);
  }

  try {
    const srcRes = await VrpianoApi.cloudLibrarySources();
    if (srcRes && srcRes.sources) {
      sources.value = srcRes.sources;
    }
  } catch (e: any) {
    console.warn('获取数据源失败:', e);
  }
};

// 执行曲库检索
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

// 切换分类
const selectCategory = (catId: string) => {
  activeCategory.value = catId;
  currentPage.value = 1;
  searchTracks();
};

// 点击热门关键词
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
    // 构造请求，将云端 download_url 传给试听解析接口
    const midiData = await VrpianoApi.midishowPreviewData({
      midiId: 999999,
      title: `${track.title} - ${track.artist}`,
      downloadUrl: track.download_url,
      source: 'vrcdog_cloud',
    });
    currentPlayingTrack.value = track;
    isPlaying.value = true;
    emit('preview-midi', midiData);
  } catch (e: any) {
    alert(`试听失败: ${e.message || String(e)}`);
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
    }
  } catch (e: any) {
    alert(`下载失败: ${e.message || String(e)}`);
  } finally {
    downloadLoadingId.value = null;
  }
};

// 立即弹奏
const handlePlayInGame = async (track: MidiCloudTrack) => {
  // 如果尚未下载，先自动下载后直接触发弹奏
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
      alert(`准备弹奏失败: ${e.message || String(e)}`);
    } finally {
      downloadLoadingId.value = null;
    }
  } else {
    // 已下载，在 localSongs 中找到并播放
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

const formatSize = (bytes: number) => {
  if (bytes < 1024) return `${bytes} B`;
  return `${(bytes / 1024).toFixed(1)} KB`;
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
    const nextList = [...existing, newSrc];
    await VrpianoApi.cloudSourcesSave(nextList);
    newSourceName.value = '';
    newSourceUrl.value = '';
    newSourceDesc.value = '';
    await loadCategoriesAndSources();
  } catch (e: any) {
    sourceAddError.value = `保存失败: ${e.message || String(e)}`;
  } finally {
    addingSource.value = false;
  }
};

const removeCustomSource = async (id: string) => {
  try {
    const existing = await VrpianoApi.cloudSourcesGet();
    const nextList = existing.filter((s) => s.id !== id);
    await VrpianoApi.cloudSourcesSave(nextList);
    await loadCategoriesAndSources();
  } catch (e: any) {
    alert(`删除失败: ${e.message || String(e)}`);
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
            <SlidersHorizontal :size="15" />
            <span>数据源管理</span>
          </button>

          <button class="close-btn" @click="emit('close')" title="关闭">
            <X :size="18" />
          </button>
        </div>
      </div>

      <!-- 分类标签导航栏 (网易云音乐风格) -->
      <div class="category-navbar">
        <button
          v-for="cat in categories"
          :key="cat.id"
          class="cat-pill"
          :class="{ active: activeCategory === cat.id }"
          @click="selectCategory(cat.id)"
        >
          <component :is="getCategoryIcon(cat.id)" :size="14" class="cat-icon" />
          <span class="cat-name">{{ cat.name }}</span>
          <span v-if="cat.count > 0" class="cat-count">{{ cat.count }}</span>
        </button>
      </div>

      <!-- 搜索与筛选工具栏 -->
      <div class="search-toolbar">
        <div class="search-box">
          <Search :size="16" class="search-icon" />
          <input
            v-model="searchQuery"
            type="text"
            placeholder="搜索歌名、艺术家、原神、周杰伦、动漫名或标签..."
            @keyup.enter="searchTracks"
          />
          <button v-if="searchQuery" class="clear-input-btn" @click="searchQuery = ''; searchTracks()">
            <X :size="14" />
          </button>
          <button class="search-submit-btn" :disabled="loading" @click="searchTracks">
            <Loader2 v-if="loading" :size="14" class="spin" />
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
          <Loader2 :size="32" class="spin icon-center" />
          <p>正在连接曲库并检索优质音源...</p>
        </div>

        <div v-else-if="errorMsg" class="state-container error">
          <HelpCircle :size="32" class="icon-center" />
          <p>{{ errorMsg }}</p>
          <button class="retry-btn" @click="searchTracks">重试</button>
        </div>

        <div v-else-if="tracks.length === 0" class="state-container empty">
          <Music :size="36" class="icon-center text-muted" />
          <p>未找到符合条件的曲目</p>
          <div class="empty-hint">您可以尝试输入其他关键词，或在上方切换至其他第三方数据源</div>
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
              >
                <!-- 序号 / 试听播放按钮 -->
                <td class="col-num">
                  <button
                    class="play-audition-btn"
                    :disabled="previewLoadingId === track.id"
                    :title="currentPlayingTrack?.id === track.id && isPlaying ? '暂停试听' : '试听此曲'"
                    @click="handlePreview(track)"
                  >
                    <Loader2 v-if="previewLoadingId === track.id" :size="14" class="spin" />
                    <Pause v-else-if="currentPlayingTrack?.id === track.id && isPlaying" :size="14" />
                    <Play v-else :size="14" />
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
                      :title="isDownloaded(track) ? '已保存在本地曲库中' : '下载到本地曲库'"
                      @click="handleDownload(track)"
                    >
                      <Loader2 v-if="downloadLoadingId === track.id" :size="13" class="spin" />
                      <Check v-else-if="isDownloaded(track)" :size="13" />
                      <Download v-else :size="13" />
                      <span>{{ isDownloaded(track) ? '已入库' : '下载' }}</span>
                    </button>

                    <!-- 立即弹奏按钮 -->
                    <button
                      class="action-btn play-game"
                      title="立即在 VRChat 中弹奏这首曲目"
                      @click="handlePlayInGame(track)"
                    >
                      <Piano :size="13" />
                      <span>弹奏</span>
                    </button>
                  </div>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      <!-- 底部播放器控制条 (网易云音乐风格) -->
      <div v-if="currentPlayingTrack" class="bottom-player-bar">
        <div class="player-left">
          <div class="player-cover">
            <Music :size="18" />
          </div>
          <div class="player-info">
            <div class="player-title">{{ currentPlayingTrack.title }}</div>
            <div class="player-artist">{{ currentPlayingTrack.artist }} · {{ currentPlayingTrack.category_name }}</div>
          </div>
        </div>

        <div class="player-center">
          <div class="player-controls">
            <button
              class="ctrl-btn play-pause-btn"
              @click="handlePreview(currentPlayingTrack)"
            >
              <Pause v-if="isPlaying" :size="16" />
              <Play v-else :size="16" />
            </button>
            <div class="playing-indicator" v-if="isPlaying">
              <span class="bar bar1"></span>
              <span class="bar bar2"></span>
              <span class="bar bar3"></span>
            </div>
            <span class="playing-state-text">{{ isPlaying ? '正在试听' : '已暂停' }}</span>
          </div>
        </div>

        <div class="player-right">
          <button
            class="player-action-btn"
            :class="{ downloaded: isDownloaded(currentPlayingTrack) }"
            :disabled="downloadLoadingId === currentPlayingTrack.id"
            @click="handleDownload(currentPlayingTrack)"
          >
            <Loader2 v-if="downloadLoadingId === currentPlayingTrack.id" :size="13" class="spin" />
            <Check v-else-if="isDownloaded(currentPlayingTrack)" :size="13" />
            <Download v-else :size="13" />
            <span>{{ isDownloaded(currentPlayingTrack) ? '已下载到本地' : '下载此曲' }}</span>
          </button>
          <button
            class="player-action-btn play-direct"
            @click="handlePlayInGame(currentPlayingTrack)"
          >
            <Piano :size="13" />
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
.midi-cloud-modal-backdrop {
  position: fixed;
  inset: 0;
  z-index: 9999;
  background: rgba(10, 11, 16, 0.75);
  backdrop-filter: blur(8px);
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 24px;
}

.midi-cloud-window {
  width: 100%;
  max-width: 1100px;
  height: 85vh;
  max-height: 820px;
  background: #141622;
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 14px;
  box-shadow: 0 20px 50px rgba(0, 0, 0, 0.6);
  display: flex;
  flex-direction: column;
  overflow: hidden;
  color: #f1f5f9;
  font-family: inherit;
}

/* 顶部控制条 */
.window-header {
  padding: 16px 20px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  display: flex;
  align-items: center;
  justify-content: space-between;
  background: #181b29;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 12px;
}

.brand-badge {
  display: flex;
  align-items: center;
  gap: 12px;
}

.brand-icon {
  color: #818cf8;
}

.brand-titles {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.brand-name {
  font-size: 16px;
  font-weight: 600;
  color: #fff;
}

.brand-sub {
  font-size: 11px;
  color: #94a3b8;
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
  background: rgba(255, 255, 255, 0.06);
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 6px;
  padding: 5px 10px;
}

.source-icon {
  color: #94a3b8;
}

.source-selector select {
  background: transparent;
  border: none;
  color: #e2e8f0;
  font-size: 12px;
  outline: none;
  cursor: pointer;
}

.source-selector select option {
  background: #1e2235;
  color: #e2e8f0;
}

.header-btn {
  display: flex;
  align-items: center;
  gap: 6px;
  background: rgba(255, 255, 255, 0.06);
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 6px;
  padding: 6px 12px;
  color: #cbd5e1;
  font-size: 12px;
  cursor: pointer;
  transition: all 0.2s;
}

.header-btn:hover {
  background: rgba(255, 255, 255, 0.12);
  color: #fff;
}

.close-btn {
  background: transparent;
  border: none;
  color: #94a3b8;
  cursor: pointer;
  padding: 6px;
  border-radius: 6px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.2s;
}

.close-btn:hover {
  background: rgba(255, 255, 255, 0.1);
  color: #fff;
}

/* 分类标签栏 */
.category-navbar {
  padding: 10px 20px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.06);
  display: flex;
  align-items: center;
  gap: 8px;
  overflow-x: auto;
  background: #141624;
}

.cat-pill {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 14px;
  border-radius: 20px;
  font-size: 12px;
  font-weight: 500;
  background: rgba(255, 255, 255, 0.04);
  border: 1px solid rgba(255, 255, 255, 0.08);
  color: #94a3b8;
  cursor: pointer;
  white-space: nowrap;
  transition: all 0.2s;
}

.cat-pill:hover {
  background: rgba(255, 255, 255, 0.08);
  color: #f1f5f9;
}

.cat-pill.active {
  background: linear-gradient(135deg, #6366f1, #818cf8);
  color: #fff;
  border-color: rgba(99, 102, 241, 0.4);
  box-shadow: 0 2px 10px rgba(99, 102, 241, 0.35);
}

.cat-count {
  font-size: 10px;
  padding: 1px 6px;
  background: rgba(0, 0, 0, 0.25);
  border-radius: 10px;
}

/* 搜索与工具栏 */
.search-toolbar {
  padding: 12px 20px;
  display: flex;
  align-items: center;
  gap: 14px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.06);
  background: #161827;
}

.search-box {
  flex: 1;
  max-width: 480px;
  display: flex;
  align-items: center;
  background: rgba(0, 0, 0, 0.25);
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 8px;
  padding: 4px 6px 4px 12px;
  gap: 8px;
}

.search-icon {
  color: #64748b;
}

.search-box input {
  flex: 1;
  background: transparent;
  border: none;
  outline: none;
  color: #fff;
  font-size: 13px;
}

.clear-input-btn {
  background: transparent;
  border: none;
  color: #64748b;
  cursor: pointer;
  padding: 4px;
  display: flex;
}

.search-submit-btn {
  background: #6366f1;
  color: #fff;
  border: none;
  border-radius: 6px;
  padding: 5px 12px;
  font-size: 12px;
  cursor: pointer;
  transition: all 0.2s;
}

.search-submit-btn:hover {
  background: #4f46e5;
}

.hot-chips {
  flex: 1;
  display: flex;
  align-items: center;
  gap: 6px;
  overflow-x: auto;
}

.hot-label {
  font-size: 11px;
  color: #64748b;
  white-space: nowrap;
}

.hot-chip {
  background: rgba(255, 255, 255, 0.04);
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 4px;
  padding: 3px 8px;
  font-size: 11px;
  color: #cbd5e1;
  cursor: pointer;
  white-space: nowrap;
  transition: all 0.15s;
}

.hot-chip:hover {
  background: rgba(99, 102, 241, 0.15);
  border-color: rgba(99, 102, 241, 0.3);
  color: #818cf8;
}

.sort-selector select {
  background: rgba(255, 255, 255, 0.06);
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 6px;
  color: #cbd5e1;
  font-size: 12px;
  padding: 6px 10px;
  outline: none;
  cursor: pointer;
}

.sort-selector select option {
  background: #1e2235;
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
  color: #94a3b8;
  gap: 12px;
}

.icon-center {
  color: #6366f1;
}

.retry-btn {
  background: #6366f1;
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
  padding: 10px 16px;
  font-size: 11px;
  color: #64748b;
  border-bottom: 1px solid rgba(255, 255, 255, 0.06);
  font-weight: 500;
}

.track-row {
  border-bottom: 1px solid rgba(255, 255, 255, 0.03);
  transition: background 0.15s;
}

.track-row:hover {
  background: rgba(255, 255, 255, 0.04);
}

.track-row.is-playing {
  background: rgba(99, 102, 241, 0.08);
}

.tracks-table td {
  padding: 10px 16px;
  vertical-align: middle;
}

.col-num {
  width: 44px;
  text-align: center;
}

.play-audition-btn {
  width: 28px;
  height: 28px;
  border-radius: 50%;
  background: rgba(255, 255, 255, 0.06);
  border: 1px solid rgba(255, 255, 255, 0.1);
  color: #cbd5e1;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: all 0.2s;
}

.play-audition-btn:hover {
  background: #6366f1;
  color: #fff;
  border-color: #6366f1;
  transform: scale(1.05);
}

.title-cell {
  display: flex;
  align-items: center;
  gap: 8px;
}

.track-title-text {
  font-size: 13px;
  font-weight: 600;
  color: #f1f5f9;
}

.difficulty-badge {
  font-size: 10px;
  padding: 1px 6px;
  border-radius: 4px;
}

.difficulty-badge.easy {
  background: rgba(34, 197, 94, 0.15);
  color: #4ade80;
}

.difficulty-badge.medium {
  background: rgba(245, 158, 11, 0.15);
  color: #fbbf24;
}

.difficulty-badge.master {
  background: rgba(168, 85, 247, 0.15);
  color: #c084fc;
}

.tags-row {
  display: flex;
  gap: 4px;
  margin-top: 4px;
}

.tag-pill {
  font-size: 10px;
  background: rgba(255, 255, 255, 0.04);
  color: #94a3b8;
  padding: 1px 5px;
  border-radius: 3px;
}

.artist-text {
  font-size: 12px;
  color: #cbd5e1;
}

.album-sub {
  font-size: 10px;
  color: #64748b;
  margin-top: 2px;
}

.category-tag {
  font-size: 11px;
  color: #94a3b8;
}

.duration-text {
  font-size: 12px;
  color: #cbd5e1;
}

.notes-sub {
  font-size: 10px;
  color: #64748b;
  margin-top: 2px;
}

.source-badge {
  font-size: 11px;
  padding: 2px 8px;
  border-radius: 4px;
  background: rgba(255, 255, 255, 0.06);
  color: #94a3b8;
}

.source-badge.official {
  background: rgba(99, 102, 241, 0.15);
  color: #818cf8;
  border: 1px solid rgba(99, 102, 241, 0.25);
}

.col-actions {
  width: 170px;
}

.action-buttons {
  display: flex;
  align-items: center;
  gap: 8px;
}

.action-btn {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 5px 10px;
  border-radius: 6px;
  font-size: 11px;
  border: 1px solid rgba(255, 255, 255, 0.1);
  background: rgba(255, 255, 255, 0.06);
  color: #cbd5e1;
  cursor: pointer;
  transition: all 0.15s;
}

.action-btn:hover {
  background: rgba(255, 255, 255, 0.12);
  color: #fff;
}

.action-btn.download.downloaded {
  background: rgba(34, 197, 94, 0.12);
  border-color: rgba(34, 197, 94, 0.25);
  color: #4ade80;
}

.action-btn.play-game {
  background: linear-gradient(135deg, #4f46e5, #6366f1);
  border: none;
  color: #fff;
}

.action-btn.play-game:hover {
  opacity: 0.9;
}

/* 底部播放条 */
.bottom-player-bar {
  padding: 12px 20px;
  border-top: 1px solid rgba(255, 255, 255, 0.08);
  background: #181b29;
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.player-left {
  display: flex;
  align-items: center;
  gap: 12px;
  width: 240px;
}

.player-cover {
  width: 36px;
  height: 36px;
  background: rgba(99, 102, 241, 0.15);
  border: 1px solid rgba(99, 102, 241, 0.3);
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #818cf8;
}

.player-title {
  font-size: 13px;
  font-weight: 600;
  color: #fff;
}

.player-artist {
  font-size: 11px;
  color: #94a3b8;
}

.player-center {
  display: flex;
  align-items: center;
  gap: 12px;
}

.player-controls {
  display: flex;
  align-items: center;
  gap: 10px;
}

.play-pause-btn {
  width: 34px;
  height: 34px;
  border-radius: 50%;
  background: #6366f1;
  border: none;
  color: #fff;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: all 0.2s;
}

.play-pause-btn:hover {
  background: #4f46e5;
  transform: scale(1.05);
}

.playing-state-text {
  font-size: 12px;
  color: #94a3b8;
}

.playing-indicator {
  display: flex;
  align-items: flex-end;
  gap: 2px;
  height: 14px;
}

.playing-indicator .bar {
  width: 3px;
  background: #818cf8;
  border-radius: 2px;
  animation: soundwave 1s ease-in-out infinite alternate;
}

.playing-indicator .bar1 { height: 8px; animation-delay: 0.1s; }
.playing-indicator .bar2 { height: 14px; animation-delay: 0.3s; }
.playing-indicator .bar3 { height: 6px; animation-delay: 0.2s; }

@keyframes soundwave {
  0% { height: 3px; }
  100% { height: 14px; }
}

.player-right {
  display: flex;
  align-items: center;
  gap: 10px;
}

.player-action-btn {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 14px;
  border-radius: 6px;
  font-size: 12px;
  background: rgba(255, 255, 255, 0.08);
  border: 1px solid rgba(255, 255, 255, 0.12);
  color: #e2e8f0;
  cursor: pointer;
  transition: all 0.15s;
}

.player-action-btn:hover {
  background: rgba(255, 255, 255, 0.15);
}

.player-action-btn.play-direct {
  background: linear-gradient(135deg, #4f46e5, #6366f1);
  border: none;
  color: #fff;
}

/* 数据源管理弹窗 */
.source-modal-overlay {
  position: fixed;
  inset: 0;
  z-index: 10000;
  background: rgba(0, 0, 0, 0.6);
  backdrop-filter: blur(4px);
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 20px;
}

.source-modal-card {
  width: 100%;
  max-width: 680px;
  background: #181b2a;
  border: 1px solid rgba(255, 255, 255, 0.12);
  border-radius: 12px;
  overflow: hidden;
  box-shadow: 0 15px 40px rgba(0, 0, 0, 0.5);
  display: flex;
  flex-direction: column;
}

.source-modal-header {
  padding: 16px 20px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.source-modal-header .title {
  font-size: 15px;
  font-weight: 600;
  color: #fff;
}

.source-modal-header .close-icon {
  background: transparent;
  border: none;
  color: #94a3b8;
  cursor: pointer;
}

.source-modal-body {
  padding: 20px;
  max-height: 520px;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.section-title {
  font-size: 13px;
  font-weight: 600;
  color: #cbd5e1;
  margin-bottom: 10px;
}

.source-items {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.source-item {
  padding: 10px 14px;
  background: rgba(255, 255, 255, 0.04);
  border: 1px solid rgba(255, 255, 255, 0.06);
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.name-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.s-name {
  font-size: 13px;
  font-weight: 600;
  color: #fff;
}

.s-type {
  font-size: 10px;
  padding: 1px 6px;
  border-radius: 3px;
  background: rgba(255, 255, 255, 0.08);
  color: #94a3b8;
}

.s-type.official {
  background: rgba(99, 102, 241, 0.15);
  color: #818cf8;
}

.s-type.custom {
  background: rgba(16, 185, 129, 0.15);
  color: #34d399;
}

.s-url {
  font-size: 11px;
  color: #64748b;
  margin-top: 2px;
}

.s-desc {
  font-size: 11px;
  color: #94a3b8;
  margin-top: 2px;
}

.delete-btn {
  background: transparent;
  border: none;
  color: #ef4444;
  cursor: pointer;
  padding: 6px;
}

.add-source-section {
  padding: 14px;
  background: rgba(0, 0, 0, 0.2);
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 8px;
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.form-row {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.form-row label {
  font-size: 11px;
  color: #94a3b8;
}

.form-row input {
  background: rgba(255, 255, 255, 0.05);
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 6px;
  padding: 6px 10px;
  font-size: 12px;
  color: #fff;
  outline: none;
}

.form-error {
  font-size: 11px;
  color: #f87171;
}

.form-submit {
  display: flex;
  justify-content: flex-end;
}

.add-btn {
  display: flex;
  align-items: center;
  gap: 6px;
  background: #6366f1;
  color: #fff;
  border: none;
  border-radius: 6px;
  padding: 7px 16px;
  font-size: 12px;
  cursor: pointer;
  transition: all 0.2s;
}

.add-btn:hover {
  background: #4f46e5;
}

.spin {
  animation: spin 1s linear infinite;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}
</style>
