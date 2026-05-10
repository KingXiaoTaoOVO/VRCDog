const fs = require('fs');
let content = fs.readFileSync('src/components/SettingsView.vue', 'utf8');

const replacements = {
  "<Settings :size=\"16\" /> {{ t('settings.nav_vr') || 'VR 叠加层' }}": "<Settings :size=\"16\" /> {{ t('settings.nav_vr') }}",
  "<Settings :size=\"16\" /> OCR 图像识别": "<Settings :size=\"16\" /> {{ t('settings.nav_ocr') }}",
  "<Globe :size=\"16\" /> 翻译服务引擎": "<Globe :size=\"16\" /> {{ t('settings.nav_translation') }}",
  "软件更新": "{{ t('settings.software_update') }}",
  "检查并下载最新版本的 VrcDog": "{{ t('settings.software_update_desc') }}",
  "{{ checkUpdateStatus || (t('settings.check_update') || '检查更新') }}": "{{ checkUpdateStatus || t('settings.check_update') }}",
  "桌面悬浮球": "{{ t('settings.desktop_overlay') }}",
  "在桌面上显示一个可以拖拽的快捷悬浮球": "{{ t('settings.desktop_overlay_desc') }}",
  "系统托盘图标": "{{ t('settings.system_tray') }}",
  "最小化时隐藏到系统托盘": "{{ t('settings.system_tray_desc') }}",
  "退出确认提示": "{{ t('settings.exit_confirm') }}",
  "点击关闭按钮时弹出确认对话框": "{{ t('settings.exit_confirm_desc') }}",
  "深色模式": "{{ t('settings.dark_mode') }}",
  "启用深蓝色调夜间主题": "{{ t('settings.dark_mode_desc') }}",
  "主题切换": "{{ t('settings.theme_switch') }}",
  "语言设置": "{{ t('settings.language') }}",
  "Discord 状态展示 (RPC)": "{{ t('settings.discord_rpc') }}",
  "在 Discord 个人资料展示你所在的 VRChat 世界": "{{ t('settings.discord_rpc_desc') }}",
  "开机自动启动": "{{ t('settings.auto_start') }}",
  "SteamVR 自动启动": "{{ t('settings.steamvr_auto_start') }}",
  "随 SteamVR 一起自动启动 VrcDog": "{{ t('settings.steamvr_auto_start_desc') }}",
  "向 SteamVR 注册清单": "{{ t('settings.steamvr_register') }}",
  "如果 VrcDog 未能随 SteamVR 启动，请点击此按钮注册 App Manifest": "{{ t('settings.steamvr_register_desc') }}",
  "注册 Manifest": "{{ t('settings.steamvr_register_btn') }}",
  "SteamVR 键位绑定": "{{ t('settings.steamvr_bindings') }}",
  "打开 SteamVR 控制器键位设置面板 (必须在运行 SteamVR 时有效)": "{{ t('settings.steamvr_bindings_desc') }}",
  "打开键位设置": "{{ t('settings.steamvr_bindings_btn') }}",
  "VRChat 游戏日志解析": "{{ t('settings.vrc_log_parse') }}",
  "启用后才能在动态页面看到游戏内进出房间记录": "{{ t('settings.vrc_log_parse_desc') }}",
  "OSC 本地监听端口": "{{ t('settings.osc_receive_port') }}",
  "接收 VRChat 参数的端口 (默认 9001)": "{{ t('settings.osc_receive_port_desc') }}",
  "OSC 发送端口": "{{ t('settings.osc_send_port') }}",
  "发送参数到 VRChat 的端口 (默认 9000)": "{{ t('settings.osc_send_port_desc') }}",
  "静音状态同步": "{{ t('settings.mute_sync') }}",
  "使用 VRChat 的静音状态控制麦克风": "{{ t('settings.mute_sync_desc') }}",
  "声音通知提醒": "{{ t('settings.sound_notify') }}",
  "有新消息或好友上线时播放提示音": "{{ t('settings.sound_notify_desc') }}",
  "桌面消息弹窗": "{{ t('settings.desktop_notify') }}",
  "在 Windows 桌面右下角弹出通知提示": "{{ t('settings.desktop_notify_desc') }}",
  "测试桌面通知": "{{ t('settings.test_desktop_notify') }}",
  "语音播报 (TTS)": "{{ t('settings.tts') }}",
  "将游戏内通知或聊天消息转为语音读出": "{{ t('settings.tts_desc') }}",
  "播报音量": "{{ t('settings.tts_volume') }}",
  "播报员声音": "{{ t('settings.tts_voice') }}",
  "测试语音播报": "{{ t('settings.test_tts') }}",
  "测试发音": "{{ t('settings.test_tts_btn') }}"
};

let modified = false;
for (const [key, val] of Object.entries(replacements)) {
  if (content.includes(key)) {
    content = content.replace(new RegExp(key.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'), 'g'), val);
    modified = true;
  }
}

if (modified) {
  fs.writeFileSync('src/components/SettingsView.vue', content, 'utf8');
}
console.log('Replaced more strings');
