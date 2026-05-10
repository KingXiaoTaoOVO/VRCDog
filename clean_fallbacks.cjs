const fs = require('fs');

let content = fs.readFileSync('src/components/SettingsView.vue', 'utf8');

// 1. Remove all `|| '中文'` from `t('...') || '中文'`
content = content.replace(/\{\{\s*(t\('[^']+'\))\s*\|\|\s*'[^']+'\s*\}\}/g, '{{ $1 }}');
content = content.replace(/\{\{\s*(t\('[^']+'\))\s*\|\|\s*"[^"]+"\s*\}\}/g, '{{ $1 }}');
// Remove from JS calls
content = content.replace(/(t\('[^']+'\))\s*\|\|\s*'[^']+'/g, '$1');

fs.writeFileSync('src/components/SettingsView.vue', content, 'utf8');
console.log('Cleaned up t() fallbacks in SettingsView');
