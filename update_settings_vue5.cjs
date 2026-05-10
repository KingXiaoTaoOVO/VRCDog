const fs = require('fs');
let content = fs.readFileSync('src/components/SettingsView.vue', 'utf8');

// Fix messed up lines
content = content.replace("何时显示{{ t('settings.desktop_notification_title') }}", "{{ t('settings.when_to_show_desktop_notification') }}");
content = content.replace("当处于 AFK 状态时显示{{ t('settings.desktop_notification_title') }}", "{{ t('settings.show_desktop_notify_when_afk') }}");

// Replace remaining Chinese strings in SettingsView
const replacements = {
  "console.warn('加载设置失败:', err);": "console.warn('Failed to load settings:', err);",
  "console.warn('保存设置失败:', err);": "console.warn('Failed to save settings:', err);",
  "自定义 VRChat 启动参数，例如 <code>--no-vr</code>, <code>--profile=1</code>": "{{ t('settings.vrc_launch_args_desc_2') || 'Custom VRChat launch arguments, e.g., <code>--no-vr</code>, <code>--profile=1</code>' }}",
  "高级与调试": "{{ t('settings.advanced_debug') || 'Advanced & Debug' }}",
  "开发者控制台 (调试)": "{{ t('settings.dev_console') || 'Developer Console (Debug)' }}",
  "启用内部开发工具以调试应用程序": "{{ t('settings.dev_console_desc') || 'Enable internal developer tools for debugging' }}",
  "缓存与本地数据": "{{ t('settings.cache_local_data') || 'Cache & Local Data' }}",
  "清除": "{{ t('settings.clear') || 'Clear' }}",
  "配置与恢复": "{{ t('settings.config_restore') || 'Configuration & Restore' }}",
  "清除本地身份验证信息 (登出)": "{{ t('settings.clear_auth') || 'Clear local authentication (Logout)' }}",
  "强制重新获取所有本地数据，用于解决数据不同步问题": "{{ t('settings.clear_auth_desc') || 'Force re-fetch all local data to resolve sync issues' }}",
  "执行登出": "{{ t('settings.execute_logout') || 'Execute Logout' }}",
  "危险区域": "{{ t('settings.danger_zone') || 'Danger Zone' }}",
  "完全恢复出厂设置": "{{ t('settings.factory_reset') || 'Factory Reset' }}",
  "清除所有设置、缓存并恢复到初始状态": "{{ t('settings.factory_reset_desc') || 'Clear all settings, cache, and restore to initial state' }}",
  "恢复出厂设置": "{{ t('settings.factory_reset_btn') || 'Factory Reset' }}",
  "翻译服务配置": "{{ t('settings.trans_service_config') || 'Translation Service Config' }}",
  "腾讯翻译君 (Tencent)": "{{ t('settings.trans_tencent') || 'Tencent Translator' }}",
  "百度翻译 (Baidu)": "{{ t('settings.trans_baidu') || 'Baidu Translator' }}",
  "火山翻译 (Volcengine)": "{{ t('settings.trans_volcengine') || 'Volcengine Translator' }}",
  "彩云小译 (Caiyun)": "{{ t('settings.trans_caiyun') || 'Caiyun Translator' }}",
  "DeepL (需代理)": "{{ t('settings.trans_deepl') || 'DeepL (Proxy Required)' }}",
  "本地/大语言模型": "{{ t('settings.trans_llm') || 'Local/LLM' }}",
  "API 密钥": "{{ t('settings.api_key') || 'API Key' }}",
  "请输入您的翻译服务密钥": "{{ t('settings.enter_api_key') || 'Please enter your translation service key' }}",
  "对于大语言模型，您可以在此输入 System Prompt": "{{ t('settings.llm_prompt_desc') || 'For LLMs, you can enter the System Prompt here' }}",
  "网络与代理": "{{ t('settings.network_proxy') || 'Network & Proxy' }}",
  "全局加速代理": "{{ t('settings.global_proxy') || 'Global Acceleration Proxy' }}",
  "在网络请求不佳的地区，建议启用此选项并配置代理以加速 API 请求": "{{ t('settings.global_proxy_desc') || 'Recommended in poor network areas to accelerate API requests' }}",
  "代理地址": "{{ t('settings.proxy_url_label') || 'Proxy Address' }}"
};

for (const [key, val] of Object.entries(replacements)) {
  if (content.includes(key)) {
    content = content.replace(new RegExp(key.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'), 'g'), val);
  }
}

fs.writeFileSync('src/components/SettingsView.vue', content, 'utf8');
console.log('Replaced more strings part 5');
