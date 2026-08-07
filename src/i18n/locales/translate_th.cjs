const fs = require('fs');
const { execFile } = require('child_process');

const DIR = 'C:/Users/Administrator/Documents/Project/VRCDog/src/i18n/locales';
const en = JSON.parse(fs.readFileSync(DIR + '/en-US.json', 'utf8'));
const th = JSON.parse(fs.readFileSync(DIR + '/th.json', 'utf8'));
const zh = JSON.parse(fs.readFileSync(DIR + '/zh-CN.json', 'utf8'));
const THAI = /[฀-๿]/;

const CACHE_FILE = DIR + '/.th_translate_cache.json';
let cache = {};
try { cache = JSON.parse(fs.readFileSync(CACHE_FILE, 'utf8')); } catch (e) { cache = {}; }

const getLeaf = (o, p) => p.split('.').reduce((a, k) => (a == null ? a : a[k]), o);
function leaves(o, p, a) {
  for (const [k, v] of Object.entries(o)) {
    const n = p ? p + '.' + k : k;
    if (v && typeof v === 'object' && !Array.isArray(v)) leaves(v, n, a);
    else a.push([n, v]);
  }
  return a;
}

// Collect target strings (dedup). For each en leaf:
//  - string: keep thai th value if present, else need translate
//  - array: per element; keep thai th element if present, else need translate
const need = new Map(); // enString -> true
function needStr(s) { if (typeof s === 'string' && s.trim()) need.set(s, true); }

const L = leaves(en, '', []);
const keepSet = new Set(); // paths to keep th thai value
for (const [p, v] of L) {
  const ex = getLeaf(th, p);
  if (typeof v === 'string') {
    if (ex && typeof ex === 'string' && THAI.test(ex)) keepSet.add(p);
    else needStr(v);
  } else if (Array.isArray(v)) {
    const exArr = Array.isArray(ex) ? ex : null;
    v.forEach((el, i) => {
      if (typeof el === 'string') {
        const exEl = exArr && typeof exArr[i] === 'string' ? exArr[i] : null;
        if (exEl && THAI.test(exEl)) { /* keep element, handled in build */ }
        else needStr(el);
      }
    });
  }
}

const unique = [...need.keys()];
const todo = unique.filter(s => !(s in cache) || !cache[s]);
console.error(`unique=${unique.length} cached=${unique.length - todo.length} todo=${todo.length}`);

// ---- masking ----
function mask(s) {
  const ph = [], tags = [];
  let t = s;
  t = t.replace(/\{([^{}]*)\}/g, (m) => { const i = ph.length; ph.push(m); return '' + i + ''; });
  t = t.replace(/<[^>]+>/g, (m) => { const i = tags.length; tags.push(m); return '' + i + ''; });
  return { t, ph, tags };
}
function unmask(t, ph, tags) {
  t = t.replace(/(\d+)/g, (_, i) => tags[+i]);
  t = t.replace(/(\d+)/g, (_, i) => ph[+i]);
  return t;
}

function curlTranslate(text) {
  return new Promise((resolve, reject) => {
    const args = ['-s', '--get', 'https://translate.googleapis.com/translate_a/single',
      '-d', 'client=gtx', '-d', 'sl=en', '-d', 'tl=th', '-d', 'dt=t',
      '--data-urlencode', 'q=' + text];
    execFile('curl', args, { maxBuffer: 1 << 26 }, (err, stdout) => {
      if (err) return reject(err);
      try {
        const j = JSON.parse(stdout);
        const out = j && j[0] && j[0][0] && j[0][0][0];
        if (!out) return reject(new Error('empty'));
        resolve(out);
      } catch (e) { reject(e); }
    });
  });
}

async function tr1(s) {
  if (!s || !s.trim()) return s;
  const { t, ph, tags } = mask(s);
  let lastErr;
  for (let attempt = 0; attempt < 4; attempt++) {
    try {
      const r = await curlTranslate(t);
      return unmask(r, ph, tags);
    } catch (e) { lastErr = e; await new Promise(r => setTimeout(r, 400 * (attempt + 1))); }
  }
  // fallback: return original english (structure stays complete)
  return s;
}

async function pool(items, worker, concurrency) {
  let idx = 0; const results = new Array(items.length);
  async function run() {
    while (idx < items.length) {
      const i = idx++;
      try { results[i] = await worker(items[i]); }
      catch (e) { results[i] = items[i]; }
      if (i % 50 === 0) console.error(`  progress ${i}/${items.length}`);
    }
  }
  await Promise.all(Array.from({ length: Math.min(concurrency, items.length) }, () => run()));
  return results;
}

(async () => {
  const translated = await pool(todo, tr1, 14);
  todo.forEach((s, i) => { cache[s] = translated[i]; });
  fs.writeFileSync(CACHE_FILE, JSON.stringify(cache), 'utf8');
  console.error('translation done, building output');

  // ---- build output from en structure ----
  function build(o, p) {
    const ex = p ? getLeaf(th, p) : null;
    if (o && typeof o === 'object' && !Array.isArray(o)) {
      const out = {};
      for (const [k, v] of Object.entries(o)) {
        out[k] = build(v, p ? p + '.' + k : k);
      }
      return out;
    } else if (Array.isArray(o)) {
      const exArr = Array.isArray(ex) ? ex : null;
      return o.map((el, i) => {
        if (typeof el === 'string') {
          const exEl = exArr && typeof exArr[i] === 'string' ? exArr[i] : null;
          if (exEl && THAI.test(exEl)) return exEl;
          return cache[el] != null ? cache[el] : el;
        }
        return el;
      });
    } else if (typeof o === 'string') {
      if (p && keepSet.has(p)) return ex;
      return cache[o] != null ? cache[o] : o;
    }
    return o;
  }

  const out = build(en, '');
  fs.writeFileSync(DIR + '/th.json', JSON.stringify(out, null, 2), 'utf8');
  console.error('written th.json');
})().catch(e => { console.error('FATAL', e); process.exit(1); });
