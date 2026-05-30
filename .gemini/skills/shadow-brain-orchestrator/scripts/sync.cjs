/**
 * Sync and Index Script for Shadow Brain Orchestrator
 * 
 * Usage: node sync.cjs [brainPath]
 */
const { execSync } = require('child_process');
const { resolve, join } = require('path');

const brainPath = process.argv[2] || process.env.MSP_BRAIN_PATH;

if (!brainPath) {
  console.error('Error: MSP_BRAIN_PATH not set.');
  process.exit(1);
}

try {
  console.log(`Syncing brain at: ${brainPath}`);
  execSync('git pull origin main', { cwd: brainPath, stdio: 'inherit' });
  
  console.log('Regenerating atomic index...');
  execSync('npm run msp:index', { cwd: process.cwd(), stdio: 'inherit' });
  
  console.log('✅ Sync and indexing complete.');
} catch (error) {
  console.error('❌ Sync failed:', error.message);
  process.exit(1);
}
