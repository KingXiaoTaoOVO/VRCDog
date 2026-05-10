const fs = require('fs');
const path = require('path');

const keysToAdd = {
  'charts': {
    'mutual_topology': { 'zh-CN': '共同好友拓扑图', 'en-US': 'Mutual Friends Topology' },
    'mutual_desc': { 'zh-CN': '展示您与好友之间的相互联系与社区聚类', 'en-US': 'Shows interconnections and community clustering between your friends' },
    'scanning': { 'zh-CN': '正在扫描: ', 'en-US': 'Scanning: ' },
    'generate_topology': { 'zh-CN': '开始生成拓扑图', 'en-US': 'Generate Topology' },
    'regenerate': { 'zh-CN': '重新生成', 'en-US': 'Regenerate' },
    'no_topology_data': { 'zh-CN': '无拓扑图数据', 'en-US': 'No Topology Data' },
    'topology_help': { 'zh-CN': '点击右上角按钮开始扫描。由于需要获取所有好友的共同好友列表，这可能需要一定时间，具体取决于您的好友数量。', 'en-US': 'Click the top right button to start scanning. Since it needs to fetch mutual friends for all your friends, this might take some time depending on your friend count.' },
    'traversing_network': { 'zh-CN': '正在深度遍历关系网...', 'en-US': 'Deep traversing network...' },
    'pulling_data': { 'zh-CN': '请耐心等待，正在拉取 VRChat 服务器数据', 'en-US': 'Please wait, pulling data from VRChat servers' },
    'updating_realtime': { 'zh-CN': '图表数据实时更新中...', 'en-US': 'Chart data updating in real time...' },
    'day_1': { 'zh-CN': '一', 'en-US': 'Mon' },
    'day_2': { 'zh-CN': '二', 'en-US': 'Tue' },
    'day_3': { 'zh-CN': '三', 'en-US': 'Wed' },
    'day_4': { 'zh-CN': '四', 'en-US': 'Thu' },
    'day_5': { 'zh-CN': '五', 'en-US': 'Fri' },
    'day_6': { 'zh-CN': '六', 'en-US': 'Sat' },
    'day_7': { 'zh-CN': '日', 'en-US': 'Sun' }
  }
};

const localesDir = path.join(__dirname, 'src/i18n/locales');
const files = fs.readdirSync(localesDir).filter(f => f.endsWith('.json'));

for (const file of files) {
  const filePath = path.join(localesDir, file);
  const locale = file.replace('.json', '');
  const data = JSON.parse(fs.readFileSync(filePath, 'utf-8'));
  
  if (!data['charts']) data['charts'] = {};
  
  for (const [key, translations] of Object.entries(keysToAdd['charts'])) {
    if (!data['charts'][key]) {
      data['charts'][key] = translations[locale] || translations['en-US'];
    }
  }
  
  fs.writeFileSync(filePath, JSON.stringify(data, null, 2), 'utf-8');
}
console.log('Updated i18n locales for ChartsView');