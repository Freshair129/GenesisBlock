const fs = require('fs');
const path = require('path');
const dir = 'data';
if (!fs.existsSync(dir)) fs.mkdirSync(dir, { recursive: true });
let person = 'id,name,gender\n';
for(let i=1; i<=1000; i++) person += 'p' + i + ',User' + i + ',' + (i%2===0?'M':'F') + '\n';
fs.writeFileSync(path.join(dir, 'person.csv'), person);
let post = 'id,content\n';
for(let i=1; i<=5000; i++) post += 'pst' + i + ',This is a sample post ' + i + ' content.\n';
fs.writeFileSync(path.join(dir, 'post.csv'), post);
let knows = 'source_id,target_id\n';
for(let i=1; i<=1000; i++) {
    for(let j=1; j<=10; j++) {
        let target = Math.floor(Math.random() * 1000) + 1;
        if (target !== i) knows += 'p' + i + ',p' + target + '\n';
    }
}
fs.writeFileSync(path.join(dir, 'knows.csv'), knows);
console.log('✓ Mock SNB SF0.1 data generated.');
