const fs = require('fs');

const content = fs.readFileSync('src/components/SettingsView.vue', 'utf8');
const lines = content.split('\n');

const chineseRegex = /[\u4e00-\u9fa5]+/g;
const results = [];

lines.forEach((line, index) => {
  if (chineseRegex.test(line)) {
    // skip comments if possible
    if (line.trim().startsWith('//') || line.trim().startsWith('<!--')) return;
    results.push({ line: index + 1, text: line.trim() });
  }
});

fs.writeFileSync('chinese_strings.json', JSON.stringify(results, null, 2));
console.log('Found ' + results.length + ' lines with Chinese');