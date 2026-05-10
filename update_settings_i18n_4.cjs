const fs = require('fs');
const path = require('path');

const keysToAdd = {
  'settings': {
    'top_window': { 'zh-CN': '窗口置顶', 'en-US': 'Always on Top' },
    'top_window_desc': { 'zh-CN': '保持应用程序窗口总在最前面', 'en-US': 'Keep application window always on top' },
    'theme': { 'zh-CN': '主题 (Theme)', 'en-US': 'Theme' },
    'theme_light': { 'zh-CN': '明亮 (Light)', 'en-US': 'Light' },
    'theme_dark': { 'zh-CN': '暗黑 (Dark)', 'en-US': 'Dark' },
    'theme_system': { 'zh-CN': '跟随系统', 'en-US': 'System Default' },
    'desktop_notification_title': { 'zh-CN': '桌面通知', 'en-US': 'Desktop Notifications' },
    'when_to_show_desktop_notification': { 'zh-CN': '何时显示桌面通知', 'en-US': 'When to show desktop notifications' },
    'never': { 'zh-CN': '永不 (Never)', 'en-US': 'Never' },
    'in_desktop': { 'zh-CN': '在桌面模式时 (Desktop)', 'en-US': 'In Desktop Mode' },
    'in_vr': { 'zh-CN': '在 VR 里时 (In VR)', 'en-US': 'In VR' },
    'not_in_vr': { 'zh-CN': '在 VR 外时 (Not in VR)', 'en-US': 'Not in VR' },
    'vrc_running': { 'zh-CN': 'VRChat 运行时 (VRChat running)', 'en-US': 'VRChat is running' },
    'vrc_not_running': { 'zh-CN': 'VRChat 未运行时 (VRChat not running)', 'en-US': 'VRChat not running' },
    'always': { 'zh-CN': '总是 (Always)', 'en-US': 'Always' },
    'show_desktop_notify_when_afk': { 'zh-CN': '当处于 AFK 状态时显示桌面通知', 'en-US': 'Show desktop notification when AFK' },
    'tts_options_title': { 'zh-CN': '文字转语音选项 (TTS)', 'en-US': 'Text-to-Speech Options (TTS)' },
    'when_to_use_tts': { 'zh-CN': '何时使用文字转语音', 'en-US': 'When to use Text-to-Speech' },
    'tts_vol': { 'zh-CN': 'TTS 音量', 'en-US': 'TTS Volume' },
    'play_test_tts': { 'zh-CN': '播放测试 TTS', 'en-US': 'Play Test TTS' },
    'send_test_notify': { 'zh-CN': '发送测试通知', 'en-US': 'Send Test Notification' },
    'notify_tts': { 'zh-CN': '启用 TTS 语音播报', 'en-US': 'Enable TTS Voice Announcer' },
    'notify_tts_desc': { 'zh-CN': '当收到重要通知时，自动通过系统语音合成播报', 'en-US': 'Automatically read aloud important notifications using system voice synthesis' },
    'tts_volume': { 'zh-CN': 'TTS 语音音量', 'en-US': 'TTS Volume' },
    'discord_rpc_title': { 'zh-CN': 'Discord 状态面板', 'en-US': 'Discord Rich Presence' },
    'discord_rpc_only_vrc': { 'zh-CN': '仅在 VRChat 运行时生效', 'en-US': 'Only effective when VRChat is running' },
    'discord_rpc_conflict_warning': { 'zh-CN': '建议在 VRChat 的 “config.json” 中停用原生的 Discord 状态面板来防止冲突', 'en-US': 'It is recommended to disable native Discord RPC in VRChat\'s config.json to prevent conflicts' },
    'discord_rpc_world_integration': { 'zh-CN': '打开与特定世界的集成', 'en-US': 'Enable integration with specific worlds' },
    'discord_rpc_world_integration_desc': { 'zh-CN': '为 Popcorn Palace、PyPyDance、VRDancing 和 LS Media 显示“正在观看/正在收听”的状态', 'en-US': 'Show "Watching/Listening" status for Popcorn Palace, PyPyDance, VRDancing, and LS Media' },
    'discord_rpc_show_room_type': { 'zh-CN': '显示房间类型和人数', 'en-US': 'Show room type and player count' },
    'discord_rpc_show_platform': { 'zh-CN': '显示当前所在的平台', 'en-US': 'Show current platform' },
    'discord_rpc_show_private_info': { 'zh-CN': '在私人房间时显示房间信息', 'en-US': 'Show room info in private instances' },
    'discord_rpc_show_join_btn': { 'zh-CN': '在面板上显示加入按钮 (仅限公开房间)', 'en-US': 'Show Join button on profile (Public instances only)' },
    'discord_rpc_show_thumbnail': { 'zh-CN': '显示世界缩略图', 'en-US': 'Show world thumbnail' },
    'discord_rpc_show_world_name': { 'zh-CN': '在 Discord 状态中显示世界名称', 'en-US': 'Show world name in Discord status' },
    'translation_api_title': { 'zh-CN': '翻译 API', 'en-US': 'Translation API' },
    'enable': { 'zh-CN': '启用', 'en-US': 'Enable' }
  }
};

const localesDir = path.join(__dirname, 'src/i18n/locales');
const files = fs.readdirSync(localesDir).filter(f => f.endsWith('.json'));

for (const file of files) {
  const filePath = path.join(localesDir, file);
  const locale = file.replace('.json', '');
  const data = JSON.parse(fs.readFileSync(filePath, 'utf-8'));
  
  if (!data['settings']) data['settings'] = {};
  
  for (const [key, translations] of Object.entries(keysToAdd['settings'])) {
    if (!data['settings'][key]) {
      data['settings'][key] = translations[locale] || translations['en-US'];
    }
  }
  
  fs.writeFileSync(filePath, JSON.stringify(data, null, 2), 'utf-8');
}
console.log('Updated i18n locales for SettingsView part 4');
