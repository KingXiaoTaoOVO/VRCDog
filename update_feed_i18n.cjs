const fs = require('fs');
const path = require('path');

const keysToAdd = {
  'feed': {
    'confirm_clear': { 'zh-CN': '确定要清空日志吗？', 'en-US': 'Are you sure you want to clear the logs?' },
    'search_logs': { 'zh-CN': '搜索动态...', 'en-US': 'Search logs...' },
    'game_logs': { 'zh-CN': '游戏日志 (Game)', 'en-US': 'Game Logs' },
    'friend_logs': { 'zh-CN': '好友动态 (Friends)', 'en-US': 'Friend Logs' },
    'clear': { 'zh-CN': '清空', 'en-US': 'Clear' },
    'no_friend_logs': { 'zh-CN': '暂无好友动态', 'en-US': 'No friend logs' },
    'status_online': { 'zh-CN': '上线了', 'en-US': 'Online' },
    'status_offline': { 'zh-CN': '下线了', 'en-US': 'Offline' },
    'status_location': { 'zh-CN': '切换位置', 'en-US': 'Changed Location' }
  }
};

const localesDir = path.join(__dirname, 'src/i18n/locales');
const files = fs.readdirSync(localesDir).filter(f => f.endsWith('.json'));

for (const file of files) {
  const filePath = path.join(localesDir, file);
  const locale = file.replace('.json', '');
  const data = JSON.parse(fs.readFileSync(filePath, 'utf-8'));
  
  if (!data['feed']) data['feed'] = {};
  
  for (const [key, translations] of Object.entries(keysToAdd['feed'])) {
    if (!data['feed'][key]) {
      data['feed'][key] = translations[locale] || translations['en-US'];
    }
  }
  
  fs.writeFileSync(filePath, JSON.stringify(data, null, 2), 'utf-8');
}
console.log('Updated i18n locales for FeedView');