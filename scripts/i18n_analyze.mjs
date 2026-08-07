import fs from 'fs';

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

const files = ['zh-CN.json','en-US.json','ja-JP.json','ko.json','es.json','fr.json','hu.json','pl.json','pt.json','ru.json','th.json','vi.json','cs.json','zh-TW.json'];
const data = {};
for (const f of files) {
  data[f] = flatten(JSON.parse(fs.readFileSync(f,'utf8')));
}

const master = data['zh-CN.json'];
const masterKeys = Object.keys(master).sort();
const enUS = data['en-US.json'];

console.log('Master (zh-CN) leaf keys:', masterKeys.length);
console.log('en-US leaf keys:', Object.keys(enUS).length);
console.log('');

const results = [];
for (const f of files) {
  if (f === 'zh-CN.json') continue;
  const d = data[f];
  const present = masterKeys.filter(k => k in d);
  const missing = masterKeys.filter(k => !(k in d));
  let sameAsEn = 0;
  for (const k of present) {
    if (enUS[k] !== undefined && d[k] === enUS[k] && typeof d[k] === 'string') sameAsEn++;
  }
  const coveragePct = (present.length / masterKeys.length * 100).toFixed(1);
  results.push({ f, present: present.length, missing: missing.length, sameAsEn, coveragePct });
  console.log(f.padEnd(14) + ' present=' + present.length + ' (' + coveragePct + '%)  missing=' + missing.length + '  valuesSameAsEn=' + sameAsEn);
}

// sample the missing keys for ko
console.log('\n=== Sample missing keys in ko.json (first 30) ===');
const koMissing = masterKeys.filter(k => !(k in data['ko.json']));
console.log(koMissing.slice(0, 30).join('\n'));
