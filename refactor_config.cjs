const fs = require('fs');
const path = require('path');

const vueFile = path.resolve(__dirname, 'src/components/OvrTranslatorView.vue');
let content = fs.readFileSync(vueFile, 'utf8');

const mappings = {
  // general
  'ovrEnabled': 'general.enabled',
  'ovrDualDisplay': 'general.dualDisplay',
  'ovrTheme': 'general.theme',
  'ovrWristMode': 'general.wristMode',
  'ovrTriggerKey': 'general.triggerKey',
  'ovrClearKey': 'general.clearKey',
  'autoScanInterval': 'general.autoScanInterval',
  'overlayTextColor': 'general.overlayTextColor',
  'overlayBgColor': 'general.overlayBgColor',
  'overlayBgOpacity': 'general.overlayBgOpacity',
  'overlayScale': 'general.overlayScale',
  'overlayLockMode': 'general.overlayLockMode',
  
  // ocr
  'ocrModel': 'ocr.model',
  'ocrSpeedMode': 'ocr.speedMode',
  'ocrEnhanceContrast': 'ocr.enhanceContrast',
  
  // chaperone
  'chapVisibility': 'chaperone.visibility',
  'chapForceBounds': 'chaperone.forceBounds',
  'chapHapticFeedback': 'chaperone.hapticFeedback',
  
  // playspace
  'spaceRotation': 'playspace.rotation',
  'spaceOffsetX': 'playspace.offsetX',
  'spaceOffsetY': 'playspace.offsetY',
  'spaceOffsetZ': 'playspace.offsetZ',
  'motionGravity': 'playspace.gravity',
  'motionDragLeft': 'playspace.dragLeft',
  'motionDragRight': 'playspace.dragRight',
  'motionHeightToggle': 'playspace.heightToggle',
  'motionHeightOffset': 'playspace.heightOffset',
  
  // video
  'videoMotionSmooth': 'video.motionSmooth',
  'videoSuperSampling': 'video.superSampling',
  
  // utilities
  'utilMediaKeys': 'utilities.mediaKeys',
};

// Also apply for other unmapped ones by grouping by prefix:
// chapXXX -> chaperone.xxx
// videoXXX -> video.xxx
// audioXXX -> audio.xxx
// steamvrXXX -> steamvr.xxx

// First, let's extract the config block
const configRegex = /const config = ref\(\{\n([\s\S]*?)\n\}\);/;
const match = content.match(configRegex);

if (!match) {
  console.log('Could not find config object');
  process.exit(1);
}

let configContent = match[1];
const newConfig = {
  general: {},
  ocr: {},
  chaperone: {},
  playspace: {},
  video: {},
  audio: {},
  steamvr: {},
  utilities: {}
};

configContent.split('\n').forEach(line => {
  const lineTrimmed = line.trim();
  if (lineTrimmed.startsWith('//') || !lineTrimmed) return;
  
  const kvMatch = lineTrimmed.match(/^([a-zA-Z0-9_]+):\s*(.*),?$/);
  if (kvMatch) {
    let key = kvMatch[1];
    let valStr = kvMatch[2].endsWith(',') ? kvMatch[2].slice(0, -1) : kvMatch[2];
    
    if (mappings[key]) {
      const [group, subkey] = mappings[key].split('.');
      if (!newConfig[group]) newConfig[group] = {};
      newConfig[group][subkey] = valStr;
    } else {
      // Auto-group by prefix
      let grouped = false;
      for (const prefix of ['chap', 'video', 'audio', 'steamvr', 'util']) {
        if (key.startsWith(prefix)) {
          let subkey = key.slice(prefix.length);
          subkey = subkey.charAt(0).toLowerCase() + subkey.slice(1);
          let group = prefix === 'chap' ? 'chaperone' : (prefix === 'util' ? 'utilities' : prefix);
          newConfig[group][subkey] = valStr;
          mappings[key] = `${group}.${subkey}`;
          grouped = true;
          break;
        }
      }
      if (!grouped) {
        newConfig.general[key] = valStr;
        mappings[key] = `general.${key}`;
      }
    }
  }
});

let newConfigStr = 'const config = ref({\n';
for (const [group, keys] of Object.entries(newConfig)) {
  newConfigStr += `  ${group}: {\n`;
  for (const [k, v] of Object.entries(keys)) {
    newConfigStr += `    ${k}: ${v},\n`;
  }
  newConfigStr += `  },\n`;
}
newConfigStr += `});`;

// Replace config block
content = content.replace(configRegex, newConfigStr);

// Now replace all config.key with config.group.subkey
// Be careful to match config.key exactly
for (const [oldKey, newPath] of Object.entries(mappings)) {
  const regex = new RegExp(`config\\.value\\.${oldKey}\\b`, 'g');
  content = content.replace(regex, `config.value.${newPath}`);
  
  const regex2 = new RegExp(`config\\.${oldKey}\\b`, 'g');
  content = content.replace(regex2, `config.${newPath}`);
}

fs.writeFileSync(vueFile, content);
console.log('Successfully refactored config in Vue file!');

// Save mappings to use in Rust rewrite
fs.writeFileSync(path.resolve(__dirname, 'mappings.json'), JSON.stringify(mappings, null, 2));

