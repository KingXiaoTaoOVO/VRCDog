const fs = require('fs');
const path = require('path');

const keysToAdd = {
  'settings': {
    'nav_vr': { 'zh-CN': 'VR 叠加层', 'en-US': 'VR Overlay' },
    'nav_ocr': { 'zh-CN': 'OCR 图像识别', 'en-US': 'OCR Recognition' },
    'nav_translation': { 'zh-CN': '翻译服务引擎', 'en-US': 'Translation Engine' },
    'software_update': { 'zh-CN': '软件更新', 'en-US': 'Software Update' },
    'software_update_desc': { 'zh-CN': '检查并下载最新版本的 VrcDog', 'en-US': 'Check and download the latest version of VrcDog' },
    'check_update': { 'zh-CN': '检查更新', 'en-US': 'Check for Updates' },
    'desktop_overlay': { 'zh-CN': '桌面悬浮球', 'en-US': 'Desktop Overlay' },
    'desktop_overlay_desc': { 'zh-CN': '在桌面上显示一个可以拖拽的快捷悬浮球', 'en-US': 'Show a draggable quick overlay on the desktop' },
    'system_tray': { 'zh-CN': '系统托盘图标', 'en-US': 'System Tray Icon' },
    'system_tray_desc': { 'zh-CN': '最小化时隐藏到系统托盘', 'en-US': 'Hide to system tray when minimized' },
    'exit_confirm': { 'zh-CN': '退出确认提示', 'en-US': 'Exit Confirmation' },
    'exit_confirm_desc': { 'zh-CN': '点击关闭按钮时弹出确认对话框', 'en-US': 'Show confirmation dialog when clicking the close button' },
    'dark_mode': { 'zh-CN': '深色模式', 'en-US': 'Dark Mode' },
    'dark_mode_desc': { 'zh-CN': '启用深蓝色调夜间主题', 'en-US': 'Enable dark blue night theme' },
    'theme_switch': { 'zh-CN': '主题切换', 'en-US': 'Theme Switch' },
    'language': { 'zh-CN': '语言设置', 'en-US': 'Language' },
    'discord_rpc': { 'zh-CN': 'Discord 状态展示 (RPC)', 'en-US': 'Discord Rich Presence (RPC)' },
    'discord_rpc_desc': { 'zh-CN': '在 Discord 个人资料展示你所在的 VRChat 世界', 'en-US': 'Show your current VRChat world in your Discord profile' },
    'auto_start': { 'zh-CN': '开机自动启动', 'en-US': 'Run on Startup' },
    'steamvr_auto_start': { 'zh-CN': 'SteamVR 自动启动', 'en-US': 'SteamVR Auto Start' },
    'steamvr_auto_start_desc': { 'zh-CN': '随 SteamVR 一起自动启动 VrcDog', 'en-US': 'Start VrcDog automatically with SteamVR' },
    'steamvr_register': { 'zh-CN': '向 SteamVR 注册清单', 'en-US': 'Register SteamVR Manifest' },
    'steamvr_register_desc': { 'zh-CN': '如果 VrcDog 未能随 SteamVR 启动，请点击此按钮注册 App Manifest', 'en-US': 'If VrcDog fails to start with SteamVR, click this to register App Manifest' },
    'steamvr_register_btn': { 'zh-CN': '注册 Manifest', 'en-US': 'Register Manifest' },
    'steamvr_bindings': { 'zh-CN': 'SteamVR 键位绑定', 'en-US': 'SteamVR Bindings' },
    'steamvr_bindings_desc': { 'zh-CN': '打开 SteamVR 控制器键位设置面板 (必须在运行 SteamVR 时有效)', 'en-US': 'Open SteamVR controller bindings panel (SteamVR must be running)' },
    'steamvr_bindings_btn': { 'zh-CN': '打开键位设置', 'en-US': 'Open Bindings' },
    'vrc_log_parse': { 'zh-CN': 'VRChat 游戏日志解析', 'en-US': 'VRChat Game Log Parser' },
    'vrc_log_parse_desc': { 'zh-CN': '启用后才能在动态页面看到游戏内进出房间记录', 'en-US': 'Required to see room join/leave events in the Feed page' },
    'osc_receive_port': { 'zh-CN': 'OSC 本地监听端口', 'en-US': 'OSC Local Receive Port' },
    'osc_receive_port_desc': { 'zh-CN': '接收 VRChat 参数的端口 (默认 9001)', 'en-US': 'Port to receive VRChat parameters (default 9001)' },
    'osc_send_port': { 'zh-CN': 'OSC 发送端口', 'en-US': 'OSC Send Port' },
    'osc_send_port_desc': { 'zh-CN': '发送参数到 VRChat 的端口 (默认 9000)', 'en-US': 'Port to send parameters to VRChat (default 9000)' },
    'mute_sync': { 'zh-CN': '静音状态同步', 'en-US': 'Mute Status Sync' },
    'mute_sync_desc': { 'zh-CN': '使用 VRChat 的静音状态控制麦克风', 'en-US': 'Control microphone using VRChat mute status' },
    'sound_notify': { 'zh-CN': '声音通知提醒', 'en-US': 'Sound Notification' },
    'sound_notify_desc': { 'zh-CN': '有新消息或好友上线时播放提示音', 'en-US': 'Play sound when receiving new messages or friends come online' },
    'desktop_notify': { 'zh-CN': '桌面消息弹窗', 'en-US': 'Desktop Notification' },
    'desktop_notify_desc': { 'zh-CN': '在 Windows 桌面右下角弹出通知提示', 'en-US': 'Show notification popups in the Windows system tray' },
    'test_desktop_notify': { 'zh-CN': '测试桌面通知', 'en-US': 'Test Desktop Notification' },
    'tts': { 'zh-CN': '语音播报 (TTS)', 'en-US': 'Text-to-Speech (TTS)' },
    'tts_desc': { 'zh-CN': '将游戏内通知或聊天消息转为语音读出', 'en-US': 'Read out game notifications or chat messages' },
    'tts_volume': { 'zh-CN': '播报音量', 'en-US': 'TTS Volume' },
    'tts_voice': { 'zh-CN': '播报员声音', 'en-US': 'TTS Voice' },
    'test_tts': { 'zh-CN': '测试语音播报', 'en-US': 'Test TTS' },
    'test_tts_btn': { 'zh-CN': '测试发音', 'en-US': 'Test Voice' },
    'hardware_osc': { 'zh-CN': '硬件与OSC', 'en-US': 'Hardware & OSC' }
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
console.log('Updated i18n locales for SettingsView part 2');
