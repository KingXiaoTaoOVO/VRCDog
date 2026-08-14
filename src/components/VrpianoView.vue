<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { emit, listen } from '@tauri-apps/api/event';
import { convertFileSrc, isTauri } from '@tauri-apps/api/core';
import { WebviewWindow } from '@tauri-apps/api/webviewWindow';
import { Effect } from '@tauri-apps/api/window';
import { open } from '@tauri-apps/plugin-dialog';
import {
  AlertTriangle,
  Clock3,
  Download,
  Edit3,
  ExternalLink,
  FolderOpen,
  Gauge,
  Headphones,
  ImagePlus,
  LogIn,
  LogOut,
  Link2,
  Loader2,
  Music,
  Pause,
  PictureInPicture2,
  Play,
  RefreshCcw,
  Search,
  ShieldCheck,
  Square,
  Trash2,
  Upload,
  Volume2,
  X,
} from 'lucide-vue-next';
import { VrpianoApi, type VrpianoMidiData, type VrpianoMidishowAccount, type VrpianoMidishowLoginStatus, type VrpianoOnlineSong, type VrpianoSong, type VrpianoStatus } from '../api';
import { SysApi } from '../api';
import {
  GENERAL_MIDI_GROUPS,
  GeneralMidiSynth,
  getGeneralMidiInstrumentName,
  parseGeneralMidi,
  type MidiNote,
} from '../audio/generalMidi';
import { isVrpianoOverlayBlurEnabled, VRPIANO_OVERLAY_BLUR_KEY } from './vrpianoOverlayAppearance';

const { locale } = useI18n();
const l = (zh: string, en: string) => locale.value.startsWith('zh') ? zh : en;
const instrumentName = (program: number) => locale.value.startsWith('zh')
  ? getGeneralMidiInstrumentName(program)
  : `General MIDI Program ${program + 1}`;
const instrumentGroupName = (groupIndex: number, zhName: string) => locale.value.startsWith('zh')
  ? zhName
  : `Programs ${groupIndex * 8 + 1}-${groupIndex * 8 + 8}`;

const emptyStatus = (): VrpianoStatus => ({
  running: false,
  paused: false,
  song_name: '',
  song_path: '',
  progress: 0,
  played_notes: 0,
  total_notes: 0,
  duration_ms: 0,
  elapsed_ms: 0,
  last_event: l('正在初始化 VRPiano', 'Initializing VRPiano'),
  last_error: '',
  songs_dir: '',
  speed: 1,
  hotkeys_enabled: false,
  hotkeys_available: true,
  last_hotkey: '',
  last_hotkey_at_ms: 0,
});

const songs = ref<VrpianoSong[]>([]);
const onlineResults = ref<VrpianoOnlineSong[]>([]);
const selectedPath = ref('');
const localSongQuery = ref('');
const status = ref<VrpianoStatus>(emptyStatus());
const loading = ref(false);
const onlineLoading = ref(false);
const ONLINE_SEARCH_TIMEOUT_MS = 12_000;
let onlineSearchRequestId = 0;
let onlineSearchTimeout: number | null = null;
const hasSearchedOnline = ref(false);
const lastOnlineKeyword = ref('');
const onlineBusyId = ref<number | null>(null);
const error = ref('');
const delaySecs = ref(5);
const speed = ref(1);
const hotkeysEnabled = ref(false);
const onlineKeyword = ref('');
const urlInput = ref('');
const urlFilename = ref('');
const logs = ref<string[]>([]);
const midishowAccounts = ref<VrpianoMidishowAccount[]>([]);
const midishowAccount = ref('');
const midishowPassword = ref('');
const midishowLoginOpen = ref(false);
const midishowLoginStatus = ref<VrpianoMidishowLoginStatus>({
  state: 'idle',
  message: l('等待登录', 'Waiting to sign in'),
  username: null,
});
const accountLoading = ref(false);
const externalLinkLoading = ref(false);
const loginError = ref('');
// 用户重新输入账号或密码时，清除登录错误提示
watch([midishowAccount, midishowPassword], () => {
  if (loginError.value) loginError.value = '';
});
const accountInputRef = ref<HTMLInputElement | null>(null);
const passwordInputRef = ref<HTMLInputElement | null>(null);
// 浏览器自动填充可能在 Vue 渲染之后才发生，强制清空 DOM 的 value 兜底
const forceClearLoginFields = () => {
  setTimeout(() => {
    if (accountInputRef.value) accountInputRef.value.value = '';
    if (passwordInputRef.value) passwordInputRef.value.value = '';
  }, 250);
};
watch(midishowLoginOpen, (open) => {
  if (open) forceClearLoginFields();
});
onMounted(() => {
  if (midishowLoginOpen.value) forceClearLoginFields();
});
const signupUrl = 'https://www.midishow.com/user/account/signup';
const songIcons = ref<Record<string, string>>({});
const iconFileInput = ref<HTMLInputElement | null>(null);
const iconTargetPath = ref('');
const editDialogMode = ref<'icon' | 'rename' | null>(null);
const editIconText = ref('');
const editIconUrl = ref('');
const editSongName = ref('');
const playerTitle = ref(l('未加载曲目', 'No song loaded'));
const playerPositionMs = ref(0);
const playerDurationMs = ref(0);
const playerVolume = ref(0.9);
const playerPlaying = ref(false);
const playerLoading = ref(false);
const parsedPlayerNotes = ref<MidiNote[]>([]);
const playerInstrument = ref('source');
const sourcePrograms = ref<number[]>([]);
const sourceHasPercussion = ref(false);
const overlayOpen = ref(false);

const formatVrpianoError = (e: unknown) => {
  const message = e instanceof Error ? e.message : String(e);
  if (/403|cloudflare|challenge|cookie|cf_chl|javascript/i.test(message)) {
    return l('当前操作未完成，请重新登录后再试。', 'This action could not be completed. Sign in again and retry.');
  }
  if (/invalid|expired|会话已失效/i.test(message)) {
    return l('登录状态已失效，请重新登录。', 'Your session expired. Sign in again.');
  }
  return message || l('操作未完成，请稍后重试。', 'The action could not be completed. Try again later.');
};

let unlistenStatus: (() => void) | null = null;
let unlistenOverlayClosed: (() => void) | null = null;
let unlistenMidishowLogin: (() => void) | null = null;
let pollTimer: number | null = null;
let midishowLoginPollTimer: number | null = null;
let speedApplyTimer: number | null = null;
let hotkeyApplyTimer: number | null = null;
let audioContext: AudioContext | null = null;
let playerMasterGain: GainNode | null = null;
let playerCompressor: DynamicsCompressorNode | null = null;
let playerSynth: GeneralMidiSynth | null = null;
let playerTimer: number | null = null;
let playerStartedAt = 0;
let playerAnchorContextTime = 0;
let playerAnchorPositionMs = 0;
let nextNoteIndex = 0;
const PLAYER_LOOKAHEAD_MS = 1500;
const PLAYER_TICK_MS = 80;
const playerInstrumentStorageKey = 'vrcdog.vrpiano.playerInstrument.v1';

const selectedSong = computed(() => songs.value.find((song) => song.path === selectedPath.value) || null);
const filteredSongs = computed(() => {
  const query = localSongQuery.value.trim().toLowerCase();
  if (!query) return songs.value;
  const terms = query.split(/\s+/).filter(Boolean);
  return songs.value.filter((song) => {
    const haystack = `${song.name} ${song.path}`.toLowerCase();
    return terms.every((term) => haystack.includes(term));
  });
});
const progressPercent = computed(() => Math.round(Math.min(1, Math.max(0, status.value.progress || 0)) * 100));
const canStart = computed(() => Boolean(selectedSong.value) && !status.value.running && !loading.value);
const speedText = computed(() => `${clampSpeed(speed.value).toFixed(2)}x`);
const defaultMidishowAccount = computed(() => midishowAccounts.value[0] || null);
const defaultMidishowLoginTypeText = computed(() => (
  defaultMidishowAccount.value?.login_type ? l('登录会话', 'signed-in session') : ''
));
const canTogglePlayer = computed(() => Boolean(parsedPlayerNotes.value.length || selectedSong.value) && !playerLoading.value);
const onlineEmptyText = computed(() => {
  // 搜索进行中：显示"正在搜索"，而不是"未找到"——避免请求未完成时就
  // 误把"未找到 XX 相关结果"展示给用户，让用户以为真的搜不到。
  if (onlineLoading.value) return l('正在搜索…', 'Searching…');
  if (!hasSearchedOnline.value) return l('输入关键词搜索，或直接粘贴 URL / ID 下载。', 'Search by keyword, or paste a URL or ID to download.');
  return l(`未找到“${lastOnlineKeyword.value}”相关结果，换个关键词或粘贴 ID/URL 试试。`, `No results for "${lastOnlineKeyword.value}". Try another keyword, ID, or URL.`);
});
const playerProgressPercent = computed(() => {
  if (!playerDurationMs.value) return 0;
  return Math.round(Math.min(1, playerPositionMs.value / playerDurationMs.value) * 100);
});
const sourceInstrumentText = computed(() => {
  const names: string[] = sourcePrograms.value.map(instrumentName);
  if (sourceHasPercussion.value) names.push(l('标准鼓组', 'Standard drum kit'));
  return names.length ? names.join(locale.value.startsWith('zh') ? '、' : ', ') : instrumentName(0);
});
const activeInstrumentText = computed(() => (
  playerInstrument.value === 'source'
    ? l(`跟随 MIDI：${sourceInstrumentText.value}`, `Follow MIDI: ${sourceInstrumentText.value}`)
    : l(`手动音色：${getGeneralMidiInstrumentName(Number(playerInstrument.value))}`, `Manual instrument: ${instrumentName(Number(playerInstrument.value))}`)
));
const hotkeyStatusText = computed(() => {
  if (!status.value.hotkeys_available) return l('当前系统不支持', 'Not supported on this system');
  return hotkeysEnabled.value ? l('全局快捷键已开启', 'Global shortcuts enabled') : l('全局快捷键已关闭', 'Global shortcuts disabled');
});

const addLog = (message: string) => {
  const time = new Date().toLocaleTimeString(locale.value.startsWith('zh') ? 'zh-CN' : 'en-US', { hour12: false });
  logs.value = [`${time} ${message}`, ...logs.value].slice(0, 100);
};

const selectFirstFilteredSong = () => {
  if (filteredSongs.value[0]) selectedPath.value = filteredSongs.value[0].path;
};

const clearLocalSongQuery = () => {
  localSongQuery.value = '';
};

const formatBytes = (bytes: number) => {
  if (!Number.isFinite(bytes)) return '-';
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
};

const formatTime = (ms: number) => {
  const total = Math.max(0, Math.round((ms || 0) / 1000));
  const minutes = Math.floor(total / 60);
  const seconds = total % 60;
  return `${minutes}:${String(seconds).padStart(2, '0')}`;
};

const clampSpeed = (value: unknown) => {
  const parsed = Number(value);
  if (!Number.isFinite(parsed)) return 1;
  return Math.round(Math.min(3, Math.max(0.25, parsed)) * 100) / 100;
};

const clampPlayerVolume = (value: unknown) => {
  const parsed = Number(value);
  if (!Number.isFinite(parsed)) return 0.9;
  return Math.min(1, Math.max(0, parsed));
};

const base64ToBytes = (data: string) => {
  const binary = atob(data);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i += 1) bytes[i] = binary.charCodeAt(i);
  return bytes;
};

const stopScheduledAudio = () => {
  playerSynth?.stopAll();
};

const stopPlayerTimer = () => {
  if (playerTimer !== null) {
    window.clearInterval(playerTimer);
    playerTimer = null;
  }
};

const updatePlayerClock = () => {
  if (!playerPlaying.value) return;
  playerPositionMs.value = Math.min(playerDurationMs.value, Date.now() - playerStartedAt);
  if (playerPositionMs.value >= playerDurationMs.value) {
    playerPlaying.value = false;
    stopPlayerTimer();
    stopScheduledAudio();
  }
};

const ensureAudioContext = () => {
  const AudioCtor = window.AudioContext || (window as any).webkitAudioContext;
  if (!audioContext) audioContext = new AudioCtor();
  if (!playerMasterGain) {
    playerMasterGain = audioContext.createGain();
    playerCompressor = audioContext.createDynamicsCompressor();
    playerCompressor.threshold.value = -18;
    playerCompressor.knee.value = 18;
    playerCompressor.ratio.value = 4;
    playerCompressor.attack.value = 0.005;
    playerCompressor.release.value = 0.18;
    playerMasterGain.gain.value = clampPlayerVolume(playerVolume.value);
    playerMasterGain.connect(playerCompressor).connect(audioContext.destination);
    playerSynth = new GeneralMidiSynth(audioContext, playerMasterGain);
  }
  return audioContext;
};

const scheduleNote = (context: AudioContext, note: MidiNote, currentPositionMs: number) => {
  if (!playerSynth) return;
  const noteEndMs = note.timeMs + note.durationMs;
  const startAt = Math.max(
    context.currentTime + 0.005,
    playerAnchorContextTime + (note.timeMs - playerAnchorPositionMs) / 1000,
  );
  const remainingMs = noteEndMs - Math.max(currentPositionMs, note.timeMs);
  const duration = Math.max(0.05, remainingMs / 1000);
  const overrideProgram = playerInstrument.value === 'source' ? null : Number(playerInstrument.value);
  playerSynth.schedule(note, startAt, duration, overrideProgram);
};

const schedulePlayerWindow = () => {
  if (!playerPlaying.value || !audioContext) return;
  const currentPositionMs = Math.min(
    playerDurationMs.value,
    playerAnchorPositionMs + (audioContext.currentTime - playerAnchorContextTime) * 1000,
  );
  const horizonMs = currentPositionMs + PLAYER_LOOKAHEAD_MS;
  const notes = parsedPlayerNotes.value;

  while (nextNoteIndex < notes.length && notes[nextNoteIndex].timeMs <= horizonMs) {
    const note = notes[nextNoteIndex];
    if (note.timeMs + note.durationMs >= currentPositionMs) {
      scheduleNote(audioContext, note, currentPositionMs);
    }
    nextNoteIndex += 1;
  }
};

const schedulePlayer = async (startMs = playerPositionMs.value) => {
  const context = ensureAudioContext();
  if (context.state === 'suspended') await context.resume();
  stopScheduledAudio();
  stopPlayerTimer();

  const safeStartMs = Math.min(playerDurationMs.value, Math.max(0, startMs));
  playerPositionMs.value = safeStartMs;
  playerStartedAt = Date.now() - safeStartMs;
  playerAnchorContextTime = context.currentTime + 0.025;
  playerAnchorPositionMs = safeStartMs;
  nextNoteIndex = parsedPlayerNotes.value.findIndex(
    (note) => note.timeMs + note.durationMs >= safeStartMs,
  );
  if (nextNoteIndex < 0) nextNoteIndex = parsedPlayerNotes.value.length;
  playerPlaying.value = true;
  schedulePlayerWindow();
  playerTimer = window.setInterval(() => {
    updatePlayerClock();
    schedulePlayerWindow();
  }, PLAYER_TICK_MS);
};

const pausePlayer = () => {
  updatePlayerClock();
  playerPlaying.value = false;
  stopPlayerTimer();
  stopScheduledAudio();
};

const togglePlayer = async () => {
  if (!parsedPlayerNotes.value.length) {
    if (selectedSong.value) await previewLocalSong();
    return;
  }
  if (playerPlaying.value) {
    pausePlayer();
  } else {
    await schedulePlayer(playerPositionMs.value);
  }
};

const seekPlayer = async () => {
  playerPositionMs.value = Math.min(playerDurationMs.value, Math.max(0, Number(playerPositionMs.value) || 0));
  if (playerPlaying.value) await schedulePlayer(playerPositionMs.value);
};

const applyPlayerVolume = () => {
  playerVolume.value = clampPlayerVolume(playerVolume.value);
  if (audioContext && playerMasterGain) {
    playerMasterGain.gain.setTargetAtTime(playerVolume.value, audioContext.currentTime, 0.02);
  }
};

const applyPlayerInstrument = async () => {
  if (playerInstrument.value !== 'source') {
    const program = Number(playerInstrument.value);
    playerInstrument.value = Number.isInteger(program) && program >= 0 && program <= 127 ? String(program) : 'source';
  }
  localStorage.setItem(playerInstrumentStorageKey, playerInstrument.value);
  addLog(activeInstrumentText.value);
  if (playerPlaying.value) await schedulePlayer(playerPositionMs.value);
};

const toggleVrpianoOverlay = async () => {
  if (!isTauri()) {
    addLog(l('浏览器预览中无法创建桌面悬浮窗', 'Desktop overlay is unavailable in browser preview'));
    return;
  }

  const existing = await WebviewWindow.getByLabel('vrpiano-overlay');
  if (existing) {
    await emit('cmd-close-vrpiano-overlay');
    overlayOpen.value = false;
    return;
  }

  let savedPosition: { x?: number; y?: number } = {};
  try {
    savedPosition = JSON.parse(localStorage.getItem('vrcdog.vrpiano.overlay.position') || '{}');
  } catch {
    savedPosition = {};
  }

  const backdropEnabled = isVrpianoOverlayBlurEnabled(localStorage.getItem(VRPIANO_OVERLAY_BLUR_KEY));

  const overlay = new WebviewWindow('vrpiano-overlay', {
    url: '/?mode=vrpiano-overlay',
    title: l('VRPiano 悬浮控制器', 'VRPiano Overlay Controller'),
    transparent: true,
    decorations: false,
    shadow: false,
    alwaysOnTop: true,
    skipTaskbar: true,
    resizable: true,
    width: 500,
    height: 620,
    minWidth: 320,
    minHeight: 420,
    ...(backdropEnabled ? { windowEffects: { effects: [Effect.Acrylic] } } : {}),
    ...(Number.isFinite(savedPosition.x) ? { x: savedPosition.x } : {}),
    ...(Number.isFinite(savedPosition.y) ? { y: savedPosition.y } : {}),
  });

  overlay.once('tauri://created', () => {
    overlayOpen.value = true;
    addLog(l('VRPiano 悬浮窗已开启', 'VRPiano overlay opened'));
  });
  overlay.once('tauri://error', (event) => {
    overlayOpen.value = false;
    error.value = l(`悬浮窗创建失败：${JSON.stringify(event)}`, `Could not create overlay: ${JSON.stringify(event)}`);
  });
  overlay.onCloseRequested(() => {
    overlayOpen.value = false;
  });
};

const loadMidiIntoPlayer = async (midi: VrpianoMidiData) => {
  playerLoading.value = true;
  error.value = '';
  try {
    const parsed = parseGeneralMidi(base64ToBytes(midi.data));
    pausePlayer();
    parsedPlayerNotes.value = parsed.notes;
    sourcePrograms.value = parsed.programs;
    sourceHasPercussion.value = parsed.hasPercussion;
    playerTitle.value = midi.name;
    playerPositionMs.value = 0;
    playerDurationMs.value = Math.ceil(Math.max(...parsed.notes.map((note) => note.timeMs + note.durationMs)));
    await schedulePlayer(0);
    addLog(l(`内置播放器开始试听：${midi.name}（${activeInstrumentText.value}）`, `Built-in player preview started: ${midi.name} (${activeInstrumentText.value})`));
  } catch (e: any) {
    error.value = e.message || String(e);
    addLog(l(`播放器加载失败：${error.value}`, `Player load failed: ${error.value}`));
  } finally {
    playerLoading.value = false;
  }
};

const iconStorageKey = 'vrcdog.vrpiano.songIcons.v1';

const loadSongIcons = () => {
  try {
    songIcons.value = JSON.parse(localStorage.getItem(iconStorageKey) || '{}');
  } catch {
    songIcons.value = {};
  }
};

const saveSongIcons = () => {
  localStorage.setItem(iconStorageKey, JSON.stringify(songIcons.value));
};

const songIcon = (song: VrpianoSong) => songIcons.value[song.path] || '';
const isImageIcon = (value: string) => /^(data:image\/|https?:\/\/|blob:)/i.test(value);

/**
 * 把本地封面图绝对路径转换为可在 WebView 中渲染的 `asset://` URL。
 * 若路径缺失/转换失败则返回空字符串，由模板回落到默认 Music 图标。
 * 这里用 try/catch 包住 convertFileSrc，避免在未配置 assetProtocol 时
 * 让单首曲目错误把整个曲库列表搞挂。
 */
const songCover = (song: VrpianoSong): string => {
  const cover = song.cover_path;
  if (!cover) return '';
  try {
    return convertFileSrc(cover);
  } catch {
    return '';
  }
};

const closeEditDialog = () => {
  if (loading.value) return;
  editDialogMode.value = null;
  editIconText.value = '';
  editIconUrl.value = '';
  editSongName.value = '';
};

const setSongEmojiIcon = () => {
  if (!selectedSong.value) return;
  const current = songIcon(selectedSong.value);
  editIconText.value = current && !isImageIcon(current) ? current : '';
  editIconUrl.value = current && isImageIcon(current) ? current : '';
  editDialogMode.value = 'icon';
};

const saveSongIconEditor = () => {
  if (!selectedSong.value) return;
  const nextUrl = editIconUrl.value.trim();
  const nextText = editIconText.value.trim();
  if (nextUrl) {
    if (!isImageIcon(nextUrl)) {
      error.value = l('图标 URL 需要以 http://、https:// 或 data:image/ 开头', 'Icon URL must start with http://, https://, or data:image/');
      return;
    }
    songIcons.value = { ...songIcons.value, [selectedSong.value.path]: nextUrl };
  } else if (nextText) {
    songIcons.value = { ...songIcons.value, [selectedSong.value.path]: nextText.slice(0, 4) };
  } else {
    const nextIcons = { ...songIcons.value };
    delete nextIcons[selectedSong.value.path];
    songIcons.value = nextIcons;
  }
  saveSongIcons();
  closeEditDialog();
};

const chooseSongImageIcon = () => {
  if (!selectedSong.value || !iconFileInput.value) return;
  iconTargetPath.value = selectedSong.value.path;
  iconFileInput.value.value = '';
  iconFileInput.value.click();
};

const handleSongIconFile = (event: Event) => {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0];
  if (!file || !iconTargetPath.value) return;
  if (!file.type.startsWith('image/')) {
    error.value = l('请选择图片文件作为曲目图标', 'Choose an image file for the song icon');
    return;
  }
  const reader = new FileReader();
  reader.onload = () => {
    songIcons.value = { ...songIcons.value, [iconTargetPath.value]: String(reader.result || '') };
    saveSongIcons();
  };
  reader.readAsDataURL(file);
};

const loadMidishowAccounts = async () => {
  try {
    midishowAccounts.value = await VrpianoApi.midishowAccounts();
  } catch (e: any) {
    error.value = e.message || String(e);
  }
};

const copySignupUrl = async () => {
  error.value = '';
  try {
    await navigator.clipboard.writeText(signupUrl);
    addLog(l('Midishow 注册链接已复制', 'Midishow registration link copied'));
  } catch (e: any) {
    error.value = l(`复制注册链接失败：${e.message || String(e)}。请手动复制：${signupUrl}`, `Could not copy registration link: ${e.message || String(e)}. Copy it manually: ${signupUrl}`);
  }
};

const openMidishowSignup = async () => {
  if (externalLinkLoading.value) return;
  externalLinkLoading.value = true;
  error.value = '';
  try {
    await SysApi.openUrl({ url: signupUrl });
    addLog(l('已在默认浏览器打开 Midishow 注册页面', 'Opened the Midishow registration page in your default browser'));
  } catch (e: any) {
    error.value = l(`无法打开默认浏览器：${e.message || String(e)}。请复制注册链接后手动打开。`, `Could not open the default browser: ${e.message || String(e)}. Copy and open the registration link manually.`);
  } finally {
    externalLinkLoading.value = false;
  }
};

const openMidishowSearch = async () => {
  try {
    const url = new URL('https://www.midishow.com/search/result');
    const keyword = onlineKeyword.value.trim();
    if (keyword) url.searchParams.set('q', keyword);
    await SysApi.openUrl({ url: url.toString() });
    addLog(l('已在浏览器打开 Midishow 官方搜索', 'Opened official Midishow search in the browser'));
  } catch (e: any) {
    error.value = e.message || String(e);
  }
};

const stopMidishowLoginPolling = () => {
  if (midishowLoginPollTimer !== null) {
    window.clearInterval(midishowLoginPollTimer);
    midishowLoginPollTimer = null;
  }
};

const applyMidishowLoginStatus = async (next: VrpianoMidishowLoginStatus) => {
  const alreadySignedIn = midishowLoginStatus.value.state === 'signed_in'
    && midishowLoginStatus.value.username === next.username;
  // 需要手动在弹出的浏览器窗口中完成验证（如 Cloudflare 人机验证）
  if (next.state === 'needs_confirmation') {
    next = {
      ...next,
      message: l('请在弹出的登录窗口中完成验证（如浏览器人机验证），完成后会自动继续。', 'Complete the verification in the popup login window (e.g. browser human verification); it will continue automatically.'),
    };
  }
  midishowLoginStatus.value = next;
  if (next.state === 'idle' || next.state === 'failed') {
    stopMidishowLoginPolling();
    accountLoading.value = false;
    if (next.state === 'failed') loginError.value = next.message;
    return;
  }
  if (next.state !== 'signed_in') return;
  stopMidishowLoginPolling();
  midishowPassword.value = '';
  await loadMidishowAccounts();
  midishowLoginOpen.value = false;
  accountLoading.value = false;
  if (!alreadySignedIn) {
    addLog(next.username ? l(`Midishow 登录成功：${next.username}`, `Signed in to Midishow as ${next.username}`) : l('Midishow 登录成功', 'Signed in to Midishow'));
  }
};

const refreshMidishowLoginStatus = async () => {
  try {
    await applyMidishowLoginStatus(await VrpianoApi.midishowLoginStatus());
  } catch (e) {
    // 轮询失败（如读取 cookie 临时失败）不要中断轮询，也不要污染顶部错误条。
    // Rust 侧超时后会通过事件推送 failed 状态；这里仅记录，避免 UI 卡在错误态。
    console.warn('[vrpiano] midishow login status poll failed:', e);
  }
};

const startMidishowLoginPolling = () => {
  stopMidishowLoginPolling();
  // 轮询间隔从 1200ms 缩到 500ms，让登录进度的反馈更跟手
  midishowLoginPollTimer = window.setInterval(refreshMidishowLoginStatus, 500);
};

const toggleMidishowLogin = () => {
  midishowLoginOpen.value = !midishowLoginOpen.value;
  if (midishowLoginOpen.value) {
    // 展开时也清空，避免上一次的明文账号/密码残留以及浏览器自动填充的值
    midishowAccount.value = '';
    midishowPassword.value = '';
    loginError.value = '';
  } else {
    // 收起时清空输入，避免明文账号/密码残留在表单中
    midishowAccount.value = '';
    midishowPassword.value = '';
    loginError.value = '';
  }
};

const loginMidishow = async () => {
  const account = midishowAccount.value.trim();
  const password = midishowPassword.value;
  if (!account || !password) {
    loginError.value = l('请输入 Midishow 账号和密码', 'Enter your Midishow account and password');
    return;
  }
  accountLoading.value = true;
  loginError.value = '';
  error.value = ''; // 清除顶部错误条（上一轮登录失败残留的提示）
  try {
    const next = await VrpianoApi.midishowLogin({ account, password });
    midishowPassword.value = '';
    await applyMidishowLoginStatus(next);
    if (next.state !== 'signed_in') {
      startMidishowLoginPolling();
      addLog(l('Midishow 正在自动登录', 'Signing in to Midishow automatically'));
    }
  } catch (e) {
    midishowPassword.value = '';
    accountLoading.value = false;
    loginError.value = formatVrpianoError(e);
    addLog(l(`Midishow 登录未完成：${loginError.value}`, `Midishow sign-in was not completed: ${loginError.value}`));
  }
};

const logoutMidishow = async () => {
  if (!defaultMidishowAccount.value) return;
  accountLoading.value = true;
  error.value = '';
  try {
    midishowAccounts.value = await VrpianoApi.midishowRemoveAccount({ username: defaultMidishowAccount.value.username });
    addLog(l('已退出 Midishow 登录', 'Signed out of Midishow'));
  } catch (e: any) {
    error.value = e.message || String(e);
  } finally {
    accountLoading.value = false;
  }
};

const refreshSongs = async () => {
  loading.value = true;
  error.value = '';
  try {
    songs.value = await VrpianoApi.listSongs();
    if (!selectedPath.value && songs.value[0]) selectedPath.value = songs.value[0].path;
    if (selectedPath.value && !songs.value.some((song) => song.path === selectedPath.value)) {
      selectedPath.value = songs.value[0]?.path || '';
    }
    addLog(l(`曲库已刷新，共 ${songs.value.length} 首`, `Library refreshed: ${songs.value.length} ${songs.value.length === 1 ? 'song' : 'songs'}`));
  } catch (e: any) {
    error.value = e.message || String(e);
    addLog(l(`刷新曲库失败：${error.value}`, `Could not refresh library: ${error.value}`));
  } finally {
    loading.value = false;
  }
};

const init = async () => {
  loading.value = true;
  error.value = '';
  try {
    loadSongIcons();
    status.value = await VrpianoApi.init();
    hotkeysEnabled.value = Boolean(status.value.hotkeys_enabled);
    await Promise.all([refreshSongs(), loadMidishowAccounts()]);
    addLog(l('VRPiano 模块已就绪', 'VRPiano is ready'));
  } catch (e: any) {
    error.value = e.message || String(e);
    addLog(l(`初始化失败：${error.value}`, `Initialization failed: ${error.value}`));
  } finally {
    loading.value = false;
  }
};

const importMidi = async () => {
  const selected = await open({
    multiple: false,
    filters: [{ name: 'MIDI', extensions: ['mid', 'midi'] }],
  });
  if (!selected || Array.isArray(selected)) return;

  loading.value = true;
  error.value = '';
  try {
    const song = await VrpianoApi.importSong({ sourcePath: selected });
    await refreshSongs();
    selectedPath.value = song.path;
    addLog(l(`已导入 ${song.name}`, `Imported ${song.name}`));
  } catch (e: any) {
    error.value = e.message || String(e);
    addLog(l(`导入失败：${error.value}`, `Import failed: ${error.value}`));
  } finally {
    loading.value = false;
  }
};

const renameSong = () => {
  if (!selectedSong.value) return;
  editSongName.value = selectedSong.value.name;
  editDialogMode.value = 'rename';
};

const submitRenameSong = async () => {
  if (!selectedSong.value) return;
  const nextName = editSongName.value.trim();
  if (!nextName || nextName === selectedSong.value.name) {
    closeEditDialog();
    return;
  }
  loading.value = true;
  error.value = '';
  try {
    const renamed = await VrpianoApi.renameSong({
      songPath: selectedSong.value.path,
      newName: nextName,
      overwrite: false,
    });
    if (songIcons.value[selectedSong.value.path]) {
      const nextIcons = { ...songIcons.value, [renamed.path]: songIcons.value[selectedSong.value.path] };
      delete nextIcons[selectedSong.value.path];
      songIcons.value = nextIcons;
      saveSongIcons();
    }
    await refreshSongs();
    selectedPath.value = renamed.path;
    addLog(l(`已重命名为 ${renamed.name}`, `Renamed to ${renamed.name}`));
    closeEditDialog();
  } catch (e: any) {
    error.value = e.message || String(e);
    addLog(l(`重命名失败：${error.value}`, `Rename failed: ${error.value}`));
  } finally {
    loading.value = false;
  }
};

const submitEditDialog = async () => {
  if (editDialogMode.value === 'icon') {
    saveSongIconEditor();
  } else if (editDialogMode.value === 'rename') {
    await submitRenameSong();
  }
};

const deleteSong = async () => {
  if (!selectedSong.value) return;
  if (!window.confirm(l(`确定删除「${selectedSong.value.name}」吗？此操作不可撤销。`, `Delete "${selectedSong.value.name}"? This cannot be undone.`))) return;
  loading.value = true;
  error.value = '';
  try {
    await VrpianoApi.deleteSong({ songPath: selectedSong.value.path });
    if (songIcons.value[selectedSong.value.path]) {
      const nextIcons = { ...songIcons.value };
      delete nextIcons[selectedSong.value.path];
      songIcons.value = nextIcons;
      saveSongIcons();
    }
    addLog(l(`已删除 ${selectedSong.value.name}`, `Deleted ${selectedSong.value.name}`));
    selectedPath.value = '';
    await refreshSongs();
  } catch (e: any) {
    error.value = e.message || String(e);
    addLog(l(`删除失败：${error.value}`, `Delete failed: ${error.value}`));
  } finally {
    loading.value = false;
  }
};

const previewLocalSong = async () => {
  if (!selectedSong.value) return;
  try {
    const midi = await VrpianoApi.readSongData({ songPath: selectedSong.value.path });
    await loadMidiIntoPlayer(midi);
  } catch (e: any) {
    error.value = e.message || String(e);
  }
};

const previewSong = async (song: VrpianoSong) => {
  selectedPath.value = song.path;
  await previewLocalSong();
};

const openSongsDir = async () => {
  try {
    await VrpianoApi.openSongsDir();
  } catch (e: any) {
    error.value = e.message || String(e);
  }
};

const downloadFromUrl = async () => {
  if (!urlInput.value.trim()) return;
  onlineLoading.value = true;
  error.value = '';
  try {
    const song = await VrpianoApi.downloadUrl({
      url: urlInput.value.trim(),
      filename: urlFilename.value.trim() || undefined,
    });
    await refreshSongs();
    selectedPath.value = song.path;
    addLog(l(`URL/ID 下载完成：${song.name}`, `URL/ID download completed: ${song.name}`));
  } catch (e: any) {
    error.value = formatVrpianoError(e);
    addLog(l(`URL/ID 下载失败：${error.value}`, `URL/ID download failed: ${error.value}`));
  } finally {
    onlineLoading.value = false;
  }
};

const searchOnline = async () => {
  const keyword = onlineKeyword.value.trim();
  if (!keyword || onlineLoading.value) return;
  const requestId = ++onlineSearchRequestId;
  onlineLoading.value = true;
  hasSearchedOnline.value = true;
  lastOnlineKeyword.value = keyword;
  onlineResults.value = [];
  error.value = '';
  if (onlineSearchTimeout !== null) window.clearTimeout(onlineSearchTimeout);
  onlineSearchTimeout = window.setTimeout(() => {
    onlineSearchTimeout = null;
    if (requestId !== onlineSearchRequestId) return;
    onlineSearchRequestId += 1;
    onlineLoading.value = false;
    error.value = l(
      'Midishow 搜索超时，请检查代理连接，或点击右侧按钮在浏览器打开官方搜索。',
      'Midishow search timed out. Check your proxy, or use the browser button to open the official search.',
    );
    addLog(l(`在线搜索失败：${error.value}`, `Online search failed: ${error.value}`));
  }, ONLINE_SEARCH_TIMEOUT_MS);
  try {
    const results = await VrpianoApi.searchMidishow({
      keyword,
      maxResults: 40,
    });
    if (requestId !== onlineSearchRequestId) return;
    onlineResults.value = results;
    addLog(l(`Midishow 搜索到 ${results.length} 个结果`, `Midishow returned ${results.length} results`));
  } catch (e: any) {
    if (requestId !== onlineSearchRequestId) return;
    error.value = formatVrpianoError(e);
    addLog(l(`在线搜索失败：${error.value}`, `Online search failed: ${error.value}`));
  } finally {
    if (requestId === onlineSearchRequestId) {
      onlineLoading.value = false;
      if (onlineSearchTimeout !== null) {
        window.clearTimeout(onlineSearchTimeout);
        onlineSearchTimeout = null;
      }
    }
  }
};

const previewOnline = async (song: VrpianoOnlineSong) => {
  onlineBusyId.value = song.id;
  error.value = '';
  try {
    const midi = await VrpianoApi.midishowPreviewData({ midiId: song.id, title: song.title });
    await loadMidiIntoPlayer(midi);
  } catch (e: any) {
    error.value = formatVrpianoError(e);
    addLog(l(`试听失败：${error.value}`, `Preview failed: ${error.value}`));
  } finally {
    onlineBusyId.value = null;
  }
};

const downloadOnline = async (song: VrpianoOnlineSong) => {
  onlineBusyId.value = song.id;
  error.value = '';
  try {
    const downloaded = await VrpianoApi.downloadMidishow({
      midiId: song.id,
      title: song.title,
      coverUrl: song.cover_url || null,
    });
    if (downloaded) {
      await refreshSongs();
      selectedPath.value = downloaded.path;
      addLog(l(`Midishow 下载完成：${downloaded.name}`, `Midishow download completed: ${downloaded.name}`));
    }
  } catch (e: any) {
    error.value = formatVrpianoError(e);
    addLog(l(`下载失败：${error.value}`, `Download failed: ${error.value}`));
  } finally {
    onlineBusyId.value = null;
  }
};

const openOnlinePage = async (song: VrpianoOnlineSong) => {
  try {
    await SysApi.openUrl({ url: song.page_url });
  } catch (e: any) {
    error.value = e.message || String(e);
  }
};

const applySpeed = async (announce = false) => {
  const nextSpeed = clampSpeed(speed.value);
  speed.value = nextSpeed;
  try {
    status.value = await VrpianoApi.setSpeed({ speed: nextSpeed });
    if (announce) addLog(l(`演奏速度 ${nextSpeed.toFixed(2)}x`, `Playback speed ${nextSpeed.toFixed(2)}x`));
  } catch (e: any) {
    error.value = e.message || String(e);
  }
};

const scheduleSpeedApply = () => {
  if (speedApplyTimer !== null) window.clearTimeout(speedApplyTimer);
  speedApplyTimer = window.setTimeout(() => {
    speedApplyTimer = null;
    void applySpeed();
  }, 90);
};

const adjustSpeed = (delta: number) => {
  speed.value = clampSpeed(speed.value + delta);
  void applySpeed(true);
};

const resetSpeed = () => {
  speed.value = 1;
  void applySpeed(true);
};

const applyHotkeys = async (announce = false) => {
  try {
    status.value = await VrpianoApi.setHotkeys({
      enabled: hotkeysEnabled.value,
      songPath: selectedSong.value?.path || selectedPath.value,
      delaySecs: Math.max(0, Math.round(delaySecs.value || 0)),
      speed: clampSpeed(speed.value),
    });
    hotkeysEnabled.value = Boolean(status.value.hotkeys_enabled);
    if (Number.isFinite(status.value.speed)) speed.value = status.value.speed;
    if (announce) addLog(hotkeysEnabled.value
      ? l('全局快捷键已开启，可在 VRChat 内使用', 'Global shortcuts enabled and available inside VRChat')
      : l('全局快捷键已关闭', 'Global shortcuts disabled'));
  } catch (e: any) {
    error.value = e.message || String(e);
    hotkeysEnabled.value = false;
  }
};

const scheduleHotkeyApply = () => {
  if (hotkeyApplyTimer !== null) window.clearTimeout(hotkeyApplyTimer);
  hotkeyApplyTimer = window.setTimeout(() => {
    hotkeyApplyTimer = null;
    void applyHotkeys();
  }, 160);
};

const toggleHotkeys = async () => {
  hotkeysEnabled.value = !hotkeysEnabled.value;
  await applyHotkeys(true);
};

const start = async () => {
  if (!selectedSong.value) return;
  loading.value = true;
  error.value = '';
  try {
    status.value = await VrpianoApi.start({
      songPath: selectedSong.value.path,
      delaySecs: Math.max(0, Math.round(delaySecs.value || 0)),
      speed: clampSpeed(speed.value),
    });
    addLog(l(`准备演奏 ${selectedSong.value.name}`, `Preparing to play ${selectedSong.value.name}`));
  } catch (e: any) {
    error.value = e.message || String(e);
    addLog(l(`启动失败：${error.value}`, `Start failed: ${error.value}`));
  } finally {
    loading.value = false;
  }
};

const stop = async () => {
  loading.value = true;
  error.value = '';
  try {
    status.value = await VrpianoApi.stop();
    addLog(l('已发送停止指令', 'Stop command sent'));
  } catch (e: any) {
    error.value = e.message || String(e);
  } finally {
    loading.value = false;
  }
};

let statusRefreshing = false;

const refreshStatus = async () => {
  // 在 KeepAlive 下组件不会卸载，避免窗口/标签页不可见时仍每 1.5s 打后端
  if (typeof document !== 'undefined' && document.hidden) return;
  if (statusRefreshing) return;
  statusRefreshing = true;
  try {
    status.value = await VrpianoApi.getStatus();
    if (Number.isFinite(status.value.speed)) speed.value = status.value.speed;
    hotkeysEnabled.value = Boolean(status.value.hotkeys_enabled);
    if (status.value.song_path && songs.value.some((song) => song.path === status.value.song_path)) {
      selectedPath.value = status.value.song_path;
    }
  } catch {
    // Backend may be unavailable in browser preview.
  }
};

const handleHotkey = (event: KeyboardEvent) => {
  if (!hotkeysEnabled.value) return;
  if (!['F1', 'F2', 'F3', 'F4', 'F5'].includes(event.key)) return;
  event.preventDefault();
  event.stopPropagation();
  if (event.key === 'F1') void start();
  else if (event.key === 'F2') void stop();
  else if (event.key === 'F3') adjustSpeed(0.1);
  else if (event.key === 'F4') adjustSpeed(-0.1);
  else if (event.key === 'F5') resetSpeed();
};

watch(speed, () => {
  scheduleSpeedApply();
  if (hotkeysEnabled.value) scheduleHotkeyApply();
});

watch([selectedPath, delaySecs], () => {
  if (hotkeysEnabled.value) scheduleHotkeyApply();
});

onMounted(async () => {
  const savedInstrument = localStorage.getItem(playerInstrumentStorageKey);
  if (savedInstrument === 'source' || (savedInstrument && Number(savedInstrument) >= 0 && Number(savedInstrument) <= 127)) {
    playerInstrument.value = savedInstrument;
  }
  await init();
  try {
    overlayOpen.value = Boolean(await WebviewWindow.getByLabel('vrpiano-overlay'));
    unlistenOverlayClosed = await listen('vrpiano-overlay-closed', () => {
      overlayOpen.value = false;
    });
    unlistenMidishowLogin = await listen<VrpianoMidishowLoginStatus>('vrpiano_midishow_login_status', (event) => {
      void applyMidishowLoginStatus(event.payload);
    });
    unlistenStatus = await listen<VrpianoStatus>('vrpiano_status', (event) => {
      status.value = event.payload;
      if (Number.isFinite(event.payload.speed)) speed.value = event.payload.speed;
      hotkeysEnabled.value = Boolean(event.payload.hotkeys_enabled);
      if (event.payload.song_path && songs.value.some((song) => song.path === event.payload.song_path)) {
        selectedPath.value = event.payload.song_path;
      }
      if (event.payload.last_event) addLog(event.payload.last_event);
      if (event.payload.last_error) error.value = event.payload.last_error;
    });
  } catch {
    // Non-Tauri preview.
  }
  if (pollTimer === null) {
    pollTimer = window.setInterval(refreshStatus, 1500);
  }
  window.addEventListener('keydown', handleHotkey, { capture: true });
});

onUnmounted(() => {
  if (unlistenStatus) unlistenStatus();
  if (unlistenOverlayClosed) unlistenOverlayClosed();
  if (unlistenMidishowLogin) unlistenMidishowLogin();
  stopMidishowLoginPolling();
  if (pollTimer !== null) window.clearInterval(pollTimer);
  if (speedApplyTimer !== null) window.clearTimeout(speedApplyTimer);
  if (hotkeyApplyTimer !== null) window.clearTimeout(hotkeyApplyTimer);
  if (onlineSearchTimeout !== null) window.clearTimeout(onlineSearchTimeout);
  onlineSearchRequestId += 1;
  pausePlayer();
  void audioContext?.close();
  window.removeEventListener('keydown', handleHotkey, { capture: true } as any);
});
</script>

<template>
  <div class="vrpiano-shell">
    <input ref="iconFileInput" class="sr-only-file" type="file" accept="image/*" @change="handleSongIconFile">
    <header class="vrpiano-header">
      <div class="title-block">
        <div class="title-icon"><Music :size="22" /></div>
        <div>
          <h1>{{ l('VRPiano 自动演奏', 'VRPiano Autoplay') }}</h1>
          <p>{{ l('本地曲库、在线下载、在线试听、MIDI 映射与全局快捷键控制', 'Local library, online downloads, previews, MIDI mapping, and global shortcuts') }}</p>
        </div>
      </div>
      <div class="header-actions">
        <button class="overlay-toggle" :class="{ active: overlayOpen }" @click="toggleVrpianoOverlay">
          <PictureInPicture2 :size="16" />
          {{ overlayOpen ? l('关闭悬浮窗', 'Close overlay') : l('开启悬浮窗', 'Open overlay') }}
        </button>
        <div class="status-pill" :class="{ active: status.running && !status.paused }">
          <span class="status-dot" />
          {{ status.paused ? l('已暂停', 'Paused') : status.running ? l('演奏中', 'Playing') : l('待命', 'Ready') }}
        </div>
      </div>
    </header>

    <section class="quick-stats">
      <div><Music :size="16" /><span>{{ songs.length }} {{ l('首曲目', songs.length === 1 ? 'song' : 'songs') }}</span></div>
      <div><Clock3 :size="16" /><span>{{ formatTime(status.elapsed_ms) }} / {{ formatTime(status.duration_ms) }}</span></div>
      <div><Gauge :size="16" /><span>{{ speedText }} {{ l('速度', 'speed') }}</span></div>
      <div><ShieldCheck :size="16" /><span>{{ hotkeyStatusText }}</span></div>
    </section>

    <div v-if="error || status.last_error" class="error-banner">
      <AlertTriangle :size="16" />
      <span>{{ error || status.last_error }}</span>
    </div>

    <main class="vrpiano-main">
      <section class="library-pane">
        <div class="pane-toolbar">
          <strong>{{ l('本地曲库', 'Local library') }}</strong>
          <div class="library-search">
            <Search :size="15" />
            <input
              v-model="localSongQuery"
              :placeholder="l('搜索本地曲库', 'Search local library')"
              @keydown.enter.prevent="selectFirstFilteredSong"
            >
            <button
              v-if="localSongQuery"
              class="clear-search-btn"
              type="button"
              :title="l('清空搜索', 'Clear search')"
              @click="clearLocalSongQuery"
            >
              <X :size="14" />
            </button>
          </div>
          <div class="tool-buttons">
            <button class="icon-btn" :title="l('导入 MIDI', 'Import MIDI')" :disabled="loading" @click="importMidi"><Upload :size="16" /></button>
            <button class="icon-btn" :title="l('试听曲目', 'Preview song')" :disabled="!selectedSong" @click="previewLocalSong"><Headphones :size="16" /></button>
            <button class="icon-btn" :title="l('设置文字图标', 'Set text icon')" :disabled="!selectedSong" @click="setSongEmojiIcon"><Music :size="16" /></button>
            <button class="icon-btn" :title="l('选择图片图标', 'Choose image icon')" :disabled="!selectedSong" @click="chooseSongImageIcon"><ImagePlus :size="16" /></button>
            <button class="icon-btn" :title="l('重命名', 'Rename')" :disabled="!selectedSong || loading" @click="renameSong"><Edit3 :size="16" /></button>
            <button class="icon-btn danger" :title="l('删除', 'Delete')" :disabled="!selectedSong || loading" @click="deleteSong"><Trash2 :size="16" /></button>
            <button class="icon-btn" :title="l('刷新曲库', 'Refresh library')" :disabled="loading" @click="refreshSongs">
              <RefreshCcw :size="16" :class="{ spin: loading }" />
            </button>
            <button class="icon-btn" :title="l('打开曲库目录', 'Open library folder')" @click="openSongsDir"><FolderOpen :size="16" /></button>
          </div>
        </div>

        <div class="song-list">
          <button
            v-for="song in filteredSongs"
            :key="song.path"
            class="song-row"
            :class="{ selected: selectedPath === song.path }"
            @click="selectedPath = song.path"
            @dblclick="previewSong(song)"
          >
            <span class="song-note" :class="{ custom: Boolean(songIcon(song) || songCover(song)) }">
              <img v-if="isImageIcon(songIcon(song))" :src="songIcon(song)" alt="">
              <span v-else-if="songIcon(song)">{{ songIcon(song) }}</span>
              <img v-else-if="songCover(song)" :src="songCover(song)" :title="l('来自 Midishow 的封面', 'Cover from Midishow')" alt="">
              <Music v-else :size="15" />
            </span>
            <span class="song-meta">
              <strong>{{ song.name }}</strong>
              <small>{{ formatBytes(song.size) }}</small>
            </span>
          </button>
          <div v-if="!songs.length" class="empty-state">
            <Music :size="24" />
            <span>{{ l('暂无 MIDI 曲目，导入、搜索或粘贴链接添加。', 'No MIDI songs yet. Import, search, or paste a link to add one.') }}</span>
          </div>
          <div v-else-if="!filteredSongs.length" class="empty-state">
            <Search :size="24" />
            <span>{{ l(`没有匹配“${localSongQuery.trim()}”的本地曲目。`, `No local songs match "${localSongQuery.trim()}".`) }}</span>
          </div>
        </div>
      </section>

      <section class="control-pane">
        <div class="now-playing">
          <span>{{ l('当前曲目', 'Current song') }}</span>
          <strong :title="selectedSong?.name || l('未选择', 'None selected')">{{ selectedSong?.name || l('未选择', 'None selected') }}</strong>
          <small :title="selectedSong?.path || status.songs_dir">{{ selectedSong?.path || status.songs_dir }}</small>
        </div>

        <section class="player-panel">
          <div class="player-head">
            <div>
              <span>{{ l('内置播放器', 'Built-in player') }}</span>
              <strong>{{ playerTitle }}</strong>
            </div>
            <button class="player-toggle" :disabled="!canTogglePlayer" @click="togglePlayer">
              <Loader2 v-if="playerLoading" :size="16" class="spin" />
              <Pause v-else-if="playerPlaying" :size="16" />
              <Play v-else :size="16" />
              {{ playerPlaying ? l('暂停', 'Pause') : l('播放', 'Play') }}
            </button>
          </div>
          <label class="player-instrument">
            <Music :size="15" />
            <span>{{ l('播放音色', 'Playback instrument') }}</span>
            <select v-model="playerInstrument" @change="applyPlayerInstrument">
              <option value="source">{{ l('跟随 MIDI 源文件（默认）', 'Follow MIDI source (default)') }}</option>
              <optgroup v-for="(group, groupIndex) in GENERAL_MIDI_GROUPS" :key="group.name" :label="instrumentGroupName(groupIndex, group.name)">
                <option v-for="instrument in group.instruments" :key="instrument.program" :value="String(instrument.program)">
                  {{ instrument.program + 1 }} · {{ instrumentName(instrument.program) }}
                </option>
              </optgroup>
            </select>
            <small :title="activeInstrumentText">{{ activeInstrumentText }}</small>
          </label>
          <div class="player-slider">
            <span>{{ formatTime(playerPositionMs) }}</span>
            <input
              v-model.number="playerPositionMs"
              type="range"
              min="0"
              :max="Math.max(1, playerDurationMs)"
              step="250"
              :disabled="!parsedPlayerNotes.length"
              @change="seekPlayer"
            >
            <span>{{ formatTime(playerDurationMs) }}</span>
          </div>
          <div class="player-volume">
            <Volume2 :size="15" />
            <input v-model.number="playerVolume" type="range" min="0" max="1" step="0.05" @input="applyPlayerVolume">
            <b>{{ Math.round(playerVolume * 100) }}%</b>
            <small>{{ playerProgressPercent }}%</small>
          </div>
        </section>

        <div class="progress-area">
          <div class="progress-head">
            <span>{{ status.last_event || l('待命', 'Ready') }}</span>
            <strong>{{ progressPercent }}%</strong>
          </div>
          <div class="progress-track"><div class="progress-fill" :style="{ width: `${progressPercent}%` }" /></div>
          <div class="progress-foot">
            <span>{{ status.played_notes }} / {{ status.total_notes }} notes</span>
            <span>{{ formatTime(status.duration_ms) }}</span>
          </div>
        </div>

        <div class="control-grid">
          <label>
            <span>{{ l('开始延迟', 'Start delay') }}</span>
            <input v-model.number="delaySecs" type="number" min="0" max="60">
            <b>{{ l('秒', 'sec') }}</b>
          </label>
          <label>
            <span>{{ l('速度倍率', 'Speed multiplier') }}</span>
            <input v-model.number="speed" type="range" min="0.25" max="3" step="0.05">
            <b>{{ speedText }}</b>
          </label>
        </div>

        <div class="speed-actions">
          <button class="small-action" @click="adjustSpeed(-0.1)">F4 {{ l('减慢', 'Slower') }}</button>
          <button class="small-action" @click="resetSpeed">F5 {{ l('默认', 'Default') }}</button>
          <button class="small-action" @click="adjustSpeed(0.1)">F3 {{ l('加快', 'Faster') }}</button>
        </div>

        <div class="hotkey-panel" :class="{ enabled: hotkeysEnabled }">
          <div>
            <strong>{{ l('全局快捷键', 'Global shortcuts') }}</strong>
            <span>{{ l('开启后 F1 开始、F2 停止、F3 加快、F4 减慢、F5 恢复默认速度，可在 VRChat 内响应。', 'When enabled, F1 starts, F2 stops, F3 speeds up, F4 slows down, and F5 restores the default speed, including inside VRChat.') }}</span>
          </div>
          <button class="toggle-btn" :class="{ enabled: hotkeysEnabled }" :disabled="!status.hotkeys_available" @click="toggleHotkeys">
            {{ hotkeysEnabled ? l('已开启', 'Enabled') : l('已关闭', 'Disabled') }}
          </button>
        </div>

        <div class="action-row">
          <button class="primary-action" :disabled="!canStart" @click="start">
            <Loader2 v-if="loading && !status.running" :size="18" class="spin" />
            <Play v-else :size="18" />
            F1 {{ l('开始演奏', 'Start') }}
          </button>
          <button class="danger-action" :disabled="!status.running && !loading" @click="stop">
            <Square :size="18" />
            F2 {{ l('停止', 'Stop') }}
          </button>
        </div>

        <section class="online-panel">
          <div class="online-head">
            <strong>{{ l('在线曲库', 'Online library') }}</strong>
            <span>{{ l('Midishow 搜索、ID/URL 下载、在线试听', 'Midishow search, ID/URL downloads, and online previews') }}</span>
          </div>

          <div class="midishow-account">
            <div>
              <strong>{{ defaultMidishowAccount ? l(`已登录 ${defaultMidishowAccount.username}${defaultMidishowLoginTypeText ? `（${defaultMidishowLoginTypeText}）` : ''}`, `Signed in as ${defaultMidishowAccount.username}${defaultMidishowLoginTypeText ? ` (${defaultMidishowLoginTypeText})` : ''}`) : l('Midishow 未登录', 'Midishow signed out') }}</strong>
              <span>{{ defaultMidishowAccount ? l('下载与试听会优先使用账号权限。', 'Downloads and previews will use your account access.') : l('登录后可访问需要账号权限的 MIDI 下载与试听。', 'Sign in to access MIDI downloads and previews that require an account.') }}</span>
            </div>
            <button v-if="defaultMidishowAccount" class="account-btn ghost" :disabled="accountLoading" @click="logoutMidishow">
              <LogOut :size="15" /> {{ l('退出', 'Sign out') }}
            </button>
            <button v-else class="account-btn" @click="toggleMidishowLogin">
              <LogIn :size="15" /> {{ midishowLoginOpen ? l('收起', 'Hide') : l('登录', 'Sign in') }}
            </button>
          </div>

          <form v-if="midishowLoginOpen && !defaultMidishowAccount" :key="midishowLoginOpen ? 'open' : 'closed'" class="login-form" @submit.prevent="loginMidishow">
            <label class="login-field">
              <span>{{ l('账号', 'Account') }}</span>
              <input
                ref="accountInputRef"
                v-model="midishowAccount"
                type="text"
                name="ms_account"
                autocomplete="off"
                autocapitalize="off"
                autocorrect="off"
                spellcheck="false"
                data-lpignore="true"
                data-1p-ignore="true"
                data-form-type="other"
                :placeholder="l('Midishow 用户名或邮箱', 'Midishow username or email')"
              >
            </label>
            <label class="login-field">
              <span>{{ l('密码', 'Password') }}</span>
              <input
                ref="passwordInputRef"
                v-model="midishowPassword"
                type="password"
                name="ms_password"
                autocomplete="new-password"
                autocapitalize="off"
                autocorrect="off"
                spellcheck="false"
                data-lpignore="true"
                data-1p-ignore="true"
                data-form-type="other"
                :placeholder="l('Midishow 密码', 'Midishow password')"
              >
            </label>
            <p v-if="loginError" class="login-error" role="alert">
              <AlertTriangle :size="14" />
              <span>{{ loginError }}</span>
            </p>
            <p class="login-hint">{{ l('点击登录后会自动完成。只有页面需要确认时才会显示登录窗口。', 'Sign-in completes automatically. A login window appears only when confirmation is required.') }}</p>
            <p v-if="accountLoading" class="login-status" aria-live="polite">
              <Loader2 :size="15" class="spin" />
              {{ midishowLoginStatus.message }}
            </p>
            <div class="login-form-actions">
              <button type="submit" :disabled="accountLoading || !midishowAccount.trim() || !midishowPassword">
                <Loader2 v-if="accountLoading" :size="16" class="spin" />
                <LogIn v-else :size="16" />
                <span>{{ accountLoading ? l('正在登录', 'Signing in') : l('登录', 'Sign in') }}</span>
              </button>
              <button type="button" class="account-btn ghost" :disabled="externalLinkLoading || accountLoading" @click="openMidishowSignup">
                <Loader2 v-if="externalLinkLoading" :size="15" class="spin" />
                <ExternalLink v-else :size="15" /> {{ l('注册', 'Register') }}
              </button>
              <button type="button" class="account-btn ghost" :title="l('复制注册链接', 'Copy registration link')" :disabled="accountLoading" @click="copySignupUrl">
                {{ l('复制链接', 'Copy link') }}
              </button>
            </div>
          </form>

          <div class="online-form">
            <div class="input-row">
              <Search :size="16" />
              <input v-model="onlineKeyword" :placeholder="l('搜索歌名、作者或关键词', 'Search by title, artist, or keyword')" @keydown.enter="searchOnline">
              <div class="online-search-actions">
                <button :disabled="onlineLoading || !onlineKeyword.trim()" @click="searchOnline">
                  <Loader2 v-if="onlineLoading" :size="16" class="spin" />
                  <span v-else>{{ l('搜索', 'Search') }}</span>
                </button>
                <button type="button" :title="l('在浏览器打开 Midishow 官方搜索', 'Open Midishow search in browser')" :disabled="onlineLoading" @click="openMidishowSearch">
                  <ExternalLink :size="16" />
                </button>
              </div>
            </div>
            <div class="input-row download-row">
              <Link2 :size="16" />
              <input v-model="urlInput" :placeholder="l('粘贴 MIDI 直链、Midishow 链接或 ID', 'Paste a direct MIDI URL, Midishow link, or ID')" @keydown.enter="downloadFromUrl">
              <input v-model="urlFilename" class="name-input" :placeholder="l('保存名', 'Save as')">
              <button :disabled="onlineLoading || !urlInput.trim()" @click="downloadFromUrl">
                <Download :size="16" />
              </button>
            </div>
          </div>

          <div class="online-results">
            <div v-for="item in onlineResults" :key="item.id" class="online-row">
              <div class="online-meta">
                <strong>{{ item.title }}</strong>
                <small>{{ item.artist || l('未知作者', 'Unknown artist') }} · ID {{ item.id }}</small>
              </div>
              <div class="online-actions">
                <button :title="l('在线试听', 'Preview online')" :disabled="onlineBusyId === item.id" @click="previewOnline(item)">
                  <Loader2 v-if="onlineBusyId === item.id" :size="15" class="spin" />
                  <Headphones v-else :size="15" />
                </button>
                <button :title="l('下载到曲库', 'Download to library')" :disabled="onlineBusyId === item.id" @click="downloadOnline(item)">
                  <Download :size="15" />
                </button>
                <button :title="l('打开网页', 'Open webpage')" @click="openOnlinePage(item)">
                  <ExternalLink :size="15" />
                </button>
              </div>
            </div>
            <div v-if="!onlineResults.length" class="online-empty">
              <Search :size="20" />
              <span>{{ onlineEmptyText }}</span>
            </div>
          </div>
        </section>

        <div class="log-pane">
          <div v-for="line in logs" :key="line" class="log-line">{{ line }}</div>
        </div>
      </section>
    </main>

    <div v-if="editDialogMode" class="edit-backdrop" @click.self="closeEditDialog">
      <form class="edit-dialog" @submit.prevent="submitEditDialog">
        <div class="edit-dialog-head">
          <div>
            <strong>{{ editDialogMode === 'icon' ? l('编辑曲目图标', 'Edit song icon') : l('重命名曲目', 'Rename song') }}</strong>
            <span>{{ selectedSong?.name }}</span>
          </div>
          <button class="dialog-close" type="button" :disabled="loading" @click="closeEditDialog">×</button>
        </div>

        <div v-if="editDialogMode === 'icon'" class="edit-dialog-body">
          <label class="theme-field">
            <span>{{ l('文字 / Emoji', 'Text / Emoji') }}</span>
            <input v-model="editIconText" maxlength="4" :placeholder="l('例如 ♪、钢琴、A1', 'For example: ♪, Piano, A1')">
          </label>
          <label class="theme-field">
            <span>{{ l('图片 URL', 'Image URL') }}</span>
            <input v-model="editIconUrl" :placeholder="l('https://... 或 data:image/...', 'https://... or data:image/...')">
          </label>
          <div class="icon-preview">
            <span class="song-note custom">
              <img v-if="isImageIcon(editIconUrl.trim())" :src="editIconUrl.trim()" alt="">
              <span v-else-if="editIconText.trim()">{{ editIconText.trim().slice(0, 4) }}</span>
              <Music v-else :size="15" />
            </span>
            <small>{{ l('图片 URL 优先；两个输入框都留空会恢复默认图标。', 'Image URL takes priority. Leave both fields empty to restore the default icon.') }}</small>
          </div>
        </div>

        <div v-else class="edit-dialog-body">
          <label class="theme-field">
            <span>{{ l('新的曲目名称', 'New song name') }}</span>
            <input v-model="editSongName" autofocus :placeholder="l('输入新的 MIDI 文件名', 'Enter a new MIDI filename')">
          </label>
        </div>

        <div class="edit-dialog-actions">
          <button class="dialog-secondary" type="button" :disabled="loading" @click="closeEditDialog">{{ l('取消', 'Cancel') }}</button>
          <button class="dialog-primary" type="submit" :disabled="loading">
            <Loader2 v-if="loading" :size="16" class="spin" />
            <span v-else>{{ l('保存', 'Save') }}</span>
          </button>
        </div>
      </form>
    </div>
  </div>
</template>

<style scoped>
.vrpiano-shell {
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
  height: 100%;
  min-height: 0;
  display: flex;
  flex-direction: column;
  gap: 14px;
  color: var(--vp-text);
}

.sr-only-file {
  position: fixed;
  width: 1px;
  height: 1px;
  opacity: 0;
  pointer-events: none;
}

.vrpiano-header,
.quick-stats,
.library-pane,
.control-pane {
  border: 1px solid var(--vp-border);
  background: var(--vp-surface);
  backdrop-filter: var(--theme-glass-effect, blur(18px));
  box-shadow: 0 18px 40px rgba(74, 45, 15, 0.08);
}

.vrpiano-header {
  min-height: 76px;
  border-radius: 8px;
  padding: 14px 16px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
}

.title-block {
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 12px;
}

.title-icon,
.song-note {
  flex-shrink: 0;
  display: grid;
  place-items: center;
  color: white;
  background: linear-gradient(135deg, var(--vp-primary), color-mix(in srgb, var(--vp-primary) 45%, #22c55e));
}

.song-note.custom {
  color: var(--vp-text);
  background: var(--vp-panel);
  box-shadow: inset 0 0 0 1px var(--vp-border);
  font-weight: 900;
  font-size: 15px;
}

.song-note img {
  width: 100%;
  height: 100%;
  border-radius: inherit;
  object-fit: cover;
}

.title-icon {
  width: 44px;
  height: 44px;
  border-radius: 8px;
}

h1,
p {
  margin: 0;
}

h1 {
  font-size: 20px;
  line-height: 1.2;
}

.title-block p,
.now-playing span,
.now-playing small,
.progress-foot,
.online-head span,
.online-row small,
.online-empty,
.log-line {
  color: var(--vp-muted);
}

.header-actions {
  flex: 0 0 auto;
  display: flex;
  align-items: center;
  gap: 8px;
}

.overlay-toggle {
  min-height: 36px;
  padding: 0 11px;
  border: 1px solid var(--vp-border);
  border-radius: 7px;
  display: inline-flex;
  align-items: center;
  gap: 7px;
  color: var(--vp-muted);
  background: var(--vp-panel);
  font: inherit;
  font-size: 12px;
  font-weight: 800;
  cursor: pointer;
}

.overlay-toggle:hover,
.overlay-toggle.active {
  color: var(--vp-primary);
  border-color: color-mix(in srgb, var(--vp-primary) 38%, transparent);
  background: var(--vp-hover);
}

.status-pill {
  flex-shrink: 0;
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  border-radius: 999px;
  color: var(--vp-muted);
  background: var(--vp-panel);
  box-shadow: inset 0 0 0 1px var(--vp-border);
  font-weight: 700;
  font-size: 13px;
}

.status-dot {
  width: 8px;
  height: 8px;
  border-radius: 999px;
  background: var(--vp-dim);
}

.status-pill.active .status-dot {
  background: #22c55e;
  box-shadow: 0 0 0 4px rgba(34, 197, 94, 0.16);
}

.quick-stats {
  min-height: 52px;
  border-radius: 8px;
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  overflow: hidden;
}

.quick-stats div {
  min-width: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  color: var(--vp-muted);
  font-weight: 700;
  font-size: 13px;
}

.quick-stats div + div {
  box-shadow: inset 1px 0 0 var(--vp-border);
}

.error-banner {
  min-height: 42px;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 12px;
  border-radius: 8px;
  border: 1px solid rgba(239, 68, 68, 0.2);
  color: #b91c1c;
  background: rgba(239, 68, 68, 0.1);
}

.vrpiano-main {
  min-height: 0;
  flex: 1;
  display: grid;
  grid-template-columns: minmax(340px, 38%) minmax(0, 1fr);
  gap: 14px;
}

.library-pane,
.control-pane {
  min-width: 0;
  min-height: 0;
  border-radius: 8px;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

.pane-toolbar {
  flex: 0 0 auto;
  min-height: 0;
  padding: 12px;
  display: flex;
  flex-direction: column;
  align-items: stretch;
  box-shadow: inset 0 -1px 0 var(--vp-border);
  gap: 9px;
}

.pane-toolbar strong {
  flex-shrink: 0;
  white-space: nowrap;
}

.library-search {
  min-width: 0;
  min-height: 34px;
  display: grid;
  grid-template-columns: auto minmax(0, 1fr) auto;
  align-items: center;
  gap: 7px;
  padding: 7px 9px;
  border-radius: 8px;
  color: var(--vp-muted);
  background: var(--vp-panel);
  box-shadow: inset 0 0 0 1px var(--vp-border);
}

.library-search input {
  min-width: 0;
  border: 0;
  outline: none;
  color: var(--vp-text);
  background: transparent;
}

.clear-search-btn {
  width: 24px;
  height: 24px;
  border: 0;
  border-radius: 6px;
  display: grid;
  place-items: center;
  color: var(--vp-muted);
  background: transparent;
  cursor: pointer;
}

.clear-search-btn:hover {
  color: var(--vp-text);
  background: var(--vp-hover);
}

.tool-buttons,
.action-row,
.speed-actions {
  display: flex;
  gap: 8px;
}

.tool-buttons {
  display: grid;
  grid-template-columns: repeat(8, minmax(28px, 32px));
  justify-content: start;
  gap: 6px;
  width: 100%;
}

button,
input {
  font: inherit;
}

.icon-btn,
.online-actions button {
  width: 32px;
  height: 32px;
  border: 0;
  border-radius: 6px;
  display: grid;
  place-items: center;
  color: var(--vp-muted);
  background: var(--vp-panel);
  box-shadow: inset 0 0 0 1px var(--vp-border);
  cursor: pointer;
}

.icon-btn:hover:not(:disabled),
.online-actions button:hover:not(:disabled) {
  color: var(--vp-text);
  background: var(--vp-hover);
}

.icon-btn.danger {
  color: #b91c1c;
}

.icon-btn:disabled,
.primary-action:disabled,
.danger-action:disabled,
.small-action:disabled,
.online-form button:disabled,
.online-actions button:disabled {
  cursor: not-allowed;
  opacity: 0.55;
}

.song-list {
  flex: 1 1 auto;
  min-height: 0;
  overflow-y: auto;
  padding: 10px;
  display: grid;
  align-content: start;
  gap: 8px;
}

.song-row {
  width: 100%;
  min-width: 0;
  min-height: 62px;
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 10px;
  border: 1px solid transparent;
  border-radius: 8px;
  color: var(--vp-text);
  background: color-mix(in srgb, var(--vp-panel) 72%, transparent);
  text-align: left;
  cursor: pointer;
}

.song-row:hover,
.song-row.selected {
  border-color: var(--vp-border-strong);
  background: var(--vp-hover);
}

.song-note {
  width: 34px;
  height: 34px;
  border-radius: 8px;
}

.song-meta {
  flex: 1;
  min-width: 0;
  display: grid;
  gap: 4px;
}

.song-meta strong,
.online-meta strong,
.now-playing strong {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.song-meta small {
  color: var(--vp-dim);
}

.empty-state,
.online-empty {
  min-height: 120px;
  display: grid;
  place-items: center;
  align-content: center;
  gap: 8px;
  color: var(--vp-muted);
  text-align: center;
}

.control-pane {
  padding: 14px;
  gap: 14px;
  overflow-y: auto;
  align-items: stretch;
  justify-content: flex-start;
}

.control-pane > * {
  flex: 0 0 auto;
}

.now-playing,
.progress-area,
.player-panel,
.online-panel,
.hotkey-panel {
  border-radius: 8px;
  background: var(--vp-panel);
  box-shadow: inset 0 0 0 1px var(--vp-border);
}

.now-playing {
  min-width: 0;
  max-width: 100%;
  overflow: hidden;
  display: grid;
  gap: 6px;
  padding: 12px;
}

.now-playing > * {
  min-width: 0;
  max-width: 100%;
}

.now-playing small {
  overflow: hidden;
  display: -webkit-box;
  overflow-wrap: anywhere;
  word-break: break-word;
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 2;
  line-height: 1.45;
}

.progress-area {
  padding: 12px;
  display: grid;
  gap: 9px;
}

.player-panel {
  display: grid;
  gap: 10px;
  padding: 12px;
}

.player-head,
.player-slider,
.player-volume,
.midishow-account,
.login-form {
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 10px;
}

.player-instrument {
  min-width: 0;
  display: grid;
  grid-template-columns: 16px 64px minmax(0, 1fr);
  align-items: center;
  gap: 6px 10px;
  color: var(--vp-muted);
}

.player-instrument > span {
  font-size: 12px;
  font-weight: 800;
}

.player-instrument select {
  min-width: 0;
  width: 100%;
  height: 34px;
  padding: 0 30px 0 10px;
  border: 1px solid var(--vp-border-strong);
  border-radius: 6px;
  outline: none;
  color: var(--vp-text);
  background: var(--vp-surface);
  font: inherit;
  font-size: 12px;
  font-weight: 750;
}

.player-instrument select:focus {
  border-color: var(--vp-primary);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--vp-primary) 14%, transparent);
}

.player-instrument small {
  min-width: 0;
  grid-column: 3;
  overflow: hidden;
  color: var(--vp-dim);
  font-size: 11px;
  font-weight: 700;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.player-head {
  justify-content: space-between;
}

.player-head div {
  min-width: 0;
  display: grid;
  gap: 3px;
}

.player-head span,
.player-volume small,
.midishow-account span {
  color: var(--vp-muted);
  font-size: 12px;
  font-weight: 700;
}

.player-head strong {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.player-toggle,
.account-btn,
.login-form button {
  min-height: 34px;
  border: 0;
  border-radius: 8px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 7px;
  color: white;
  background: var(--vp-primary);
  font-weight: 850;
  white-space: nowrap;
  cursor: pointer;
}

.player-toggle {
  min-width: 92px;
  flex-shrink: 0;
}

.player-toggle:disabled,
.account-btn:disabled,
.login-form button:disabled {
  color: var(--vp-text);
  background: var(--vp-surface);
  box-shadow: inset 0 0 0 1px var(--vp-border);
  cursor: not-allowed;
}

.player-slider span,
.player-volume b {
  width: 48px;
  color: var(--vp-dim);
  font-size: 12px;
  font-weight: 800;
  text-align: center;
}

.player-slider input,
.player-volume input {
  min-width: 0;
  flex: 1;
  appearance: none;
  height: 4px;
  border-radius: 999px;
  background: color-mix(in srgb, var(--vp-text) 12%, transparent);
}

.player-slider input::-webkit-slider-thumb,
.player-volume input::-webkit-slider-thumb {
  appearance: none;
  width: 14px;
  height: 14px;
  border-radius: 999px;
  background: var(--vp-primary);
  box-shadow: 0 0 0 4px color-mix(in srgb, var(--vp-primary) 18%, transparent);
}

.progress-head,
.progress-foot,
.online-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.progress-track {
  height: 9px;
  overflow: hidden;
  border-radius: 999px;
  background: color-mix(in srgb, var(--vp-text) 12%, transparent);
}

.progress-fill {
  height: 100%;
  border-radius: inherit;
  background: var(--vp-primary);
}

.control-grid {
  display: grid;
  grid-template-columns: minmax(0, 180px) minmax(0, 1fr);
  gap: 12px;
}

.control-grid label {
  min-height: 72px;
  display: grid;
  grid-template-columns: 1fr auto;
  grid-template-rows: auto 1fr;
  gap: 8px;
  padding: 12px;
  border-radius: 8px;
  background: var(--vp-panel);
}

.control-grid span {
  color: var(--vp-muted);
  font-weight: 700;
}

.control-grid b {
  color: var(--vp-dim);
  font-size: 12px;
}

.control-grid input[type="number"] {
  width: 100%;
  min-width: 0;
  grid-column: 1 / -1;
  border: 1px solid var(--vp-border);
  border-radius: 6px;
  padding: 8px 10px;
  color: var(--vp-text);
  background: var(--vp-surface);
  outline: none;
}

.control-grid input[type="range"] {
  grid-column: 1 / -1;
  appearance: none;
  height: 4px;
  align-self: center;
  border-radius: 999px;
  background: color-mix(in srgb, var(--vp-text) 12%, transparent);
}

.control-grid input[type="range"]::-webkit-slider-thumb {
  appearance: none;
  width: 14px;
  height: 14px;
  border-radius: 999px;
  background: var(--vp-primary);
  box-shadow: 0 0 0 4px color-mix(in srgb, var(--vp-primary) 18%, transparent);
}

.primary-action,
.danger-action,
.small-action,
.toggle-btn,
.online-form button {
  min-height: 40px;
  border: 0;
  border-radius: 8px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  cursor: pointer;
  font-weight: 800;
  white-space: nowrap;
}

.small-action {
  flex: 1;
  min-height: 36px;
  color: var(--vp-muted);
  background: var(--vp-panel);
  box-shadow: inset 0 0 0 1px var(--vp-border);
}

.small-action:hover {
  color: var(--vp-text);
  background: var(--vp-hover);
}

.hotkey-panel {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 12px;
}

.hotkey-panel.enabled {
  background: color-mix(in srgb, var(--vp-primary) 12%, var(--vp-panel));
}

.hotkey-panel div {
  min-width: 0;
  display: grid;
  gap: 4px;
}

.hotkey-panel span {
  color: var(--vp-muted);
  font-size: 12px;
  line-height: 1.45;
}

.toggle-btn {
  flex-shrink: 0;
  min-width: 82px;
  min-height: 36px;
  color: var(--vp-muted);
  background: var(--vp-surface);
  box-shadow: inset 0 0 0 1px var(--vp-border);
}

.toggle-btn.enabled,
.primary-action {
  color: white;
  background: var(--vp-primary);
}

.primary-action {
  flex: 1;
}

.primary-action:hover:not(:disabled) {
  background: var(--vp-primary-hover);
}

.danger-action {
  min-width: 130px;
  color: #b91c1c;
  background: rgba(239, 68, 68, 0.12);
  box-shadow: inset 0 0 0 1px rgba(239, 68, 68, 0.16);
}

.online-panel {
  flex: 0 0 auto;
  min-height: 0;
  padding: 12px;
  display: grid;
  gap: 12px;
}

.midishow-account {
  justify-content: space-between;
  padding: 10px;
  border-radius: 8px;
  background: var(--vp-surface);
  box-shadow: inset 0 0 0 1px var(--vp-border);
}

.midishow-account div {
  min-width: 0;
  max-width: 100%;
  display: grid;
  gap: 3px;
}

.midishow-account strong,
.midishow-account span {
  min-width: 0;
  max-width: 100%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.account-btn {
  min-width: 78px;
  padding: 0 12px;
}

.account-btn.ghost {
  color: var(--vp-text);
  background: var(--vp-panel);
  box-shadow: inset 0 0 0 1px var(--vp-border);
}

.login-form {
  padding: 10px;
  border-radius: 8px;
  background: var(--vp-surface);
  box-shadow: inset 0 0 0 1px var(--vp-border);
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  align-items: center;
  gap: 8px;
}

.login-field {
  min-width: 0;
  display: grid;
  gap: 5px;
}

.login-field span {
  color: var(--vp-muted);
  font-size: 12px;
  font-weight: 750;
}

.login-form input {
  min-width: 0;
  width: 100%;
  border: 1px solid var(--vp-border);
  border-radius: 8px;
  padding: 8px 10px;
  color: var(--vp-text);
  background: color-mix(in srgb, var(--vp-panel) 76%, transparent);
  outline: none;
}

.login-hint,
.login-status {
  grid-column: 1 / -1;
  margin: 0;
  color: var(--vp-dim);
  font-size: 12px;
  line-height: 1.5;
}

.login-status {
  display: inline-flex;
  align-items: center;
  gap: 7px;
  color: var(--vp-primary);
  font-weight: 750;
}

.login-error {
  grid-column: 1 / -1;
  margin: 0;
  display: flex;
  align-items: center;
  gap: 6px;
  color: #ef4444;
  font-size: 12px;
  font-weight: 750;
  line-height: 1.5;
}

.login-form button {
  min-width: 92px;
  padding: 0 12px;
}

.login-form-actions {
  grid-column: 1 / -1;
  min-width: 0;
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 6px;
  align-items: stretch;
}

.login-form-actions button {
  min-width: 0;
  width: 100%;
  overflow: hidden;
  text-overflow: ellipsis;
}

.online-head strong {
  font-size: 15px;
}

.online-form {
  display: grid;
  gap: 8px;
}

.online-search-actions {
  display: flex;
  gap: 6px;
}

.online-search-actions button {
  min-width: 32px;
}

.online-search-actions button:first-child {
  min-width: 68px;
}

.input-row {
  min-width: 0;
  display: grid;
  grid-template-columns: auto minmax(0, 1fr) auto;
  align-items: center;
  gap: 8px;
  padding: 8px;
  border-radius: 8px;
  background: var(--vp-surface);
  box-shadow: inset 0 0 0 1px var(--vp-border);
}

.input-row:has(.name-input) {
  grid-template-columns: auto minmax(0, 1fr) minmax(96px, 22%) auto;
}

.input-row.download-row {
  grid-template-columns: auto minmax(0, 1.35fr) minmax(132px, 0.65fr) auto;
}

.input-row input {
  min-width: 0;
  border: 0;
  color: var(--vp-text);
  background: transparent;
  outline: none;
}

.online-form button {
  min-width: 68px;
  min-height: 32px;
  color: white;
  background: var(--vp-primary);
}

.online-form button:disabled {
  color: var(--vp-text);
  background: var(--vp-panel);
  box-shadow: inset 0 0 0 1px var(--vp-border);
  opacity: 1;
}

.online-form button:disabled svg {
  color: var(--vp-text);
}

.online-results {
  max-height: 260px;
  min-height: 120px;
  overflow-y: auto;
  display: grid;
  align-content: start;
  gap: 8px;
}

.online-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  padding: 10px;
  border-radius: 8px;
  background: color-mix(in srgb, var(--vp-surface) 82%, transparent);
}

.online-meta {
  min-width: 0;
  display: grid;
  gap: 4px;
}

.online-actions {
  display: flex;
  gap: 6px;
  flex-shrink: 0;
}

.log-pane {
  min-height: 110px;
  overflow-y: auto;
  padding: 10px 12px;
  border-radius: 8px;
  background: var(--theme-terminal-bg, rgba(10, 10, 10, 0.92));
  font-family: Consolas, "SF Mono", monospace;
  font-size: 12px;
}

.log-line {
  padding: 3px 0;
  color: rgba(255, 255, 255, 0.72);
  word-break: break-all;
}

.edit-backdrop {
  position: fixed;
  inset: 0;
  z-index: 40;
  display: grid;
  place-items: center;
  padding: 24px;
  background: rgba(36, 22, 8, 0.26);
  backdrop-filter: blur(8px);
}

.edit-dialog {
  width: min(460px, 100%);
  border: 1px solid var(--vp-border-strong);
  border-radius: 8px;
  background: var(--vp-surface);
  box-shadow: 0 24px 70px rgba(74, 45, 15, 0.22);
  overflow: hidden;
}

.edit-dialog-head {
  min-height: 62px;
  padding: 14px 16px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  background: var(--vp-panel);
  box-shadow: inset 0 -1px 0 var(--vp-border);
}

.edit-dialog-head div,
.edit-dialog-body {
  min-width: 0;
  display: grid;
  gap: 10px;
}

.edit-dialog-head strong {
  font-size: 16px;
}

.edit-dialog-head span,
.theme-field span,
.icon-preview small {
  color: var(--vp-muted);
  font-size: 12px;
  font-weight: 700;
}

.dialog-close {
  width: 32px;
  height: 32px;
  border: 0;
  border-radius: 8px;
  color: var(--vp-muted);
  background: var(--vp-surface);
  box-shadow: inset 0 0 0 1px var(--vp-border);
  cursor: pointer;
  font-size: 20px;
  line-height: 1;
}

.dialog-close:hover:not(:disabled) {
  color: var(--vp-text);
  background: var(--vp-hover);
}

.edit-dialog-body {
  padding: 16px;
}

.theme-field {
  min-width: 0;
  display: grid;
  gap: 6px;
}

.theme-field input {
  width: 100%;
  min-width: 0;
  border: 1px solid var(--vp-border);
  border-radius: 8px;
  padding: 10px 12px;
  color: var(--vp-text);
  background: var(--vp-panel);
  outline: none;
}

.theme-field input:focus,
.login-form input:focus,
.login-form textarea:focus,
.input-row:focus-within,
.control-grid input[type="number"]:focus {
  border-color: var(--vp-border-strong);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--vp-primary) 16%, transparent);
}

.icon-preview {
  min-height: 54px;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px;
  border-radius: 8px;
  background: var(--vp-panel);
  box-shadow: inset 0 0 0 1px var(--vp-border);
}

.icon-preview .song-note {
  width: 36px;
  height: 36px;
}

.edit-dialog-actions {
  padding: 12px 16px 16px;
  display: flex;
  justify-content: flex-end;
  gap: 10px;
}

.dialog-secondary,
.dialog-primary {
  min-height: 36px;
  border: 0;
  border-radius: 8px;
  padding: 0 14px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  font-weight: 850;
  cursor: pointer;
}

.dialog-secondary {
  color: var(--vp-muted);
  background: var(--vp-panel);
  box-shadow: inset 0 0 0 1px var(--vp-border);
}

.dialog-primary {
  min-width: 76px;
  color: white;
  background: var(--vp-primary);
}

.dialog-secondary:hover:not(:disabled) {
  color: var(--vp-text);
  background: var(--vp-hover);
}

.dialog-primary:hover:not(:disabled) {
  background: var(--vp-primary-hover);
}

.dialog-close:disabled,
.dialog-secondary:disabled,
.dialog-primary:disabled {
  cursor: not-allowed;
  opacity: 0.58;
}

.spin {
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

::-webkit-scrollbar {
  width: 7px;
}

::-webkit-scrollbar-thumb {
  border-radius: 999px;
  background: var(--vp-border-strong);
}

@media (max-width: 980px) {
  .quick-stats,
  .vrpiano-main,
  .control-grid {
    grid-template-columns: 1fr;
  }

  .quick-stats div + div {
    box-shadow: inset 0 1px 0 var(--vp-border);
  }

  .library-pane {
    min-height: 260px;
  }

  .hotkey-panel,
  .midishow-account,
  .login-form,
  .player-head,
  .online-row,
  .action-row {
    align-items: stretch;
    flex-direction: column;
  }

  .login-form {
    grid-template-columns: 1fr;
  }

  .login-form-actions {
    grid-template-columns: repeat(3, minmax(0, 1fr));
  }

  .danger-action,
  .toggle-btn {
    width: 100%;
  }

  .input-row,
  .input-row:has(.name-input),
  .input-row.download-row {
    grid-template-columns: auto minmax(0, 1fr);
  }

  .input-row button,
  .name-input {
    grid-column: 1 / -1;
  }

}

@media (max-width: 620px) {
  .login-form-actions {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .login-form-actions button:first-child {
    grid-column: 1 / -1;
  }
}
</style>
