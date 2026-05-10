const fs = require('fs');
const files = [
  'C:/Users/27457/Desktop/Project/UnityEXE/src/components/PlayerListView.vue',
  'C:/Users/27457/Desktop/Project/UnityEXE/src/components/ModerationView.vue',
  'C:/Users/27457/Desktop/Project/UnityEXE/src/components/FriendLogView.vue',
  'C:/Users/27457/Desktop/Project/UnityEXE/src/components/FeedView.vue'
];
files.forEach(f => {
  let c = fs.readFileSync(f, 'utf8');
  if (c.includes(':item=')) {
    c = c.replace(/:item=/g, ':data=');
    fs.writeFileSync(f, c);
  }
});
