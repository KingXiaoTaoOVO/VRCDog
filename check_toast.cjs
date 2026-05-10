const fs = require('fs');
const content = fs.readFileSync('src/App.vue', 'utf8');
const lines = content.split('\n');
lines.forEach((l, i) => {
  if(l.toLowerCase().includes('message') || l.toLowerCase().includes('toast') || l.toLowerCase().includes('alert')) {
    console.log(`${i+1}: ${l}`);
  }
});
