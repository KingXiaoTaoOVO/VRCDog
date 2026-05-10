const fs = require('fs');
const path = require('path');

const keysToAdd = {
  'user_profile': {
    'friend_request_sent': { 'zh-CN': '已发送好友请求！', 'en-US': 'Friend request sent!' },
    'confirm_unfriend': { 'zh-CN': '确定要解除好友关系吗？', 'en-US': 'Are you sure you want to unfriend?' },
    'unfriend_success': { 'zh-CN': '已解除好友关系。', 'en-US': 'Unfriended successfully.' },
    'action_failed': { 'zh-CN': '操作失败: ', 'en-US': 'Action failed: ' },
    'action_success': { 'zh-CN': '操作成功: ', 'en-US': 'Action successful: ' },
    'invite_sent': { 'zh-CN': '已发送邀请！', 'en-US': 'Invite sent!' },
    'req_invite_sent': { 'zh-CN': '已发送请求邀请！', 'en-US': 'Request Invite sent!' },
    'send_failed': { 'zh-CN': '发送失败: ', 'en-US': 'Failed: ' },
    'friend_badge': { 'zh-CN': '好友', 'en-US': 'Friend' }
  }
};

const localesDir = path.join(__dirname, 'src/i18n/locales');
const files = fs.readdirSync(localesDir).filter(f => f.endsWith('.json'));

for (const file of files) {
  const filePath = path.join(localesDir, file);
  const locale = file.replace('.json', '');
  const data = JSON.parse(fs.readFileSync(filePath, 'utf-8'));
  
  if (!data['user_profile']) data['user_profile'] = {};
  
  for (const [key, translations] of Object.entries(keysToAdd['user_profile'])) {
    if (!data['user_profile'][key]) {
      data['user_profile'][key] = translations[locale] || translations['en-US'];
    }
  }
  
  fs.writeFileSync(filePath, JSON.stringify(data, null, 2), 'utf-8');
}
console.log('Updated i18n locales for UserProfileModal alerts');