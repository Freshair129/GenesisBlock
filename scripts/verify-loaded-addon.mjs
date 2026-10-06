import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';

const expectedPath = fs.realpathSync(path.resolve(process.argv[2] ?? ''));
const hashFile = (filename) => crypto.createHash('sha256').update(fs.readFileSync(filename)).digest('hex');
const expectedHash = hashFile(expectedPath);
const require = createRequire(import.meta.url);
const Module = require('node:module');
const originalNativeLoader = Module._extensions['.node'];
const loadedPaths = [];

Module._extensions['.node'] = (module, filename) => {
  loadedPaths.push(fs.realpathSync(filename));
  return originalNativeLoader(module, filename);
};
try {
  require('../index.js');
} finally {
  Module._extensions['.node'] = originalNativeLoader;
}

if (loadedPaths.length !== 1 || loadedPaths[0] !== expectedPath) {
  throw new Error(`NATIVE_ADDON_PATH_MISMATCH:${loadedPaths.join(',')}`);
}
if (hashFile(loadedPaths[0]) !== expectedHash) {
  throw new Error('NATIVE_ADDON_CHANGED_DURING_LOAD');
}
process.stdout.write(`Loaded native addon SHA-256 ${expectedHash} at ${expectedPath}\n`);
