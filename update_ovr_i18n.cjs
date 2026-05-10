const fs = require('fs');
const path = require('path');

const keysToAdd = {
  'ovr': {
    'trans_llm_prompt_default': { 'zh-CN': '你是一个翻译专家，综合所有的 OCR 乱入文本，给出现在最好的目标语言。', 'en-US': 'You are a translation expert. Synthesize all OCR text and provide the best translation in the target language.' },
    'toast_sync_success': { 'zh-CN': 'OVR 设置已自动同步至 SteamVR', 'en-US': 'OVR settings synced to SteamVR' },
    'toast_sync_fail': { 'zh-CN': '同步失败: {error}', 'en-US': 'Sync failed: {error}' },
    'toast_scan_stopped': { 'zh-CN': '自动扫描已停止', 'en-US': 'Auto-scan stopped' },
    'toast_scan_started': { 'zh-CN': '自动扫描已开启 ({interval}s)', 'en-US': 'Auto-scan started ({interval}s)' },
    'toast_scan_loading': { 'zh-CN': '桌面截图翻译中...', 'en-US': 'Capturing and translating desktop...' },
    'toast_scan_failed': { 'zh-CN': '扫描失败: {error}', 'en-US': 'Scan failed: {error}' },
    'log_ovr_connected': { 'zh-CN': '[OVR] 已连接 {model}', 'en-US': '[OVR] Connected to {model}' },
    'log_ovr_init_fail': { 'zh-CN': '[OVR] 初始化失败: {error}', 'en-US': '[OVR] Init failed: {error}' },
    'trigger': { 'zh-CN': '扳机键', 'en-US': 'Trigger' },
    'grip': { 'zh-CN': '侧边键', 'en-US': 'Grip' },
    'a_button': { 'zh-CN': 'A / X 键', 'en-US': 'A / X Button' },
    'b_button': { 'zh-CN': 'B / Y 键', 'en-US': 'B / Y Button' },
    'left_stick': { 'zh-CN': '左摇杆', 'en-US': 'Left Stick' },
    'right_stick': { 'zh-CN': '右摇杆', 'en-US': 'Right Stick' },
    'trigger_full': { 'zh-CN': '扳机键 (Trigger)', 'en-US': 'Trigger' },
    'grip_full': { 'zh-CN': '侧边键 (Grip)', 'en-US': 'Grip' },
    'left_stick_full': { 'zh-CN': '左摇杆拨动 (Left Stick)', 'en-US': 'Left Stick (Flick)' },
    'right_stick_full': { 'zh-CN': '右摇杆拨动 (Right Stick)', 'en-US': 'Right Stick (Flick)' },
    'ovr_connected': { 'zh-CN': 'OpenVR 已连接 · {model}', 'en-US': 'OpenVR Connected · {model}' },
    'ovr_disconnected': { 'zh-CN': 'OpenVR 未连接（VR 运行时未启动）', 'en-US': 'OpenVR Disconnected (VR runtime not started)' },
    'controller_mapping': { 'zh-CN': '手柄按键映射', 'en-US': 'Controller Mapping' },
    'trigger_key': { 'zh-CN': '触发截图翻译按键', 'en-US': 'Trigger Translation Key' },
    'clear_key': { 'zh-CN': '清除翻译按键', 'en-US': 'Clear Translation Key' },
    'restore_defaults': { 'zh-CN': '恢复默认值', 'en-US': 'Restore Defaults' },
    'axis_x': { 'zh-CN': 'X 轴', 'en-US': 'X Axis' },
    'axis_y': { 'zh-CN': 'Y 轴', 'en-US': 'Y Axis' },
    'axis_z': { 'zh-CN': 'Z 轴', 'en-US': 'Z Axis' },
    'state_on': { 'zh-CN': '开', 'en-US': 'On' },
    'state_off': { 'zh-CN': '关', 'en-US': 'Off' },
    'desc_trigger_trans': { 'zh-CN': '- {key}：触发截图翻译', 'en-US': '- {key}: Trigger translation' },
    'desc_clear_trans': { 'zh-CN': '- {key}：清除翻译', 'en-US': '- {key}: Clear translation' },
    'sim_text_1_orig': { 'zh-CN': 'Welcome to this world!\nPlease enjoy your stay.', 'en-US': 'Welcome to this world!\nPlease enjoy your stay.' },
    'sim_text_1_trans': { 'zh-CN': '欢迎来到这个世界！\n请享受你的旅程。', 'en-US': 'Welcome to this world!\nPlease enjoy your stay.' },
    'sim_text_2_orig': { 'zh-CN': 'Press trigger to interact\nwith objects around you.', 'en-US': 'Press trigger to interact\nwith objects around you.' },
    'sim_text_2_trans': { 'zh-CN': '按下扳机键与周围的\n物体进行互动。', 'en-US': 'Press trigger to interact\nwith objects around you.' },
    'sim_text_3_orig': { 'zh-CN': 'This avatar is private.\nYou cannot clone it.', 'en-US': 'This avatar is private.\nYou cannot clone it.' },
    'sim_text_3_trans': { 'zh-CN': '此模型为私密模型。\n你无法复制。', 'en-US': 'This avatar is private.\nYou cannot clone it.' },
    'sim_text_4_orig': { 'zh-CN': 'Instance capacity: 20/40\nRegion: US West', 'en-US': 'Instance capacity: 20/40\nRegion: US West' },
    'sim_text_4_trans': { 'zh-CN': '房间容量: 20/40\n地区: 美国西部', 'en-US': 'Instance capacity: 20/40\nRegion: US West' }
  }
};

const localesDir = path.join(__dirname, 'src/i18n/locales');
const files = fs.readdirSync(localesDir).filter(f => f.endsWith('.json'));

for (const file of files) {
  const filePath = path.join(localesDir, file);
  const locale = file.replace('.json', '');
  const data = JSON.parse(fs.readFileSync(filePath, 'utf-8'));
  
  if (!data['ovr']) data['ovr'] = {};
  
  for (const [key, translations] of Object.entries(keysToAdd['ovr'])) {
    if (!data['ovr'][key]) {
      data['ovr'][key] = translations[locale] || translations['en-US'];
    }
  }
  
  fs.writeFileSync(filePath, JSON.stringify(data, null, 2), 'utf-8');
}
console.log('Updated i18n locales for OvrTranslatorView');