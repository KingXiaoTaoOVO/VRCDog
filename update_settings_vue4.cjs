const fs = require('fs');
let content = fs.readFileSync('src/components/SettingsView.vue', 'utf8');

const replacements = {
  "{{ t('settings.top_window') || '窗口置顶' }}": "{{ t('settings.top_window') }}",
  "{{ t('settings.top_window_desc') || '保持应用程序窗口总在最前面' }}": "{{ t('settings.top_window_desc') }}",
  "简体中文": "简体中文",
  "English": "English",
  "日本語": "日本語",
  "{{ t('settings.theme') || '主题 (Theme)' }}": "{{ t('settings.theme') }}",
  "{{ t('settings.theme_light') || '明亮 (Light)' }}": "{{ t('settings.theme_light') }}",
  "{{ t('settings.theme_dark') || '暗黑 (Dark)' }}": "{{ t('settings.theme_dark') }}",
  "{{ t('settings.theme_system') || '跟随系统' }}": "{{ t('settings.theme_system') }}",
  "桌面通知": "{{ t('settings.desktop_notification_title') }}",
  "何时显示桌面通知": "{{ t('settings.when_to_show_desktop_notification') }}",
  "永不 (Never)": "{{ t('settings.never') }}",
  "在桌面模式时 (Desktop)": "{{ t('settings.in_desktop') }}",
  "在 VR 里时 (In VR)": "{{ t('settings.in_vr') }}",
  "在 VR 外时 (Not in VR)": "{{ t('settings.not_in_vr') }}",
  "VRChat 运行时 (VRChat running)": "{{ t('settings.vrc_running') }}",
  "VRChat 未运行时 (VRChat not running)": "{{ t('settings.vrc_not_running') }}",
  "总是 (Always)": "{{ t('settings.always') }}",
  "当处于 AFK 状态时显示桌面通知": "{{ t('settings.show_desktop_notify_when_afk') }}",
  "文字转语音选项 (TTS)": "{{ t('settings.tts_options_title') }}",
  "何时使用文字转语音": "{{ t('settings.when_to_use_tts') }}",
  "TTS 音量": "{{ t('settings.tts_vol') }}",
  "播放测试 TTS": "{{ t('settings.play_test_tts') }}",
  "发送测试通知": "{{ t('settings.send_test_notify') }}",
  "{{ t('settings.notify_tts') || '{{ t('settings.state_on') }}启 TTS 语音播报' }}": "{{ t('settings.notify_tts') }}",
  "{{ t('settings.notify_tts_desc') || '当收到重要通知时，自动通过系统语音合成播报' }}": "{{ t('settings.notify_tts_desc') }}",
  "{{ t('settings.tts_volume') || 'TTS 语音{{ t('settings.tts_volume') }}' }}": "{{ t('settings.tts_volume') }}",
  "Discord 状态面板": "{{ t('settings.discord_rpc_title') }}",
  "仅在 VRChat 运行时生效": "{{ t('settings.discord_rpc_only_vrc') }}",
  "建议在 VRChat 的 “config.json” 中停用原生的 Discord 状态面板来防止冲突": "{{ t('settings.discord_rpc_conflict_warning') }}",
  "打{{ t('settings.state_on') }}与特定世界的集成": "{{ t('settings.discord_rpc_world_integration') }}",
  "为 Popcorn Palace、PyPyDance、VRDancing 和 LS Media 显示“正在观看/正在收听”的状态": "{{ t('settings.discord_rpc_world_integration_desc') }}",
  "显示房间类型和人数": "{{ t('settings.discord_rpc_show_room_type') }}",
  "显示当前所在的平台": "{{ t('settings.discord_rpc_show_platform') }}",
  "在私人房间时显示房间信息": "{{ t('settings.discord_rpc_show_private_info') }}",
  "在面板上显示加入按钮 (仅限公{{ t('settings.state_on') }}房间)": "{{ t('settings.discord_rpc_show_join_btn') }}",
  "显示世界缩略图": "{{ t('settings.discord_rpc_show_thumbnail') }}",
  "在 Discord 状态中显示世界名称": "{{ t('settings.discord_rpc_show_world_name') }}",
  "翻译 API": "{{ t('settings.translation_api_title') }}"
};

let modified = false;
for (const [key, val] of Object.entries(replacements)) {
  if (content.includes(key)) {
    content = content.replace(new RegExp(key.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'), 'g'), val);
    modified = true;
  }
}

// Additional specific replacements for "启用" since it appears multiple times
content = content.replace(/<div class="text-sm font-bold text-slate-800">启用<\/div>/g, '<div class="text-sm font-bold text-slate-800">{{ t(\'settings.enable\') }}</div>');

if (modified) {
  fs.writeFileSync('src/components/SettingsView.vue', content, 'utf8');
}
console.log('Replaced part 4 settings strings');
