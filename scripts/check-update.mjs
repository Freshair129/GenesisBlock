#!/usr/bin/env node
// Update notifier (notify-only — never installs anything). Compares the locally
// installed engine version against what is published on the npm registry and
// prints a notice if a newer version exists. Safe to run in any environment:
// network failures are swallowed and exit code stays 0 so it never breaks a
// build or a user's workflow.
//
//   node scripts/check-update.mjs            # check the `beta` dist-tag
//   node scripts/check-update.mjs latest     # check the `latest` dist-tag
//
// For a DB engine we deliberately do NOT auto-update: the operator decides when
// to upgrade (and run any schema migration). This only surfaces availability.

import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');
const pkg = JSON.parse(readFileSync(join(ROOT, 'package.json'), 'utf8'));
const NAME = pkg.name;
const CURRENT = pkg.version;
const channel = process.argv[2] || 'beta';

// Compare two semver strings. Returns >0 if a is newer than b. A release
// (no prerelease) outranks a prerelease of the same core (1.0.0 > 1.0.0-beta.1).
function cmp(a, b) {
  const re = /^(\d+)\.(\d+)\.(\d+)(?:-(.+))?$/;
  const ma = re.exec(a), mb = re.exec(b);
  if (!ma || !mb) return a === b ? 0 : a > b ? 1 : -1;
  for (let i = 1; i <= 3; i++) {
    const d = +ma[i] - +mb[i];
    if (d) return d;
  }
  const pa = ma[4], pb = mb[4];
  if (!pa && !pb) return 0;
  if (!pa) return 1; // release > prerelease
  if (!pb) return -1;
  // Both prerelease: dot-wise, numeric identifiers compare numerically.
  const sa = pa.split('.'), sb = pb.split('.');
  for (let i = 0; i < Math.max(sa.length, sb.length); i++) {
    if (sa[i] === undefined) return -1;
    if (sb[i] === undefined) return 1;
    const na = /^\d+$/.test(sa[i]), nb = /^\d+$/.test(sb[i]);
    if (na && nb) {
      const d = +sa[i] - +sb[i];
      if (d) return d;
    } else if (sa[i] !== sb[i]) {
      return sa[i] > sb[i] ? 1 : -1;
    }
  }
  return 0;
}

async function main() {
  let data;
  try {
    const res = await fetch(`https://registry.npmjs.org/${NAME}`, {
      headers: { accept: 'application/json' },
    });
    if (!res.ok) throw new Error(`registry returned ${res.status}`);
    data = await res.json();
  } catch (e) {
    // Notify-only: a failed check must never be fatal.
    console.log(`update check skipped (${e.message}); current: ${CURRENT}`);
    return;
  }

  const tags = data['dist-tags'] || {};
  const published = tags[channel];
  if (!published) {
    console.log(`no '${channel}' release published yet; current: ${CURRENT}`);
    return;
  }

  if (cmp(published, CURRENT) > 0) {
    console.log(
      `\n  ┌─ update available ─────────────────────────────────────────\n` +
      `  │ ${NAME}\n` +
      `  │ installed: ${CURRENT}   →   ${channel}: ${published}\n` +
      `  │ upgrade:   npm install ${NAME}@${channel}\n` +
      `  │ note: review CHANGELOG / run schema migration before upgrading.\n` +
      `  └────────────────────────────────────────────────────────────\n`
    );
  } else {
    console.log(`up to date (${CURRENT} is the newest on '${channel}').`);
  }
}

main();
