const fs = require('fs');
const path = require('path');

const keysToAdd = {
  'settings': {
    'update_info_desc': { 'zh-CN': '保持您的 VrcDog 客户端处于最新状态，获取最新的功能体验与性能修复', 'en-US': 'Keep your VrcDog client up-to-date for the latest features and performance fixes' },
    'checking_update': { 'zh-CN': '正在检查...', 'en-US': 'Checking...' },
    'auto_check_update': { 'zh-CN': '自动检查更新', 'en-US': 'Auto Check for Updates' },
    'auto_check_update_desc': { 'zh-CN': '每次打开软件时自动在后台检查是否有新版本发布', 'en-US': 'Automatically check for new versions in the background when starting the software' },
    'vrc_launch_args': { 'zh-CN': 'VRChat 启动参数 (Launch Args)', 'en-US': 'VRChat Launch Args' },
    'vrc_launch_args_desc': { 'zh-CN': '当您通过 VrcDog 启动 VRChat 或加入实例时，将附加这些命令行参数。', 'en-US': 'These arguments will be appended when launching VRChat or joining an instance through VrcDog.' },
    'vrc_cache_clear': { 'zh-CN': 'VRChat 存档与缓存清理', 'en-US': 'VRChat Cache Clear' },
    'vrc_cache_clear_desc': { 'zh-CN': '清理 VRChat 下载的图片、世界和头像缓存，释放磁盘空间', 'en-US': 'Clear VRChat downloaded images, worlds, and avatar caches to free up disk space' },
    'open_cache_dir': { 'zh-CN': '打开缓存目录', 'en-US': 'Open Cache Dir' },
    'open_local_appdata': { 'zh-CN': '打开本地应用数据', 'en-US': 'Open Local AppData' },
    'db_path': { 'zh-CN': '本地数据库路径 (SQLite)', 'en-US': 'Local Database Path (SQLite)' },
    'db_path_desc': { 'zh-CN': 'VrcDog 本地存储数据的数据库文件路径', 'en-US': 'Path to the database file where VrcDog stores local data' },
    'open_dir': { 'zh-CN': '打开所在目录', 'en-US': 'Open Directory' },
    'vrc_log_path': { 'zh-CN': 'VRChat 游戏日志文件 (output_log)', 'en-US': 'VRChat Game Log File (output_log)' },
    'vrc_log_path_desc': { 'zh-CN': '请选择包含 output_log 文件的目录 (默认在 AppData/LocalLow/VRChat/VRChat)', 'en-US': 'Select the directory containing output_log files (default: AppData/LocalLow/VRChat/VRChat)' },
    'select_dir': { 'zh-CN': '选择目录', 'en-US': 'Select Directory' },
    'basic_settings': { 'zh-CN': '基础设置', 'en-US': 'Basic Settings' },
    'toggle_main_features': { 'zh-CN': '开启/关闭主功能', 'en-US': 'Toggle Main Features' },
    'toggle_on_off': { 'zh-CN': '开启或关闭', 'en-US': 'Toggle On or Off' },
    'state_on': { 'zh-CN': '开', 'en-US': 'On' },
    'state_off': { 'zh-CN': '关', 'en-US': 'Off' },
    'about_vrcdog': { 'zh-CN': '关于 VrcDog', 'en-US': 'About VrcDog' },
    'about_vrcdog_desc': { 'zh-CN': '开源的 VRChat 工具箱', 'en-US': 'Open-source VRChat Toolbox' },
    'official_website': { 'zh-CN': '官网 / 获取最新版本', 'en-US': 'Official Website / Get Latest Version' },
    'about_p1': { 'zh-CN': 'VrcDog 是一个致力于提升 VRChat 玩家体验的开源工具集。', 'en-US': 'VrcDog is an open-source toolset dedicated to improving the VRChat player experience.' },
    'about_p2': { 'zh-CN': '我们提供了如：好友在线追踪、世界监控、游戏日志分析、多语言翻译及更多实用功能。', 'en-US': 'We provide features such as friend online tracking, world monitoring, game log analysis, multi-language translation, and more.' },
    'core_contributors': { 'zh-CN': '核心贡献者', 'en-US': 'Core Contributors' },
    'contribute': { 'zh-CN': '参与贡献', 'en-US': 'Contribute' },
    'contribute_desc': { 'zh-CN': '该项目完全开源，欢迎提交 PR 或是 Issues 帮助我们变得更好！', 'en-US': 'This project is completely open-source. Welcome to submit PRs or Issues to help us improve!' },
    'view_github': { 'zh-CN': '查看 GitHub 仓库', 'en-US': 'View GitHub Repository' },
    'current_version': { 'zh-CN': '当前版本号', 'en-US': 'Current Version' }
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
console.log('Updated i18n locales for SettingsView part 3');
