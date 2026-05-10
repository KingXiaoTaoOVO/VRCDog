const fs = require('fs');
let content = fs.readFileSync('src/components/SettingsView.vue', 'utf8');

const replacements = [
  // Alerts and status strings
  ["vrcConfigError.value = 'JSON 格式错误 (Invalid JSON format)';", "vrcConfigError.value = t('settings.vrc_config_invalid');"],
  ["vrcConfigError.value = '保存成功! (Saved successfully)';", "vrcConfigError.value = t('settings.vrc_config_saved');"],
  ["if (vrcConfigError.value.includes('成功')) vrcConfigError.value = '';", "if (vrcConfigError.value === t('settings.vrc_config_saved')) vrcConfigError.value = '';"],
  ["vrcConfigError.value = '写入失败 (Failed to write)';", "vrcConfigError.value = t('settings.vrc_config_failed');"],
  ["title: '选择目录 (Select Directory)'", "title: t('settings.select_dir_title')"],
  ["transLlmPrompt: '你是一个翻译专家，综合所有的 OCR 乱入文本，给出现在最好的目标语言。'", "transLlmPrompt: t('ovr.trans_llm_prompt_default')"],
  ["alert('成功向 SteamVR 注册自动启动!');", "alert(t('settings.steamvr_register_success'));"],
  ["alert('注册 SteamVR 自动启动失败: ' + err);", "alert(t('settings.steamvr_register_fail').replace('{error}', err));"],
  ["alert('打开 SteamVR 键位面板失败: ' + err);", "alert(t('settings.steamvr_bindings_fail').replace('{error}', err));"],
  ["checkUpdateStatus.value = silent ? '' : '检查更新中...';", "checkUpdateStatus.value = silent ? '' : t('settings.update_checking');"],
  ["if (confirm(\发现新版本 v\！\\n\\n\\\n\\n是否立即下载并更新？\)) {", "if (confirm(t('settings.update_found').replace('{version}', update.version).replace('{body}', update.body))) {"],
  ["checkUpdateStatus.value = '正在下载更新...';", "checkUpdateStatus.value = t('settings.update_downloading');"],
  ["checkUpdateStatus.value = '已取消更新';", "checkUpdateStatus.value = t('settings.update_cancelled');"],
  ["checkUpdateStatus.value = '当前已是最新版本';", "checkUpdateStatus.value = t('settings.update_latest');"],
  ["checkUpdateStatus.value = '检查更新失败: ' + String(err);", "checkUpdateStatus.value = t('settings.update_failed').replace('{error}', String(err));"],
  ["playTts('这是一条测试语音通知', config.value.notifyTtsVoice, config.value.notifyTtsVolume);", "playTts(t('settings.test_tts_msg'), config.value.notifyTtsVoice, config.value.notifyTtsVolume);"],
  ["notify('VrcDog 测试', '这是一条测试桌面通知！如果能看到我，说明通知正常工作。', 'test');", "notify(t('settings.test_notify_title'), t('settings.test_notify_msg'), 'test');"],
  
  // Navigation Tabs
  ["<Gamepad2 :size=\"16\" /> {{ t('settings.nav_integration') || '集成' }}", "<Gamepad2 :size=\"16\" /> {{ t('settings.nav_integration') }}"],
  ["<Rocket :size=\"16\" /> 自动启动程序", "<Rocket :size=\"16\" /> {{ t('settings.nav_auto_start') }}"],
  ["<Settings :size=\"16\" /> {{ t('settings.nav_advanced') || '高级' }}", "<Settings :size=\"16\" /> {{ t('settings.nav_advanced') }}"],
  ["<Cpu :size=\"16\" /> 硬件与OSC", "<Cpu :size=\"16\" /> {{ t('settings.nav_hardware') }}"],
  ["<Monitor :size=\"16\" /> {{ t('settings.nav_general') || '常规设置' }}", "<Monitor :size=\"16\" /> {{ t('settings.nav_general') }}"],
  ["<Palette :size=\"16\" /> {{ t('settings.nav_theme') || '主题与外观' }}", "<Palette :size=\"16\" /> {{ t('settings.nav_theme') }}"],
  ["<Bell :size=\"16\" /> {{ t('settings.nav_notify') || '通知与语音' }}", "<Bell :size=\"16\" /> {{ t('settings.nav_notify') }}"],
  ["<Globe :size=\"16\" /> VRChat 配置", "<Globe :size=\"16\" /> {{ t('settings.nav_vrc_config') }}"],
  ["<Info :size=\"16\" /> {{ t('settings.nav_about') || '关于软件' }}", "<Info :size=\"16\" /> {{ t('settings.nav_about') }}"]
];

replacements.forEach(([target, replaceWith]) => {
  content = content.replace(target, replaceWith);
});

fs.writeFileSync('src/components/SettingsView.vue', content, 'utf8');
console.log('Replaced SettingsView script sections');