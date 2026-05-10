const fs = require('fs');
const path = require('path');

const keysToAdd = {
  'entity_modal': {
    'upload_avatar_success': { 'zh-CN': '模型图片上传成功！', 'en-US': 'Avatar image uploaded successfully!' },
    'no_url_returned': { 'zh-CN': '未返回文件 URL', 'en-US': 'No file URL returned' },
    'upload_failed': { 'zh-CN': '上传失败: ', 'en-US': 'Upload failed: ' },
    'upload_world_success': { 'zh-CN': '世界图片上传成功！', 'en-US': 'World image uploaded successfully!' },
    'no_description': { 'zh-CN': '暂无简介', 'en-US': 'No description available' },
    'uploading': { 'zh-CN': '上传中...', 'en-US': 'Uploading...' },
    'change_world_img': { 'zh-CN': '更换世界图片', 'en-US': 'Change World Image' },
    'change_avatar_img': { 'zh-CN': '更换模型图片', 'en-US': 'Change Avatar Image' },
    'active_instances': { 'zh-CN': '活跃实例 (Active Instances)', 'en-US': 'Active Instances' },
    'join': { 'zh-CN': '加入', 'en-US': 'Join' },
    'drop_portal': { 'zh-CN': '投掷传送门', 'en-US': 'Drop Portal' },
    'members': { 'zh-CN': '成员', 'en-US': 'Members' },
    'info': { 'zh-CN': '详情 (Info)', 'en-US': 'Info' },
    'requests': { 'zh-CN': '申请 (Requests)', 'en-US': 'Requests' },
    'no_group_desc': { 'zh-CN': '暂无群组简介', 'en-US': 'No group description' },
    'privacy_status': { 'zh-CN': '隐私状态', 'en-US': 'Privacy' },
    'public_group': { 'zh-CN': '公开群组', 'en-US': 'Public Group' },
    'private_group': { 'zh-CN': '私密群组', 'en-US': 'Private Group' },
    'join_state': { 'zh-CN': '加入方式', 'en-US': 'Join State' },
    'open_join': { 'zh-CN': '自由加入', 'en-US': 'Open' },
    'request_join': { 'zh-CN': '需申请', 'en-US': 'Request' },
    'invite_only': { 'zh-CN': '邀请制', 'en-US': 'Invite Only' },
    'loading_members': { 'zh-CN': '正在加载成员...', 'en-US': 'Loading members...' },
    'no_member_data': { 'zh-CN': '无成员数据', 'en-US': 'No member data' },
    'loading_requests': { 'zh-CN': '正在加载申请...', 'en-US': 'Loading requests...' },
    'no_pending_requests': { 'zh-CN': '暂无待处理的申请', 'en-US': 'No pending requests' },
    'view_in_vrc': { 'zh-CN': '在 VRChat 中查看', 'en-US': 'View in VRChat' }
  }
};

const localesDir = path.join(__dirname, 'src/i18n/locales');
const files = fs.readdirSync(localesDir).filter(f => f.endsWith('.json'));

for (const file of files) {
  const filePath = path.join(localesDir, file);
  const locale = file.replace('.json', '');
  const data = JSON.parse(fs.readFileSync(filePath, 'utf-8'));
  
  if (!data['entity_modal']) data['entity_modal'] = {};
  
  for (const [key, translations] of Object.entries(keysToAdd['entity_modal'])) {
    if (!data['entity_modal'][key]) {
      data['entity_modal'][key] = translations[locale] || translations['en-US'];
    }
  }
  
  fs.writeFileSync(filePath, JSON.stringify(data, null, 2), 'utf-8');
}
console.log('Updated i18n locales for EntityDetailModals');