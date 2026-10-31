<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount, watch } from 'vue';
import { listen } from '@tauri-apps/api/event';
import {
  Search,
  Cloud,
  X,
  Play,
  Pause,
  Download,
  RotateCcw,
  ExternalLink,
  Volume2,
  Check,
  ShieldCheck,
  Loader2,
  Music,
  Mic,
  Sparkles,
  Gamepad2,
  Piano,
  Disc,
  Film,
  BookOpen,
  Globe,
  Radio,
  Library,
  HelpCircle,
  AlertCircle,
  Info,
  ChevronDown,
  User,
  LogIn,
} from 'lucide-vue-next';
import {
  VrpianoApi,
  type MidiCloudTrack,
  type MidiCloudCategory,
  type MidiCloudSource,
  type MidiCloudSourceStatus,
  type VrpianoSong,
} from '../api';
import { katakanaToHiragana, toRomaji } from '../utils/transliteration';

const props = defineProps<{
  show: boolean;
  localSongs?: VrpianoSong[];
  /** 父页面内置播放器当前挂载的云端曲目 id；空串/未定义表示播放器里不是云端曲目。 */
  previewTrackId?: string | null;
  /** 父页面内置播放器是否正在出声，暂停/继续按钮的状态以此为准。 */
  previewPlaying?: boolean;
  playerPositionMs?: number;
  playerDurationMs?: number;
}>();

const emit = defineEmits<{
  (e: 'close'): void;
  (e: 'play-song', song: VrpianoSong): void;
  (e: 'song-downloaded', song: VrpianoSong): void;
  (e: 'preview-midi', data: { id: string; name: string; data: string }): void;
  (e: 'preview-toggle'): void;
  (e: 'preview-seek', positionMs: number): void;
  (e: 'open-midishow-login'): void;
  (e: 'midishow-session', state: 'signed_in' | 'expired'): void;
}>();

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
// 逐源检索结果：不显示这个的话，「8 个数据源」里其实只有一个在干活，用户无从察觉。
const sourceStatuses = ref<MidiCloudSourceStatus[]>([]);

// 原生 select 的弹层由系统绘制（白底 + 灰高亮），和主题完全脱节，改成自绘菜单。
const sourceMenuOpen = ref(false);
const sortMenuOpen = ref(false);
const sourceMenuRef = ref<HTMLElement | null>(null);
const sortMenuRef = ref<HTMLElement | null>(null);
const sortOptions: Array<{ id: 'hot' | 'latest' | 'title' | 'notes'; label: string }> = [
  { id: 'hot', label: '按热度排序' },
  { id: 'latest', label: '按最新收录' },
  { id: 'title', label: '按歌名拼音' },
  { id: 'notes', label: '按音符数量' },
];

// 试听播放器状态
const currentPlayingTrack = ref<MidiCloudTrack | null>(null);
// 真正发声的是父页面的 WebAudio 播放器：这里只镜像它的状态。
// 之前 isPlaying 是弹窗自己的 ref，暂停时只改了按钮图标，音乐照旧在放。
const isPlaying = computed(() => Boolean(
  props.previewPlaying
  && currentPlayingTrack.value
  && props.previewTrackId === currentPlayingTrack.value.id,
));
const previewLoadingId = ref<string | null>(null);
const downloadLoadingId = ref<string | null>(null);
const downloadedIds = ref<Set<string>>(new Set());

// Midishow 账号状态管理
const midishowAccount = ref<string | null>(null);
const midishowLoginModalOpen = ref(false);
const midishowCookieInput = ref('');
const midishowAccountInput = ref('');
const midishowAuthLoading = ref(false);

const checkMidishowAccount = async () => {
  try {
    const accounts = await VrpianoApi.midishowAccounts();
    const active = accounts.find((a) => Boolean(a.username));
    if (active) {
      midishowAccount.value = active.username;
      emit('midishow-session', 'signed_in');
    } else {
      const status = await VrpianoApi.midishowLoginStatus();
      if (status.state === 'signed_in' && status.username) {
        midishowAccount.value = status.username;
        emit('midishow-session', 'signed_in');
      } else {
        midishowAccount.value = null;
      }
    }
  } catch {
    midishowAccount.value = null;
  }
};

const handleMidishowWebLogin = async () => {
  midishowAuthLoading.value = true;
  try {
    await VrpianoApi.midishowOpenWebLogin();
    showToast('已打开 Midishow 网页登录窗口，请在弹出窗口中完成登录', 'info');
    let attempts = 0;
    const pollTimer = window.setInterval(async () => {
      attempts++;
      const status = await VrpianoApi.midishowLoginStatus();
      if (status.state === 'signed_in' && status.username) {
        window.clearInterval(pollTimer);
        midishowAccount.value = status.username;
        midishowAuthLoading.value = false;
        midishowLoginModalOpen.value = false;
        emit('midishow-session', 'signed_in');
        showToast(`Midishow 账号 ${status.username} 登录成功！`, 'success');
        searchTracks();
      } else if (attempts > 60 || status.state === 'failed') {
        window.clearInterval(pollTimer);
        midishowAuthLoading.value = false;
      }
    }, 1500);
  } catch (e: any) {
    midishowAuthLoading.value = false;
    showToast(`打开登录窗口失败: ${e.message || String(e)}`, 'error');
  }
};

const handleImportMidishowCookie = async () => {
  const cookie = midishowCookieInput.value.trim();
  if (!cookie) {
    showToast('请粘贴 Midishow 网站的 Cookie 或 _identity 值', 'error');
    return;
  }
  midishowAuthLoading.value = true;
  try {
    const acc = await VrpianoApi.midishowImportCookie({
      cookie,
      accountName: midishowAccountInput.value.trim() || undefined,
    });
    midishowAccount.value = acc.username;
    midishowAuthLoading.value = false;
    midishowLoginModalOpen.value = false;
    midishowCookieInput.value = '';
    emit('midishow-session', 'signed_in');
    showToast(`成功导入 Midishow 会话账号：${acc.username}`, 'success');
    searchTracks();
  } catch (e: any) {
    midishowAuthLoading.value = false;
    showToast(`Cookie 导入失败: ${e.message || String(e)}`, 'error');
  }
};

const handleLogoutMidishow = async () => {
  if (midishowAccount.value) {
    try {
      await VrpianoApi.midishowRemoveAccount({ username: midishowAccount.value });
      midishowAccount.value = null;
      emit('midishow-session', 'expired');
      showToast('已退出 Midishow 账号', 'info');
    } catch (e: any) {
      showToast(`退出失败: ${e.message || String(e)}`, 'error');
    }
  }
};

// 全曲试听进度与拖拽跳转
const playbackTotalDurationMs = computed(() => {
  if (props.playerDurationMs && props.playerDurationMs > 0) return props.playerDurationMs;
  if (currentPlayingTrack.value?.duration_secs) return currentPlayingTrack.value.duration_secs * 1000;
  return 180000;
});

const playbackProgressPercent = computed(() => {
  const total = playbackTotalDurationMs.value;
  if (total <= 0) return 0;
  const current = props.playerPositionMs || 0;
  return Math.min(100, Math.max(0, (current / total) * 100));
});

const handleSeekProgress = (event: MouseEvent) => {
  const bar = event.currentTarget as HTMLElement;
  const rect = bar.getBoundingClientRect();
  if (rect.width <= 0) return;
  const ratio = Math.max(0, Math.min(1, (event.clientX - rect.left) / rect.width));
  const targetMs = Math.round(ratio * playbackTotalDurationMs.value);
  emit('preview-seek', targetMs);
};


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

// 热门词都经过 BitMidi / Tuneonmusic / MIDIWorld 实际检索验证，点开必有结果。
const hotKeywords = [
  'Canon',
  'Clair de Lune',
  'Für Elise',
  'Gurenge',
  'Megalovania',
  'Bad Apple',
  'Shape of You',
  'Hanon',
  'Jingle Bells',
  'Toccata',
  'Kiss the Rain',
  'Pachelbel',
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
  if (track.source === 'local') return true;
  if (downloadedIds.value.has(track.id)) return true;
  if (!props.localSongs) return false;
  const titleLower = track.title.toLowerCase();
  return props.localSongs.some((s) => s.name.toLowerCase().includes(titleLower));
};

// 本地曲库条目的 download_url 就是磁盘上的绝对路径，不能当作网络直链。
const localSongPath = (track: MidiCloudTrack) =>
  track.source === 'local' ? track.download_url : (track.page_url || track.download_url);

const findLocalSong = (track: MidiCloudTrack) =>
  props.localSongs?.find((s) => s.path === track.download_url)
  || props.localSongs?.find((s) => s.name.includes(track.title));

// BitMidi 等英文源用罗马音/英文命名，中文与日文关键词必须先给出发音变体，
// 否则「残酷な天使のテーゼ」这类查询在英文源上恒为 0 结果。
const buildAltKeywords = (keyword: string): string[] => {
  const alts: string[] = [];
  const push = (value: string) => {
    const cleaned = value.replace(/\s+/g, ' ').trim();
    if (!cleaned || cleaned.toLowerCase() === keyword.toLowerCase()) return;
    if (!alts.includes(cleaned)) alts.push(cleaned);
  };
  const stripped = keyword.replace(/[（(【[][^)）】\]]*[)）】\]]/g, ' ');
  push(stripped);
  if (/[ぁ-ゟァ-ヿ]/.test(stripped)) {
    push(toRomaji(katakanaToHiragana(stripped)));
  }
  return alts.slice(0, 2);
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
// 各源耗时差异很大（Midishow 隐藏窗口最长 7 秒），慢请求不能覆盖新请求的结果。
let searchSeq = 0;

const searchTracks = async () => {
  // 输入防抖已经排了一次检索，用户随即回车时不能再发第二次：两次并发会让
  // 隐藏 WebView2 的固定 label 冲突（第二个 Midishow 检索直接失败）。
  if (searchDebounceTimer !== null) {
    window.clearTimeout(searchDebounceTimer);
    searchDebounceTimer = null;
  }
  const seq = ++searchSeq;
  loading.value = true;
  errorMsg.value = '';
  try {
    const keyword = searchQuery.value.trim();
    const res = await VrpianoApi.cloudLibrarySearch({
      keyword: keyword || undefined,
      altKeywords: buildAltKeywords(keyword),
      category: activeCategory.value === 'all' ? undefined : activeCategory.value,
      source: activeSource.value === 'all' ? undefined : activeSource.value,
      sort: activeSort.value,
      page: currentPage.value,
      limit: pageSize.value,
    });
    if (seq !== searchSeq) return;
    if (res && res.items) {
      tracks.value = res.items;
      totalTracks.value = res.total;
    } else {
      tracks.value = [];
      totalTracks.value = 0;
    }
    sourceStatuses.value = res?.sources || [];
  } catch (e: any) {
    if (seq !== searchSeq) return;
    errorMsg.value = e.message || String(e);
    tracks.value = [];
  } finally {
    if (seq === searchSeq) loading.value = false;
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

const resolveTrackDownloadUrl = (url: string | undefined): string => {
  if (!url) return '';
  if (url.startsWith('http://') || url.startsWith('https://')) return url;
  if (url.startsWith('/')) {
    if (typeof window !== 'undefined' && window.location && window.location.origin && window.location.origin.startsWith('http') && !window.location.origin.includes('localhost') && !window.location.origin.includes('tauri')) {
      return `${window.location.origin}${url}`;
    }
    return `https://vrcdog.pcb.im${url}`;
  }
  return url;
};

// 试听曲目：所有真实音源都会给出直链 .mid，本地条目直接读磁盘。
const handlePreview = async (track: MidiCloudTrack) => {
  // 同一首已经载入父页面播放器：让父页面真正暂停/继续，而不是重新拉一遍从头播。
  if (props.previewTrackId && props.previewTrackId === track.id) {
    currentPlayingTrack.value = track;
    emit('preview-toggle');
    return;
  }

  previewLoadingId.value = track.id;
  try {
    const fullDlUrl = resolveTrackDownloadUrl(track.download_url);
    const midiData = await VrpianoApi.midishowPreviewData({
      midiId: track.midi_id || 0,
      title: track.title,
      downloadUrl: fullDlUrl,
      pageUrl: localSongPath(track),
      source: track.source,
    });
    currentPlayingTrack.value = track;
    emit('preview-midi', { ...midiData, id: track.id });
    reportMidishowSession(track);
    showToast(`正在试听《${track.title}》`, 'info');
  } catch (e: any) {
    reportMidishowSession(track, e.message || String(e));
    showToast(`试听失败: ${describePlaybackError(e.message || String(e))}`, 'error');
  } finally {
    previewLoadingId.value = null;
  }
};

// 双击整行试听；按钮自带 click.stop，但 dblclick 仍会冒泡到行，
// 连点播放键会触发两次 toggle（播完立刻被暂停），这里直接忽略来自按钮的双击。
const handleRowDblClick = (track: MidiCloudTrack, event: MouseEvent) => {
  if ((event.target as HTMLElement | null)?.closest('button')) return;
  void handlePreview(track);
};

// 关闭弹窗时顺手暂停云端试听：用户关窗后音乐还在响会显得「关不掉」。
const closeModal = () => {
  closeMenus();
  if (isPlaying.value) emit('preview-toggle');
  emit('close');
};

// 一键下载到本地曲库
const handleDownload = async (track: MidiCloudTrack) => {
  if (isDownloaded(track)) {
    showToast(`《${track.title}》已在本地曲库`, 'info');
    return;
  }
  downloadLoadingId.value = track.id;
  try {
    const fullDlUrl = resolveTrackDownloadUrl(track.download_url);
    const downloaded = await VrpianoApi.downloadMidishow({
      midiId: track.midi_id || 0,
      title: track.title,
      downloadUrl: fullDlUrl,
      pageUrl: track.page_url,
      source: track.source,
    });
    if (downloaded) {
      downloadedIds.value.add(track.id);
      emit('song-downloaded', downloaded);
      reportMidishowSession(track);
      showToast(`《${track.title}》已成功收录至本地钢琴曲库`, 'success');
    }
  } catch (e: any) {
    reportMidishowSession(track, e.message || String(e));
    showToast(`下载失败: ${describePlaybackError(e.message || String(e))}`, 'error');
  } finally {
    downloadLoadingId.value = null;
  }
};

// 立即弹奏
const handlePlayInGame = async (track: MidiCloudTrack) => {
  if (track.source === 'local' || isDownloaded(track)) {
    const found = findLocalSong(track);
    if (found) {
      emit('play-song', found);
      emit('close');
    } else {
      showToast('本地曲库中找不到这首曲目，请重新下载', 'error');
    }
    return;
  }

  downloadLoadingId.value = track.id;
  try {
    const fullDlUrl = resolveTrackDownloadUrl(track.download_url);
    const downloaded = await VrpianoApi.downloadMidishow({
      midiId: track.midi_id || 0,
      title: track.title,
      downloadUrl: fullDlUrl,
      pageUrl: track.page_url,
      source: track.source,
    });
    if (downloaded) {
      downloadedIds.value.add(track.id);
      emit('song-downloaded', downloaded);
      reportMidishowSession(track);
      emit('play-song', downloaded);
      emit('close');
    }
  } catch (e: any) {
    reportMidishowSession(track, e.message || String(e));
    showToast(`准备弹奏失败: ${describePlaybackError(e.message || String(e))}`, 'error');
  } finally {
    downloadLoadingId.value = null;
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

const selectedSourceLabel = computed(() => (
  activeSource.value === 'all'
    ? '全部分源'
    : sources.value.find((s) => s.id === activeSource.value)?.name || '全部分源'
));
const activeSortLabel = computed(
  () => sortOptions.find((o) => o.id === activeSort.value)?.label || '按热度排序',
);
const sourceCountOf = (id: string) => {
  const status = sourceStatuses.value.find((s) => s.id === id);
  return status ? status.count : null;
};

const toggleSourceMenu = () => {
  sortMenuOpen.value = false;
  sourceMenuOpen.value = !sourceMenuOpen.value;
};
const toggleSortMenu = () => {
  sourceMenuOpen.value = false;
  sortMenuOpen.value = !sortMenuOpen.value;
};
const pickSource = (id: string) => {
  sourceMenuOpen.value = false;
  if (activeSource.value === id) return;
  activeSource.value = id;
  onSourceChange();
};
const pickSort = (id: 'hot' | 'latest' | 'title' | 'notes') => {
  sortMenuOpen.value = false;
  if (activeSort.value === id) return;
  activeSort.value = id;
  currentPage.value = 1;
  searchTracks();
};
const closeMenus = () => {
  sourceMenuOpen.value = false;
  sortMenuOpen.value = false;
};
const onGlobalPointerDown = (event: PointerEvent) => {
  if (!sourceMenuOpen.value && !sortMenuOpen.value) return;
  const target = event.target as Node;
  if (sourceMenuRef.value?.contains(target) || sortMenuRef.value?.contains(target)) return;
  closeMenus();
};
const onGlobalKeydown = (event: KeyboardEvent) => {
  if (event.key === 'Escape') closeMenus();
};

// Midishow 是中文流行曲唯一真实可用的来源，但会被 Cloudflare 拦在门外。
// 用户主动点「解锁」时弹出验证窗口完成一次人机验证，之后 cf_clearance
// 留在 WebView2 profile 里，后续搜索就能静默拿到结果。
const midishowStatus = computed(() => sourceStatuses.value.find((item) => item.id === 'midishow'));
// 需要人机验证，或后台检索失败（隐藏 WebView2 超时/被拦）时，都要给用户
// 一个主动解锁的入口，不能把错误只藏在 chip 的 title 里。
const midishowNeedsUnlock = computed(() => {
  const state = midishowStatus.value?.state;
  if (state === 'needs-verify' || state === 'failed') return Boolean(searchQuery.value.trim());
  return false;
});
// 中文关键词在英文源上必为 0，这不是故障；空结果页要把每个源的真实状态写清楚。
const cjkQuery = computed(() => /[\u3040-\u30ff\u3400-\u4dbf\u4e00-\u9fff\uac00-\ud7af]/.test(searchQuery.value));
const sourceStateText = (item: MidiCloudSourceStatus) => {
  if (item.count > 0) return `${item.count} 首`;
  switch (item.state) {
    case 'needs-verify':
      return '需要一次人机验证';
    case 'failed':
      return item.message || '检索失败';
    case 'idle':
      return item.message || '当前条件不检索该源';
    case 'empty':
      return item.message || '该源没有收录此关键词';
    default:
      return item.message || '无结果';
  }
};
// Midishow 未登录时下载接口返回 403，原始 HTTP 文案对用户没有意义。
const describePlaybackError = (message: string) => {
  if (/midishow/i.test(message) && /登录已失效|需要重新登录/.test(message)) {
    return 'Midishow 登录已失效：请到 VRPiano 主面板重新登录 Midishow 账号，或先在官网下载再导入';
  }
  if (/midishow/i.test(message) && /\b403\b/.test(message)) {
    return 'Midishow 曲目需要登录后才能试听/下载：请先在 VRPiano 主面板登录 Midishow 账号，或在官网下载后导入';
  }
  if (/midishow/i.test(message) && message.includes('人机验证')) {
    return 'Midishow 需要一次人机验证，请点上方「解锁 Midishow 中文曲库」';
  }
  return message;
};

// 把真实的 Midishow 会话结果回报给主面板：直接探测会被 Cloudflare 拦，
// 只有真实请求的成败才能证明 Cookie 是否还有效。
const reportMidishowSession = (track: MidiCloudTrack, message?: string) => {
  if ((track.source || 'midishow') !== 'midishow') return;
  if (message !== undefined) {
    if (/登录已失效|需要重新登录|登录后才能/.test(message)) emit('midishow-session', 'expired');
    return;
  }
  // 直链下载不经过 Midishow 登录，成功不能证明会话有效。
  if (!track.download_url) emit('midishow-session', 'signed_in');
};
const midishowVerifying = ref(false);

const verifyMidishow = async () => {
  const keyword = searchQuery.value.trim();
  if (!keyword) {
    showToast('请先输入要搜索的曲名（如「逆战」），再解锁 Midishow', 'info');
    return;
  }
  midishowVerifying.value = true;
  try {
    const songs = await VrpianoApi.searchMidishowVerified({ keyword, maxResults: pageSize.value });
    await searchTracks();
    showToast(
      songs.length
        ? `Midishow 已解锁，检索到 ${songs.length} 首曲目`
        : '验证已通过，但 Midishow 没有收录该关键词',
      songs.length ? 'success' : 'info',
    );
  } catch (e: any) {
    showToast(`Midishow 解锁失败: ${e.message || String(e)}`, 'error');
  } finally {
    midishowVerifying.value = false;
  }
};

// 解锁时后端会把验证窗口弹到前台，这里提示一句，用户才知道要去点那一下。
let stopChallengeListener: (() => void) | null = null;
onMounted(async () => {
  document.addEventListener('pointerdown', onGlobalPointerDown, true);
  document.addEventListener('keydown', onGlobalKeydown);
  if (props.show) {
    loadCategoriesAndSources();
    searchTracks();
  }
  try {
    void checkMidishowAccount();
    stopChallengeListener = await listen<string>('vrpiano_midishow_search_challenge', (event) => {
      showToast(
        event.payload || 'Midishow 需要一次人机验证，请在弹出的窗口里点「请验证您是真人」',
        'info',
      );
    });
  } catch {
    // 非 Tauri 环境没有事件通道
  }
});

onBeforeUnmount(() => {
  document.removeEventListener('pointerdown', onGlobalPointerDown, true);
  document.removeEventListener('keydown', onGlobalKeydown);
  stopChallengeListener?.();
  stopChallengeListener = null;
});

watch(
  () => props.show,
  (val) => {
    if (val) {
      loadCategoriesAndSources();
      void checkMidishowAccount();
      searchTracks();
    }
  }
);
</script>

<template>
  <div v-if="show" class="midi-cloud-modal-backdrop" @click.self="closeModal">
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
              <div class="brand-name">VRCDog 云端曲库</div>
              <div class="brand-sub">BitMidi · Tuneonmusic · MIDIWorld · Internet Archive · Midishow · 本地曲库，全部为真实 MIDI 直链</div>
            </div>
          </div>
        </div>

        <div class="header-actions">
          <div ref="sourceMenuRef" class="source-selector" :class="{ open: sourceMenuOpen }">
            <button
              type="button"
              class="source-trigger"
              :aria-expanded="sourceMenuOpen"
              aria-haspopup="listbox"
              @click="toggleSourceMenu"
            >
              <Globe :size="14" class="source-icon" />
              <span class="source-trigger-label">{{ selectedSourceLabel }}</span>
              <ChevronDown :size="12" class="source-caret" :class="{ open: sourceMenuOpen }" />
            </button>
            <transition name="drop-fade">
              <div v-if="sourceMenuOpen" class="source-menu" role="listbox" aria-label="选择数据源">
                <button
                  type="button"
                  role="option"
                  :aria-selected="activeSource === 'all'"
                  class="source-option"
                  :class="{ active: activeSource === 'all' }"
                  @click="pickSource('all')"
                >
                  <span class="source-option-text">
                    <span class="source-option-name">全部分源</span>
                    <span class="source-option-desc">同时检索下面列出的全部真实音源</span>
                  </span>
                  <Check v-if="activeSource === 'all'" :size="13" class="source-option-check" />
                </button>
                <button
                  v-for="s in sources"
                  :key="s.id"
                  type="button"
                  role="option"
                  :aria-selected="activeSource === s.id"
                  class="source-option"
                  :class="{ active: activeSource === s.id }"
                  @click="pickSource(s.id)"
                >
                  <span class="source-option-text">
                    <span class="source-option-name">{{ s.name }}</span>
                    <span class="source-option-desc">
                      {{ s.searchable === false ? '仅官网跳转，不可检索' : s.description }}
                    </span>
                  </span>
                  <span v-if="sourceCountOf(s.id) !== null" class="source-option-count">
                    {{ sourceCountOf(s.id) }}
                  </span>
                  <Check v-if="activeSource === s.id" :size="13" class="source-option-check" />
                </button>
              </div>
            </transition>
          </div>

          <!-- Midishow 账号与登录状态 -->
          <div class="midishow-account-badge">
            <button
              type="button"
              class="account-status-btn"
              :class="{ logged: Boolean(midishowAccount) }"
              :title="midishowAccount ? `当前已登录 Midishow 账号: ${midishowAccount}` : '点击登录 Midishow 或导入 Cookie，解锁海量华语流行 MIDI'"
              @click="midishowLoginModalOpen = true"
            >
              <User :size="13" />
              <span class="account-name-text">{{ midishowAccount ? midishowAccount : 'Midishow: 未登录' }}</span>
              <span class="account-tag">{{ midishowAccount ? '已认证' : '点击登录' }}</span>
            </button>
          </div>

          <button class="close-btn" @click="closeModal" title="关闭">
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
            placeholder="搜索曲名或艺术家，如 Canon、Gurenge、Clair de Lune..."
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

        <!-- Midishow 真实登录提示栏 -->
        <div v-if="!midishowAccount && (activeSource === 'midishow' || cjkQuery)" class="midishow-auth-banner">
          <div class="banner-left">
            <User :size="15" class="banner-icon" />
            <span>{{ activeSource === 'midishow' ? '提示：Midishow 中文曲库需真实登录账号后方可搜索与下载曲目资源。' : '提示：Midishow 拥有海量华语流行与 ACG 曲谱，需真实登录 Midishow 账号后方可同步搜索。' }}</span>
          </div>
          <button class="banner-login-btn" type="button" @click="midishowLoginModalOpen = true">
            <LogIn :size="13" />
            <span>立即登录 Midishow</span>
          </button>
        </div>

        <!-- 逐源检索结果：哪个源出了多少条、哪个源挂了，一目了然 -->
        <div v-if="sourceStatuses.length" class="source-status-row">
          <span
            v-for="status in sourceStatuses"
            :key="status.id"
            class="source-status-chip"
            :class="status.state"
            :title="status.message || ''"
          >
            {{ status.label }} · {{ status.count }}
          </span>
          <button
            v-if="midishowNeedsUnlock"
            class="verify-midishow-btn"
            :disabled="midishowVerifying"
            :title="midishowStatus?.message || '完成一次人机验证后即可检索 Midishow 中文曲库'"
            @click="verifyMidishow"
          >
            <Loader2 v-if="midishowVerifying" :size="11" class="spin" />
            <ShieldCheck v-else :size="11" />
            <span>{{ midishowVerifying ? '等待验证窗口…' : '解锁 Midishow 中文曲库' }}</span>
          </button>
        </div>

        <!-- 排序方式 -->
        <div ref="sortMenuRef" class="sort-selector">
          <button
            type="button"
            class="sort-trigger"
            :class="{ open: sortMenuOpen }"
            :aria-expanded="sortMenuOpen"
            aria-haspopup="listbox"
            @click="toggleSortMenu"
          >
            <span>{{ activeSortLabel }}</span>
            <ChevronDown :size="12" class="source-caret" :class="{ open: sortMenuOpen }" />
          </button>
          <transition name="drop-fade">
            <div v-if="sortMenuOpen" class="source-menu sort-menu" role="listbox" aria-label="选择排序方式">
              <button
                v-for="opt in sortOptions"
                :key="opt.id"
                type="button"
                role="option"
                :aria-selected="activeSort === opt.id"
                class="source-option"
                :class="{ active: activeSort === opt.id }"
                @click="pickSort(opt.id)"
              >
                <span class="source-option-text">
                  <span class="source-option-name">{{ opt.label }}</span>
                </span>
                <Check v-if="activeSort === opt.id" :size="13" class="source-option-check" />
              </button>
            </div>
          </transition>
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
          <div v-if="midishowNeedsUnlock" class="empty-hint empty-verify-hint">
            中文流行曲目（如《逆战》）收录在 Midishow，点击上方「解锁 Midishow 中文曲库」完成一次人机验证即可检索
          </div>
          <div v-else-if="!midishowAccount && (activeSource === 'midishow' || cjkQuery)" class="empty-midishow-card">
            <p>Midishow 需要登录真实账号后方可搜索并试听海量华语流行曲目</p>
            <button class="empty-login-btn" type="button" @click="midishowLoginModalOpen = true">
              <User :size="14" />
              <span>登录 Midishow 真实账号</span>
            </button>
          </div>
          <div v-else-if="cjkQuery" class="empty-hint empty-verify-hint">
            BitMidi / Tuneonmusic / MIDIWORLD / Internet Archive 按英文、日文曲名收录，中文曲名通常只在 Midishow 命中
          </div>
          <div v-if="sourceStatuses.length" class="empty-source-list">
            <div
              v-for="item in sourceStatuses"
              :key="item.id"
              class="empty-source-row"
              :class="item.state"
            >
              <span class="empty-source-name">{{ item.label }}</span>
              <span class="empty-source-state">{{ sourceStateText(item) }}</span>
            </div>
          </div>
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
                @dblclick="handleRowDblClick(track, $event)"
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
                      v-if="track.difficulty"
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

                <!-- 时长 / 音符数：来源没给元数据时显示占位，不能编造 00:00 -->
                <td class="col-duration">
                  <div class="duration-text">{{ track.duration_secs > 0 ? formatDuration(track.duration_secs) : '—' }}</div>
                  <div v-if="track.notes_count > 0" class="notes-sub">{{ track.notes_count }} 音符</div>
                </td>

                <!-- 音源标签 -->
                <td class="col-source">
                  <span class="source-badge" :class="`src-${track.source}`">
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
          <div class="player-center-controls">
            <button
              class="ctrl-btn play-pause-btn"
              :title="isPlaying ? '暂停试听' : '继续试听'"
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
            <span class="playing-state-text">{{ isPlaying ? '试听播放中' : '已暂停' }}</span>
          </div>
          <!-- 试听进度条与全曲时长 -->
          <div class="player-timeline">
            <span class="time-label current">{{ formatDuration(Math.floor((props.playerPositionMs || 0) / 1000)) }}</span>
            <div class="progress-bar-track" @click="handleSeekProgress">
              <div
                class="progress-bar-fill"
                :style="{ width: `${playbackProgressPercent}%` }"
              ></div>
              <div
                class="progress-bar-thumb"
                :style="{ left: `${playbackProgressPercent}%` }"
              ></div>
            </div>
            <span class="time-label total">{{ formatDuration(Math.floor(playbackTotalDurationMs / 1000)) }}</span>
          </div>
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

      <!-- Midishow 登录/认证窗口 -->
      <transition name="fade">
        <div v-if="midishowLoginModalOpen" class="midishow-login-dialog-overlay" @click.self="midishowLoginModalOpen = false">
          <div class="midishow-login-card">
            <div class="login-card-header">
              <div class="login-card-title">
                <User :size="16" />
                <span>Midishow 账号登录与认证</span>
              </div>
              <button class="close-card-btn" @click="midishowLoginModalOpen = false">
                <X :size="14" />
              </button>
            </div>

            <div class="login-card-body">
              <p class="login-tip">
                Midishow 是中文曲库的主要来源。真实登录后可直接检索海量华语流行曲目并试听完整真实钢琴音效。
              </p>

              <div class="login-methods">
                <!-- 方式1: 网页窗口直接登录 -->
                <div class="login-method-box">
                  <div class="method-title">方式一：网页窗口快捷登录 (推荐)</div>
                  <div class="method-desc">在内置安全窗口中完成登录，自动通过 Cloudflare 人机验证并同步会话凭证。</div>
                  <button
                    type="button"
                    class="btn-primary web-login-btn"
                    :disabled="midishowAuthLoading"
                    @click="handleMidishowWebLogin"
                  >
                    <Loader2 v-if="midishowAuthLoading" :size="14" class="spin" />
                    <Globe v-else :size="14" />
                    <span>打开 Midishow 网页快捷登录</span>
                  </button>
                </div>

                <!-- 方式2: 粘贴 Cookie 凭据 -->
                <div class="login-method-box">
                  <div class="method-title">方式二：直接导入 Cookie / 会话凭据</div>
                  <div class="method-desc">在浏览器登录 midishow.com 后，复制 Cookie 中的 _identity 或完整 Cookie 字符串粘贴至下方：</div>
                  <input
                    v-model="midishowCookieInput"
                    type="text"
                    class="login-input"
                    placeholder="例如: _identity=xxxx; 或完整 Cookie"
                  />
                  <div class="cookie-actions">
                    <input
                      v-model="midishowAccountInput"
                      type="text"
                      class="login-input name-input"
                      placeholder="备注账号名 (选填)"
                    />
                    <button
                      type="button"
                      class="btn-secondary import-btn"
                      :disabled="midishowAuthLoading || !midishowCookieInput.trim()"
                      @click="handleImportMidishowCookie"
                    >
                      <Check :size="13" />
                      <span>导入并保存</span>
                    </button>
                  </div>
                </div>
              </div>

              <div v-if="midishowAccount" class="current-account-info">
                <span>当前已登录账号：<strong>{{ midishowAccount }}</strong></span>
                <button class="btn-text danger" @click="handleLogoutMidishow">退出此账号</button>
              </div>
            </div>
          </div>
        </div>
      </transition>
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
  position: relative;
  display: flex;
  align-items: center;
  height: 30px;
  border-radius: 8px;
  border: 1px solid var(--vp-border);
  background: var(--vp-surface);
  color: var(--vp-muted);
  transition: border-color 0.16s ease, background 0.16s ease;
}

.source-selector:hover,
.source-selector.open {
  border-color: var(--vp-border-strong);
  background: var(--vp-panel);
}

.source-trigger {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  height: 100%;
  padding: 0 10px 0 9px;
  border: none;
  border-radius: 8px;
  background: transparent;
  color: var(--vp-text);
  font-family: inherit;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
}

.source-icon {
  color: var(--vp-primary);
  flex: none;
}

.source-trigger-label {
  max-width: 190px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.source-caret {
  color: var(--vp-dim);
  flex: none;
  transition: transform 0.16s ease;
}

.source-caret.open {
  transform: rotate(180deg);
}

/* 自绘下拉菜单：跟随主题变量，原生 select 的系统弹层做不到这一点 */
.source-menu {
  position: absolute;
  top: calc(100% + 6px);
  right: 0;
  z-index: 60;
  width: 300px;
  max-height: 362px;
  overflow-y: auto;
  padding: 5px;
  border-radius: 10px;
  border: 1px solid var(--vp-border-strong);
  background: var(--vp-panel);
  backdrop-filter: blur(18px) saturate(160%);
  -webkit-backdrop-filter: blur(18px) saturate(160%);
  box-shadow: 0 16px 38px rgba(69, 26, 3, 0.18), 0 2px 8px rgba(69, 26, 3, 0.08);
}

.sort-menu {
  width: 176px;
}

.source-option {
  width: 100%;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 7px 9px;
  border: none;
  border-radius: 7px;
  background: transparent;
  color: var(--vp-text);
  font-family: inherit;
  text-align: left;
  cursor: pointer;
  transition: background 0.13s ease;
}

.source-option:hover {
  background: var(--vp-hover);
}

.source-option.active {
  background: color-mix(in srgb, var(--vp-primary) 12%, transparent);
}

.source-option-text {
  display: flex;
  flex-direction: column;
  gap: 2px;
  flex: 1;
  min-width: 0;
}

.source-option-name {
  font-size: 12px;
  font-weight: 600;
  line-height: 1.3;
}

.source-option-desc {
  font-size: 10.5px;
  color: var(--vp-dim);
  line-height: 1.35;
}

.source-option-count {
  flex: none;
  min-width: 22px;
  padding: 1px 6px;
  border-radius: 999px;
  background: color-mix(in srgb, var(--vp-primary) 12%, transparent);
  color: var(--vp-primary);
  font-size: 10px;
  font-weight: 700;
  text-align: center;
}

.source-option-check {
  color: var(--vp-primary);
  flex: none;
}

.drop-fade-enter-active,
.drop-fade-leave-active {
  transition: opacity 0.14s ease, transform 0.14s ease;
}

.drop-fade-enter-from,
.drop-fade-leave-to {
  opacity: 0;
  transform: translateY(-4px);
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

.sort-selector {
  position: relative;
  display: flex;
  align-items: center;
}

.sort-trigger {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  height: 30px;
  padding: 0 10px;
  border-radius: 7px;
  border: 1px solid var(--vp-border);
  background: var(--vp-surface);
  color: var(--vp-text);
  font-family: inherit;
  font-size: 11px;
  font-weight: 600;
  cursor: pointer;
  transition: border-color 0.16s ease, background 0.16s ease;
}

.sort-trigger:hover,
.sort-trigger.open {
  border-color: var(--vp-border-strong);
  background: var(--vp-panel);
}

/* 键盘操作（如 Esc 关闭菜单）后浏览器会画默认黑框，换成主题色细环。 */
.source-trigger:focus-visible,
.sort-trigger:focus-visible {
  outline: 2px solid var(--vp-primary);
  outline-offset: 1px;
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
  background: color-mix(in srgb, var(--vp-text) 10%, transparent);
  color: var(--vp-text);
  border: 1px solid var(--vp-border-strong);
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
  display: inline-block;
  max-width: 130px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 11px;
  padding: 2px 8px;
  border-radius: 4px;
  border: 1px solid var(--vp-border);
  background: var(--vp-panel);
  color: var(--vp-muted);
}

.source-badge.src-bitmidi {
  background: color-mix(in srgb, var(--vp-primary) 10%, var(--vp-panel));
  color: var(--vp-primary);
  border-color: color-mix(in srgb, var(--vp-primary) 25%, transparent);
}

.source-badge.src-tuneon {
  background: color-mix(in srgb, #16a34a 10%, var(--vp-panel));
  color: #15803d;
  border-color: color-mix(in srgb, #16a34a 25%, transparent);
}

.source-badge.src-midiworld {
  background: color-mix(in srgb, #a16207 10%, var(--vp-panel));
  color: #a16207;
  border-color: color-mix(in srgb, #a16207 28%, transparent);
}

.source-badge.src-local {
  background: color-mix(in srgb, #475569 8%, var(--vp-panel));
  color: #475569;
  border-color: color-mix(in srgb, #475569 22%, transparent);
}

.source-badge.src-archive {
  background: color-mix(in srgb, #0e7490 10%, var(--vp-panel));
  color: #0e7490;
  border-color: color-mix(in srgb, #0e7490 25%, transparent);
}

.source-badge.src-midishow {
  background: color-mix(in srgb, #be123c 10%, var(--vp-panel));
  color: #be123c;
  border-color: color-mix(in srgb, #be123c 25%, transparent);
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

.spin {
  animation: spin 1s linear infinite;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

/* 逐源检索状态 */
.source-status-row {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-top: 8px;
}
.source-status-chip {
  display: inline-flex;
  align-items: center;
  height: 20px;
  padding: 0 8px;
  border-radius: 10px;
  border: 1px solid var(--vp-border);
  background: var(--vp-panel);
  color: var(--vp-muted);
  font-size: 10.5px;
  font-weight: 600;
}
.source-status-chip.ok {
  border-color: var(--vp-accent, var(--vp-border));
  color: var(--vp-text);
}
.source-status-chip.failed {
  border-style: dashed;
  color: var(--vp-danger, #b45309);
}
.source-status-chip.empty {
  opacity: 0.65;
}
.source-status-chip.idle {
  opacity: 0.55;
}
.source-status-chip.needs-verify {
  border-style: dashed;
  border-color: color-mix(in srgb, #b45309 45%, transparent);
  color: #b45309;
}

.verify-midishow-btn {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  height: 20px;
  padding: 0 9px;
  border-radius: 10px;
  border: 1px solid color-mix(in srgb, var(--vp-primary) 40%, transparent);
  background: color-mix(in srgb, var(--vp-primary) 10%, var(--vp-panel));
  color: var(--vp-primary);
  font-size: 10.5px;
  font-weight: 600;
  cursor: pointer;
  transition: background 0.15s ease, border-color 0.15s ease;
}
.verify-midishow-btn:hover:not(:disabled) {
  background: color-mix(in srgb, var(--vp-primary) 18%, var(--vp-panel));
  border-color: color-mix(in srgb, var(--vp-primary) 55%, transparent);
}
.verify-midishow-btn:disabled {
  opacity: 0.65;
  cursor: default;
}

.empty-verify-hint {
  margin-top: 6px;
  color: var(--vp-primary);
  opacity: 0.85;
}

.empty-source-list {
  margin-top: 14px;
  display: flex;
  flex-direction: column;
  gap: 4px;
  width: min(420px, 100%);
}

.empty-source-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 18px;
  padding: 4px 10px;
  border-radius: 6px;
  background: color-mix(in srgb, var(--vp-primary) 5%, transparent);
  color: var(--vp-muted);
  font-size: 11px;
}

.empty-source-name {
  font-weight: 600;
}

.empty-source-state {
  font-size: 10.5px;
  color: var(--vp-dim);
  text-align: right;
}

.empty-source-row.needs-verify .empty-source-state,
.empty-source-row.failed .empty-source-state {
  color: var(--vp-primary);
}


/* Midishow 账号状态徽章 */
.midishow-account-badge {
  display: flex;
  align-items: center;
}
.account-status-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 5px 10px;
  border-radius: 8px;
  font-size: 12px;
  font-weight: 500;
  background: color-mix(in srgb, var(--vp-primary) 10%, transparent);
  border: 1px solid color-mix(in srgb, var(--vp-primary) 28%, transparent);
  color: var(--vp-primary);
  cursor: pointer;
  transition: all 0.18s ease;
}
.account-status-btn:hover {
  background: color-mix(in srgb, var(--vp-primary) 18%, transparent);
  border-color: var(--vp-primary);
}
.account-status-btn.logged {
  background: color-mix(in srgb, #10b981 12%, transparent);
  border-color: color-mix(in srgb, #10b981 32%, transparent);
  color: #10b981;
}
.account-tag {
  font-size: 10px;
  padding: 1px 5px;
  border-radius: 4px;
  background: color-mix(in srgb, currentColor 15%, transparent);
}

/* 底部播放进度条 */
.player-center {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  min-width: 280px;
  flex: 1;
  max-width: 520px;
}
.player-center-controls {
  display: flex;
  align-items: center;
  gap: 10px;
}
.player-timeline {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
}
.time-label {
  font-size: 11px;
  font-variant-numeric: tabular-nums;
  opacity: 0.65;
  min-width: 34px;
}
.time-label.current {
  text-align: right;
}
.time-label.total {
  text-align: left;
}
.progress-bar-track {
  position: relative;
  flex: 1;
  height: 5px;
  border-radius: 3px;
  background: color-mix(in srgb, currentColor 15%, transparent);
  cursor: pointer;
  display: flex;
  align-items: center;
}
.progress-bar-fill {
  height: 100%;
  border-radius: 3px;
  background: var(--vp-primary);
  pointer-events: none;
}
.progress-bar-thumb {
  position: absolute;
  top: 50%;
  transform: translate(-50%, -50%);
  width: 11px;
  height: 11px;
  border-radius: 50%;
  background: #ffffff;
  box-shadow: 0 1px 4px rgba(0, 0, 0, 0.4);
  pointer-events: none;
  opacity: 0;
  transition: opacity 0.15s ease;
}
.progress-bar-track:hover .progress-bar-thumb {
  opacity: 1;
}

/* Midishow 登录弹窗 */
.midishow-login-dialog-overlay {
  position: absolute;
  inset: 0;
  background: rgba(0, 0, 0, 0.65);
  backdrop-filter: blur(8px);
  z-index: 1000;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 20px;
}
.midishow-login-card {
  width: 100%;
  max-width: 480px;
  background: color-mix(in srgb, var(--vp-modal-bg) 95%, #000);
  border: 1px solid color-mix(in srgb, var(--vp-primary) 35%, transparent);
  border-radius: 14px;
  box-shadow: 0 12px 36px rgba(0, 0, 0, 0.45);
  overflow: hidden;
}
.login-card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 14px 18px;
  border-bottom: 1px solid color-mix(in srgb, currentColor 10%, transparent);
}
.login-card-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 14px;
  font-weight: 600;
  color: var(--vp-primary);
}
.close-card-btn {
  background: transparent;
  border: none;
  color: inherit;
  opacity: 0.6;
  cursor: pointer;
  padding: 4px;
  border-radius: 6px;
}
.close-card-btn:hover {
  opacity: 1;
  background: color-mix(in srgb, currentColor 10%, transparent);
}
.login-card-body {
  padding: 18px;
  display: flex;
  flex-direction: column;
  gap: 14px;
}
.login-tip {
  font-size: 12px;
  line-height: 1.5;
  opacity: 0.75;
  margin: 0;
}
.login-methods {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.login-method-box {
  background: color-mix(in srgb, currentColor 4%, transparent);
  border: 1px solid color-mix(in srgb, currentColor 10%, transparent);
  border-radius: 10px;
  padding: 12px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.method-title {
  font-size: 12px;
  font-weight: 600;
  color: var(--vp-primary);
}
.method-desc {
  font-size: 11px;
  opacity: 0.65;
  line-height: 1.4;
}
.web-login-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  padding: 8px 14px;
  border-radius: 8px;
  font-size: 12px;
  font-weight: 600;
  background: var(--vp-primary);
  color: #ffffff;
  border: none;
  cursor: pointer;
  transition: opacity 0.15s ease;
}
.web-login-btn:hover {
  opacity: 0.9;
}
.login-input {
  width: 100%;
  padding: 7px 10px;
  border-radius: 7px;
  border: 1px solid color-mix(in srgb, currentColor 15%, transparent);
  background: color-mix(in srgb, currentColor 4%, transparent);
  color: inherit;
  font-size: 12px;
  outline: none;
}
.login-input:focus {
  border-color: var(--vp-primary);
}
.cookie-actions {
  display: flex;
  gap: 8px;
}
.name-input {
  flex: 1;
}
.import-btn {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 7px 12px;
  border-radius: 7px;
  font-size: 12px;
  font-weight: 600;
  background: color-mix(in srgb, var(--vp-primary) 15%, transparent);
  border: 1px solid color-mix(in srgb, var(--vp-primary) 35%, transparent);
  color: var(--vp-primary);
  cursor: pointer;
  white-space: nowrap;
}
.current-account-info {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding-top: 8px;
  border-top: 1px solid color-mix(in srgb, currentColor 10%, transparent);
  font-size: 12px;
}
.btn-text.danger {
  background: transparent;
  border: none;
  color: #ef4444;
  cursor: pointer;
  font-size: 12px;
}

/* Midishow 认证提示栏与空态引导卡片 */
.midishow-auth-banner {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 8px 14px;
  border-radius: 10px;
  background: color-mix(in srgb, #f59e0b 12%, transparent);
  border: 1px solid color-mix(in srgb, #f59e0b 35%, transparent);
  color: #fbbf24;
  font-size: 12px;
  margin-bottom: 10px;
  animation: fadeIn 0.2s ease;
}
.banner-left {
  display: flex;
  align-items: center;
  gap: 8px;
  line-height: 1.4;
}
.banner-icon {
  flex-shrink: 0;
  color: #f59e0b;
}
.banner-login-btn {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 4px 10px;
  border-radius: 6px;
  font-size: 12px;
  font-weight: 500;
  background: #f59e0b;
  color: #0f172a;
  border: none;
  cursor: pointer;
  white-space: nowrap;
  transition: all 0.18s ease;
}
.banner-login-btn:hover {
  background: #fbbf24;
  transform: translateY(-1px);
}
.empty-midishow-card {
  margin-top: 14px;
  padding: 12px 18px;
  border-radius: 10px;
  background: color-mix(in srgb, #f59e0b 10%, transparent);
  border: 1px dashed color-mix(in srgb, #f59e0b 40%, transparent);
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
}
.empty-midishow-card p {
  margin: 0;
  font-size: 13px;
  color: #fbbf24;
}
.empty-login-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 6px 14px;
  border-radius: 8px;
  background: #f59e0b;
  color: #0f172a;
  font-size: 12px;
  font-weight: 600;
  border: none;
  cursor: pointer;
  transition: all 0.18s ease;
}
.empty-login-btn:hover {
  background: #fbbf24;
  transform: translateY(-1px);
}
</style>
