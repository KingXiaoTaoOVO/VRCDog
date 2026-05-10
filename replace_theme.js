const fs = require('fs');
const path = require('path');

const componentsDir = path.join('C:\\Users\\27457\\Desktop\\Project\\UnityEXE\\src\\components');

const replacements = [
  { from: /text-\[\#451a03\]/g, to: 'text-slate-900' },
  { from: /text-amber-950/g, to: 'text-slate-900' },
  { from: /text-amber-900/g, to: 'text-slate-900' },
  { from: /text-amber-800/g, to: 'text-slate-800' },
  { from: /text-amber-700\/80/g, to: 'text-slate-500' },
  { from: /text-amber-700\/60/g, to: 'text-slate-400' },
  { from: /text-amber-700/g, to: 'text-slate-600' },
  { from: /text-amber-600\/70/g, to: 'text-slate-500' },
  { from: /text-amber-600/g, to: 'text-indigo-600' },
  { from: /text-amber-500/g, to: 'text-indigo-500' },
  { from: /text-amber-400\/50/g, to: 'text-slate-400' },
  { from: /text-amber-400/g, to: 'text-indigo-400' },
  { from: /text-amber-300/g, to: 'text-indigo-300' },
  
  { from: /bg-amber-900\/5/g, to: 'bg-slate-900/5' },
  { from: /bg-amber-600/g, to: 'bg-indigo-600' },
  { from: /bg-amber-500/g, to: 'bg-indigo-500' },
  { from: /bg-amber-100\/50/g, to: 'bg-slate-100' },
  { from: /bg-amber-100/g, to: 'bg-indigo-50' },
  { from: /bg-amber-50\/80/g, to: 'bg-slate-50/80' },
  { from: /bg-amber-50\/50/g, to: 'bg-slate-50/50' },
  { from: /bg-amber-50/g, to: 'bg-slate-50' },

  { from: /border-amber-400/g, to: 'border-indigo-400' },
  { from: /border-amber-300/g, to: 'border-indigo-300' },
  { from: /border-amber-200\/50/g, to: 'border-slate-200' },
  { from: /border-amber-200\/30/g, to: 'border-slate-100' },
  { from: /border-amber-200/g, to: 'border-slate-200' },
  { from: /border-amber-100\/50/g, to: 'border-slate-100' },
  { from: /border-amber-100/g, to: 'border-slate-200' },
  { from: /border-amber-50/g, to: 'border-slate-100' },

  { from: /shadow-amber-900\/5/g, to: 'shadow-slate-900/5' },
  { from: /shadow-amber-500\/20/g, to: 'shadow-indigo-500/20' },
  { from: /shadow-amber-500\/30/g, to: 'shadow-indigo-500/30' },
  
  { from: /hover:bg-amber-600/g, to: 'hover:bg-indigo-600' },
  { from: /hover:bg-amber-500/g, to: 'hover:bg-indigo-500' },
  { from: /hover:bg-amber-200/g, to: 'hover:bg-slate-200' },
  { from: /hover:bg-amber-100/g, to: 'hover:bg-slate-100' },
  { from: /hover:bg-amber-50/g, to: 'hover:bg-slate-50' },

  { from: /hover:text-amber-900/g, to: 'hover:text-slate-900' },
  { from: /hover:text-amber-600/g, to: 'hover:text-indigo-600' },

  { from: /hover:border-amber-400/g, to: 'hover:border-indigo-400' },
  { from: /hover:border-amber-300/g, to: 'hover:border-indigo-300' },
  
  { from: /peer-checked:bg-amber-500/g, to: 'peer-checked:bg-indigo-500' },
  { from: /after:border-amber-300/g, to: 'after:border-indigo-300' },

  { from: /ring-amber-500/g, to: 'ring-indigo-500' },
  { from: /focus:ring-amber-500/g, to: 'focus:ring-indigo-500' },
  { from: /focus:border-amber-500/g, to: 'focus:border-indigo-500' },
  { from: /focus:border-amber-400/g, to: 'focus:border-indigo-400' },
  { from: /focus:border-amber-300/g, to: 'focus:border-indigo-300' },

  { from: /bg-white\/90 backdrop-blur rounded-2xl p-5 border-2 border-amber-200/g, to: 'bg-white/80 backdrop-blur-md rounded-3xl p-6 border border-slate-200' },
  { from: /bg-white\/90 backdrop-blur rounded-2xl p-6 border border-amber-100/g, to: 'bg-white/80 backdrop-blur-md rounded-3xl p-6 border border-slate-200' },
  { from: /bg-white\/60 backdrop-blur rounded-2xl p-4 border border-amber-100/g, to: 'bg-white/70 backdrop-blur-xl rounded-3xl p-5 border border-white' },

  // For specific linear gradients from old design
  { from: /from-amber-50 to-orange-50/g, to: 'from-slate-50 to-indigo-50/30' },
  { from: /from-amber-100 to-orange-100/g, to: 'from-slate-100 to-indigo-100/30' },
  
  // Specific views updates
  { from: /border-2 border-white/g, to: 'border border-slate-200' },
];

function processDir(dir) {
  const files = fs.readdirSync(dir);
  for (const file of files) {
    const fullPath = path.join(dir, file);
    if (fs.statSync(fullPath).isDirectory()) {
      processDir(fullPath);
    } else if (fullPath.endsWith('.vue')) {
      let content = fs.readFileSync(fullPath, 'utf8');
      let newContent = content;
      
      for (const rep of replacements) {
        newContent = newContent.replace(rep.from, rep.to);
      }
      
      if (content !== newContent) {
        fs.writeFileSync(fullPath, newContent);
        console.log(`Updated ${file}`);
      }
    }
  }
}

processDir(componentsDir);
console.log('Done replacing theme colors.');
