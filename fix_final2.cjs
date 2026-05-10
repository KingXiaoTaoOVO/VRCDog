const fs = require('fs');
let code = fs.readFileSync('src/components/UserProfileModal.vue', 'utf8');

// 1. Fix 404 error on favorites endpoint
code = code.replace(
  /\/api\/1\/favorites\//g,
  '/favorites/'
);
code = code.replace(
  /\/api\/1\/favorites/g,
  '/favorites'
);

// 2. Fix Modal background (it is still rendering grey because of bg-black/70 overlay)
code = code.replace(
  /class="fixed inset-0 bg-black\/70 z-\[100\] flex items-center justify-center p-4 backdrop-blur-sm"/g,
  'class="fixed inset-0 bg-slate-900/40 z-[100] flex items-center justify-center p-4 backdrop-blur-md"'
);

// 3. Brighten the modal glass panel
code = code.replace(
  /class="bg-white\/30 backdrop-blur-3xl border border-white\/60 shadow-\[0_0_50px_rgba\(0,0,0,0\.15\)\] rounded-3xl w-full max-w-\[950px\] max-h-\[90vh\] flex flex-col overflow-hidden text-slate-800"/g,
  'class="bg-white/60 backdrop-blur-3xl border border-white/60 shadow-[0_0_50px_rgba(0,0,0,0.15)] rounded-3xl w-full max-w-[950px] max-h-[90vh] flex flex-col overflow-hidden text-slate-800"'
);

// 4. Fix dropdown getting cut off by adding max-height and overflow-y-auto
code = code.replace(
  /class="absolute top-12 right-0 w-64 bg-white\/60 backdrop-blur-2xl rounded-2xl py-1\.5 border border-white\/80 shadow-\[0_10px_40px_rgba\(0,0,0,0\.2\)\] z-\[100\] text-\[13px\] font-bold text-slate-700"/g,
  'class="absolute top-12 right-0 w-64 bg-white/70 backdrop-blur-2xl rounded-2xl py-1.5 border border-white/80 shadow-[0_10px_40px_rgba(0,0,0,0.2)] z-[100] text-[13px] font-bold text-slate-700 max-h-[50vh] overflow-y-auto custom-scrollbar"'
);

// 5. Fix isFavorite logic. Tags on User don't have group_0. Just use a local ref for now to prevent it from always being true.
// The current code is:
// const isFavorite = computed(() => {
//    return profileStore.baseInfo?.tags?.some((t: string) => t.startsWith('group_')) || false;
// });
code = code.replace(
  /const isFavorite = computed\(\(\) => \{\s*return profileStore\.baseInfo\?\.tags\?\.some\(\(t: string\) => t\.startsWith\('group_'\)\) \|\| false;\s*\}\);/g,
  `const isFavorite = ref(false); // TODO: fetch from favorites api\n`
);
// Handle case if it wasn't replaced (maybe it's slightly different)
code = code.replace(
  /const isFavorite = computed\(\(\) => \{\s*return profileStore\.baseInfo\?\.tags\?\.some\(\(t: string\) => \['group_0',\s*'group_1',\s*'group_2',\s*'group_3'\]\.includes\(t\)\) \|\| false;\s*\}\);/g,
  `const isFavorite = ref(false); // TODO: fetch from favorites api\n`
);

// Actually, I'll just do a more robust replace for isFavorite
const isFavRegex = /const isFavorite = computed\(\(\) => \{[\s\S]*?\}\);/;
code = code.replace(isFavRegex, `const isFavorite = ref(false); // TODO: fetch from favorites api`);

// Update toggle logic to swap the ref
code = code.replace(
  /alert\(isFavorite\.value \? '已取消收藏' : '已收藏'\);\s*\/\/ trigger re-fetch to update/g,
  `alert(isFavorite.value ? '已取消收藏' : '已收藏');\n        isFavorite.value = !isFavorite.value;`
);

fs.writeFileSync('src/components/UserProfileModal.vue', code);
console.log('Fixed background, favorites 404, and dropdown clipping');
