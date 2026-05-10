const fs = require('fs');
const path = require('path');
const crypto = require('crypto');

let content = fs.readFileSync('src/components/SettingsView.vue', 'utf8');

const textRegex = />([^<>\{\}]+[\u4e00-\u9fa5]+[^<>\{\}]+)</g;
const matches = [];
let match;
while ((match = textRegex.exec(content)) !== null) {
  const fullMatch = match[0];
  let text = match[1];
  text = text.trim();
  if (text.length > 0) {
    matches.push({ full: fullMatch, text: text, isAttr: false });
  }
}

const placeholderRegex = /placeholder="([^"]+[\u4e00-\u9fa5]+[^"]+)"/g;
while ((match = placeholderRegex.exec(content)) !== null) {
  const fullMatch = match[0];
  let text = match[1];
  text = text.trim();
  if (text.length > 0) {
    matches.push({ full: fullMatch, text: text, isAttr: true, attr: 'placeholder' });
  }
}

let keysToAdd = {};
matches.forEach(m => {
  const hash = crypto.createHash('md5').update(m.text).digest('hex').substring(0, 8);
  const key = `auto_${hash}`;
  keysToAdd[key] = m.text;
  
  if (m.isAttr) {
    const newAttr = `:${m.attr}="t('settings.${key}')"`;
    content = content.replace(m.full, newAttr);
  } else {
    const newFull = m.full.replace(m.text, `{{ t('settings.${key}') }}`);
    content = content.replace(m.full, newFull);
  }
});

fs.writeFileSync('src/components/SettingsView.vue', content, 'utf8');

const localesDir = path.join(__dirname, 'src/i18n/locales');
const files = fs.readdirSync(localesDir).filter(f => f.endsWith('.json'));

for (const file of files) {
  const filePath = path.join(localesDir, file);
  const data = JSON.parse(fs.readFileSync(filePath, 'utf-8'));
  if (!data['settings']) data['settings'] = {};
  
  for (const [key, val] of Object.entries(keysToAdd)) {
    if (!data['settings'][key]) {
      data['settings'][key] = val; 
    }
  }
  
  fs.writeFileSync(filePath, JSON.stringify(data, null, 2), 'utf-8');
}
console.log('Auto i18n replaced ' + matches.length + ' strings');
