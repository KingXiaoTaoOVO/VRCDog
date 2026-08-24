import { invoke, isTauri } from "@tauri-apps/api/core";
import { AuthApi } from './auth';
import { UserApi } from './user';
import { FriendApi } from './friend';
import { WorldApi } from './world';
import { AvatarApi } from './avatar';
import { GroupApi } from './group';
import { NotificationApi } from './notification';
import { FavoriteApi } from './favorite';
import { FileApi } from './file';
import { PlayerModerationApi } from './playerModeration';
import { AvatarModerationApi } from './avatarModeration';
import { InventoryApi } from './inventory';
import { PropApi } from './prop';
import { InviteMessagesApi } from './inviteMessages';
import { MiscApi } from './misc';
import { VrcPlusIconApi } from './vrcPlusIcon';
import { VrcPlusImageApi } from './vrcPlusImage';
import { InstanceApi } from './instance';
import { ImageApi } from './image';
import { QueryRequestApi } from './queryRequest';
import { getStoredAuthCookie, parseExecuteResponse, request as baseRequest, VrcRequestError } from './request';
import { isDebugLogEnabled } from './debugConfig';
import { toCleanBase64 } from './utils';
import { normalizeTwoFactorMethod } from './twoFactor';
import type { SurveyAnswerAttachment } from '../types/survey';

const SENSITIVE_ARG_KEYS = /^(?:password|passwd|pwd|cookie|cookies|authcookie|authorization|sessdata|bili_jct|csrf|csrf_token|buvid3|stream_key|token|access[_-]?token|refresh[_-]?token|secret)$/i;

const createBrowserVrpianoStatus = () => ({
  running: false,
  paused: false,
  song_name: '',
  song_path: '',
  progress: 0,
  played_notes: 0,
  total_notes: 0,
  duration_ms: 0,
  elapsed_ms: 0,
  last_event: 'browser_preview',
  last_error: '',
  songs_dir: 'Browser preview',
  speed: 1,
  hotkeys_enabled: false,
  hotkeys_available: true,
  last_hotkey: '',
  last_hotkey_at_ms: 0,
});

let browserVrpianoStatus = createBrowserVrpianoStatus();
let browserDrawingStatus = {
  prepared: false,
  running: false,
  paused: false,
  progress: 0,
  current_stroke: 0,
  total_strokes: 0,
  total_points: 0,
  source_path: '',
  last_event: 'browser_preview',
  last_error: '',
  hotkeys_enabled: true,
  hotkeys_available: true,
  last_hotkey: '',
  last_hotkey_at_ms: 0,
  stage: '',
};

const sanitizeInvokeValue = (value: unknown, seen = new WeakSet<object>()): unknown => {
  if (value === null || typeof value !== 'object') return value;
  if (seen.has(value as object)) return '[Circular]';
  seen.add(value as object);

  if (Array.isArray(value)) {
    return value.map((item) => sanitizeInvokeValue(item, seen));
  }

  return Object.fromEntries(
    Object.entries(value as Record<string, unknown>).map(([key, nested]) => [
      key,
      SENSITIVE_ARG_KEYS.test(key) ? '***' : sanitizeInvokeValue(nested, seen),
    ]),
  );
};

export async function safeInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauri()) {
    console.warn(`[Browser Mock] API Command: ${cmd}`, sanitizeInvokeValue(args));
    const mockDanmakuStatus = (overrides: Record<string, unknown> = {}) => ({
      running: false,
      bili_connected: false,
      osc_input_running: false,
      vr_initialized: false,
      overlay_visible: true,
      vr_menu_visible: false,
      room_id: 0,
      online: 0,
      message_count: 0,
      last_error: '',
      last_event: 'browser_preview',
      ...overrides,
    });
    const mockDanmakuConfig = {
      enable_bilibili: true,
      room_id: 0,
      bili_sessdata: '',
      enable_osc_input: true,
      osc_input_host: '127.0.0.1',
      osc_input_port: 9011,
      osc_input_address: '/vrcdog/danmaku',
      enable_osc_output: false,
      osc_output_host: '127.0.0.1',
      osc_output_port: 9000,
      osc_output_address: '/vrcdog/danmaku',
      enable_vrc_chatbox: false,
      vrc_chatbox_port: 9000,
      chatbox_interval_ms: 1600,
      enable_vr_overlay: true,
      overlay_visible: true,
      vr_menu_visible: false,
      attach_mode: 'hmd',
      toggle_hand: 'left',
      x: -0.4,
      y: 0.1,
      z: -0.8,
      pitch: 0,
      yaw: 15,
      roll: 0,
      overlay_width_m: 0.4,
      overlay_alpha: 0.92,
      bg_alpha: 0.85,
      font_size: 14,
      text_color: '#FFFFFF',
      bg_color: '#10141F',
      max_messages: 50,
      show_danmaku: true,
      show_gift: true,
      show_enter: true,
      show_follow: true,
      show_guard: true,
      show_sc: true,
      vr_input_text: '',
    };
    if (cmd === 'vrc_execute') {
      const requestUrl = String((args?.options as any)?.url || '');
      if (requestUrl.includes('/auth/user')) {
        return Promise.resolve({
          status: 200,
          data: JSON.stringify({
            id: 'usr_browser_preview',
            username: 'preview-user',
            displayName: 'VrcDog Preview',
            status: 'active',
            statusDescription: 'Browser UI preview',
            currentAvatarThumbnailImageUrl: '',
            tags: [],
          }),
          auth_cookie: 'auth=browser_preview',
        }) as any;
      }
      if (requestUrl.includes('/config')) {
        return Promise.resolve({
          status: 200,
          data: JSON.stringify({
            apiUrl: 'https://api.vrchat.cloud/api/1',
            websocketUrl: 'wss://pipeline.vrchat.cloud',
          }),
        }) as any;
      }
      if (
        requestUrl.includes('/api/client/register')
        || requestUrl.includes('/api/client/heartbeat')
        || requestUrl.includes('/api/client/check-status/')
      ) {
        return Promise.resolve({
          status: 200,
          data: JSON.stringify({ status: 'ok', allowed: true, features: [] }),
        }) as any;
      }
      if (requestUrl.includes('/api/client/features/')) {
        return Promise.resolve({ status: 200, data: JSON.stringify({ features: [] }) }) as any;
      }
      return Promise.resolve({ status: 200, data: '{}' }) as any;
    }
    if (cmd === 'vrc_get_server_status') return Promise.resolve({ status: { description: 'All Systems Operational' } }) as any;
    if (cmd === 'db_get_auth') return Promise.resolve('mock_auth_cookie_abc123') as any;
    if (cmd === 'db_get_setting') return Promise.resolve(null) as any;
    if (cmd === 'check_system_status') return Promise.resolve({ hub_installed: true, unity_installed: true, tool_installed: true, vcc_installed: true, alcom_installed: false }) as any;
    if (cmd === 'vrc_fetch_config' || cmd === 'db_getAllSettings' || cmd === 'db_get_all_settings') return Promise.resolve({}) as any;
    if (cmd === 'vrc_get_friends' || cmd === 'vrc_search_users' || cmd === 'vrc_search_worlds' || cmd === 'vrc_get_notifications' || cmd === 'vrc_get_avatars' || cmd === 'db_get_status_presets' || cmd === 'vrc_get_latest_gamelogs' || cmd === 'db_get_all_notes') {
      return Promise.resolve([]) as any;
    }
    if (cmd === 'osc_get_status') return Promise.resolve({ monitorRunning: false, automationRunning: false }) as any;
    if (cmd === 'osc_get_system_snapshot') {
      return Promise.resolve({
        cpuUsage: 28.4,
        cpuName: 'Intel Core i7-12700K',
        cpuPhysicalCores: 12,
        cpuLogicalCores: 20,
        cpuFrequencyMhz: 4900,
        ramUsage: 54.2,
        memoryUsedGb: 17.3,
        memoryTotalGb: 32,
        gpuName: 'NVIDIA GeForce RTX 4070',
        gpuUsage: 41,
        gpuMemoryUsedGb: 4.8,
        gpuMemoryTotalGb: 12,
        diskUsage: 66,
        diskUsedGb: 1320,
        diskTotalGb: 2000,
        osName: 'Windows 11 Pro',
        hostName: 'VRC-PC',
        systemUptimeSeconds: 13740,
        idleSeconds: 12,
        activeWindow: 'VrcDog Preview',
        localTime: new Date().toLocaleTimeString('zh-CN', { hour12: false }),
        localDate: new Date().toISOString().slice(0, 10),
        vrcRunning: true,
      }) as any;
    }
    if (cmd.startsWith('osc_')) return Promise.resolve(undefined) as any;
    if (cmd === 'ovr_load_ovras_ini') return Promise.resolve('{}') as any;
    if (cmd === 'ovr_sync_ovras_ini') return Promise.resolve(undefined) as any;
    if (cmd === 'danmaku_get_config') return Promise.resolve(mockDanmakuConfig) as any;
    if (cmd === 'bili_live_get_areas') return Promise.resolve([
      { id: 235, name: '虚拟主播', parent_id: 9, parent_name: '娱乐', pic: '' },
      { id: 216, name: '日常', parent_id: 1, parent_name: '生活', pic: '' },
    ]) as any;
    if (cmd === 'bili_live_get_room_info' || cmd === 'bili_live_get_own_room') return Promise.resolve({
      room_id: Number(args?.roomId || 123456), uid: 10001, title: 'VrcDog 直播预览', area_id: 235,
      area_name: '虚拟主播', parent_area_id: 9, parent_area_name: '娱乐', live_status: 0,
      online: 128, cover: '', announcement: '欢迎来到 VrcDog 直播间',
    }) as any;
    if (cmd === 'bili_live_start') return Promise.resolve({
      live: true, requires_face_auth: false, face_auth_url: null, message: 'ok', endpoints: [
        { protocol: 'RTMP', address: 'rtmp://live.example.com/live', stream_key: 'preview-secret-key', provider: 'preview' },
        { protocol: 'SRT', address: 'srt://live.example.com:10080', stream_key: 'preview-srt-key', provider: 'preview' },
      ],
    }) as any;
    if (cmd === 'bili_live_get_contribution_rank') return Promise.resolve([
      { uid: 1, name: '星空旅人', face: '', rank: 1, score: 5200 },
      { uid: 2, name: 'VR 小狗', face: '', rank: 2, score: 3200 },
    ]) as any;
    if (cmd === 'bili_live_update_title' || cmd === 'bili_live_update_area' || cmd === 'bili_live_update_announcement' || cmd === 'bili_live_stop' || cmd === 'bili_live_send_danmaku') return Promise.resolve(undefined) as any;
    if (cmd === 'danmaku_get_status') return Promise.resolve(mockDanmakuStatus()) as any;
    if (cmd === 'danmaku_get_messages') return Promise.resolve([]) as any;
    if (cmd === 'danmaku_set_config') {
      const config = (args?.config || mockDanmakuConfig) as any;
      return Promise.resolve(mockDanmakuStatus({ overlay_visible: config.overlay_visible, vr_menu_visible: config.vr_menu_visible, room_id: config.room_id })) as any;
    }
    if (cmd === 'danmaku_start') {
      const config = (args?.config || mockDanmakuConfig) as any;
      return Promise.resolve(mockDanmakuStatus({
        running: true,
        osc_input_running: Boolean(config.enable_osc_input),
        vr_initialized: Boolean(config.enable_vr_overlay),
        overlay_visible: config.overlay_visible !== false,
        vr_menu_visible: Boolean(config.vr_menu_visible),
        room_id: config.room_id || 0,
        last_event: 'browser_preview_started',
      })) as any;
    }
    if (cmd === 'danmaku_stop') return Promise.resolve(mockDanmakuStatus({ last_event: 'browser_preview_stopped' })) as any;
    if (cmd === 'danmaku_clear_messages') return Promise.resolve(undefined) as any;
    if (cmd === 'danmaku_set_overlay_visible') return Promise.resolve(mockDanmakuStatus({ overlay_visible: Boolean(args?.visible) })) as any;
    if (cmd === 'danmaku_set_vr_input_text') return Promise.resolve(mockDanmakuStatus({ vr_input_text: String(args?.text || '') })) as any;
    if (cmd === 'danmaku_submit_vr_input') {
      return Promise.resolve({
        id: Date.now(),
        source: 'browser',
        message_type: 'input',
        user: 'VR输入',
        text: String(args?.text || mockDanmakuConfig.vr_input_text || '测试输入'),
        price: null,
        gift_count: null,
        medal_name: null,
        medal_level: null,
        guard_level: null,
        timestamp_ms: Date.now(),
      }) as any;
    }
    if (cmd === 'danmaku_send_test') {
      return Promise.resolve({
        id: Date.now(),
        source: 'browser',
        message_type: String(args?.messageType || 'danmaku'),
        user: 'PreviewUser',
        text: String(args?.text || 'VrcDog浏览器预览消息'),
        price: null,
        gift_count: null,
        medal_name: null,
        medal_level: null,
        guard_level: null,
        timestamp_ms: Date.now(),
      }) as any;
    }
    if (cmd === 'vrpiano_init' || cmd === 'vrpiano_get_status') return Promise.resolve({ ...browserVrpianoStatus }) as any;
    if (cmd === 'vrdrawing_get_status') return Promise.resolve({ ...browserDrawingStatus }) as any;
    if (cmd === 'vrdrawing_get_plan') return Promise.resolve(null) as any;
    if (cmd === 'vrdrawing_set_config') {
      browserDrawingStatus = { ...browserDrawingStatus, hotkeys_enabled: Boolean((args?.config as any)?.hotkeys_enabled) };
      return Promise.resolve({ ...browserDrawingStatus }) as any;
    }
    if (cmd === 'vrdrawing_start') {
      browserDrawingStatus = { ...browserDrawingStatus, running: true, paused: false, last_event: 'Drawing started' };
      return Promise.resolve({ ...browserDrawingStatus }) as any;
    }
    if (cmd === 'vrdrawing_pause' || cmd === 'vrdrawing_resume') {
      browserDrawingStatus = { ...browserDrawingStatus, paused: cmd === 'vrdrawing_pause', last_event: cmd === 'vrdrawing_pause' ? 'Drawing paused' : 'Drawing resumed' };
      return Promise.resolve({ ...browserDrawingStatus }) as any;
    }
    if (cmd === 'vrdrawing_stop') {
      browserDrawingStatus = { ...browserDrawingStatus, running: false, paused: false, last_event: 'Drawing stopped' };
      return Promise.resolve({ ...browserDrawingStatus }) as any;
    }
    if (cmd === 'vrpiano_stop') {
      browserVrpianoStatus = { ...browserVrpianoStatus, running: false, paused: false, last_event: 'Browser preview stopped' };
      return Promise.resolve({ ...browserVrpianoStatus }) as any;
    }
    if (cmd === 'vrpiano_set_speed') {
      browserVrpianoStatus = { ...browserVrpianoStatus, speed: Number(args?.speed || 1), last_event: `Browser preview speed ${Number(args?.speed || 1).toFixed(2)}x` };
      return Promise.resolve({ ...browserVrpianoStatus }) as any;
    }
    if (cmd === 'vrpiano_toggle_pause') {
      browserVrpianoStatus = {
        ...browserVrpianoStatus,
        paused: browserVrpianoStatus.running ? !browserVrpianoStatus.paused : false,
        last_event: browserVrpianoStatus.paused ? 'Browser preview resumed' : 'Browser preview paused',
      };
      return Promise.resolve({ ...browserVrpianoStatus }) as any;
    }
    if (cmd === 'vrpiano_set_hotkeys') {
      const config = (args?.config || {}) as any;
      browserVrpianoStatus = {
        ...browserVrpianoStatus,
        speed: Number(config.speed || 1),
        hotkeys_enabled: Boolean(config.enabled),
        last_event: Boolean(config.enabled) ? 'Browser preview hotkeys enabled' : 'Browser preview hotkeys disabled',
      };
      return Promise.resolve({ ...browserVrpianoStatus }) as any;
    }
    if (cmd === 'vrpiano_start') {
      browserVrpianoStatus = {
        ...browserVrpianoStatus,
        running: true,
        paused: false,
        song_name: 'Preview Song',
        song_path: String((args?.request as any)?.song_path || 'preview-1.mid'),
        total_notes: 128,
        duration_ms: 128000,
        last_event: 'Browser preview started',
      };
      return Promise.resolve({ ...browserVrpianoStatus }) as any;
    }
    if (cmd === 'vrpiano_list_songs') {
      return Promise.resolve([
        { id: 'preview-1', name: '晴天（预览）', path: 'preview-1.mid', size: 43419, modified_ms: Date.now() },
        { id: 'preview-2', name: '钟（预览）', path: 'preview-2.mid', size: 37423, modified_ms: Date.now() },
      ]) as any;
    }
    if (cmd === 'vrpiano_import_song') return Promise.resolve({ id: 'imported', name: 'Imported', path: 'imported.mid', size: 1024, modified_ms: Date.now() }) as any;
    if (cmd === 'vrpiano_rename_song') return Promise.resolve({ id: 'renamed', name: String((args?.request as any)?.new_name || 'Renamed'), path: 'renamed.mid', size: 1024, modified_ms: Date.now() }) as any;
    if (cmd === 'vrpiano_delete_song' || cmd === 'vrpiano_preview_song') return Promise.resolve(undefined) as any;
    if (cmd === 'vrpiano_read_song_data') return Promise.resolve({ name: 'Preview MIDI', data: 'TVRoZAAAAAYAAAABAGBNVHJrAAAAEwD/UQMHoSAAkDxkYIA8AAD/LwA=' }) as any;
    if (cmd === 'vrpiano_download_url') return Promise.resolve({ id: 'download-url', name: 'Downloaded URL', path: 'download-url.mid', size: 2048, modified_ms: Date.now() }) as any;
    if (cmd === 'vrpiano_search_midishow') {
      return Promise.resolve([
        { id: 10001, title: 'A Little Story', artist: 'Preview Artist', page_url: 'https://www.midishow.com/en/midi/10001.html' },
        { id: 10002, title: 'River Flows In You', artist: 'Yiruma', page_url: 'https://www.midishow.com/en/midi/10002.html' },
      ]) as any;
    }
    if (cmd === 'vrpiano_download_midishow') {
      const request = (args?.request || {}) as any;
      if (request.preview) return Promise.resolve(null) as any;
      return Promise.resolve({ id: `midishow-${request.midi_id}`, name: request.title || `MIDI_${request.midi_id}`, path: `midishow-${request.midi_id}.mid`, size: 4096, modified_ms: Date.now() }) as any;
    }
    if (cmd === 'vrpiano_midishow_preview_data') {
      const request = (args?.request || {}) as any;
      return Promise.resolve({ name: request.title || `MIDI_${request.midi_id}`, data: 'TVRoZAAAAAYAAAABAGBNVHJrAAAAEwD/UQMHoSAAkDxkYIA8AAD/LwA=' }) as any;
    }
    if (cmd === 'vrpiano_midishow_accounts') return Promise.resolve([]) as any;
    if (cmd === 'vrpiano_midishow_login') {
      const account = String((args?.request as any)?.account || 'preview');
      return Promise.resolve({ state: 'signed_in', message: '登录成功', username: account }) as any;
    }
    if (cmd === 'vrpiano_midishow_login_status') {
      return Promise.resolve({ state: 'signed_in', message: '登录成功', username: 'preview' }) as any;
    }
    if (cmd === 'vrpiano_midishow_remove_account') return Promise.resolve([]) as any;
    if (cmd === 'vrpiano_open_songs_dir') return Promise.resolve(undefined) as any;
    return Promise.resolve({}) as any;
  }
  const startTime = performance.now();

  try {
    const res = await invoke<T>(cmd, args);
    const duration = performance.now() - startTime;
    if (isDebugLogEnabled()) {
      window.dispatchEvent(new CustomEvent('app-debug-log', {
        detail: { type: 'success', cmd, args: sanitizeInvokeValue(args), duration: duration.toFixed(1), response: res, timestamp: new Date().toLocaleTimeString() }
      }));
    }
    return res;
  } catch (error: any) {
    const duration = performance.now() - startTime;
    const errorMsg = error.message || (typeof error === 'string' ? error : "Unknown backend error");
    console.error(`[Tauri API Error] ${cmd}:`, error);
    if (isDebugLogEnabled()) {
      window.dispatchEvent(new CustomEvent('app-debug-log', {
        detail: { type: 'error', cmd, args: sanitizeInvokeValue(args), duration: duration.toFixed(1), error: errorMsg, timestamp: new Date().toLocaleTimeString() }
      }));
    }
    throw new Error(errorMsg);
  }
}

export const request = baseRequest;
export const authRequest = AuthApi;
export const userRequest = UserApi;
export const friendRequest = FriendApi;
export const worldRequest = WorldApi;
export const avatarRequest = AvatarApi;
export const groupRequest = GroupApi;
export const notificationRequest = NotificationApi;
export const favoriteRequest = FavoriteApi;
export const fileRequest = FileApi;
export const playerModerationRequest = PlayerModerationApi;
export const avatarModerationRequest = AvatarModerationApi;
export const inventoryRequest = InventoryApi;
export const propRequest = PropApi;
export const inviteMessagesRequest = InviteMessagesApi;
export const miscRequest = MiscApi;
export const vrcPlusIconRequest = VrcPlusIconApi;
export const vrcPlusImageRequest = VrcPlusImageApi;
export const instanceRequest = InstanceApi;
export const imageRequest = ImageApi;
export const queryRequest = QueryRequestApi;

/**
 * [VrcDog 对齐] VRChat API 核心导出
 * 全部重构为模块化 API，同时保持 VrcApi 对象兼容性
 */
export const VrcApi = {
  // 核心请求方法 (转发到 request.ts)
  request: baseRequest,
  
  // Tauri 命令
  setProxy: (params: { proxyUrl: string | null, authCookie?: string | null }) => safeInvoke<void>('vrc_set_proxy', params),
  applyAuthCookie: (params: { authCookie: string }) => safeInvoke<void>('vrc_apply_auth_cookie', params),
  loadCookiesOnStartup: (params: { authCookie: string }) => safeInvoke<void>('vrc_load_cookies_on_startup', params),
  getImageBytes: (params: any) => safeInvoke<string>('vrc_get_image_bytes', params),
  clearCookies: () => safeInvoke('vrc_clear_cookies'),

  // VRChat realtime pipeline WebSocket (managed in Rust so it honours the proxy + UA)
  startPipelineWs: (params: { authToken: string; pipelineUrl?: string | null }) =>
    safeInvoke<void>('start_pipeline_ws', { ...params, pipelineUrl: params.pipelineUrl ?? null }),
  stopPipelineWs: () => safeInvoke<void>('stop_pipeline_ws'),

  // 认证模块
  login: AuthApi.login,
  verifyOTP: AuthApi.verifyOTP,
  verifyTOTP: AuthApi.verifyTOTP,
  verifyEmailOTP: AuthApi.verifyEmailOTP,
  verify2fa: async (params: any) => {
    const method = normalizeTwoFactorMethod(params.method) || 'totp';
    const requestParams = { code: params.code, authCookie: params.authCookie };
    if (method === 'emailOtp') return AuthApi.verifyEmailOTP(requestParams);
    if (method === 'otp') return AuthApi.verifyOTP(requestParams);
    return AuthApi.verifyTOTP(requestParams);
  },
  logout: () => safeInvoke('vrc_set_proxy', { proxyUrl: null, authCookie: null }),
  getConfig: AuthApi.getConfig,
  fetchConfig: AuthApi.getConfig,
  getServerStatus: async () => {
    // 直接使用 VRChat API /config 端点检测 API 可用性
    // status.vrchat.com 域名已不可达（~15s 超时），不再尝试
    try {
      const config: any = await baseRequest('/config', { method: 'GET' });
      return {
        status: {
          indicator: 'none',
          description: 'All Systems Operational'
        }
      };
    } catch {
      // 探测失败 ≠ VRChat 服务中断。/config 失败原因很多：
      // 网络抖动、cookie/auth 过期、代理瞬时问题、客户端前置代理拦截等，
      // 用户大概率仍能正常使用 VRChat（好友列表正常加载就是证据）。
      // 返回 unknown，让 UI 不要显示吓人的"Service Disruption"红色告警；
      // 只有 indicator 为 critical/major 才代表确认的服务中断。
      return {
        status: {
          indicator: 'unknown',
          description: 'Status unavailable'
        }
      };
    }
  },

  // 用户模块
  getCurrentUser: (params: { authCookie?: string } = {}) =>
    baseRequest('/auth/user', { authCookie: params.authCookie }),
  getUser: UserApi.getUser,
  searchUsers: (params: any) => UserApi.getUsers({ search: params.query || params.search, ...params }),
  updateStatus: UserApi.updateStatus,
  getMutualCounts: UserApi.getMutualCounts,
  getMutualFriends: UserApi.getMutualFriends,
  getMutualGroups: UserApi.getMutualGroups,
  addUserTags: UserApi.addUserTags,
  removeUserTags: UserApi.removeUserTags,
  getUserFeedback: UserApi.getUserFeedback,
  saveCurrentUser: UserApi.saveCurrentUser,
  getUserNotes: UserApi.getUserNotes,
  saveUserNote: MiscApi.saveNote,
  reportUser: MiscApi.reportUser,
  getVRChatCredits: MiscApi.getVRChatCredits,
  updateBadge: MiscApi.updateBadge,
  getVisits: MiscApi.getVisits,
  sendBoop: MiscApi.sendBoop,

  // 好友模块
  getFriends: FriendApi.getFriends,
  getAllFriends: FriendApi.getAllFriends,
  friendRequest: FriendApi.sendFriendRequest,
  sendFriendRequest: FriendApi.sendFriendRequest,
  cancelFriendRequest: FriendApi.cancelFriendRequest,
  unfriend: FriendApi.deleteFriend,
  deleteFriend: FriendApi.deleteFriend,
  getFriendStatus: FriendApi.getFriendStatus,
  deleteHiddenFriendRequest: FriendApi.deleteHiddenFriendRequest,

  // 世界模块
  getWorld: WorldApi.getWorld,
  getWorlds: WorldApi.getWorlds,
  getWorldsByUser: WorldApi.getWorldsByUser,
  searchWorlds: (params: any) => WorldApi.getWorlds({ search: params.query || params.search, ...params }),
  searchGroups: GroupApi.searchGroups,
  saveWorld: WorldApi.saveWorld,
  updateWorld: WorldApi.saveWorld,
  deleteWorld: WorldApi.deleteWorld,
  publishWorld: WorldApi.publishWorld,
  unpublishWorld: WorldApi.unpublishWorld,
  uploadWorldImage: WorldApi.uploadWorldImage,

  // 形象模块
  getAvatar: AvatarApi.getAvatar,
  getAvatars: AvatarApi.getAvatars,
  saveAvatar: AvatarApi.saveAvatar,
  updateAvatar: AvatarApi.saveAvatar,
  selectAvatar: AvatarApi.selectAvatar,
  selectFallbackAvatar: AvatarApi.selectFallbackAvatar,
  deleteAvatar: AvatarApi.deleteAvatar,
  createImposter: AvatarApi.createImposter,
  deleteImposter: AvatarApi.deleteImposter,
  getAvailableAvatarStyles: AvatarApi.getAvailableAvatarStyles,
  getAvatarGallery: AvatarApi.getAvatarGallery,
  uploadAvatarImage: AvatarApi.uploadAvatarImage,
  uploadAvatarGalleryImage: AvatarApi.uploadAvatarGalleryImage,
  setAvatarGalleryOrder: AvatarApi.setAvatarGalleryOrder,
  getLicensedAvatars: AvatarApi.getLicensedAvatars,

  // 实例模块
  createInstance: InstanceApi.createInstance,
  getInstance: InstanceApi.getInstance,
  getInstanceShortName: InstanceApi.getInstanceShortName,
  getInstanceFromShortName: InstanceApi.getInstanceFromShortName,
  selfInvite: InstanceApi.selfInvite,
  inviteMyself: InstanceApi.selfInvite,
  inviteUser: (params: any) => {
    const body: any = {};
    for (const key of ['instanceId', 'worldId', 'worldName', 'messageSlot', 'rsvp', 'platform', 'details', 'message']) {
      if (params[key] !== undefined && params[key] !== null && params[key] !== '') body[key] = params[key];
    }
    return baseRequest(`/invite/${params.userId}`, { method: 'POST', params: body });
  },
  requestInvite: (params: any) => {
    const body: any = {};
    for (const key of ['platform', 'requestSlot', 'messageSlot', 'details', 'message']) {
      if (params[key] !== undefined && params[key] !== null && params[key] !== '') body[key] = params[key];
    }
    return baseRequest(`/requestInvite/${params.userId}`, { method: 'POST', params: body });
  },
  closeInstance: MiscApi.closeInstance,
  deleteWorldPersistData: MiscApi.deleteWorldPersistData,
  hasWorldPersistData: MiscApi.hasWorldPersistData,

  // 组模块
  getGroup: GroupApi.getGroup,
  createGroup: GroupApi.createGroup,
  updateGroup: GroupApi.updateGroup,
  editGroup: GroupApi.editGroup,
  getGroups: async (params?: { userId?: string }) => {
    let userId = params?.userId;
    if (!userId) {
      const cached = await DbApi.getCachedCurrentUser();
      userId = cached?.id || (await baseRequest('/auth/user'))?.id;
    }
    if (!userId) throw new VrcRequestError('无法获取当前用户 ID', { code: 'VRCHAT_AUTH_EXPIRED', status: 401, url: '/users/<id>/groups' });
    return GroupApi.getGroups({ userId });
  },
  getGroupAnnouncement: GroupApi.getGroupAnnouncement,
  setGroupAnnouncement: GroupApi.setGroupAnnouncement,
  joinGroup: GroupApi.joinGroup,
  leaveGroup: GroupApi.leaveGroup,
  cancelGroupRequest: GroupApi.cancelGroupRequest,
  setGroupRepresentation: GroupApi.setGroupRepresentation,
  getGroupMembers: GroupApi.getGroupMembers,
  getGroupMembersSearch: GroupApi.getGroupMembersSearch,
  getGroupMember: GroupApi.getGroupMember,
  setGroupMemberProps: GroupApi.setGroupMemberProps,
  getGroupRoles: GroupApi.getGroupRoles,
  getGroupRoleTemplates: GroupApi.getGroupRoleTemplates,
  getRoleTemplates: GroupApi.getRoleTemplates,
  createGroupRole: GroupApi.createGroupRole,
  editGroupRole: GroupApi.editGroupRole,
  deleteGroupRole: GroupApi.deleteGroupRole,
  addGroupMemberRole: GroupApi.addGroupMemberRole,
  removeGroupMemberRole: GroupApi.removeGroupMemberRole,
  getGroupPosts: GroupApi.getGroupPosts,
  createGroupPost: GroupApi.createGroupPost,
  updateGroupPost: GroupApi.updateGroupPost,
  editGroupPost: GroupApi.editGroupPost,
  deleteGroupPost: GroupApi.deleteGroupPost,
  getGroupLogs: GroupApi.getGroupLogs,
  getGroupAuditLogTypes: GroupApi.getGroupAuditLogTypes,
  getGroupInvites: GroupApi.getGroupInvites,
  sendGroupInvite: GroupApi.sendGroupInvite,
  deleteGroupInvite: GroupApi.deleteGroupInvite,
  deleteSentGroupInvite: GroupApi.deleteSentGroupInvite,
  getGroupBans: GroupApi.getGroupBans,
  kickGroupMember: GroupApi.kickGroupMember,
  banGroupMember: GroupApi.banGroupMember,
  unbanGroupMember: GroupApi.unbanGroupMember,
  blockGroup: GroupApi.blockGroup,
  unblockGroup: GroupApi.unblockGroup,
  getBlockedGroups: GroupApi.getBlockedGroups,
  getUserGroupPermissions: GroupApi.getUserGroupPermissions,
  getGroupPermissions: GroupApi.getGroupPermissions,
  getGroupInstances: GroupApi.getGroupInstances,
  getUsersGroupInstances: GroupApi.getUsersGroupInstances,
  getGroupJoinRequests: GroupApi.getGroupJoinRequests,
  respondGroupJoinRequest: GroupApi.respondGroupJoinRequest,
  acceptGroupInviteRequest: GroupApi.acceptGroupInviteRequest,
  rejectGroupInviteRequest: GroupApi.rejectGroupInviteRequest,
  blockGroupInviteRequest: GroupApi.blockGroupInviteRequest,
  deleteBlockedGroupRequest: GroupApi.deleteBlockedGroupRequest,
  getRepresentedGroup: GroupApi.getRepresentedGroup,
  getGroupCalendar: GroupApi.getGroupCalendar,
  getGroupCalendarEvent: GroupApi.getGroupCalendarEvent,
  getGroupCalendars: GroupApi.getGroupCalendars,
  getFollowingGroupCalendars: GroupApi.getFollowingGroupCalendars,
  getFeaturedGroupCalendars: GroupApi.getFeaturedGroupCalendars,
  followGroupEvent: GroupApi.followGroupEvent,
  deleteGroupEvent: GroupApi.deleteGroupEvent,
  createGroupEvent: GroupApi.createGroupEvent,
  editGroupEvent: GroupApi.editGroupEvent,
  groupSearch: GroupApi.groupSearch,
  getGroupGallery: GroupApi.getGroupGallery,
  groupStrictsearch: GroupApi.groupStrictsearch,

  // 通知模块
  getNotifications: NotificationApi.getNotifications,
  clearNotifications: NotificationApi.clearNotifications,
  getHiddenFriendRequests: NotificationApi.getHiddenFriendRequests,
  acceptNotification: NotificationApi.acceptNotification,
  hideNotification: NotificationApi.hideNotification,
  seeNotification: NotificationApi.seeNotification,
  getNotificationsV2: NotificationApi.getNotificationsV2,
  clearNotificationsV2: NotificationApi.clearNotificationsV2,
  sendInviteNotification: NotificationApi.sendInvite,
  sendInvitePhoto: NotificationApi.sendInvitePhoto,
  sendRequestInviteNotification: NotificationApi.sendRequestInvite,
  sendRequestInvitePhoto: NotificationApi.sendRequestInvitePhoto,
  sendInviteResponse: NotificationApi.sendInviteResponse,
  sendInviteResponsePhoto: NotificationApi.sendInviteResponsePhoto,
  seeNotificationV2: NotificationApi.seeNotificationV2,
  sendNotificationResponse: NotificationApi.sendNotificationResponse,
  deleteNotificationV2: NotificationApi.deleteNotificationV2,
  hideNotificationV2: NotificationApi.hideNotificationV2,

  // 收藏模块
  getFavorites: FavoriteApi.getFavorites,
  getFavoriteWorlds: FavoriteApi.getFavoriteWorlds,
  getFavoriteAvatars: FavoriteApi.getFavoriteAvatars,
  getFavoriteGroups: FavoriteApi.getFavoriteGroups,
  addFavorite: FavoriteApi.addFavorite,
  deleteFavorite: FavoriteApi.deleteFavorite,
  removeFavorite: FavoriteApi.removeFavorite,
  saveFavoriteGroup: FavoriteApi.saveFavoriteGroup,
  clearFavoriteGroup: FavoriteApi.clearFavoriteGroup,
  getFavoriteLimits: FavoriteApi.getFavoriteLimits,

  // 文件模块
  getFile: FileApi.getFile,
  deleteFile: FileApi.deleteFile,
  deleteFileVersion: FileApi.deleteFileVersion,
  getFileAnalysis: FileApi.getFileAnalysis,
  miscGetFile: MiscApi.getFile,
  miscDeleteFile: MiscApi.deleteFile,
  queryFetch: QueryRequestApi.fetch,
  uploadAvatarFailCleanup: ImageApi.uploadAvatarFailCleanup,
  uploadAvatarImageFileStart: ImageApi.uploadAvatarImageFileStart,
  uploadAvatarImageFileFinish: ImageApi.uploadAvatarImageFileFinish,
  uploadAvatarImageSigStart: ImageApi.uploadAvatarImageSigStart,
  uploadAvatarImageSigFinish: ImageApi.uploadAvatarImageSigFinish,
  setAvatarImage: ImageApi.setAvatarImage,
  uploadWorldFailCleanup: ImageApi.uploadWorldFailCleanup,
  uploadWorldImageFileStart: ImageApi.uploadWorldImageFileStart,
  uploadWorldImageFileFinish: ImageApi.uploadWorldImageFileFinish,
  uploadWorldImageSigStart: ImageApi.uploadWorldImageSigStart,
  uploadWorldImageSigFinish: ImageApi.uploadWorldImageSigFinish,
  setWorldImage: ImageApi.setWorldImage,
  getAvatarImages: ImageApi.getAvatarImages,
  getWorldImages: ImageApi.getWorldImages,

  // 播放器操作
  getModerations: PlayerModerationApi.getPlayerModerations,
  getPlayerModerations: PlayerModerationApi.getPlayerModerations,
  moderateUser: PlayerModerationApi.sendPlayerModeration,
  unmoderateUser: PlayerModerationApi.deletePlayerModeration,
  getAvatarModerations: AvatarModerationApi.getAvatarModerations,
  sendAvatarModeration: AvatarModerationApi.sendAvatarModeration,
  deleteAvatarModeration: AvatarModerationApi.deleteAvatarModeration,
  getInventoryItem: InventoryApi.getInventoryItem,
  getUserInventoryItem: InventoryApi.getUserInventoryItem,
  getInventoryItems: InventoryApi.getInventoryItems,
  consumeInventoryBundle: InventoryApi.consumeInventoryBundle,
  getInventoryTemplate: InventoryApi.getInventoryTemplate,
  redeemReward: InventoryApi.redeemReward,
  getGlobalInventory: InventoryApi.getGlobalInventory,
  getEquipSlot: InventoryApi.getEquipSlot,
  equipItem: InventoryApi.equipItem,
  archiveInventoryItem: InventoryApi.archiveItem,
  unarchiveInventoryItem: InventoryApi.unarchiveItem,
  unArchiveItem: InventoryApi.unArchiveItem,
  getProp: PropApi.getProp,
  refreshInviteMessageTableData: InviteMessagesApi.refreshInviteMessageTableData,
  getInviteMessages: InviteMessagesApi.getInviteMessages,
  editInviteMessage: InviteMessagesApi.editInviteMessage,
  getVrcPlusFiles: VrcPlusIconApi.getFileList,
  deleteVrcPlusFile: VrcPlusIconApi.deleteFile,
  deleteVrcPlusFileVersion: VrcPlusIconApi.deleteFileVersion,
  uploadVrcPlusIcon: VrcPlusIconApi.uploadVrcPlusIcon,
  uploadVRCPlusIcon: VrcPlusIconApi.uploadVRCPlusIcon,
  uploadGalleryImage: VrcPlusImageApi.uploadGalleryImage,
  uploadSticker: VrcPlusImageApi.uploadSticker,
  uploadEmoji: VrcPlusImageApi.uploadEmoji,
  getPrints: VrcPlusImageApi.getPrints,
  getPrint: VrcPlusImageApi.getPrint,
  deletePrint: VrcPlusImageApi.deletePrint,
  uploadPrint: VrcPlusImageApi.uploadPrint,
  createPrint: VrcPlusImageApi.createPrint,
  
  // 图片上传
  uploadVrcPlusImage: async (base64Data: string, tag: string = 'gallery', entityId?: string) => {
    const cleanBase64 = toCleanBase64(base64Data);
    const authCookie = await getStoredAuthCookie();
    const data: any = { tag };
    if (entityId) data[tag === 'avatargallery' ? 'galleryId' : 'entityId'] = entityId;
    const formData: any[] = [
      { name: 'data', value: JSON.stringify(data) },
      { name: 'image', file_name: 'image.png', file_content_base64: cleanBase64, file_mime: 'image/png' }
    ];

    const res: any = await safeInvoke('vrc_execute', {
      options: { url: 'https://api.vrchat.cloud/api/1/file/image', method: 'POST', auth_cookie: authCookie, form_data: formData }
    });
    return parseExecuteResponse(res, 'https://api.vrchat.cloud/api/1/file/image');
  }
};

export const DbApi = {
  getAuth: () => safeInvoke<string | null>('db_get_auth'),
  saveAuth: (params: any) => safeInvoke<void>('db_save_auth', params),
  clearAuth: () => safeInvoke<void>('db_clear_auth'),
  clearGameLogs: () => safeInvoke<void>('db_clear_game_logs'),
  clearFriendLogs: () => safeInvoke<void>('db_clear_friend_logs'),
  getHeatmap: () => safeInvoke<any[]>('db_get_heatmap'),
  getHeatmapDetails: (params: { day: number, hour: number }) => safeInvoke<any[]>('db_get_heatmap_details', params),
  getNotes: () => safeInvoke<any[]>('db_get_all_notes'),
  getNote: (params: any) => safeInvoke<any>('db_get_note', params),
  saveNote: (params: any) => safeInvoke<void>('db_save_note', params),
  getPresets: () => safeInvoke<any[]>('db_get_status_presets'),
  savePreset: (params: any) => safeInvoke<void>('db_save_status_preset', params),
  deletePreset: (params: any) => safeInvoke<void>('db_delete_status_preset', params),
  exportAll: () => safeInvoke<any>('db_export_all'),
  recordActivity: (params: any) => safeInvoke<void>('db_record_activity', params),
  batchRecordFriends: (params: any) => safeInvoke<number>('db_batch_record_friends', params),
  addFriendLog: (params: any) => safeInvoke<void>('db_add_friend_log', params),
  getFriendLogs: (params: any) => safeInvoke<any[]>('db_get_friend_logs', params),
  saveSetting: (params: { key: string; value: string }) => safeInvoke<void>('db_save_setting', params),
  saveSettings: (params: { settings: Array<[string, string]> }) => safeInvoke<void>('db_save_settings', params),
  getSetting: (params: { key: string }) => safeInvoke<string | null>('db_get_setting', params),
  getAllSettings: () => safeInvoke<any>('db_get_all_settings'),
  saveFriend: (params: any) => safeInvoke<void>('db_save_friend', params),
  batchSaveFriends: (params: { friendsJson: string }) => safeInvoke<number>('db_batch_save_friends', params),
  getCachedFriends: () => safeInvoke<any[]>('db_get_friends'),
  getCachedCurrentUser: async (): Promise<any | null> => {
    try {
      const raw = await safeInvoke<string | null>('db_get_setting', { key: 'cached_vrc_user' });
      if (!raw) return null;
      const parsed = JSON.parse(raw);
      if (parsed && parsed.expiresAt && Date.now() < parsed.expiresAt) {
        return parsed.user || null;
      }
    } catch {
      // ignore cache read errors
    }
    return null;
  },
  removeFriend: (params: { userId: string }) => safeInvoke<void>('db_remove_friend', params),
  saveGameLogs: (params: { logsJson: string }) => safeInvoke<number>('db_save_game_logs', params),
  getGameLogs: (params: { limit?: number; offset?: number }) => safeInvoke<any[]>('db_get_game_logs', params),
  saveNotification: (params: { notificationJson: string }) => safeInvoke<void>('db_save_notification', params),
  batchSaveNotifications: (params: { notificationsJson: string }) => safeInvoke<number>('db_batch_save_notifications', params),
  getNotifications: (params: { limit?: number; offset?: number }) => safeInvoke<any[]>('db_get_notifications', params),
  deleteNotification: (params: { id: string }) => safeInvoke<void>('db_delete_notification', params),
  addFavoriteWorld: (params: { worldId: string; name: string; imageUrl?: string | null }) => safeInvoke<void>('db_add_favorite_world', params),
  getFavoriteWorlds: () => safeInvoke<any[]>('db_get_favorite_worlds'),
  removeFavoriteWorld: (params: { worldId: string }) => safeInvoke<void>('db_remove_favorite_world', params),
  addFavoriteAvatar: (params: { avatarId: string; name: string; imageUrl?: string | null; authorId?: string | null; authorName?: string | null }) => safeInvoke<void>('db_add_favorite_avatar', params),
  getFavoriteAvatars: () => safeInvoke<any[]>('db_get_favorite_avatars'),
  removeFavoriteAvatar: (params: { avatarId: string }) => safeInvoke<void>('db_remove_favorite_avatar', params),
  getApiCache: (params: { key: string }) => safeInvoke<string | null>('db_get_api_cache', params),
  saveApiCache: (params: { key: string, data: string }) => safeInvoke<void>('db_save_api_cache', params),
};

export type AudioSource = 'mic' | 'speaker';

export interface AudioDevice {
  id: string;
  index: number;
  name: string;
  source: AudioSource;
  is_default: boolean;
  sample_rate: number;
  channels: number;
}

export interface AudioCaptureConfig {
  source: AudioSource;
  sourceLang: string;
  engine: 'cloud' | 'local' | 'whisper' | 'sensevoice';
  deviceIndex?: number;
  energyThreshold?: number;
  dynamicEnergyThreshold?: boolean;
  phraseTimeLimit?: number;
  whisperModel?: string;
  vadType?: string;
  vadAggressiveness?: number;
  denoiseStrength?: number;
  correctionEnabled?: boolean;
  minSegmentS?: number;
  maxSegmentS?: number;
  partialInterval?: number;
  captureMode?: string;
  targetProcess?: string;
  selfSuppressSeconds?: number;
}

export interface AudioCaptureStatus {
  source: AudioSource;
  running: boolean;
}

export const SysApi = {
  checkSystemStatus: () => safeInvoke<any>('check_system_status'),
  checkSteamVR: () => safeInvoke<boolean>('sys_check_steamvr'),
  synthesizeGptSovits: (params: {
    baseUrl: string;
    text: string;
    textLanguage: string;
    sovitsWeights?: string;
    gptWeights?: string;
    referenceAudio?: string;
    promptText?: string;
    promptLanguage?: string;
  }) => safeInvoke<string>('sys_gpt_sovits_synthesize', { request: params }),
  installSoftware: (params: any) => safeInvoke<void>('install_software', params),
  uninstallSoftware: (params: any) => safeInvoke<void>('uninstall_software', params),
  launchSoftware: (params: any) => safeInvoke<void>('launch_software', params),
  clearVrcCache: () => safeInvoke<number>('sys_clear_vrchat_cache'),
  isVrcRunning: () => safeInvoke<boolean>('sys_is_vrchat_running'),
  launchVrc: (params?: { launchArgs?: string }) => safeInvoke<void>('sys_launch_vrchat', params),
  killVrc: () => safeInvoke<void>('sys_kill_vrchat'),
  sendOscParam: (params: { address: string; value: number }) => safeInvoke<void>('sys_send_osc_param', params),
  sendOscChatbox: (params: { text: string; complete: boolean; delaySecs?: number }) => safeInvoke<void>('sys_send_osc_chatbox', params),
  setDiscordRpc: (params: { details: string; state: string; showWorldThumbnail?: boolean; showJoinButton?: boolean }) => safeInvoke<void>('sys_set_discord_rpc', params),
  saveTextFile: (params: { path: string; content: string }) => safeInvoke<void>('sys_save_text_file', params),
  saveBinaryFile: (params: { path: string; content: number[] }) => safeInvoke<void>('sys_save_binary_file', params),
  getAudioDevices: () => safeInvoke<AudioDevice[]>('vrct_get_audio_devices'),
  startAudioCapture: (params: AudioCaptureConfig) => safeInvoke<void>('vrct_start_audio_capture', { ...params }),
  stopAudioCapture: (params: { source: AudioSource }) => safeInvoke<void>('vrct_stop_audio_capture', params),
  setAudioCapturePaused: (params: { source: AudioSource; paused: boolean }) => safeInvoke<void>('vrct_set_audio_capture_paused', params),
  getAudioCaptureStatus: () => safeInvoke<AudioCaptureStatus[]>('vrct_get_audio_capture_status'),
  startOscAutomation: () => safeInvoke<void>('sys_start_osc_automation'),
  stopOscAutomation: () => safeInvoke<void>('sys_stop_osc_automation'),
  startAutoLaunchApps: (params: { apps: string[] }) => safeInvoke<void>('sys_start_auto_launch_apps', params),
  killAutoLaunchApps: () => safeInvoke<void>('sys_kill_auto_launch_apps'),
  showInExplorer: (params: { path: string }) => safeInvoke<void>('sys_show_in_explorer', params),
  verifyServerPassword: (params: { password: string }) => safeInvoke<void>('sys_verify_server_password', params),
  startServer: (params: { host: string, port: number }) => safeInvoke<void>('sys_start_server', params),
  stopServer: () => safeInvoke<void>('sys_stop_server'),
  isServerRunning: () => safeInvoke<boolean>('sys_is_server_running'),
  pingServer: (params: { url: string }) => safeInvoke<string>('sys_ping_server', params),
  openNewClient: () => safeInvoke<void>('sys_open_new_client'),
  setAutostart: (params: { enable: boolean }) => safeInvoke<void>('sys_set_autostart', params),
  registerUrlScheme: (params: { enable: boolean }) => safeInvoke<void>('sys_register_url_scheme', params),
  getLaunchArgs: () => safeInvoke<string[]>('sys_get_launch_args'),
  getClientServerConfig: () => safeInvoke<{ server_url: string; config_path: string }>('sys_get_client_server_config'),
  saveClientServerConfig: (params: { serverUrl: string }) => safeInvoke<{ server_url: string; config_path: string }>('sys_save_client_server_config', params),
  openDir: (params: { target: string }) => safeInvoke<void>('sys_open_dir', params),
  openUrl: (params: { url: string }) => safeInvoke<void>('sys_open_url', params),
  getVrcScreenshotDir: () => safeInvoke<string>('sys_get_vrc_screenshot_dir'),
  setVrcScreenshotDir: (params: { path: string }) => safeInvoke<void>('sys_set_vrc_screenshot_dir', params),
  getVrcConfig: () => safeInvoke<string>('sys_get_vrc_config'),
  saveVrcConfig: (params: { content: string }) => safeInvoke<void>('sys_save_vrc_config', params),
  backupDatabase: (params: { destPath: string }) => safeInvoke<void>('sys_backup_database', params),
  restoreDatabase: (params: { srcPath: string }) => safeInvoke<void>('sys_restore_database', params),
};

export type OscValueType = 'float' | 'double' | 'int' | 'long' | 'bool' | 'string' | 'impulse';

export interface OscMonitorArgument {
  valueType: string;
  value: unknown;
}

export interface OscMonitorEvent {
  address: string;
  args: OscMonitorArgument[];
  sender: string;
  timestamp: string;
}

export interface OscSystemSnapshot {
  cpuUsage: number;
  cpuName: string;
  cpuPhysicalCores: number;
  cpuLogicalCores: number;
  cpuFrequencyMhz: number;
  ramUsage: number;
  memoryUsedGb: number;
  memoryTotalGb: number;
  gpuName: string;
  gpuUsage: number | null;
  gpuMemoryUsedGb: number | null;
  gpuMemoryTotalGb: number | null;
  diskUsage: number;
  diskUsedGb: number;
  diskTotalGb: number;
  osName: string;
  hostName: string;
  systemUptimeSeconds: number;
  idleSeconds: number;
  activeWindow: string;
  localTime: string;
  localDate: string;
  vrcRunning: boolean;
}

export interface OscAutomationMapping {
  enabled: boolean;
  address: string;
  source: string;
  scale: number;
  offset: number;
  min?: number | null;
  max?: number | null;
  valueType: OscValueType;
}

export interface OscAutomationConfig {
  host: string;
  port: number;
  intervalMs: number;
  mappings: OscAutomationMapping[];
}

export interface OscRouteRule {
  enabled: boolean;
  sourceAddress: string;
  targetHost: string;
  targetPort: number;
  targetAddress: string;
}

export const OscApi = {
  sendMessage: (params: { host: string; port: number; address: string; valueType: OscValueType; value: unknown }) =>
    safeInvoke<void>('osc_send_message', params),
  sendChatbox: (params: { host: string; port: number; text: string; send: boolean; notify: boolean; delaySecs?: number }) =>
    safeInvoke<void>('osc_send_chatbox', params),
  startMonitor: (params: { host: string; port: number; routes?: OscRouteRule[] }) =>
    safeInvoke<void>('osc_start_monitor', params),
  stopMonitor: () => safeInvoke<void>('osc_stop_monitor'),
  getSystemSnapshot: () => safeInvoke<OscSystemSnapshot>('osc_get_system_snapshot'),
  startAutomation: (params: { config: OscAutomationConfig }) =>
    safeInvoke<void>('osc_start_automation', params),
  stopAutomation: () => safeInvoke<void>('osc_stop_automation'),
  getStatus: () => safeInvoke<{ monitorRunning: boolean; automationRunning: boolean }>('osc_get_status'),
};

export const GamelogApi = {
  getLatestGamelogs: (params: any) => safeInvoke<any[]>('vrc_get_latest_gamelogs', params),
  getSnapshot: (params: { maxLines?: number } = {}) => safeInvoke<any[]>('vrc_get_gamelog_snapshot', params),
};

export const GalleryApi = {
  getImages: (params?: { limit?: number; offset?: number }) => safeInvoke<any[]>('gallery_get_images', params || {}),
  deleteImage: (params: { path: string }) => safeInvoke<void>('gallery_delete_image', params),
};

export const OvrApi = {
  init: () => safeInvoke<any>('ovr_init'),
  shutdown: () => safeInvoke<void>('ovr_shutdown'),
  getStatus: () => safeInvoke<any>('ovr_get_status'),
  setConfig: (params: { config: any }) => safeInvoke<void>('ovr_set_config', params),
  setVrMenuTheme: (params: { accent: string; bg: string; text: string; muted: string }) =>
    safeInvoke<void>('ovr_set_menu_theme', params),
  syncOvrasIni: (params: { payload: string }) => safeInvoke<void>('ovr_sync_ovras_ini', params),
  loadOvrasIni: () => safeInvoke<string>('ovr_load_ovras_ini'),
  toggleTranslation: () => safeInvoke<boolean>('ovr_toggle_translation'),
  toggleMenu: () => safeInvoke<void>('ovr_toggle_menu'),
  openBindingUi: () => safeInvoke<void>('ovr_open_binding_ui'),
  captureScreenshot: () => safeInvoke<string>('ovr_capture_screenshot'),
  updateOverlayText: (params: { original: string; translated: string }) => safeInvoke<void>('ovr_update_overlay_text', params),
  setOverlayVisible: (params: { visible: boolean }) => safeInvoke<void>('ovr_set_overlay_visible', params),
  clearTranslation: () => safeInvoke<void>('ovr_clear_translation'),
  translate: (params: { req: any }) => safeInvoke<any>('ovr_translate', params),
  desktopScanOnce: () => safeInvoke<void>('ovr_desktop_scan_once'),
  startAutoScan: () => safeInvoke<void>('ovr_start_auto_scan'),
  stopAutoScan: () => safeInvoke<void>('ovr_stop_auto_scan'),
  // ===== Native Playspace Control (replaces OVRAS dependency) =====
  setPlayspaceOffset: (params: { x: number; y: number; z: number }) => safeInvoke<void>('ovr_set_playspace_offset', params),
  setPlayspaceRotation: (params: { degrees: number }) => safeInvoke<void>('ovr_set_playspace_rotation', params),
  toggleHeight: () => safeInvoke<void>('ovr_toggle_height'),
  resetPlayspace: () => safeInvoke<void>('ovr_reset_playspace'),
  fixFloor: () => safeInvoke<void>('ovr_fix_floor'),
};

export interface DanmakuConfig {
  enable_bilibili: boolean;
  room_id: number;
  bili_sessdata?: string;
  enable_osc_input: boolean;
  osc_input_host: string;
  osc_input_port: number;
  osc_input_address: string;
  enable_osc_output: boolean;
  osc_output_host: string;
  osc_output_port: number;
  osc_output_address: string;
  enable_vrc_chatbox: boolean;
  vrc_chatbox_port: number;
  chatbox_interval_ms: number;
  enable_vr_overlay: boolean;
  overlay_visible: boolean;
  vr_menu_visible: boolean;
  attach_mode: string;
  toggle_hand: string;
  x: number;
  y: number;
  z: number;
  pitch: number;
  yaw: number;
  roll: number;
  overlay_width_m: number;
  overlay_alpha: number;
  bg_alpha: number;
  font_size: number;
  text_color: string;
  bg_color: string;
  max_messages: number;
  show_danmaku: boolean;
  show_gift: boolean;
  show_enter: boolean;
  show_follow: boolean;
  show_guard: boolean;
  show_sc: boolean;
  vr_input_text: string;
}

export interface DanmakuStatus {
  running: boolean;
  bili_connected: boolean;
  osc_input_running: boolean;
  vr_initialized: boolean;
  overlay_visible: boolean;
  vr_menu_visible: boolean;
  room_id: number;
  online: number;
  message_count: number;
  last_error: string;
  last_event: string;
  vr_input_text: string;
  vr_keyboard_open: boolean;
}

export interface DanmakuMessage {
  id: number;
  source: string;
  message_type: string;
  user: string;
  text: string;
  price?: number | null;
  gift_count?: number | null;
  medal_name?: string | null;
  medal_level?: number | null;
  guard_level?: number | null;
  timestamp_ms: number;
}

export const DanmakuApi = {
  getConfig: () => safeInvoke<DanmakuConfig>('danmaku_get_config'),
  getStatus: () => safeInvoke<DanmakuStatus>('danmaku_get_status'),
  getMessages: () => safeInvoke<DanmakuMessage[]>('danmaku_get_messages'),
  setConfig: (params: { config: DanmakuConfig }) => safeInvoke<DanmakuStatus>('danmaku_set_config', params),
  start: (params: { config: DanmakuConfig }) => safeInvoke<DanmakuStatus>('danmaku_start', params),
  stop: () => safeInvoke<DanmakuStatus>('danmaku_stop'),
  clearMessages: () => safeInvoke<void>('danmaku_clear_messages'),
  setOverlayVisible: (params: { visible: boolean }) => safeInvoke<DanmakuStatus>('danmaku_set_overlay_visible', params),
  setVrInputText: (params: { text: string }) => safeInvoke<DanmakuStatus>('danmaku_set_vr_input_text', params),
  submitVrInput: (params: { text?: string }) => safeInvoke<DanmakuMessage>('danmaku_submit_vr_input', params),
  sendTest: (params: { messageType: string; text?: string }) => safeInvoke<DanmakuMessage>('danmaku_send_test', params),
};

export const VrctApi = {
  processMessage: (params: { req: any }) => safeInvoke<any>('vrct_process_message', params),
  getHistory: () => safeInvoke<any[]>('vrct_get_history'),
  clearHistory: () => safeInvoke<void>('vrct_clear_history'),
};

export interface VrpianoSong {
  id: string;
  name: string;
  path: string;
  size: number;
  modified_ms: number;
  /**
   * 与 MIDI 同目录的封面图绝对路径（`<basename>.cover.{jpg,png,webp}`），
   * 由 Midishow 下载时自动落盘或 list_local_songs 探测得到。
   * 可通过 `convertFileSrc(cover_path)` 在 WebView 中显示。
   */
  cover_path?: string | null;
}

export interface VrpianoStatus {
  running: boolean;
  paused: boolean;
  song_name: string;
  song_path: string;
  progress: number;
  played_notes: number;
  total_notes: number;
  duration_ms: number;
  elapsed_ms: number;
  last_event: string;
  last_error: string;
  songs_dir: string;
  speed: number;
  hotkeys_enabled: boolean;
  hotkeys_available: boolean;
  last_hotkey: string;
  last_hotkey_at_ms: number;
}

export interface VrpianoOnlineSong {
  id: number;
  title: string;
  artist: string;
  page_url: string;
  /** Midishow 列表项的封面图 URL（已校验为 midishow 域或带图片后缀）。 */
  cover_url?: string | null;
}

export interface VrpianoMidishowAccount {
  username: string;
  login_type?: string;
}

export interface VrpianoMidishowLoginStatus {
  state: 'idle' | 'opening' | 'waiting' | 'needs_confirmation' | 'signed_in' | 'failed';
  message: string;
  username?: string | null;
}

export interface VrpianoMidiData {
  name: string;
  data: string;
}

export const VrpianoApi = {
  init: () => safeInvoke<VrpianoStatus>('vrpiano_init'),
  listSongs: () => safeInvoke<VrpianoSong[]>('vrpiano_list_songs'),
  importSong: (params: { sourcePath: string }) => safeInvoke<VrpianoSong>('vrpiano_import_song', { sourcePath: params.sourcePath }),
  renameSong: (params: { songPath: string; newName: string; overwrite?: boolean }) => safeInvoke<VrpianoSong>('vrpiano_rename_song', {
    request: {
      song_path: params.songPath,
      new_name: params.newName,
      overwrite: Boolean(params.overwrite),
    },
  }),
  deleteSong: (params: { songPath: string }) => safeInvoke<void>('vrpiano_delete_song', { songPath: params.songPath }),
  previewSong: (params: { songPath: string }) => safeInvoke<void>('vrpiano_preview_song', { songPath: params.songPath }),
  readSongData: (params: { songPath: string }) => safeInvoke<VrpianoMidiData>('vrpiano_read_song_data', { songPath: params.songPath }),
  downloadUrl: (params: { url: string; filename?: string }) => safeInvoke<VrpianoSong>('vrpiano_download_url', {
    request: {
      url: params.url,
      filename: params.filename || null,
    },
  }),
  searchMidishow: (params: { keyword: string; maxResults?: number }) => safeInvoke<VrpianoOnlineSong[]>('vrpiano_search_midishow', {
    keyword: params.keyword,
    maxResults: params.maxResults || 30,
  }),
  downloadMidishow: (params: { midiId: number; title?: string; preview?: boolean; coverUrl?: string | null }) => safeInvoke<VrpianoSong | null>('vrpiano_download_midishow', {
    request: {
      midi_id: params.midiId,
      title: params.title || null,
      preview: Boolean(params.preview),
      cover_url: params.coverUrl || null,
    },
  }),
  midishowPreviewData: (params: { midiId: number; title?: string }) => safeInvoke<VrpianoMidiData>('vrpiano_midishow_preview_data', {
    request: {
      midi_id: params.midiId,
      title: params.title || null,
      preview: true,
    },
  }),
  midishowAccounts: () => safeInvoke<VrpianoMidishowAccount[]>('vrpiano_midishow_accounts'),
  midishowLogin: (params: { account: string; password: string }) => safeInvoke<VrpianoMidishowLoginStatus>('vrpiano_midishow_login', {
    request: {
      account: params.account,
      password: params.password,
    },
  }),
  midishowLoginStatus: () => safeInvoke<VrpianoMidishowLoginStatus>('vrpiano_midishow_login_status'),
  midishowRemoveAccount: (params: { username: string }) => safeInvoke<VrpianoMidishowAccount[]>('vrpiano_midishow_remove_account', {
    username: params.username,
  }),
  openSongsDir: () => safeInvoke<void>('vrpiano_open_songs_dir'),
  getStatus: () => safeInvoke<VrpianoStatus>('vrpiano_get_status'),
  start: (params: { songPath: string; delaySecs: number; speed: number }) => safeInvoke<VrpianoStatus>('vrpiano_start', {
    request: {
      song_path: params.songPath,
      delay_secs: params.delaySecs,
      speed: params.speed,
    }
  }),
  stop: () => safeInvoke<VrpianoStatus>('vrpiano_stop'),
  togglePause: () => safeInvoke<VrpianoStatus>('vrpiano_toggle_pause'),
  setSpeed: (params: { speed: number }) => safeInvoke<VrpianoStatus>('vrpiano_set_speed', params),
  setHotkeys: (params: { enabled: boolean; songPath: string; delaySecs: number; speed: number }) => safeInvoke<VrpianoStatus>('vrpiano_set_hotkeys', {
    config: {
      enabled: params.enabled,
      song_path: params.songPath,
      delay_secs: params.delaySecs,
      speed: params.speed,
    },
  }),
};

export interface DrawingPoint {
  x: number;
  y: number;
}

export interface DrawingStroke {
  points: DrawingPoint[];
}

export interface DrawingConfig {
  mode: 'lineart' | 'edges' | 'dither' | 'ai';
  max_dimension: number;
  threshold: number;
  blur: number;
  invert: boolean;
  bridge_gaps: boolean;
  prune_length: number;
  min_stroke_length: number;
  smooth_window: number;
  simplify_epsilon: number;
  merge_distance: number;
  optimize_path: boolean;
  sensitivity: number;
  vertical_stretch: number;
  max_step_px: number;
  point_delay_ms: number;
  lift_delay_ms: number;
  start_delay_ms: number;
  focus_vrchat: boolean;
  hotkeys_enabled: boolean;
  ai_model: string;
  contrast: number;
  artifact_removal: number;
  model_size: number;
  lift_speed: number;
  pen_settle_ms: number;
  canvas_size_px: number;
  two_opt_path: boolean;
}

export interface PreparedDrawing {
  source_path: string;
  width: number;
  height: number;
  strokes: DrawingStroke[];
  total_points: number;
}

export interface DrawingStatus {
  prepared: boolean;
  running: boolean;
  paused: boolean;
  progress: number;
  current_stroke: number;
  total_strokes: number;
  total_points: number;
  source_path: string;
  last_event: string;
  last_error: string;
  hotkeys_enabled: boolean;
  hotkeys_available: boolean;
  last_hotkey: string;
  last_hotkey_at_ms: number;
  stage: string;
}

export const DrawingApi = {
  prepare: (params: { sourcePath: string; config: DrawingConfig }) => safeInvoke<PreparedDrawing>('vrdrawing_prepare', {
    sourcePath: params.sourcePath,
    config: params.config,
  }),
  getPlan: () => safeInvoke<PreparedDrawing | null>('vrdrawing_get_plan'),
  getStatus: () => safeInvoke<DrawingStatus>('vrdrawing_get_status'),
  setConfig: (params: { config: DrawingConfig }) => safeInvoke<DrawingStatus>('vrdrawing_set_config', params),
  start: () => safeInvoke<DrawingStatus>('vrdrawing_start'),
  pause: () => safeInvoke<DrawingStatus>('vrdrawing_pause'),
  resume: () => safeInvoke<DrawingStatus>('vrdrawing_resume'),
  stop: () => safeInvoke<DrawingStatus>('vrdrawing_stop'),
};

export async function uploadSurveyFile(
  serverUrl: string,
  params: { user_id: string; survey_id: string; question_id: string },
  file: File,
): Promise<SurveyAnswerAttachment> {
  const base = serverUrl.replace(/\/+$/, '');
  const query = new URLSearchParams({
    user_id: params.user_id,
    survey_id: params.survey_id,
    question_id: params.question_id,
    file_name: file.name,
  });
  const res = await fetch(`${base}/api/client/surveys/upload?${query.toString()}`, {
    method: 'POST',
    headers: { 'Content-Type': file.type || 'application/octet-stream' },
    body: file,
  });
  if (!res.ok) {
    let detail = '';
    try {
      detail = (await res.json())?.message || '';
    } catch {
      /* ignore parse errors */
    }
    throw new Error(detail || `附件上传失败（${res.status}）`);
  }
  return (await res.json()) as SurveyAnswerAttachment;
}

export {
  AuthApi,
  UserApi,
  FriendApi,
  WorldApi,
  AvatarApi,
  GroupApi,
  NotificationApi,
  FavoriteApi,
  FileApi,
  PlayerModerationApi,
  AvatarModerationApi,
  InventoryApi,
  PropApi,
  InviteMessagesApi,
  MiscApi,
  VrcPlusIconApi,
  VrcPlusImageApi,
};
