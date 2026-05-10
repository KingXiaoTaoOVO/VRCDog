const fs = require('fs');
let content = fs.readFileSync('src/components/SettingsView.vue', 'utf8');

const replacements = {
  "保持您的 VrcDog 客户端处于最新状态，获取最新的功能体验与性能修复": "{{ t('settings.update_info_desc') }}",
  "{{ isCheckingUpdate ? '正在检查...' : '检查更新' }}": "{{ isCheckingUpdate ? t('settings.checking_update') : t('settings.check_update') }}",
  "自动检查更新": "{{ t('settings.auto_check_update') }}",
  "每次打开软件时自动在后台检查是否有新版本发布": "{{ t('settings.auto_check_update_desc') }}",
  "VRChat 启动参数 (Launch Args)": "{{ t('settings.vrc_launch_args') }}",
  "当您通过 VrcDog 启动 VRChat 或加入实例时，将附加这些命令行参数。": "{{ t('settings.vrc_launch_args_desc') }}",
  "例：--no-vr --profile=1": "e.g., --no-vr --profile=1",
  "VRChat 存档与缓存清理": "{{ t('settings.vrc_cache_clear') }}",
  "清理 VRChat 下载的图片、世界和头像缓存，释放磁盘空间": "{{ t('settings.vrc_cache_clear_desc') }}",
  "打开缓存目录": "{{ t('settings.open_cache_dir') }}",
  "打开本地应用数据": "{{ t('settings.open_local_appdata') }}",
  "本地数据库路径 (SQLite)": "{{ t('settings.db_path') }}",
  "VrcDog 本地存储数据的数据库文件路径": "{{ t('settings.db_path_desc') }}",
  "打开所在目录": "{{ t('settings.open_dir') }}",
  "VRChat 游戏日志文件 (output_log)": "{{ t('settings.vrc_log_path') }}",
  "请选择包含 output_log 文件的目录 (默认在 AppData/LocalLow/VRChat/VRChat)": "{{ t('settings.vrc_log_path_desc') }}",
  "选择目录": "{{ t('settings.select_dir') }}",
  "基础设置": "{{ t('settings.basic_settings') }}",
  "开启/关闭主功能": "{{ t('settings.toggle_main_features') }}",
  "开启或关闭": "{{ t('settings.toggle_on_off') }}",
  "开": "{{ t('settings.state_on') }}",
  "关": "{{ t('settings.state_off') }}",
  "关于 VrcDog": "{{ t('settings.about_vrcdog') }}",
  "开源的 VRChat 工具箱": "{{ t('settings.about_vrcdog_desc') }}",
  "官网 / 获取最新版本": "{{ t('settings.official_website') }}",
  "VrcDog 是一个致力于提升 VRChat 玩家体验的开源工具集。": "{{ t('settings.about_p1') }}",
  "我们提供了如：好友在线追踪、世界监控、游戏日志分析、多语言翻译及更多实用功能。": "{{ t('settings.about_p2') }}",
  "核心贡献者": "{{ t('settings.core_contributors') }}",
  "参与贡献": "{{ t('settings.contribute') }}",
  "该项目完全开源，欢迎提交 PR 或是 Issues 帮助我们变得更好！": "{{ t('settings.contribute_desc') }}",
  "查看 GitHub 仓库": "{{ t('settings.view_github') }}",
  "当前版本号": "{{ t('settings.current_version') }}"
};

let modified = false;
for (const [key, val] of Object.entries(replacements)) {
  if (content.includes(key)) {
    content = content.replace(new RegExp(key.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'), 'g'), val);
    modified = true;
  }
}

// Special case for confirm dialog
const confirmTarget = "if (confirm(`发现新版本 v${update.version}！\\n\\n${update.body}\\n\\n是否立即下载并更新？`)) {";
if (content.includes(confirmTarget)) {
  content = content.replace(confirmTarget, "if (confirm(t('settings.update_found').replace('{version}', update.version).replace('{body}', update.body))) {");
  modified = true;
}

if (modified) {
  fs.writeFileSync('src/components/SettingsView.vue', content, 'utf8');
}
console.log('Replaced more settings strings');
