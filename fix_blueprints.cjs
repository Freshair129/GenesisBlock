const fs = require('fs');
const path = require('path');

const dir = 'gks/blueprint';
const files = fs.readdirSync(dir).filter(f => f.endsWith('.md'));

files.forEach(file => {
  const filePath = path.join(dir, file);
  if (file === 'BLUEPRINT--DYNAMIC-COMPOUND-ID-IMPLEMENTATION.md') return;
  
  let content = fs.readFileSync(filePath, 'utf8');
  const frontmatterMatch = content.match(/^---\n([\s\S]*?)\n---/);
  
  if (frontmatterMatch) {
    const frontmatter = frontmatterMatch[1];
    const lines = frontmatter.split('\n');
    const hasTopLevelLinkedSymbols = lines.some(line => line.startsWith('linked_symbols:'));
    
    if (!hasTopLevelLinkedSymbols) {
      const newFrontmatter = frontmatter + '\nlinked_symbols: []';
      const newContent = content.replace(/^---\n([\s\S]*?)\n---/, `---\n${newFrontmatter}\n---`);
      fs.writeFileSync(filePath, newContent);
      console.log(`Updated ${file}`);
    } else {
      console.log(`Skipped ${file} (already has linked_symbols)`);
    }
  }
});
