import fs from 'node:fs';
import path from 'node:path';

const dir = 'G:/GenesisBlock_Dev/GenesisBlock/benches/snb/data';
if (!fs.existsSync(dir)) fs.mkdirSync(dir, { recursive: true });

// Person.csv
let person = 'id,name,gender\n';
for(let i=1; i<=1000; i++) person += p,User,\n;
fs.writeFileSync(path.join(dir, 'person.csv'), person);

// Post.csv
let post = 'id,content\n';
for(let i=1; i<=5000; i++) post += pst,This is a sample post  content.\n;
fs.writeFileSync(path.join(dir, 'post.csv'), post);

// Knows.csv
let knows = 'source_id,target_id\n';
for(let i=1; i<=1000; i++) {
    for(let j=1; j<=10; j++) {
        let target = Math.floor(Math.random() * 1000) + 1;
        if (target !== i) knows += p,p\n;
    }
}
fs.writeFileSync(path.join(dir, 'knows.csv'), knows);

console.log('✓ Mock SNB SF0.1 data generated at', dir);
