import fs from 'fs';
import path from 'path';

function walkDir(dir, callback) {
    if (!fs.existsSync(dir)) return;
    fs.readdirSync(dir).forEach(f => {
        let dirPath = path.join(dir, f);
        let isDirectory = fs.statSync(dirPath).isDirectory();
        isDirectory ? walkDir(dirPath, callback) : callback(path.join(dir, f));
    });
}

// Map of plural to singular based on typical GKS types
const folderMap = {
    'adrs': 'adr',
    'concepts': 'concept',
    'features': 'feat',
    'algorithms': 'algo',
    'blueprints': 'blueprint',
    'entities': 'entity',
    'flows': 'flow',
    'frameworks': 'framework',
    'modules': 'mod',
    'parameters': 'params',
    'audits': 'audit',
    'runbooks': 'runbook',
    'policies': 'policy',
    'skills': 'skill',
    'personas': 'persona',
    'apis': 'api',
    'endpoints': 'endpoint',
    'entrypoints': 'entrypoint'
};

// 1. Move directories
Object.entries(folderMap).forEach(([plural, singular]) => {
    const oldDir = path.join('gks', plural);
    const newDir = path.join('gks', singular);
    if (fs.existsSync(oldDir)) {
        if (!fs.existsSync(newDir)) fs.mkdirSync(newDir, { recursive: true });
        fs.readdirSync(oldDir).forEach(file => {
            const oldPath = path.join(oldDir, file);
            const newPath = path.join(newDir, file);
            fs.renameSync(oldPath, newPath);
        });
        fs.rmdirSync(oldDir);
        console.log(`Moved contents of ${oldDir} to ${newDir}`);
    }
});

// 2. Process Files for ID and Links
let processedCount = 0;
let idUpdatedCount = 0;
let linksUpdatedCount = 0;

walkDir('gks', (filePath) => {
    if (!filePath.endsWith('.md')) return;
    if (filePath.includes('00_index') || filePath.includes('episode')) return;

    let content = fs.readFileSync(filePath, 'utf-8');
    const baseName = path.basename(filePath, '.md');
    
    let modified = false;

    // A. Fix ID
    const idRegex = /^id:\s*(.*)$/m;
    const match = content.match(idRegex);
    if (match) {
        if (match[1].trim() !== baseName) {
            content = content.replace(idRegex, `id: ${baseName}`);
            modified = true;
            idUpdatedCount++;
        }
    } else {
        // missing ID, add after ---
        content = content.replace(/^---\r?\n/, `---\nid: ${baseName}\n`);
        modified = true;
        idUpdatedCount++;
    }

    // B. Fix Relative Links -> Wikilinks
    // Match Markdown relative links pointing to other atoms: [label](../folder/ID.md)
    const linkRegex = /\[([^\]]+)\]\((?:\.\.\/|\.\/)*[a-z0-9_-]+\/([A-Z0-9_-]+)\.md(?:#[^\)]*)?\)/g;
    if (linkRegex.test(content)) {
        content = content.replace(linkRegex, '[[$2]]');
        modified = true;
        linksUpdatedCount++;
    }
    
    // Catch cases like [label](../ID.md)
    const linkRegex2 = /\[([^\]]+)\]\((?:\.\.\/|\.\/)*([A-Z0-9_-]+)\.md(?:#[^\)]*)?\)/g;
    if (linkRegex2.test(content)) {
        content = content.replace(linkRegex2, '[[$2]]');
        modified = true;
        linksUpdatedCount++;
    }

    if (modified) {
        fs.writeFileSync(filePath, content, 'utf-8');
        processedCount++;
    }
});

console.log(`Processing complete. Modified ${processedCount} files.`);
console.log(`- IDs updated: ${idUpdatedCount}`);
console.log(`- Files with links updated: ${linksUpdatedCount}`);