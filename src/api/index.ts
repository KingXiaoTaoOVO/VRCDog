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
import { request as baseRequest } from './request';

// [VRCX 对齐] Cookie 合并工具 — VRCX 的 CookieContainer 自动合并同名 cookie
async function mergeCookiesAndSave(newCookieJson: string | null | undefined): Promise<void> {
  if (!newCookieJson) return;
  try {
    const newCookies: string[] = JSON.parse(newCookieJson);
    if (!Array.isArray(newCookies) || newCookies.length === 0) return;

    let existing: string[] = [];
    try {
      const stored = await invoke<string | null>('db_get_auth');
      if (stored) {
        const parsed = JSON.parse(stored);
        if (Array.isArray(parsed)) existing = parsed;
      }
    } catch { /* no existing cookies */ }

    const cookieMap = new Map<string, string>();
    for (const c of existing) {
      const name = c.split('=')[0];
      if (name) cookieMap.set(name, c);
    }
    for (const c of newCookies) {
      const name = c.split('=')[0];
      if (name) cookieMap.set(name, c);
    }

    const merged = Array.from(cookieMap.values());
    await invoke('db_save_auth', { cookie: JSON.stringify(merged) });
  } catch { /* ignore merge errors */ }
}

export async function safeInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauri()) {
    console.warn(`[Browser Mock] API Command: ${cmd}`, args);
    if (cmd === 'vrc_execute') return Promise.resolve({ status: 200, data: '{}' }) as any;
    if (cmd === 'vrc_get_server_status') return Promise.resolve({ status: { description: 'All Systems Operational' } }) as any;
    if (cmd === 'db_get_auth') return Promise.resolve('mock_auth_cookie_abc123') as any;
    if (cmd === 'check_system_status') return Promise.resolve({ hub_installed: true, unity_installed: true, tool_installed: true, vcc_installed: true, alcom_installed: false }) as any;
    if (cmd === 'vrc_fetch_config' || cmd === 'db_getAllSettings' || cmd === 'db_get_all_settings') return Promise.resolve({}) as any;
    if (cmd === 'vrc_get_friends' || cmd === 'vrc_search_users' || cmd === 'vrc_search_worlds' || cmd === 'vrc_get_notifications' || cmd === 'vrc_get_avatars' || cmd === 'db_get_status_presets' || cmd === 'vrc_get_latest_gamelogs' || cmd === 'db_get_all_notes') {
      return Promise.resolve([]) as any;
    }
    return Promise.resolve({}) as any;
  }
  const startTime = performance.now();

  const sanitizeArgs = (originalArgs?: Record<string, unknown>) => {
    if (!originalArgs) return originalArgs;
    const safe: any = JSON.parse(JSON.stringify(originalArgs));
    if (safe.password) safe.password = '***';
    if (safe.authCookie) safe.authCookie = '***';
    if (safe.cookie) safe.cookie = '***';
    if (safe.options?.headers?.Authorization) safe.options.headers.Authorization = '***';
    if (safe.options?.auth_cookie) safe.options.auth_cookie = '***';
    return safe;
  };

  try {
    const res = await invoke<T>(cmd, args);
    const duration = performance.now() - startTime;
    window.dispatchEvent(new CustomEvent('app-debug-log', {
      detail: { type: 'success', cmd, args: sanitizeArgs(args), duration: duration.toFixed(1), response: res, timestamp: new Date().toLocaleTimeString() }
    }));
    return res;
  } catch (error: any) {
    const duration = performance.now() - startTime;
    const errorMsg = error.message || (typeof error === 'string' ? error : "Unknown backend error");
    console.error(`[Tauri API Error] ${cmd}:`, error);
    window.dispatchEvent(new CustomEvent('app-debug-log', {
      detail: { type: 'error', cmd, args: sanitizeArgs(args), duration: duration.toFixed(1), error: errorMsg, timestamp: new Date().toLocaleTimeString() }
    }));
    throw new Error(errorMsg);
  }
}

/**
 * [VRCX 对齐] VRChat API 核心导出
 * 全部重构为模块化 API，同时保持 VrcApi 对象兼容性
 */
export const VrcApi = {
  // 核心请求方法 (转发到 request.ts)
  request: baseRequest,
  
  // Tauri 命令
  setProxy: (params: { proxyUrl: string | null, authCookie?: string | null }) => safeInvoke<void>('vrc_set_proxy', params),
  getImageBytes: (params: any) => safeInvoke<string>('vrc_get_image_bytes', params),
  clearCookies: () => safeInvoke('vrc_clear_cookies'),

  // 认证模块
  login: AuthApi.login,
  verifyOTP: AuthApi.verifyOTP,
  verifyTOTP: AuthApi.verifyTOTP,
  verifyEmailOTP: AuthApi.verifyEmailOTP,
  verify2fa: async (params: any) => {
    const method = params.method?.toLowerCase() || 'totp';
    if (method === 'emailotp') return AuthApi.verifyEmailOTP({ code: params.code });
    if (method === 'otp') return AuthApi.verifyOTP({ code: params.code });
    return AuthApi.verifyTOTP({ code: params.code });
  },
  logout: () => safeInvoke('vrc_set_proxy', { proxyUrl: null, authCookie: null }),
  getConfig: AuthApi.getConfig,
  fetchConfig: AuthApi.getConfig,
  getServerStatus: () => baseRequest('https://status.vrchat.com/api/v2/status.json', { method: 'GET', headers: { Referer: 'https://vrcx.app' } }),

  // 用户模块
  getCurrentUser: () => baseRequest('/auth/user'),
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

  // 好友模块
  getFriends: FriendApi.getFriends,
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
  searchWorlds: (params: any) => WorldApi.getWorlds({ search: params.query || params.search, ...params }),
  searchGroups: GroupApi.searchGroups,
  saveWorld: WorldApi.saveWorld,
  updateWorld: WorldApi.saveWorld,
  deleteWorld: WorldApi.deleteWorld,
  publishWorld: WorldApi.publishWorld,
  unpublishWorld: WorldApi.unpublishWorld,

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
  getLicensedAvatars: AvatarApi.getLicensedAvatars,

  // 实例模块
  createInstance: (params: any) => baseRequest('/instances', { method: 'POST', params }),
  getInstance: (params: any) => baseRequest(`/instances/${params.worldId}:${params.instanceId}`),
  inviteMyself: (params: any) => baseRequest(`/invite/myself/to/${params.worldId}:${params.instanceId}`, { method: 'POST' }),
  inviteUser: (params: any) => baseRequest(`/invite/${params.userId}`, { method: 'POST', params: { instanceId: params.instanceId } }),
  requestInvite: (params: any) => baseRequest(`/requestInvite/${params.userId}`, { method: 'POST' }),
  closeInstance: (params: any) => baseRequest(`/instances/${params.location}`, { method: 'DELETE' }),

  // 组模块
  getGroup: GroupApi.getGroup,
  getGroups: async () => {
    const user: any = await baseRequest('/auth/user');
    return GroupApi.getGroups({ userId: user.id });
  },
  joinGroup: GroupApi.joinGroup,
  leaveGroup: GroupApi.leaveGroup,
  getGroupMembers: GroupApi.getGroupMembers,
  getGroupRoles: GroupApi.getGroupRoles,
  getGroupPosts: GroupApi.getGroupPosts,
  createGroupPost: GroupApi.createGroupPost,
  getGroupLogs: GroupApi.getGroupLogs,
  sendGroupInvite: GroupApi.sendGroupInvite,
  kickGroupMember: GroupApi.kickGroupMember,
  banGroupMember: GroupApi.banGroupMember,
  unbanGroupMember: GroupApi.unbanGroupMember,
  getUserGroupPermissions: GroupApi.getUserGroupPermissions,
  getGroupJoinRequests: GroupApi.getGroupJoinRequests,
  respondGroupJoinRequest: GroupApi.respondGroupJoinRequest,
  getRepresentedGroup: GroupApi.getRepresentedGroup,

  // 通知模块
  getNotifications: NotificationApi.getNotifications,
  acceptNotification: NotificationApi.acceptNotification,
  hideNotification: NotificationApi.hideNotification,
  getNotificationsV2: NotificationApi.getNotificationsV2,

  // 收藏模块
  getFavorites: FavoriteApi.getFavorites,
  getFavoriteWorlds: FavoriteApi.getFavoriteWorlds,
  getFavoriteAvatars: FavoriteApi.getFavoriteAvatars,
  getFavoriteGroups: FavoriteApi.getFavoriteGroups,
  addFavorite: FavoriteApi.addFavorite,
  deleteFavorite: FavoriteApi.removeFavorite,

  // 文件模块
  getFile: FileApi.getFile,
  deleteFile: FileApi.deleteFile,
  getFileAnalysis: FileApi.getFileAnalysis,

  // 播放器操作
  getModerations: () => baseRequest('/auth/user/playermoderations'),
  moderateUser: (params: any) => baseRequest('/auth/user/playermoderations', { method: 'POST', params }),
  unmoderateUser: (params: any) => baseRequest('/auth/user/unplayermoderate', { method: 'PUT', params }),
  
  // 图片上传
  uploadVrcPlusImage: async (base64Data: string, tag: string = 'gallery', entityId?: string) => {
    const cleanBase64 = base64Data.includes(',') ? base64Data.split(',')[1] : base64Data;
    const formData: any[] = [
      { name: 'tag', value: tag },
      { name: 'file', file_name: 'image.png', file_content_base64: cleanBase64, file_mime: 'image/png' }
    ];
    if (entityId) formData.push({ name: tag === 'avatargallery' ? 'galleryId' : 'entityId', value: entityId });

    const res: any = await safeInvoke('vrc_execute', {
      options: { url: 'https://api.vrchat.cloud/api/1/file/image', method: 'POST', form_data: formData }
    });
    return res.data ? JSON.parse(res.data) : res;
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
  getSetting: (params: { key: string }) => safeInvoke<string | null>('db_get_setting', params),
  getAllSettings: () => safeInvoke<any>('db_get_all_settings'),
  saveFriend: (params: any) => safeInvoke<void>('db_save_friend', params),
  batchSaveFriends: (params: { friendsJson: string }) => safeInvoke<number>('db_batch_save_friends', params),
  getCachedFriends: () => safeInvoke<any[]>('db_get_friends'),
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

export const SysApi = {
  checkSystemStatus: () => safeInvoke<any>('check_system_status'),
  checkSteamVR: () => safeInvoke<boolean>('sys_check_steamvr'),
  installSoftware: (params: any) => safeInvoke<void>('install_software', params),
  uninstallSoftware: (params: any) => safeInvoke<void>('uninstall_software', params),
  launchSoftware: (params: any) => safeInvoke<void>('launch_software', params),
  clearVrcCache: () => safeInvoke<number>('sys_clear_vrchat_cache'),
  isVrcRunning: () => safeInvoke<boolean>('sys_is_vrchat_running'),
  launchVrc: (params?: { launchArgs?: string }) => safeInvoke<void>('sys_launch_vrchat', params),
  killVrc: () => safeInvoke<void>('sys_kill_vrchat'),
  sendOscParam: (params: { address: string; value: number }) => safeInvoke<void>('sys_send_osc_param', params),
  sendOscChatbox: (params: { text: string; complete: boolean }) => safeInvoke<void>('sys_send_osc_chatbox', params),
  setDiscordRpc: (params: { details: string; state: string }) => safeInvoke<void>('sys_set_discord_rpc', params),
  saveTextFile: (params: { path: string; content: string }) => safeInvoke<void>('sys_save_text_file', params),
  startAudioCapture: (params: { sourceLang: string, engine: string }) => safeInvoke<void>('sys_start_audio_capture', params),
  stopAudioCapture: () => safeInvoke<void>('sys_stop_audio_capture'),
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
  openDir: (params: { target: string }) => safeInvoke<void>('sys_open_dir', params),
  getVrcScreenshotDir: () => safeInvoke<string>('sys_get_vrc_screenshot_dir'),
  setVrcScreenshotDir: (params: { path: string }) => safeInvoke<void>('sys_set_vrc_screenshot_dir', params),
  getVrcConfig: () => safeInvoke<string>('sys_get_vrc_config'),
  saveVrcConfig: (params: { content: string }) => safeInvoke<void>('sys_save_vrc_config', params),
  backupDatabase: (params: { destPath: string }) => safeInvoke<void>('sys_backup_database', params),
  restoreDatabase: (params: { srcPath: string }) => safeInvoke<void>('sys_restore_database', params),
};

export const GamelogApi = {
  getLatestGamelogs: (params: any) => safeInvoke<any[]>('vrc_get_latest_gamelogs', params),
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
  toggleTranslation: () => safeInvoke<boolean>('ovr_toggle_translation'),
  captureScreenshot: () => safeInvoke<string>('ovr_capture_screenshot'),
  updateOverlayText: (params: { original: string; translated: string }) => safeInvoke<void>('ovr_update_overlay_text', params),
  setOverlayVisible: (params: { visible: boolean }) => safeInvoke<void>('ovr_set_overlay_visible', params),
  clearTranslation: () => safeInvoke<void>('ovr_clear_translation'),
  translate: (params: { req: any }) => safeInvoke<any>('ovr_translate', params),
  desktopScanOnce: () => safeInvoke<void>('ovr_desktop_scan_once'),
  startAutoScan: () => safeInvoke<void>('ovr_start_auto_scan'),
  stopAutoScan: () => safeInvoke<void>('ovr_stop_auto_scan'),
};

export { AuthApi, UserApi, FriendApi, WorldApi, AvatarApi, GroupApi, NotificationApi, FavoriteApi, FileApi };
