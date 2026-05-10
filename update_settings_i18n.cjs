const fs = require('fs');
const path = require('path');

const keysToAdd = {
  'settings': {
    'vrc_config_invalid': { 'zh-CN': 'JSON 格式错误 (Invalid JSON format)', 'en-US': 'Invalid JSON format' },
    'vrc_config_saved': { 'zh-CN': '保存成功! (Saved successfully)', 'en-US': 'Saved successfully!' },
    'vrc_config_failed': { 'zh-CN': '写入失败 (Failed to write)', 'en-US': 'Failed to write' },
    'select_dir_title': { 'zh-CN': '选择目录', 'en-US': 'Select Directory' },
    'steamvr_register_success': { 'zh-CN': '成功向 SteamVR 注册自动启动!', 'en-US': 'Successfully registered SteamVR auto-start!' },
    'steamvr_register_fail': { 'zh-CN': '注册 SteamVR 自动启动失败: {error}', 'en-US': 'Failed to register SteamVR auto-start: {error}' },
    'steamvr_bindings_fail': { 'zh-CN': '打开 SteamVR 键位面板失败: {error}', 'en-US': 'Failed to open SteamVR bindings panel: {error}' },
    'update_checking': { 'zh-CN': '检查更新中...', 'en-US': 'Checking for updates...' },
    'update_found': { 'zh-CN': '发现新版本 v{version}！\n\n{body}\n\n是否立即下载并更新？', 'en-US': 'New version v{version} available!\n\n{body}\n\nDownload and update now?' },
    'update_downloading': { 'zh-CN': '正在下载更新...', 'en-US': 'Downloading update...' },
    'update_cancelled': { 'zh-CN': '已取消更新', 'en-US': 'Update cancelled' },
    'update_latest': { 'zh-CN': '当前已是最新版本', 'en-US': 'You are on the latest version' },
    'update_failed': { 'zh-CN': '检查更新失败: {error}', 'en-US': 'Update check failed: {error}' },
    'test_tts_msg': { 'zh-CN': '这是一条测试语音通知', 'en-US': 'This is a test voice notification' },
    'test_notify_title': { 'zh-CN': 'VrcDog 测试', 'en-US': 'VrcDog Test' },
    'test_notify_msg': { 'zh-CN': '这是一条测试桌面通知！如果能看到我，说明通知正常工作。', 'en-US': 'This is a test desktop notification! If you see this, notifications are working properly.' },
    'nav_integration': { 'zh-CN': '集成', 'en-US': 'Integration' },
    'nav_auto_start': { 'zh-CN': '自动启动程序', 'en-US': 'Auto Start Programs' },
    'nav_advanced': { 'zh-CN': '高级', 'en-US': 'Advanced' },
    'nav_hardware': { 'zh-CN': '硬件与OSC', 'en-US': 'Hardware & OSC' },
    'nav_general': { 'zh-CN': '常规设置', 'en-US': 'General' },
    'nav_theme': { 'zh-CN': '主题与外观', 'en-US': 'Appearance' },
    'nav_notify': { 'zh-CN': '通知与语音', 'en-US': 'Notifications' },
    'nav_vrc_config': { 'zh-CN': 'VRChat 配置', 'en-US': 'VRChat Config' },
    'nav_about': { 'zh-CN': '关于软件', 'en-US': 'About' }
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
console.log('Updated i18n locales for SettingsView part 1');