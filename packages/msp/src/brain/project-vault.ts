import fs from 'node:fs';
import path from 'node:path';

import type { AtomType } from './types.js';

const PROJECT_SUBDIR: Partial<Record<AtomType, string>> = {
  ADR: 'adr',
  FEAT: 'feat',
  BLUEPRINT: 'blueprint',
  AUDIT: 'audit',
  CONCEPT: 'concept',
  FRAMEWORK: 'framework',
  SPEC: 'spec',
  PROTOCOL: 'protocol',
  SKILL: 'skill',
  ALGO: 'algo',
  PROTO: 'proto',
  PARAMS: 'params',
  MOD: 'mod',
  MASTER: 'master',
  HOTFIX: 'hotfix',
  INC: 'inc',
  ISSUE: 'issue',
};

export function projectRoot(cwd?: string): string {
  let dir = path.resolve(cwd ?? process.cwd());
  while (dir !== path.dirname(dir)) {
    const gks = path.join(dir, 'gks');
    const git = path.join(dir, '.git');
    if (fs.existsSync(gks) && fs.existsSync(git)) {
      return gks;
    }
    dir = path.dirname(dir);
  }
  throw new Error(
    `projectRoot: no ancestor of ${cwd ?? process.cwd()} contains both gks/ and .git/`,
  );
}

export function projectSubdir(type: AtomType, root?: string): string {
  const sub = PROJECT_SUBDIR[type];
  if (sub === undefined) {
    throw new Error(`projectSubdir: ${type} is not routed to the project brain`);
  }
  return path.join(root ?? projectRoot(), sub);
}
