<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue';
import { useStorage } from '@vueuse/core';
import { useI18n } from 'vue-i18n';
import { emit, listen } from '@tauri-apps/api/event';
import { isTauri } from '@tauri-apps/api/core';
import { songIcon, songCover, isImageIcon, useVrpianoIcons } from '../composables/useVrpianoIcons';
import { WebviewWindow } from '@tauri-apps/api/webviewWindow';
import { Effect, EffectState } from '@tauri-apps/api/window';
import { open } from '@tauri-apps/plugin-dialog';
import {
  AlertTriangle,
  CircleStop,
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
  Trash2,
  Upload,
  Volume2,
  X,
  Mic,
  Square,
  Radio,
  Sliders,
  Disc3,
  SendHorizontal,
  Keyboard,
  Cable,
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
import { VRPIANO_PREVIEW_SONG_EVENT, type VrpianoPreviewSongPayload } from './vrpianoEvents';

const { locale, t } = useI18n();
const l = (zh: string, en: string) => locale.value.startsWith('zh') ? zh : en;
const instrumentName = (program: number) => locale.value.startsWith('zh')
  ? getGeneralMidiInstrumentName(program)
  : t('vrpiano.general_midi_program', { program: program + 1 });
const instrumentGroupName = (groupIndex: number, zhName: string) => locale.value.startsWith('zh')
  ? zhName
  : t('vrpiano.programs_range', { start: groupIndex * 8 + 1, end: groupIndex * 8 + 8 });

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
  last_event:t('vrpiano.initializing_vrpiano'),
  last_error: '',
  songs_dir: '',
  speed: 1,
  hotkeys_enabled: false,
  hotkeys_available: true,
  last_hotkey: '',
  last_hotkey_at_ms: 0,
  midi_connected: false,
  midi_device_name: null,
  recording: false,
  recorded_midi_path: null,
  channels: Array.from({ length: 16 }, () => ({ muted: false, solo: false, volume: 127 })),
  voice_listening: false,
  tts_enabled: false,
  last_transcription: '',
  vrchat_osc_enabled: false,
  vrchat_osc_host: '',
  vrchat_osc_port: 9000,
  vrchat_osc_running: false,
  vrchat_osc_last_error: '',
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
const vrchatOscHost = ref('127.0.0.1');
const vrchatOscPort = ref(9000);
const vrchatOscMode = useStorage<'piano' | 'avatar'>('vrcdog.vrpiano.oscMode.v1', 'piano');
const vrchatOscAvatarPrefix = useStorage('vrcdog.vrpiano.oscAvatarPrefix.v1', '/avatar/parameters/note');
const vrchatOscEnabled = ref(false);
const onlineKeyword = ref('');
const urlInput = ref('');
const urlFilename = ref('');
// 日志缓存上限：控制演奏日志内存占用，超出后自动丢弃最旧条目（内存回收）。
const MAX_PIANO_LOG_ENTRIES = 100;
const logs = ref<string[]>([]);
const midishowAccounts = ref<VrpianoMidishowAccount[]>([]);
const midishowAccount = ref('');
const midishowPassword = ref('');
const midishowLoginOpen = ref(false);
const midishowLoginStatus = ref<VrpianoMidishowLoginStatus>({
  state: 'idle',
  message:t('vrpiano.waiting_to_sign_in'),
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
const { songIcons } = useVrpianoIcons();
const iconFileInput = ref<HTMLInputElement | null>(null);
const iconTargetPath = ref('');
const editDialogMode = ref<'icon' | 'rename' | null>(null);
const editIconText = ref('');
const editIconUrl = ref('');
const editSongName = ref('');
const playerTitle = ref(t('vrpiano.no_song_loaded'));
const playerPositionMs = ref(0);
const playerDurationMs = ref(0);
const playerVolume = ref(0.9);
const playerPlaying = ref(false);
const playerLoading = ref(false);
const parsedPlayerNotes = ref<MidiNote[]>([]);
const playerInstrument = ref('source');
const sourcePrograms = ref<number[]>([]);
const sourceHasPercussion = ref(false);
const sourceHasSustainPedal = ref(false);
const overlayOpen = ref(false);
const recording = ref(false);
const recordedMidiPath = ref<string | null>(null);
const channelStates = ref<Array<{ muted: boolean; solo: boolean; volume: number }>>(
  Array.from({ length: 16 }, () => ({ muted: false, solo: false, volume: 127 })),
);
const voiceControlEnabled = ref(false);
const ttsEnabled = ref(false);
const transpose = ref(0);
// The Rust backend hard-codes drum-channel (channel index 9) exclusion inside
// `apply_transpose`, so this checkbox is informational and always reflects that
// percussion is never transposed. Kept checked + disabled to surface the behaviour.
const excludeDrums = ref(true);
const playMode = ref('sequential');
const playlist = ref<string[]>([]);
const midiDevices = ref<Array<{ id: string; name: string; kind: string }>>([]);
const selectedMidiDevice = ref('');
const midiOutputState = ref<{ connected: boolean; device_id?: string; device_name?: string }>({ connected: false });
const outputMode = useStorage<'keyboard' | 'midi' | 'osc'>('vrcdog.vrpiano.outputMode.v1', 'keyboard');
const channelRouted = ref<boolean[]>(Array.from({ length: 16 }, () => true));

const playModeOptions = [
  { value: 'sequential', label: () => t('vrpiano.play_mode_sequential') },
  { value: 'random', label: () => t('vrpiano.play_mode_shuffle') },
  { value: 'one', label: () => t('vrpiano.play_mode_repeat_one') },
  { value: 'repeat_all', label: () => t('vrpiano.play_mode_repeat_playlist') },
  { value: 'stop_at_song_end', label: () => t('vrpiano.play_mode_stop_after_current') },
  { value: 'stop_at_end', label: () => t('vrpiano.play_mode_stop_after_playlist') },
];

const playlistSongs = computed(() => playlist.value.map((path) => songs.value.find((song) => song.path === path)).filter(Boolean) as VrpianoSong[]);

const formatVrpianoError = (e: unknown) => {
  const message = e instanceof Error ? e.message : String(e);
  if (/403|cloudflare|challenge|cookie|cf_chl|javascript/i.test(message)) {
    return t('vrpiano.this_action_could_not_be_completed_sign_');
  }
  if (/invalid|expired|会话已失效/i.test(message)) {
    return t('vrpiano.your_session_expired_sign_in_again');
  }
  return message ||t('vrpiano.the_action_could_not_be_completed_try_ag');
};

let unlistenStatus: (() => void) | null = null;
let unlistenOverlayClosed: (() => void) | null = null;
let unlistenMidishowLogin: (() => void) | null = null;
let unlistenPreviewSong: (() => void) | null = null;
let unlistenVrAction: (() => void) | null = null;
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
const canTogglePlayback = computed(() => Boolean(selectedSong.value) && !loading.value);
const isPlaying = computed(() => Boolean(status.value.running));
const stopAll = async () => {
  if (!status.value.running || loading.value) return;
  loading.value = true;
  error.value = '';
  try {
    await VrpianoApi.stop();
    await waitUntilPlaybackStops();
    status.value = await VrpianoApi.getStatus();
    addLog(t('vrpiano.playback_stopped'));
  } catch (e: any) {
    error.value = e.message || String(e);
  } finally {
    loading.value = false;
  }
};
const hasStartedPlayback = computed(() => Boolean(status.value.song_path));
const playbackActionLabel = computed(() => {
  if (status.value.paused) return t('vrpiano.resume');
  if (status.value.running) return t('vrpiano.pause');
  return t('vrpiano.start');
});
const speedText = computed(() => `${clampSpeed(speed.value).toFixed(2)}x`);
const defaultMidishowAccount = computed(() => midishowAccounts.value[0] || null);
const defaultMidishowLoginTypeText = computed(() => (
  defaultMidishowAccount.value?.login_type ?t('vrpiano.signed_in_session') : ''
));
const canTogglePlayer = computed(() => Boolean(parsedPlayerNotes.value.length || selectedSong.value) && !playerLoading.value);
const onlineEmptyText = computed(() => {
  // 搜索进行中：显示"正在搜索"，而不是"未找到"——避免请求未完成时就
  // 误把"未找到 XX 相关结果"展示给用户，让用户以为真的搜不到。
  if (onlineLoading.value) return t('vrpiano.searching');
  if (!hasSearchedOnline.value) return t('vrpiano.search_by_keyword_or_paste_a_url_or_id_t');
  return t('vrpiano.no_results_for_keyword', { keyword: lastOnlineKeyword.value });
});
const playerProgressPercent = computed(() => {
  if (!playerDurationMs.value) return 0;
  return Math.round(Math.min(1, playerPositionMs.value / playerDurationMs.value) * 100);
});
const sourceInstrumentText = computed(() => {
  const names: string[] = sourcePrograms.value.map(instrumentName);
  if (sourceHasPercussion.value) names.push(t('vrpiano.standard_drum_kit'));
  return names.length ? names.join(locale.value.startsWith('zh') ? '、' : ', ') : instrumentName(0);
});
const activeInstrumentText = computed(() => (
  playerInstrument.value === 'source'
      ? t('vrpiano.active_instrument_follow', { instrument: sourceInstrumentText.value })
      : t('vrpiano.active_instrument_manual', { instrument: locale.value.startsWith('zh')
        ? getGeneralMidiInstrumentName(Number(playerInstrument.value))
        : instrumentName(Number(playerInstrument.value)) })
));
const hotkeyStatusText = computed(() => {
  if (!status.value.hotkeys_available) return t('vrpiano.not_supported_on_this_system');
  return hotkeysEnabled.value ?t('vrpiano.global_shortcuts_enabled') :t('vrpiano.global_shortcuts_disabled');
});

const addLog = (message: string) => {
  const time = new Date().toLocaleTimeString(locale.value.startsWith('zh') ? 'zh-CN' : 'en-US', { hour12: false });
  logs.value = [`${time} ${message}`, ...logs.value].slice(0, MAX_PIANO_LOG_ENTRIES);
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
    addLog(t('vrpiano.desktop_overlay_is_unavailable_in_browse'));
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
    title:t('vrpiano.vrpiano_overlay_controller'),
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
    ...(backdropEnabled ? {
      windowEffects: {
        effects: [Effect.Acrylic],
        state: EffectState.Active,
      },
    } : {}),
    ...(Number.isFinite(savedPosition.x) ? { x: savedPosition.x } : {}),
    ...(Number.isFinite(savedPosition.y) ? { y: savedPosition.y } : {}),
  });

  overlay.once('tauri://created', () => {
    overlayOpen.value = true;
    addLog(t('vrpiano.vrpiano_overlay_opened'));
  });
  overlay.once('tauri://error', (event) => {
    overlayOpen.value = false;
    error.value = t('vrpiano.overlay_creation_failed', { error: JSON.stringify(event) });
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
    sourceHasSustainPedal.value = parsed.hasSustainPedal;
    playerTitle.value = midi.name;
    playerPositionMs.value = 0;
    playerDurationMs.value = Math.ceil(Math.max(...parsed.notes.map((note) => note.timeMs + note.durationMs)));
    await schedulePlayer(0);
    const pedalHint = parsed.hasSustainPedal ? ` · ${l('延音踏板', 'Sustain pedal')}` : '';
    addLog(`${t('vrpiano.builtin_player_preview_started', { name: midi.name, instrument: activeInstrumentText.value })}${pedalHint}`);
  } catch (e: any) {
    error.value = e.message || String(e);
    addLog(t('vrpiano.player_load_failed', { error: error.value }));
  } finally {
    playerLoading.value = false;
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
      error.value =t('vrpiano.icon_url_must_start_with_http_https_or_d');
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
    error.value =t('vrpiano.choose_an_image_file_for_the_song_icon');
    return;
  }
  const reader = new FileReader();
  reader.onload = () => {
    songIcons.value = { ...songIcons.value, [iconTargetPath.value]: String(reader.result || '') };
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
    addLog(t('vrpiano.midishow_registration_link_copied'));
  } catch (e: any) {
    error.value = t('vrpiano.copy_registration_link_failed', { error: e.message || String(e), url: signupUrl });
  }
};

const openMidishowSignup = async () => {
  if (externalLinkLoading.value) return;
  externalLinkLoading.value = true;
  error.value = '';
  try {
    await SysApi.openUrl({ url: signupUrl });
    addLog(t('vrpiano.opened_the_midishow_registration_page_in'));
  } catch (e: any) {
    error.value = t('vrpiano.cannot_open_default_browser', { error: e.message || String(e) });
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
    addLog(t('vrpiano.opened_official_midishow_search_in_the_b'));
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
      message:t('vrpiano.complete_the_verification_in_the_popup_l'),
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
    addLog(next.username ? t('vrpiano.signed_in_to_midishow_as', { username: next.username }) :t('vrpiano.signed_in_to_midishow'));
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
    loginError.value =t('vrpiano.enter_your_midishow_account_and_password');
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
      addLog(t('vrpiano.signing_in_to_midishow_automatically'));
    }
  } catch (e) {
    midishowPassword.value = '';
    accountLoading.value = false;
    loginError.value = formatVrpianoError(e);
    addLog(t('vrpiano.midishow_signin_not_completed', { error: loginError.value }));
  }
};

const logoutMidishow = async () => {
  if (!defaultMidishowAccount.value) return;
  accountLoading.value = true;
  error.value = '';
  try {
    midishowAccounts.value = await VrpianoApi.midishowRemoveAccount({ username: defaultMidishowAccount.value.username });
    addLog(t('vrpiano.signed_out_of_midishow'));
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
    addLog(t('vrpiano.library_refreshed', { count: songs.value.length, unit: songs.value.length === 1 ? 'song' : 'songs' }));
  } catch (e: any) {
    error.value = e.message || String(e);
    addLog(t('vrpiano.could_not_refresh_library', { error: error.value }));
  } finally {
    loading.value = false;
  }
};

const init = async () => {
  loading.value = true;
  error.value = '';
  try {
    status.value = await VrpianoApi.init();
    hotkeysEnabled.value = Boolean(status.value.hotkeys_enabled);
    await Promise.all([refreshSongs(), loadMidishowAccounts(), refreshMidiDevices()]);
    addLog(t('vrpiano.vrpiano_is_ready'));
  } catch (e: any) {
    error.value = e.message || String(e);
    addLog(t('vrpiano.initialization_failed', { error: error.value }));
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
    addLog(t('vrpiano.imported_song', { name: song.name }));
  } catch (e: any) {
    error.value = e.message || String(e);
    addLog(t('vrpiano.import_failed', { error: error.value }));
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
    }
    await refreshSongs();
    selectedPath.value = renamed.path;
    addLog(t('vrpiano.renamed_to', { name: renamed.name }));
    closeEditDialog();
  } catch (e: any) {
    error.value = e.message || String(e);
    addLog(t('vrpiano.rename_failed', { error: error.value }));
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
  if (!window.confirm(t('vrpiano.confirm_delete_song', { name: selectedSong.value.name }))) return;
  loading.value = true;
  error.value = '';
  try {
    await VrpianoApi.deleteSong({ songPath: selectedSong.value.path });
    if (songIcons.value[selectedSong.value.path]) {
      const nextIcons = { ...songIcons.value };
      delete nextIcons[selectedSong.value.path];
      songIcons.value = nextIcons;
    }
    addLog(t('vrpiano.deleted_song', { name: selectedSong.value.name }));
    selectedPath.value = '';
    await refreshSongs();
  } catch (e: any) {
    error.value = e.message || String(e);
    addLog(t('vrpiano.delete_failed', { error: error.value }));
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

const playRelativeSongInBuiltInPlayer = async (delta: -1 | 1) => {
  if (!songs.value.length || playerLoading.value) return;
  const currentIndex = Math.max(0, songs.value.findIndex((song) => song.path === selectedPath.value));
  const nextIndex = (currentIndex + delta + songs.value.length) % songs.value.length;
  await previewSong(songs.value[nextIndex]);
};

const restartBuiltInPlayer = async () => {
  if (parsedPlayerNotes.value.length) {
    await schedulePlayer(0);
  } else if (selectedSong.value) {
    await previewLocalSong();
  }
};

const handleVrPianoAction = async (action: string) => {
  if (action === 'previous') await playRelativeSongInBuiltInPlayer(-1);
  else if (action === 'next') await playRelativeSongInBuiltInPlayer(1);
  else if (action === 'restart') await restartBuiltInPlayer();
  else if (action === 'toggle') await togglePlayer();
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
    addLog(t('vrpiano.url_id_download_completed', { name: song.name }));
  } catch (e: any) {
    error.value = formatVrpianoError(e);
    addLog(t('vrpiano.url_id_download_failed', { error: error.value }));
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
    addLog(t('vrpiano.online_search_failed', { error: error.value }));
  }, ONLINE_SEARCH_TIMEOUT_MS);
  try {
    const results = await VrpianoApi.searchMidishow({
      keyword,
      maxResults: 40,
    });
    if (requestId !== onlineSearchRequestId) return;
    onlineResults.value = results;
    addLog(t('vrpiano.midishow_returned_results', { count: results.length }));
  } catch (e: any) {
    if (requestId !== onlineSearchRequestId) return;
    error.value = formatVrpianoError(e);
    addLog(t('vrpiano.online_search_failed', { error: error.value }));
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
    addLog(t('vrpiano.preview_failed', { error: error.value }));
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
      addLog(t('vrpiano.midishow_download_completed', { name: downloaded.name }));
    }
  } catch (e: any) {
    error.value = formatVrpianoError(e);
    addLog(t('vrpiano.download_failed', { error: error.value }));
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
    if (announce) addLog(t('vrpiano.playback_speed', { speed: nextSpeed.toFixed(2) }));
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
      outputMode: outputMode.value,
      midiDeviceId: outputMode.value === 'midi' ? selectedMidiDevice.value : undefined,
      oscHost: vrchatOscHost.value,
      oscPort: vrchatOscPort.value,
    });
    hotkeysEnabled.value = Boolean(status.value.hotkeys_enabled);
    if (Number.isFinite(status.value.speed)) speed.value = status.value.speed;
    if (announce) addLog(hotkeysEnabled.value
      ?t('vrpiano.global_shortcuts_enabled_and_available_i')
      :t('vrpiano.global_shortcuts_disabled'));
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
  if (outputMode.value === 'osc') {
    await startVrchatOsc();
    return;
  }
  if (outputMode.value === 'midi') {
    await startDirectMidi();
    return;
  }
  loading.value = true;
  error.value = '';
  try {
    status.value = await VrpianoApi.start({
      songPath: selectedSong.value.path,
      delaySecs: Math.max(0, Math.round(delaySecs.value || 0)),
      speed: clampSpeed(speed.value),
      outputMode: 'keyboard',
    });
    addLog(t('vrpiano.preparing_to_play', { name: selectedSong.value.name }));
  } catch (e: any) {
    error.value = e.message || String(e);
    addLog(t('vrpiano.start_failed', { error: error.value }));
  } finally {
    loading.value = false;
  }
};

const startVrchatOsc = async () => {
  if (!selectedSong.value) return;
  loading.value = true;
  error.value = '';
  try {
    if (status.value.running) {
      await VrpianoApi.stop();
      await waitUntilPlaybackStops();
    }
    status.value = await VrpianoApi.startVrchatOsc({
      songPath: selectedSong.value.path,
      delaySecs: Math.max(0, Math.round(delaySecs.value || 0)),
      speed: clampSpeed(speed.value),
      host: vrchatOscHost.value,
      port: vrchatOscPort.value,
      mode: vrchatOscMode.value,
      avatarPrefix: vrchatOscAvatarPrefix.value,
    });
    addLog(t('vrpiano.preparing_to_play_vrchat_osc', { name: selectedSong.value.name }));
  } catch (e: any) {
    error.value = e.message || String(e);
    addLog(t('vrpiano.vrchat_osc_start_failed', { error: error.value }));
  } finally {
    loading.value = false;
  }
};

const testOscNote = async () => {
  loading.value = true;
  error.value = '';
  try {
    await VrpianoApi.testOscNote({
      host: vrchatOscHost.value,
      port: vrchatOscPort.value,
      mode: vrchatOscMode.value,
      avatarPrefix: vrchatOscAvatarPrefix.value,
      note: 60,
    });
    addLog(t('vrpiano.osc_test_sent'));
  } catch (e: any) {
    error.value = e.message || String(e);
    addLog(t('vrpiano.osc_test_failed', { error: error.value }));
  } finally {
    loading.value = false;
  }
};

const startDirectMidi = async () => {
  if (!selectedSong.value || !selectedMidiDevice.value) {
    error.value = t('vrpiano.select_midi_device_for_direct_playback');
    return;
  }
  if (!midiOutputState.value.connected || midiOutputState.value.device_id !== selectedMidiDevice.value) {
    error.value = t('vrpiano.connect_midi_device_before_direct_playback');
    return;
  }
  loading.value = true;
  error.value = '';
  try {
    status.value = await VrpianoApi.start({
      songPath: selectedSong.value.path,
      delaySecs: Math.max(0, Math.round(delaySecs.value || 0)),
      speed: clampSpeed(speed.value),
      outputMode: 'midi',
      midiDeviceId: selectedMidiDevice.value,
    });
    addLog(t('vrpiano.preparing_to_play_direct_midi', { name: selectedSong.value.name }));
  } catch (e: any) {
    error.value = e.message || String(e);
    addLog(t('vrpiano.start_failed', { error: error.value }));
  } finally {
    loading.value = false;
  }
};

const togglePlayback = async () => {
  if (!status.value.running) {
    await start();
    return;
  }

  loading.value = true;
  error.value = '';
  try {
    const wasPaused = status.value.paused;
    status.value = await VrpianoApi.togglePause();
    addLog(wasPaused ?t('vrpiano.playback_resumed') :t('vrpiano.playback_paused'));
  } catch (e: any) {
    error.value = e.message || String(e);
  } finally {
    loading.value = false;
  }
};

const waitUntilPlaybackStops = async () => {
  for (let attempt = 0; attempt < 30; attempt += 1) {
    const next = await VrpianoApi.getStatus();
    status.value = next;
    if (!next.running) return;
    await new Promise((resolve) => window.setTimeout(resolve, 40));
  }
  throw new Error(t('vrpiano.timed_out_waiting_for_the_current_song_t'));
};

const restartPlayback = async () => {
  if (!selectedSong.value || !hasStartedPlayback.value || loading.value) return;
  if (outputMode.value === 'osc') {
    await startVrchatOsc();
    return;
  }
  loading.value = true;
  error.value = '';
  try {
    if (status.value.running) {
      await VrpianoApi.stop();
      await waitUntilPlaybackStops();
    }
    status.value = await VrpianoApi.start({
      songPath: selectedSong.value.path,
      delaySecs: Math.max(0, Math.round(delaySecs.value || 0)),
      speed: clampSpeed(speed.value),
      outputMode: outputMode.value === 'midi' ? 'midi' : 'keyboard',
      midiDeviceId: outputMode.value === 'midi' ? selectedMidiDevice.value : undefined,
    });
    addLog(t('vrpiano.restarted_song', { name: selectedSong.value.name }));
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
    recording.value = Boolean(status.value.recording);
    recordedMidiPath.value = status.value.recorded_midi_path ?? null;
    voiceControlEnabled.value = Boolean(status.value.voice_listening);
    ttsEnabled.value = Boolean(status.value.tts_enabled);
    if (status.value.channels && status.value.channels.length === 16) {
      channelStates.value = status.value.channels.map((ch: any) => ({
        muted: ch.muted ?? false,
        solo: ch.solo ?? false,
        volume: ch.volume ?? 127,
      }));
    }
    if (status.value.song_path && songs.value.some((song) => song.path === status.value.song_path)) {
      selectedPath.value = status.value.song_path;
    }
  } catch {
    // Backend may be unavailable in browser preview.
  }
};

const startRecording = async () => {
  if (!isTauri()) {
    addLog(t('vrpiano.recording_desktop_only'));
    return;
  }
  try {
    const path = await VrpianoApi.startRecording();
    recording.value = true;
    recordedMidiPath.value = path;
    addLog(t('vrpiano.recording_started'));
  } catch (e: any) {
    error.value = e.message || String(e);
  }
};

const stopRecording = async () => {
  if (!isTauri()) return;
  try {
    const path = await VrpianoApi.stopRecording();
    recording.value = false;
    if (path) {
      recordedMidiPath.value = path;
      addLog(t('vrpiano.recording_saved', { path }));
    }
  } catch (e: any) {
    error.value = e.message || String(e);
  }
};

const toggleVoiceControl = async () => {
  if (!isTauri()) {
    addLog(t('vrpiano.voice_desktop_only'));
    return;
  }
  try {
    voiceControlEnabled.value = !voiceControlEnabled.value;
    await VrpianoApi.setVoiceControlEnabled({ enabled: voiceControlEnabled.value });
    if (voiceControlEnabled.value) {
      startVoiceRecognition();
      addLog(t('vrpiano.voice_enabled_hint'));
    } else {
      stopVoiceRecognition();
      addLog(t('vrpiano.voice_disabled'));
    }
  } catch (e: any) {
    error.value = e.message || String(e);
    voiceControlEnabled.value = !voiceControlEnabled.value;
  }
};

let recognition: any = null;

const startVoiceRecognition = () => {
  if (!('webkitSpeechRecognition' in window || 'SpeechRecognition' in window)) {
    addLog(t('vrpiano.voice_not_supported'));
    return;
  }
  const SpeechRecognition = (window as any).SpeechRecognition || (window as any).webkitSpeechRecognition;
  recognition = new SpeechRecognition();
  recognition.continuous = true;
  recognition.interimResults = false;
  recognition.lang = 'zh-CN';

  recognition.onresult = (event: any) => {
    const transcript = event.results[event.results.length - 1][0].transcript.toLowerCase();
    addLog(t('vrpiano.voice_recognized', { text: transcript }));
    processVoiceCommand(transcript);
  };

  recognition.onerror = (event: any) => {
    addLog(t('vrpiano.voice_error', { error: event.error }));
    if (voiceControlEnabled.value && event.error !== 'no-speech') {
      setTimeout(() => {
        if (voiceControlEnabled.value) {
          try { recognition.start(); } catch {}
        }
      }, 1000);
    }
  };

  recognition.onend = () => {
    if (voiceControlEnabled.value) {
      try { recognition.start(); } catch {}
    }
  };

  try {
    recognition.start();
  } catch (e) {
    addLog(t('vrpiano.voice_start_failed', { error: String(e) }));
  }
};

const stopVoiceRecognition = () => {
  if (recognition) {
    try { recognition.stop(); } catch {}
    recognition = null;
  }
};

const processVoiceCommand = async (transcript: string) => {
  const cmd = transcript.trim();
  if (cmd.includes('播放') || cmd.includes('play') || cmd.includes('开始')) {
    if (!status.value.running) start();
    else togglePlayback();
  } else if (cmd.includes('暂停') || cmd.includes('pause') || cmd.includes('继续')) {
    togglePlayback();
  } else if (cmd.includes('停止') || cmd.includes('stop')) {
    if (status.value.running) {
      await VrpianoApi.stop();
      addLog(t('vrpiano.playback_stopped'));
    }
  } else if (cmd.includes('快') || cmd.includes('faster') || cmd.includes('加速')) {
    adjustSpeed(0.1);
  } else if (cmd.includes('慢') || cmd.includes('slower') || cmd.includes('减速')) {
    adjustSpeed(-0.1);
  } else if (cmd.includes('默认') || cmd.includes('reset') || cmd.includes('恢复')) {
    resetSpeed();
  } else if (cmd.includes('重新') || cmd.includes('restart') || cmd.includes('重来')) {
    restartPlayback();
  }
};

const toggleTts = async () => {
  if (!isTauri()) {
    addLog(t('vrpiano.tts_desktop_only'));
    return;
  }
  try {
    ttsEnabled.value = !ttsEnabled.value;
    await VrpianoApi.setTtsEnabled({ enabled: ttsEnabled.value });
    addLog(ttsEnabled.value ? t('vrpiano.tts_enabled') : t('vrpiano.tts_disabled'));
  } catch (e: any) {
    error.value = e.message || String(e);
    ttsEnabled.value = !ttsEnabled.value;
  }
};

const speakText = async (text: string) => {
  if (!isTauri() || !ttsEnabled.value) return;
  try {
    const result = await VrpianoApi.synthesizeSpeech({
      text,
      voice: 'zh-CN-XiaoxiaoNeural',
      rate: 1.0,
      volume: 0.9,
    });
    addLog(t('vrpiano.tts_speaking', { text }));
  } catch (e: any) {
    error.value = e.message || String(e);
  }
};

const setChannelMute = async (channel: number, muted: boolean) => {
  if (!isTauri()) return;
  try {
    await VrpianoApi.setChannelMute({ channel, muted });
    channelStates.value[channel].muted = muted;
  } catch (e: any) {
    error.value = e.message || String(e);
  }
};

const setChannelSolo = async (channel: number, solo: boolean) => {
  if (!isTauri()) return;
  try {
    await VrpianoApi.setChannelSolo({ channel, solo });
    channelStates.value[channel].solo = solo;
  } catch (e: any) {
    error.value = e.message || String(e);
  }
};

const setChannelVolume = async (channel: number, volume: number) => {
  if (!isTauri()) return;
  try {
    await VrpianoApi.setChannelVolume({ channel, volume: Math.round(volume) });
    channelStates.value[channel].volume = Math.round(volume);
  } catch (e: any) {
    error.value = e.message || String(e);
  }
};

const clampTranspose = (value: unknown) => {
  const parsed = Number(value);
  if (!Number.isFinite(parsed)) return 0;
  return Math.max(-24, Math.min(24, Math.round(parsed)));
};

const applyTranspose = async () => {
  const next = clampTranspose(transpose.value);
  transpose.value = next;
  if (!isTauri()) return;
  try {
    await VrpianoApi.setTranspose({ transpose: next });
    addLog(t('vrpiano.transpose', { value: next > 0 ? `+${next}` : next }));
  } catch (e: any) {
    error.value = e.message || String(e);
  }
};

const playModeLabelKey = (value: string) => ({
  sequential: 'sequential',
  random: 'shuffle',
  one: 'repeat_one',
  repeat_all: 'repeat_playlist',
  stop_at_song_end: 'stop_after_current',
  stop_at_end: 'stop_after_playlist',
}[value] || 'sequential');

const applyPlayMode = async () => {
  if (!isTauri()) return;
  try {
    await VrpianoApi.setPlayMode({ mode: playMode.value });
    addLog(t('vrpiano.play_mode_changed', { label: t(`vrpiano.play_mode_${playModeLabelKey(playMode.value)}`) }));
  } catch (e: any) {
    error.value = e.message || String(e);
  }
};

const addToPlaylist = () => {
  if (!selectedSong.value) return;
  if (!playlist.value.includes(selectedSong.value.path)) {
    playlist.value = [...playlist.value, selectedSong.value.path];
    addLog(t('vrpiano.added_to_playlist', { name: selectedSong.value.name }));
  }
};

const removeFromPlaylist = (path: string) => {
  playlist.value = playlist.value.filter((item) => item !== path);
};

const movePlaylistItem = (index: number, delta: -1 | 1) => {
  const target = index + delta;
  if (target < 0 || target >= playlist.value.length) return;
  const next = [...playlist.value];
  [next[index], next[target]] = [next[target], next[index]];
  playlist.value = next;
};

const clearPlaylist = () => {
  playlist.value = [];
};

const applyPlaylist = async () => {
  if (!isTauri()) return;
  try {
    const nextStatus = await VrpianoApi.setPlaylist({ songs: playlist.value });
    status.value = nextStatus;
    await VrpianoApi.setPlayMode({ mode: playMode.value });
    if (playlist.value.length) {
      await VrpianoApi.start({
        songPath: playlist.value[0],
        delaySecs: Math.max(0, Math.round(delaySecs.value || 0)),
        speed: clampSpeed(speed.value),
        outputMode: outputMode.value === 'midi' ? 'midi' : 'keyboard',
        midiDeviceId: outputMode.value === 'midi' ? selectedMidiDevice.value : undefined,
      });
      addLog(t('vrpiano.playing_playlist', { count: playlist.value.length, label: t(`vrpiano.play_mode_${playModeLabelKey(playMode.value)}`) }));
    }
  } catch (e: any) {
    error.value = e.message || String(e);
  }
};

const refreshMidiDevices = async () => {
  if (!isTauri()) return;
  try {
    midiDevices.value = await VrpianoApi.listMidiDevices();
    midiOutputState.value = await VrpianoApi.getMidiOutputState();
    if (midiOutputState.value.device_id) selectedMidiDevice.value = midiOutputState.value.device_id;
  } catch (e: any) {
    error.value = e.message || String(e);
  }
};

const connectMidiDevice = async () => {
  if (!isTauri() || !selectedMidiDevice.value) return;
  try {
    midiOutputState.value = await VrpianoApi.connectMidiDevice({ deviceId: selectedMidiDevice.value });
    addLog(t('vrpiano.connected_midi_device', { device: midiOutputState.value.device_name || selectedMidiDevice.value }));
  } catch (e: any) {
    error.value = e.message || String(e);
  }
};

const disconnectMidiDevice = async () => {
  if (!isTauri()) return;
  try {
    midiOutputState.value = await VrpianoApi.disconnectMidiDevice();
    addLog(t('vrpiano.midi_device_disconnected'));
  } catch (e: any) {
    error.value = e.message || String(e);
  }
};

const setChannelRouted = async (channel: number, routed: boolean) => {
  if (!isTauri()) return;
  try {
    await VrpianoApi.setChannelRouted({ channel, routed });
    channelRouted.value[channel] = routed;
  } catch (e: any) {
    error.value = e.message || String(e);
  }
};

// Global hotkeys (F1–F5) are handled by the OS-level keyboard hook installed in
// Rust (vrpiano.rs::start_global_hotkey_hook via SetWindowsHookExW WH_KEYBOARD_LL).
// That hook is system-wide and fires regardless of which window is focused, so a
// JS window keydown listener here is redundant and would double-fire with it.

watch(speed, () => {
  scheduleSpeedApply();
  if (hotkeysEnabled.value) scheduleHotkeyApply();
});

watch([selectedPath, delaySecs], () => {
  if (hotkeysEnabled.value) scheduleHotkeyApply();
});

watch([outputMode, selectedMidiDevice, vrchatOscHost, vrchatOscPort], () => {
  if (hotkeysEnabled.value) scheduleHotkeyApply();
});

// 竞态防护：init() 是网络请求，组件可能在 await 期间被卸载。
// 卸载后不再注册监听器 / 启动轮询，否则定时器和监听器永久泄漏。
let vrpianoDisposed = false;

// 演奏状态事件由 Rust 后端高频推送（自动演奏时尤其频繁）。
// 直接每次都写日志会造成响应式更新风暴、日志列表反复重渲染，UI 越用越卡。
// 这里做去重 + 限流：仅当事件文案变化且距上次记录超过阈值时才写入日志。
let lastStatusLogMsg = '';
let lastStatusLogAt = 0;
const STATUS_LOG_THROTTLE_MS = 1000;

onMounted(async () => {
  vrpianoDisposed = false;
  const savedInstrument = localStorage.getItem(playerInstrumentStorageKey);
  if (savedInstrument === 'source' || (savedInstrument && Number(savedInstrument) >= 0 && Number(savedInstrument) <= 127)) {
    playerInstrument.value = savedInstrument;
  }
  await init();
  if (vrpianoDisposed) return;
  try {
    overlayOpen.value = Boolean(await WebviewWindow.getByLabel('vrpiano-overlay'));
    unlistenOverlayClosed = await listen('vrpiano-overlay-closed', () => {
      overlayOpen.value = false;
    });
    unlistenPreviewSong = await listen<VrpianoPreviewSongPayload>(VRPIANO_PREVIEW_SONG_EVENT, (event) => {
      const song = songs.value.find((item) => item.path === event.payload?.songPath);
      if (song) void previewSong(song);
    });
    unlistenVrAction = await listen<string>('vrpiano_vr_action', (event) => {
      void handleVrPianoAction(event.payload);
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
      const ev = event.payload.last_event;
      if (ev) {
        // 去重 + 限流：仅在事件文案相对上次记录发生变化、且距上次记录超过
        // 阈值时才写入日志，避免自动演奏时高频状态推送引发的响应式风暴与卡顿。
        const now = Date.now();
        if (ev !== lastStatusLogMsg && now - lastStatusLogAt >= STATUS_LOG_THROTTLE_MS) {
          lastStatusLogMsg = ev;
          lastStatusLogAt = now;
          addLog(ev);
        }
      }
      if (event.payload.last_error) error.value = event.payload.last_error;
    });
  } catch {
    // Non-Tauri preview.
  }
  if (vrpianoDisposed) {
    // await listen 期间组件被卸载：注销刚拿到的监听器，不再启动轮询
    if (unlistenStatus) { unlistenStatus(); unlistenStatus = null; }
    if (unlistenOverlayClosed) { unlistenOverlayClosed(); unlistenOverlayClosed = null; }
    if (unlistenMidishowLogin) { unlistenMidishowLogin(); unlistenMidishowLogin = null; }
    if (unlistenPreviewSong) { unlistenPreviewSong(); unlistenPreviewSong = null; }
    if (unlistenVrAction) { unlistenVrAction(); unlistenVrAction = null; }
    return;
  }
  if (pollTimer === null) {
    pollTimer = window.setInterval(refreshStatus, 1500);
  }
  // F1–F5 global hotkeys are owned by the Rust WH_KEYBOARD_LL hook (system-wide,
  // works without VRCDog being focused) — no JS keydown listener needed here.
});

onUnmounted(() => {
  vrpianoDisposed = true;
  if (unlistenStatus) unlistenStatus();
  if (unlistenOverlayClosed) unlistenOverlayClosed();
  if (unlistenMidishowLogin) unlistenMidishowLogin();
  if (unlistenPreviewSong) unlistenPreviewSong();
  if (unlistenVrAction) unlistenVrAction();
  stopMidishowLoginPolling();
  if (pollTimer !== null) window.clearInterval(pollTimer);
  if (speedApplyTimer !== null) window.clearTimeout(speedApplyTimer);
  if (hotkeyApplyTimer !== null) window.clearTimeout(hotkeyApplyTimer);
  if (onlineSearchTimeout !== null) window.clearTimeout(onlineSearchTimeout);
  onlineSearchRequestId += 1;
  pausePlayer();
  void audioContext?.close();
  stopVoiceRecognition();
});
</script>

<template>
  <div class="vrpiano-shell">
    <input ref="iconFileInput" class="sr-only-file" type="file" accept="image/*" @change="handleSongIconFile">
    <header class="vrpiano-header">
      <div class="title-block">
        <div class="title-icon"><Music :size="22" /></div>
        <div>
          <h1>{{t('vrpiano.vrpiano_autoplay') }}</h1>
          <p>{{t('vrpiano.local_library_online_downloads_previews_') }}</p>
        </div>
      </div>
      <div class="header-actions">
        <button class="overlay-toggle" :class="{ active: overlayOpen }" @click="toggleVrpianoOverlay">
          <PictureInPicture2 :size="16" />
          {{ overlayOpen ?t('vrpiano.close_overlay') :t('vrpiano.open_overlay') }}
        </button>
        <div class="status-pill" :class="{ active: status.running && !status.paused }">
          <span class="status-dot" />
          {{ status.paused ?t('vrpiano.paused') : status.running ?t('vrpiano.playing') :t('vrpiano.ready') }}
        </div>
      </div>
    </header>

    <section class="quick-stats">
        <div><Music :size="16" /><span>{{ songs.length }} {{ t('vrpiano.songs_unit', { count: songs.length }) }}</span></div>
      <div><Clock3 :size="16" /><span>{{ formatTime(status.elapsed_ms) }} / {{ formatTime(status.duration_ms) }}</span></div>
      <div><Gauge :size="16" /><span>{{ speedText }} {{t('vrpiano.speed') }}</span></div>
      <div><ShieldCheck :size="16" /><span>{{ hotkeyStatusText }}</span></div>
    </section>

    <div v-if="error || status.last_error" class="error-banner">
      <AlertTriangle :size="16" />
      <span>{{ error || status.last_error }}</span>
    </div>

    <main class="vrpiano-main">
      <section class="library-pane">
        <div class="pane-toolbar">
          <strong>{{t('vrpiano.local_library') }}</strong>
          <div class="library-search">
            <Search :size="15" />
            <input
              v-model="localSongQuery"
              :placeholder="t('vrpiano.search_local_library')"
              @keydown.enter.prevent="selectFirstFilteredSong"
            >
            <button
              v-if="localSongQuery"
              class="clear-search-btn"
              type="button"
              :title="t('vrpiano.clear_search')"
              @click="clearLocalSongQuery"
            >
              <X :size="14" />
            </button>
          </div>
          <div class="tool-buttons">
            <button class="icon-btn" :title="t('vrpiano.import_midi')" :disabled="loading" @click="importMidi"><Upload :size="16" /></button>
            <button class="icon-btn" :title="t('vrpiano.preview_song')" :disabled="!selectedSong" @click="previewLocalSong"><Headphones :size="16" /></button>
            <button class="icon-btn" :title="t('vrpiano.set_text_icon')" :disabled="!selectedSong" @click="setSongEmojiIcon"><Music :size="16" /></button>
            <button class="icon-btn" :title="t('vrpiano.choose_image_icon')" :disabled="!selectedSong" @click="chooseSongImageIcon"><ImagePlus :size="16" /></button>
            <button class="icon-btn" :title="t('vrpiano.rename')" :disabled="!selectedSong || loading" @click="renameSong"><Edit3 :size="16" /></button>
            <button class="icon-btn danger" :title="t('vrpiano.delete')" :disabled="!selectedSong || loading" @click="deleteSong"><Trash2 :size="16" /></button>
            <button class="icon-btn" :title="t('vrpiano.refresh_library')" :disabled="loading" @click="refreshSongs">
              <RefreshCcw :size="16" :class="{ spin: loading }" />
            </button>
            <button class="icon-btn" :title="t('vrpiano.open_library_folder')" @click="openSongsDir"><FolderOpen :size="16" /></button>
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
              <img v-else-if="songCover(song)" :src="songCover(song)" :title="t('vrpiano.cover_from_midishow')" alt="">
              <span v-else-if="songIcon(song)">{{ songIcon(song) }}</span>
              <Music v-else :size="15" />
            </span>
            <span class="song-meta">
              <strong>{{ song.name }}</strong>
              <small>{{ formatBytes(song.size) }}</small>
            </span>
          </button>
          <div v-if="!songs.length" class="empty-state">
            <Music :size="24" />
            <span>{{t('vrpiano.no_midi_songs_yet_import_search_or_paste') }}</span>
          </div>
          <div v-else-if="!filteredSongs.length" class="empty-state">
            <Search :size="24" />
            <span>{{ t('vrpiano.no_matching_local_songs', { query: localSongQuery.trim() }) }}</span>
          </div>
        </div>
      </section>

      <section class="control-pane">
        <div class="now-playing">
          <span>{{t('vrpiano.current_song') }}</span>
          <strong :title="selectedSong?.name ||t('vrpiano.none_selected')">{{ selectedSong?.name ||t('vrpiano.none_selected') }}</strong>
          <small :title="selectedSong?.path || status.songs_dir">{{ selectedSong?.path || status.songs_dir }}</small>
        </div>

        <section class="player-panel preview-panel">
          <div class="player-head">
            <div>
              <span>{{ t('vrpiano.built_in_preview_title') }}</span>
              <strong>{{ playerTitle }}</strong>
            </div>
            <button class="player-toggle" :disabled="!canTogglePlayer" @click="togglePlayer">
              <Loader2 v-if="playerLoading" :size="16" class="spin" />
              <Pause v-else-if="playerPlaying" :size="16" />
              <Play v-else :size="16" />
              {{ playerPlaying ?t('vrpiano.pause') :t('vrpiano.play') }}
            </button>
          </div>
          <p class="panel-help">{{ t('vrpiano.built_in_preview_desc') }}</p>
          <label class="player-instrument">
            <Music :size="15" />
            <span>{{t('vrpiano.playback_instrument') }}</span>
            <select v-model="playerInstrument" @change="applyPlayerInstrument">
              <option value="source">{{t('vrpiano.follow_midi_source_default') }}</option>
              <optgroup v-for="(group, groupIndex) in GENERAL_MIDI_GROUPS" :key="group.name" :label="instrumentGroupName(groupIndex, group.name)">
                <option v-for="instrument in group.instruments" :key="instrument.program" :value="String(instrument.program)">
                  {{ instrument.program + 1 }} · {{ instrumentName(instrument.program) }}
                </option>
              </optgroup>
            </select>
            <small :title="activeInstrumentText">{{ activeInstrumentText }}<template v-if="sourceHasSustainPedal"> · {{ l('延音踏板', 'Sustain pedal') }}</template></small>
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

        <section class="external-section-heading">
          <strong>{{ t('vrpiano.external_playback_title') }}</strong>
          <span>{{ t('vrpiano.external_playback_desc') }}</span>
        </section>

        <div class="progress-area external-progress">
          <div class="progress-head">
            <span>{{ status.last_event ||t('vrpiano.ready') }}</span>
            <strong>{{ progressPercent }}%</strong>
          </div>
          <div class="progress-track"><div class="progress-fill" :style="{ width: `${progressPercent}%` }" /></div>
          <div class="progress-foot">
            <span>{{ t('vrpiano.notes_count', { played: status.played_notes, total: status.total_notes }) }}</span>
            <span>{{ formatTime(status.duration_ms) }}</span>
          </div>
        </div>

        <div class="control-grid">
          <label>
            <span>{{t('vrpiano.start_delay') }}</span>
            <input v-model.number="delaySecs" type="number" min="0" max="60">
            <b>{{t('vrpiano.sec') }}</b>
          </label>
          <label>
            <span>{{t('vrpiano.speed_multiplier') }}</span>
            <input v-model.number="speed" type="range" min="0.25" max="3" step="0.05">
            <b>{{ speedText }}</b>
          </label>
        </div>

        <div class="speed-actions">
          <button class="small-action" @click="adjustSpeed(-0.1)">F4 {{t('vrpiano.slower') }}</button>
          <button class="small-action" @click="resetSpeed">F5 {{t('vrpiano.default') }}</button>
          <button class="small-action" @click="adjustSpeed(0.1)">F3 {{t('vrpiano.faster') }}</button>
        </div>

        <div class="hotkey-panel" :class="{ enabled: hotkeysEnabled }">
          <div>
            <strong>{{t('vrpiano.global_shortcuts') }}</strong>
            <span>{{t('vrpiano.f1_starts_pauses_or_resumes_after_playba') }}</span>
          </div>
          <button class="toggle-btn" :class="{ enabled: hotkeysEnabled }" :disabled="!status.hotkeys_available" @click="toggleHotkeys">
            {{ hotkeysEnabled ?t('vrpiano.enabled') :t('vrpiano.disabled') }}
          </button>
        </div>

        <div class="extra-controls">
          <div class="control-section">
            <strong>{{t('vrpiano.midi_recording') }}</strong>
            <div class="control-row">
               <button v-if="!recording" class="small-action record-start" :disabled="loading" @click="startRecording">
                 <Disc3 :size="14" /> {{ t('vrpiano.start_recording') }}
               </button>
              <button v-else class="small-action record-stop" :disabled="loading" @click="stopRecording">
                <Square :size="14" /> {{t('vrpiano.stop_recording') }}
              </button>
              <span v-if="recordedMidiPath" class="recording-path">{{ recordedMidiPath }}</span>
            </div>
          </div>

          <div class="control-section">
            <strong>{{ t('vrpiano.vrchat_osc_piano') }}</strong>
            <div class="osc-compact">
              <div class="osc-inline-inputs">
                <span class="osc-label">{{ t('vrpiano.host') }}</span>
                <input v-model="vrchatOscHost" placeholder="127.0.0.1" :disabled="loading">
                <span class="osc-label">{{ t('vrpiano.port') }}</span>
                <input v-model.number="vrchatOscPort" type="number" min="1" max="65535" :disabled="loading">
              </div>
              <div class="osc-inline-inputs" style="margin-top:6px">
                <span class="osc-label">{{ t('vrpiano.osc_protocol') }}</span>
                <select v-model="vrchatOscMode" :disabled="loading" style="flex:1">
                  <option value="piano">{{ t('vrpiano.osc_protocol_piano') }}</option>
                  <option value="avatar">{{ t('vrpiano.osc_protocol_avatar') }}</option>
                </select>
              </div>
              <div v-if="vrchatOscMode === 'avatar'" class="osc-inline-inputs" style="margin-top:6px">
                <span class="osc-label">{{ t('vrpiano.osc_avatar_prefix') }}</span>
                <input v-model.trim="vrchatOscAvatarPrefix" placeholder="/avatar/parameters/note" :disabled="loading">
              </div>
              <div class="control-row" style="margin-top:10px">
                <button class="small-action" :disabled="loading || !selectedSong" @click="startVrchatOsc">
                  <SendHorizontal :size="14" /> {{t('vrpiano.play_via_vrchat_osc') }}
                </button>
                <button class="small-action ghost" :disabled="loading" @click="testOscNote">
                  <Radio :size="14" /> {{ t('vrpiano.test_osc_note') }}
                </button>
              </div>
              <p v-if="status.vrchat_osc_running" class="osc-status" style="margin-top:8px">{{t('vrpiano.vrchat_osc_active') }}</p>
              <p v-else-if="status.vrchat_osc_last_error" class="osc-error" style="margin-top:8px">{{ status.vrchat_osc_last_error }}</p>
            </div>
          </div>

          <div class="control-section output-mode-section">
            <strong>{{ t('vrpiano.output_mode') }}</strong>
            <div class="output-mode-grid">
              <button class="output-mode-card" :class="{ active: outputMode === 'keyboard' }" @click="outputMode = 'keyboard'">
                <Keyboard :size="16" />
                <span>{{ t('vrpiano.pc_keyboard_mode') }}</span>
                <small>{{ t('vrpiano.pc_keyboard_desc') }}</small>
              </button>
              <button class="output-mode-card" :class="{ active: outputMode === 'midi' }" @click="outputMode = 'midi'">
                <Cable :size="16" />
                <span>{{ t('vrpiano.direct_midi_mode') }}</span>
                <small>{{ t('vrpiano.direct_midi_desc') }}</small>
              </button>
              <button class="output-mode-card" :class="{ active: outputMode === 'osc' }" @click="outputMode = 'osc'">
                <Radio :size="16" />
                <span>{{ t('vrpiano.vrchat_osc_mode') }}</span>
                <small>{{ t('vrpiano.vrchat_osc_desc') }}</small>
              </button>
            </div>
            <button v-if="outputMode === 'midi'" class="direct-midi-action" :disabled="loading || !selectedSong || !midiOutputState.connected" @click="startDirectMidi">
              <Cable :size="14" /> {{ t('vrpiano.start_direct_midi') }}
            </button>
          </div>

          <div class="control-section">
            <strong>{{t('vrpiano.channel_controls') }}</strong>
            <div class="channel-grid">
              <div v-for="idx in 16" :key="idx - 1" class="channel-row" :class="{ muted: channelStates[idx - 1].muted, solo: channelStates[idx - 1].solo, routed: channelRouted[idx - 1] }">
                <span class="channel-label">{{ t('vrpiano.channel_number', { channel: idx - 1 }) }}</span>
                <button class="channel-btn mute" :class="{ active: channelStates[idx - 1].muted }" :title="t('vrpiano.mute')" @click="setChannelMute(idx - 1, !channelStates[idx - 1].muted)">
                  {{ t('vrpiano.mute_short') }}
                </button>
                <button class="channel-btn solo" :class="{ active: channelStates[idx - 1].solo }" :title="t('vrpiano.solo')" @click="setChannelSolo(idx - 1, !channelStates[idx - 1].solo)">
                  {{ t('vrpiano.solo_short') }}
                </button>
                <button class="channel-btn route" :class="{ active: channelRouted[idx - 1] }" :title="t('vrpiano.route_to_piano')" @click="setChannelRouted(idx - 1, !channelRouted[idx - 1])">
                  {{ t('vrpiano.route_short') }}
                </button>
                <input type="range" min="0" max="127" :value="channelStates[idx - 1].volume" @input="setChannelVolume(idx - 1, Number(($event.target as HTMLInputElement).valueAsNumber))" class="channel-volume">
                <span class="channel-vol-label">{{ t('vrpiano.volume') }} {{ channelStates[idx - 1].volume }}</span>
              </div>
            </div>
          </div>

          <div class="control-section">
            <strong>{{t('vrpiano.transpose') }}</strong>
            <div class="control-row transpose-row">
              <label class="transpose-field">
                <span>{{t('vrpiano.transpose') }}</span>
                <input v-model.number="transpose" type="number" min="-24" max="24" step="1" @change="applyTranspose">
                <b>{{t('vrpiano.transpose_semitones') }}</b>
              </label>
              <input type="range" min="-24" max="24" step="1" :value="transpose" @input="transpose = Number(($event.target as HTMLInputElement).valueAsNumber); applyTranspose()">
              <div class="transpose-right">
                <label class="exclude-drums" :title="t('vrpiano.transpose_drums_always_excluded')">
                  <input v-model="excludeDrums" type="checkbox" disabled>
                  <span>{{t('vrpiano.transpose_exclude_drums') }}</span>
                </label>
                <small class="transpose-hint">{{t('vrpiano.transpose_drums_always_excluded') }}</small>
              </div>
            </div>
          </div>

          <div class="control-section">
            <strong>{{t('vrpiano.playlist') }} / {{t('vrpiano.play_mode') }}</strong>
            <div class="control-row playlist-control-row">
              <label class="playmode-field">
                <span>{{t('vrpiano.play_mode') }}</span>
                <select v-model="playMode" @change="applyPlayMode">
                  <option v-for="opt in playModeOptions" :key="opt.value" :value="opt.value">{{ opt.label() }}</option>
                </select>
              </label>
              <button class="small-action" :disabled="!selectedSong" @click="addToPlaylist">
                <Music :size="14" /> {{t('vrpiano.add_to_playlist') }}
              </button>
              <button class="small-action" @click="applyPlaylist" :disabled="!playlist.length || loading">
                <Play :size="14" /> {{t('vrpiano.apply_playlist') }}
              </button>
              <button class="small-action ghost" :disabled="!playlist.length" @click="clearPlaylist">
                <Trash2 :size="14" /> {{t('vrpiano.clear_playlist') }}
              </button>
            </div>
            <ul v-if="playlist.length" class="playlist-list">
              <li v-for="(path, index) in playlist" :key="path" class="playlist-item">
                <span class="playlist-index">{{ index + 1 }}</span>
                <span class="playlist-name" :title="path">{{ playlistSongs[index]?.name || path }}</span>
                <div class="playlist-actions">
                  <button class="channel-btn" :disabled="index === 0" :title="t('vrpiano.move_up')" @click="movePlaylistItem(index, -1)">↑</button>
                  <button class="channel-btn" :disabled="index === playlist.length - 1" :title="t('vrpiano.move_down')" @click="movePlaylistItem(index, 1)">↓</button>
                  <button class="channel-btn" :title="t('vrpiano.remove_from_playlist')" @click="removeFromPlaylist(path)">✕</button>
                </div>
              </li>
            </ul>
            <p v-else class="playlist-empty">{{ t('vrpiano.playlist_empty') }}</p>
          </div>

          <div class="control-section">
            <strong>{{t('vrpiano.midi_device') }}</strong>
            <div class="control-row">
              <select v-model="selectedMidiDevice" class="midi-device-select">
                <option v-if="!midiDevices.length" value="" disabled>{{t('vrpiano.no_midi_devices') }}</option>
                <option v-for="device in midiDevices" :key="device.id" :value="device.id">{{ device.name }}</option>
              </select>
              <button class="small-action" :disabled="!selectedMidiDevice || midiOutputState.connected" @click="connectMidiDevice">
                <Link2 :size="14" /> {{t('vrpiano.connect') }}
              </button>
              <button class="small-action ghost" :disabled="!midiOutputState.connected" @click="disconnectMidiDevice">
                <X :size="14" /> {{t('vrpiano.disconnect') }}
              </button>
              <button class="small-action ghost" :title="t('vrpiano.refresh_devices')" @click="refreshMidiDevices">
                <RefreshCcw :size="14" />
              </button>
            </div>
            <p v-if="midiOutputState.connected" class="osc-status">
              {{ t('vrpiano.connected_device', { device: midiOutputState.device_name || '' }) }}
            </p>
          </div>

          <div class="control-section">
            <strong>{{t('vrpiano.advanced_features') }}</strong>
            <div class="control-row">
              <button class="small-action" :class="{ enabled: voiceControlEnabled }" :disabled="loading" @click="toggleVoiceControl">
                <Mic :size="14" /> {{ voiceControlEnabled ?t('vrpiano.disable_voice') :t('vrpiano.enable_voice') }}
              </button>
              <button class="small-action" :class="{ enabled: ttsEnabled }" :disabled="loading" @click="toggleTts">
                <Radio :size="14" /> {{ ttsEnabled ?t('vrpiano.disable_tts') :t('vrpiano.enable_tts') }}
              </button>
            </div>
          </div>
        </div>

        <div class="action-row">
          <button class="primary-action" :disabled="!canTogglePlayback" @click="togglePlayback">
            <Loader2 v-if="loading" :size="18" class="spin" />
            <Pause v-else-if="status.running && !status.paused" :size="18" />
            <Play v-else :size="18" />
            F1 {{ playbackActionLabel }}
          </button>
          <button v-if="isPlaying" class="stop-action" :disabled="loading" @click="stopAll">
            <CircleStop :size="18" />
            {{ t('vrpiano.stop') }}
          </button>
          <button v-if="hasStartedPlayback" class="restart-action" :disabled="loading" @click="restartPlayback">
            <RefreshCcw :size="18" />
            F2 {{t('vrpiano.restart') }}
          </button>
        </div>

        <section class="online-panel">
          <div class="online-head">
            <strong>{{t('vrpiano.online_library') }}</strong>
            <span>{{t('vrpiano.midishow_search_id_url_downloads_and_onl') }}</span>
          </div>

          <div class="midishow-account">
            <div>
              <strong>{{ defaultMidishowAccount ? t('vrpiano.signed_in_as', { username: defaultMidishowAccount.username, type: defaultMidishowLoginTypeText ? `（${defaultMidishowLoginTypeText}）` : '' }) :t('vrpiano.midishow_signed_out') }}</strong>
              <span>{{ defaultMidishowAccount ?t('vrpiano.downloads_and_previews_will_use_your_acc') :t('vrpiano.sign_in_to_access_midi_downloads_and_pre') }}</span>
            </div>
            <button v-if="defaultMidishowAccount" class="account-btn ghost" :disabled="accountLoading" @click="logoutMidishow">
              <LogOut :size="15" /> {{t('vrpiano.sign_out') }}
            </button>
            <button v-else class="account-btn" @click="toggleMidishowLogin">
              <LogIn :size="15" /> {{ midishowLoginOpen ?t('vrpiano.hide') :t('vrpiano.sign_in') }}
            </button>
          </div>

          <form v-if="midishowLoginOpen && !defaultMidishowAccount" :key="midishowLoginOpen ? 'open' : 'closed'" class="login-form" @submit.prevent="loginMidishow">
            <label class="login-field">
              <span>{{t('vrpiano.account') }}</span>
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
                :placeholder="t('vrpiano.midishow_username_or_email')"
              >
            </label>
            <label class="login-field">
              <span>{{t('vrpiano.password') }}</span>
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
                :placeholder="t('vrpiano.midishow_password')"
              >
            </label>
            <p v-if="loginError" class="login-error" role="alert">
              <AlertTriangle :size="14" />
              <span>{{ loginError }}</span>
            </p>
            <p class="login-hint">{{t('vrpiano.sign_in_completes_automatically_a_login_') }}</p>
            <p v-if="accountLoading" class="login-status" aria-live="polite">
              <Loader2 :size="15" class="spin" />
              {{ midishowLoginStatus.message }}
            </p>
            <div class="login-form-actions">
              <button type="submit" :disabled="accountLoading || !midishowAccount.trim() || !midishowPassword">
                <Loader2 v-if="accountLoading" :size="16" class="spin" />
                <LogIn v-else :size="16" />
                <span>{{ accountLoading ?t('vrpiano.signing_in') :t('vrpiano.sign_in') }}</span>
              </button>
              <button type="button" class="account-btn ghost" :disabled="externalLinkLoading || accountLoading" @click="openMidishowSignup">
                <Loader2 v-if="externalLinkLoading" :size="15" class="spin" />
                <ExternalLink v-else :size="15" /> {{t('vrpiano.register') }}
              </button>
              <button type="button" class="account-btn ghost" :title="t('vrpiano.copy_registration_link')" :disabled="accountLoading" @click="copySignupUrl">
                {{t('vrpiano.copy_link') }}
              </button>
            </div>
          </form>

          <div class="online-form">
            <div class="input-row">
              <Search :size="16" />
              <input v-model="onlineKeyword" :placeholder="t('vrpiano.search_by_title_artist_or_keyword')" @keydown.enter="searchOnline">
              <div class="online-search-actions">
                <button :disabled="onlineLoading || !onlineKeyword.trim()" @click="searchOnline">
                  <Loader2 v-if="onlineLoading" :size="16" class="spin" />
                  <span v-else>{{t('vrpiano.search') }}</span>
                </button>
                <button type="button" :title="t('vrpiano.open_midishow_search_in_browser')" :disabled="onlineLoading" @click="openMidishowSearch">
                  <ExternalLink :size="16" />
                </button>
              </div>
            </div>
            <div class="input-row download-row">
              <Link2 :size="16" />
              <input v-model="urlInput" :placeholder="t('vrpiano.paste_a_direct_midi_url_midishow_link_or')" @keydown.enter="downloadFromUrl">
              <input v-model="urlFilename" class="name-input" :placeholder="t('vrpiano.save_as')">
              <button :disabled="onlineLoading || !urlInput.trim()" @click="downloadFromUrl">
                <Download :size="16" />
              </button>
            </div>
          </div>

          <div class="online-results">
            <div v-for="item in onlineResults" :key="item.id" class="online-row">
              <div class="online-meta">
                <strong>{{ item.title }}</strong>
                <small>{{ item.artist ||t('vrpiano.unknown_artist') }} · ID {{ item.id }}</small>
              </div>
              <div class="online-actions">
                <button :title="t('vrpiano.preview_online')" :disabled="onlineBusyId === item.id" @click="previewOnline(item)">
                  <Loader2 v-if="onlineBusyId === item.id" :size="15" class="spin" />
                  <Headphones v-else :size="15" />
                </button>
                <button :title="t('vrpiano.download_to_library')" :disabled="onlineBusyId === item.id" @click="downloadOnline(item)">
                  <Download :size="15" />
                </button>
                <button :title="t('vrpiano.open_webpage')" @click="openOnlinePage(item)">
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
            <strong>{{ editDialogMode === 'icon' ?t('vrpiano.edit_song_icon') :t('vrpiano.rename_song') }}</strong>
            <span>{{ selectedSong?.name }}</span>
          </div>
          <button class="dialog-close" type="button" :disabled="loading" @click="closeEditDialog">×</button>
        </div>

        <div v-if="editDialogMode === 'icon'" class="edit-dialog-body">
          <label class="theme-field">
            <span>{{t('vrpiano.text_emoji') }}</span>
            <input v-model="editIconText" maxlength="4" :placeholder="t('vrpiano.for_example_piano_a1')">
          </label>
          <label class="theme-field">
            <span>{{t('vrpiano.image_url') }}</span>
            <input v-model="editIconUrl" :placeholder="t('vrpiano.https_or_data_image')">
          </label>
          <div class="icon-preview">
            <span class="song-note custom">
              <img v-if="isImageIcon(editIconUrl.trim())" :src="editIconUrl.trim()" alt="">
              <span v-else-if="editIconText.trim()">{{ editIconText.trim().slice(0, 4) }}</span>
              <Music v-else :size="15" />
            </span>
            <small>{{t('vrpiano.image_url_takes_priority_leave_both_fiel') }}</small>
          </div>
        </div>

        <div v-else class="edit-dialog-body">
          <label class="theme-field">
            <span>{{t('vrpiano.new_song_name') }}</span>
            <input v-model="editSongName" autofocus :placeholder="t('vrpiano.enter_a_new_midi_filename')">
          </label>
        </div>

        <div class="edit-dialog-actions">
          <button class="dialog-secondary" type="button" :disabled="loading" @click="closeEditDialog">{{t('vrpiano.cancel') }}</button>
          <button class="dialog-primary" type="submit" :disabled="loading">
            <Loader2 v-if="loading" :size="16" class="spin" />
            <span v-else>{{t('vrpiano.save') }}</span>
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
input,
select {
  font: inherit;
}

input,
select {
  color: var(--vp-text);
  background-color: var(--vp-surface);
  border-color: var(--vp-border);
  outline: none;
}

input:focus,
select:focus {
  border-color: var(--vp-primary);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--vp-primary) 18%, transparent);
}

input[type="range"] {
  accent-color: var(--vp-primary);
}

input[type="range"]::-webkit-slider-runnable-track {
  background: color-mix(in srgb, var(--vp-primary) 22%, var(--vp-surface));
}

input[type="range"]::-moz-range-track {
  background: color-mix(in srgb, var(--vp-primary) 22%, var(--vp-surface));
}

select option {
  color: var(--vp-text);
  background: var(--vp-surface);
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
.restart-action:disabled,
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

.panel-help,
.external-section-heading span {
  margin: 0;
  color: var(--vp-muted);
  font-size: 12px;
  line-height: 1.5;
}

.external-section-heading {
  display: grid;
  gap: 4px;
  padding: 4px 2px 0;
}

.external-section-heading strong {
  color: var(--vp-text);
  font-size: 14px;
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
  appearance: auto;
}

.player-instrument select:focus {
  border-color: var(--vp-primary);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--vp-primary) 18%, transparent);
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
  height: 6px;
  border-radius: 999px;
  background: color-mix(in srgb, var(--vp-text) 12%, transparent);
  cursor: pointer;
}

.player-slider input:focus,
.player-volume input:focus {
  outline: none;
}

.player-slider input::-webkit-slider-thumb,
.player-volume input::-webkit-slider-thumb {
  appearance: none;
  width: 16px;
  height: 16px;
  border-radius: 999px;
  background: var(--vp-primary);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--vp-primary) 20%, transparent);
  cursor: pointer;
  margin-top: -5px;
}

.player-slider input::-moz-range-thumb,
.player-volume input::-moz-range-thumb,
.channel-volume::-moz-range-thumb,
.control-grid input[type="range"]::-moz-range-thumb {
  width: 16px;
  height: 16px;
  border: 0;
  border-radius: 999px;
  background: var(--vp-primary);
  cursor: pointer;
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
.restart-action,
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

.restart-action {
  min-width: 130px;
  color: var(--vp-text);
  background: var(--vp-panel);
  box-shadow: inset 0 0 0 1px var(--vp-border);
}

.restart-action:hover:not(:disabled) {
  color: var(--vp-primary);
  background: var(--vp-hover);
}

.stop-action {
  min-width: 100px;
  color: white;
  background: #ef4444;
  border: 0;
  border-radius: 8px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 7px;
  font-weight: 800;
  white-space: nowrap;
  cursor: pointer;
  padding: 0 16px;
}

.stop-action:hover:not(:disabled) {
  background: #dc2626;
}

.stop-action:disabled {
  opacity: .55;
  cursor: not-allowed;
}

.extra-controls {
  display: grid;
  gap: 12px;
}

.control-section {
  padding: 12px;
  border-radius: 8px;
  background: var(--vp-panel);
  box-shadow: inset 0 0 0 1px var(--vp-border);
}

.control-section > strong {
  display: block;
  margin-bottom: 8px;
  color: var(--vp-text);
  font-size: 13px;
  font-weight: 800;
}

.control-row {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}

.recording-path {
  color: var(--vp-dim);
  font-size: 11px;
  font-weight: 700;
  word-break: break-all;
}

.record-start {
  color: #dc2626;
  background: color-mix(in srgb, #dc2626 12%, var(--vp-panel));
  box-shadow: inset 0 0 0 1px color-mix(in srgb, #dc2626 28%, transparent);
}

.record-start:hover:not(:disabled) {
  background: color-mix(in srgb, #dc2626 22%, var(--vp-hover));
}

.record-stop {
  color: white;
  background: #dc2626;
}

.record-stop:hover:not(:disabled) {
  background: #b91c1c;
}

.output-mode-section {
  display: grid;
  gap: 10px;
}

.output-mode-grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 10px;
}

.output-mode-card {
  min-width: 0;
  min-height: 104px;
  display: grid;
  align-content: start;
  justify-items: start;
  gap: 7px;
  padding: 12px;
  border: 1px solid var(--vp-border);
  border-radius: 9px;
  color: var(--vp-muted);
  background: var(--vp-surface);
  text-align: left;
  cursor: pointer;
}

.output-mode-card:hover,
.output-mode-card.active {
  color: var(--vp-text);
  border-color: var(--vp-primary);
  background: color-mix(in srgb, var(--vp-primary) 12%, var(--vp-surface));
}

.output-mode-card span {
  font-size: 12px;
  font-weight: 850;
}

.output-mode-card small {
  color: var(--vp-dim);
  font-size: 11px;
  line-height: 1.4;
}

.direct-midi-action {
  min-height: 40px;
  justify-self: start;
  padding: 0 14px;
  border: 0;
  border-radius: 8px;
  color: white;
  background: var(--vp-primary);
  font-weight: 800;
  cursor: pointer;
}

.direct-midi-action:disabled {
  opacity: .55;
  cursor: not-allowed;
}

.osc-config {
  min-width: 0;
  flex: 1 1 180px;
  display: block;
}

.osc-compact {
  display: grid;
  gap: 6px;
}

.osc-inline-inputs {
  display: flex;
  align-items: center;
  gap: 8px;
}

.osc-label {
  min-width: 48px;
  color: var(--vp-muted);
  font-size: 12px;
  font-weight: 800;
}

.osc-config span {
  color: var(--vp-muted);
  font-size: 12px;
  font-weight: 800;
}

.osc-config input {
  flex: 1;
  min-height: 36px;
  padding: 0 10px;
  border: 1px solid var(--vp-border);
  border-radius: 7px;
  color: var(--vp-text);
  background: var(--vp-surface);
}

.channel-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 10px;
}

.channel-row {
  display: grid;
  grid-template-columns: minmax(0, 1fr) repeat(3, minmax(0, 1fr));
  gap: 8px;
  align-items: center;
  min-width: 0;
  padding: 10px;
  border-radius: 8px;
  background: var(--vp-surface);
  box-shadow: inset 0 0 0 1px var(--vp-border);
}

.channel-row.muted {
  opacity: 0.5;
}

.channel-row.solo {
  background: color-mix(in srgb, var(--vp-primary) 18%, var(--vp-surface));
  box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--vp-primary) 32%, transparent);
}

.channel-label {
  width: auto;
  min-width: 0;
  grid-column: 1 / -1;
  color: var(--vp-muted);
  font-size: 12px;
  font-weight: 800;
  text-align: left;
}

.channel-btn {
  width: 100%;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  min-height: 30px;
  border: 1px solid var(--vp-border);
  border-radius: 4px;
  display: grid;
  place-items: center;
  color: var(--vp-muted);
  background: var(--vp-panel);
  font-size: 11px;
  font-weight: 900;
  cursor: pointer;
}

.channel-btn.active {
  color: white;
  background: var(--vp-primary);
  border-color: var(--vp-primary);
}

.channel-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.transpose-row {
  display: grid;
  grid-template-columns: auto 1fr;
  gap: 10px 12px;
  align-items: center;
}

.transpose-field {
  display: flex;
  align-items: center;
  gap: 8px;
}

.transpose-field span {
  color: var(--vp-dim);
  font-size: 12px;
}

.transpose-field input {
  width: 64px;
}

.transpose-field b {
  color: var(--vp-dim);
  font-size: 11px;
  font-weight: 600;
}

.transpose-right {
  grid-column: 1 / -1;
  display: grid;
  gap: 2px;
}

.exclude-drums {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  color: var(--vp-muted);
}

.exclude-drums input {
  width: 14px;
  height: 14px;
}

.transpose-hint {
  color: var(--vp-dim);
  font-size: 11px;
  line-height: 1.3;
}

.playmode-field {
  display: flex;
  align-items: center;
  gap: 8px;
}

.playmode-field span {
  color: var(--vp-dim);
  font-size: 12px;
  white-space: nowrap;
}

.playmode-field select {
  flex: 1;
  min-width: 0;
}

.playlist-list {
  list-style: none;
  margin: 10px 0 0;
  padding: 0;
  display: grid;
  gap: 4px;
  max-height: 180px;
  overflow-y: auto;
}

.playlist-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 5px 8px;
  border-radius: 6px;
  background: color-mix(in srgb, var(--vp-text) 6%, transparent);
}

.playlist-index {
  width: 18px;
  color: var(--vp-dim);
  font-size: 11px;
  font-weight: 800;
  text-align: center;
}

.playlist-name {
  flex: 1;
  min-width: 0;
  font-size: 12px;
  color: var(--vp-text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.playlist-actions {
  display: flex;
  gap: 4px;
}

.playlist-empty {
  margin: 8px 0 0;
  color: var(--vp-dim);
  font-size: 12px;
}

.midi-device-select {
  flex: 1;
  min-width: 0;
}

.small-action.ghost {
  background: transparent;
}

.channel-volume {
  grid-column: 1 / -1;
  width: 100%;
  min-width: 0;
  appearance: none;
  height: 5px;
  border-radius: 999px;
  background: color-mix(in srgb, var(--vp-text) 12%, transparent);
  cursor: pointer;
}
.channel-volume::-webkit-slider-thumb {
  appearance: none;
  width: 14px;
  height: 14px;
  border-radius: 999px;
  background: var(--vp-primary);
  cursor: pointer;
  margin-top: -5px;
}
.channel-vol-label {
  grid-column: 1 / -1;
  width: auto;
  color: var(--vp-dim);
  font-size: 11px;
  font-weight: 800;
  text-align: left;
}

select:focus,
.midi-device-select:focus,
.playmode-field select:focus {
  border-color: var(--vp-primary);
  outline: none;
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--vp-primary) 18%, transparent);
}

select option:checked,
select option:hover {
  color: white;
  background: var(--vp-primary);
}

.player-slider input,
.player-volume input,
.control-grid input[type="range"],
.channel-volume {
  accent-color: var(--vp-primary);
  background: color-mix(in srgb, var(--vp-primary) 22%, var(--vp-surface));
}

.player-slider input::-webkit-slider-runnable-track,
.player-volume input::-webkit-slider-runnable-track,
.control-grid input[type="range"]::-webkit-slider-runnable-track,
.channel-volume::-webkit-slider-runnable-track {
  height: 5px;
  border-radius: 999px;
  background: color-mix(in srgb, var(--vp-primary) 22%, var(--vp-surface));
}

.player-slider input::-moz-range-track,
.player-volume input::-moz-range-track,
.control-grid input[type="range"]::-moz-range-track,
.channel-volume::-moz-range-track {
  height: 5px;
  border-radius: 999px;
  background: color-mix(in srgb, var(--vp-primary) 22%, var(--vp-surface));
}

.small-action.enabled {
  color: white;
  background: var(--vp-primary);
  box-shadow: inset 0 0 0 1px var(--vp-primary);
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

  .channel-grid {
    grid-template-columns: 1fr;
  }

  .output-mode-grid {
    grid-template-columns: 1fr;
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

  .restart-action,
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
  .channel-row {
    grid-template-columns: minmax(70px, auto) repeat(3, minmax(44px, 1fr));
    gap: 6px;
  }

  .login-form-actions {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .login-form-actions button:first-child {
    grid-column: 1 / -1;
  }
}
</style>

