import fs from 'fs';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';

const dir = dirname(fileURLToPath(import.meta.url));

function flatten(obj, prefix='') {
  const out = {};
  for (const [k,v] of Object.entries(obj)) {
    const key = prefix ? prefix + '.' + k : k;
    if (v && typeof v === 'object' && !Array.isArray(v)) {
      Object.assign(out, flatten(v, key));
    } else {
      out[key] = v;
    }
  }
  return out;
}

const zhCN = flatten(JSON.parse(fs.readFileSync(dir + '/zh-CN.json','utf8')));
const enUS = flatten(JSON.parse(fs.readFileSync(dir + '/en-US.json','utf8')));
const masterKeys = Array.from(new Set([...Object.keys(zhCN), ...Object.keys(enUS)])).sort();

const targets = ['en-US','ja-JP','ko','es','fr','hu','pl','pt','ru','th','vi','cs','zh-TW','yue'];

console.log('Master (zh-CN ∪ en-US) leaf keys:', masterKeys.length);
console.log('');
let allOk = true;
for (const t of targets) {
  let d;
  try {
    d = flatten(JSON.parse(fs.readFileSync(dir + '/' + t + '.json','utf8')));
  } catch (e) {
    console.log(t.padEnd(8) + ' PARSE ERROR: ' + e.message);
    allOk = false;
    continue;
  }
  const present = masterKeys.filter(k => k in d);
  const missing = masterKeys.filter(k => !(k in d));
  let sameAsEn = 0;
  for (const k of present) {
    if (enUS[k] !== undefined && d[k] === enUS[k] && typeof d[k] === 'string') sameAsEn++;
  }
  const pct = (present.length / masterKeys.length * 100).toFixed(1);
  const flag = missing.length === 0 ? 'OK ' : 'MISSING';
  if (missing.length) allOk = false;
  console.log(t.padEnd(8) + ' ' + flag + ' present=' + present.length + '/' + masterKeys.length + ' (' + pct + '%)  sameAsEn=' + sameAsEn);
  if (missing.length && missing.length <= 20) console.log('   missing: ' + missing.slice(0,20).join(', '));
}
console.log('');
console.log(allOk ? '=== ALL LANGUAGES COMPLETE (0 missing) ===' : '=== SOME LANGUAGES STILL MISSING KEYS ===');
