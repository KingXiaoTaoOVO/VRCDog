const fs = require('fs');
const path = require('path');

const keysToAdd = {
  'user_profile': {
    'req_invite': { 'zh-CN': '请求邀请 (Req)', 'en-US': 'Request Invite (Req)' },
    'invite': { 'zh-CN': '邀请 (Invite)', 'en-US': 'Invite' },
    'hide_avatar': { 'zh-CN': '隐藏化身 (Hide)', 'en-US': 'Hide Avatar' },
    'mute': { 'zh-CN': '玩家静音 (Mute)', 'en-US': 'Mute User' },
    'block': { 'zh-CN': '拉黑 (Block)', 'en-US': 'Block User' },
    'add_friend': { 'zh-CN': '加为好友', 'en-US': 'Add Friend' },
    'remove_friend': { 'zh-CN': '解除好友', 'en-US': 'Remove Friend' },
    'tab_info': { 'zh-CN': '信息', 'en-US': 'Info' },
    'tab_mutual': { 'zh-CN': '共同好友', 'en-US': 'Mutual' },
    'tab_groups': { 'zh-CN': '群组', 'en-US': 'Groups' },
    'tab_worlds': { 'zh-CN': '创建的世界', 'en-US': 'Worlds' },
    'tab_avatars': { 'zh-CN': '创建的模型', 'en-US': 'Avatars' },
    'local_note': { 'zh-CN': '本地备注', 'en-US': 'Local Note' },
    'local_note_placeholder': { 'zh-CN': '点击此处输入本地备注... (失去焦点自动保存)', 'en-US': 'Click here to add note... (Auto-saves on blur)' },
    'bio': { 'zh-CN': '个人简介', 'en-US': 'Bio' },
    'status': { 'zh-CN': '状态签名', 'en-US': 'Status' },
    'last_login': { 'zh-CN': '上一次登录', 'en-US': 'Last Login' },
    'date_joined': { 'zh-CN': '加入时间', 'en-US': 'Date Joined' },
    'dev_type': { 'zh-CN': '开发者类型', 'en-US': 'Developer Type' },
    'none': { 'zh-CN': '无', 'en-US': 'None' },
    'unknown': { 'zh-CN': '未知', 'en-US': 'Unknown' },
    'normal_player': { 'zh-CN': '普通玩家', 'en-US': 'Normal Player' },
    'languages': { 'zh-CN': '使用语言 (Languages)', 'en-US': 'Languages' },
    'pronouns': { 'zh-CN': '人称代词 (Pronouns)', 'en-US': 'Pronouns' },
    'social_links': { 'zh-CN': '社交链接 (Links)', 'en-US': 'Social Links' },
    'current_location': { 'zh-CN': '当前位置', 'en-US': 'Current Location' },
    'private_room': { 'zh-CN': '私密房间 (Private)', 'en-US': 'Private Room' },
    'join': { 'zh-CN': '加入', 'en-US': 'Join' },
    'drop_portal': { 'zh-CN': '投掷传送门', 'en-US': 'Drop Portal' },
    'no_groups': { 'zh-CN': '该用户没有公开的群组', 'en-US': 'No public groups' },
    'no_worlds': { 'zh-CN': '该用户没有公开的世界', 'en-US': 'No public worlds' },
    'no_avatars': { 'zh-CN': '该用户没有公开的模型', 'en-US': 'No public avatars' },
    'no_mutual': { 'zh-CN': '没有共同好友，或由于 VRChat API 限制无法获取', 'en-US': 'No mutual friends found or restricted by VRChat API' }
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
      // Default to en-US if language translation is missing
      data['user_profile'][key] = translations[locale] || translations['en-US'];
    }
  }
  
  fs.writeFileSync(filePath, JSON.stringify(data, null, 2), 'utf-8');
}
console.log('Updated i18n locales for UserProfileModal');