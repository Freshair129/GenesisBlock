#!/usr/bin/env node
// Vendor the iOS Swift SDK sources into react-native-genesisdb's pod.
//
// WHY THIS EXISTS
// ---------------
// `react-native-genesisdb/ios/GenesisDbModule.swift` needs the `GenesisDB`
// actor and its wire types. Those live in `ios/genesisdb/`, which sits ABOVE
// the npm package root — so npm cannot ship them (`package.json`'s `files`
// can only reach into the package's own directory). CocoaPods also cannot
// express a dependency on a Swift Package, so there is no podspec-level way
// to pull them in either.
//
// The result, before this script existed: `pod install` succeeded and the
// build then failed with `no such module 'GenesisDB'` for every consumer who
// installed from npm rather than from a monorepo checkout. Only a monorepo
// checkout worked, via a manual "Add Package Dependency" step in Xcode.
//
// Copying the sources into the pod makes it self-contained and removes that
// manual step entirely.
//
// WHY THE COPIES ARE COMMITTED (unlike `include/genesisdb.h` under ios/)
// ---------------------------------------------------------------------
// `ios/genesisdb/Sources/CGenesisDBFFI/include/genesisdb.h` is deliberately
// gitignored and copied in at build time, because nothing consumes it outside
// a build. These files are different: they must be present in the npm tarball
// at publish time and in a fresh checkout at `pod install` time, so they are
// committed. Drift is prevented the same way the header's is — by a CI job
// (`rn-ios-vendor-freshness` in .github/workflows/mobile-build.yml) that runs
// this script and fails on `git diff --exit-code`.
//
// MODULE REWRITING
// ----------------
// In the SPM package the sources live in three separate modules
// (`GenesisDBTypes`, `CGenesisDBFFI`, `GenesisDB`). Inside the pod every
// source file compiles into ONE module, so the cross-module imports must go:
//
//   import GenesisDBTypes  -> dropped (same module now)
//   import CGenesisDBFFI   -> import GenesisBlockDB
//
// `GenesisBlockDB` is the Clang module the published xcframework vends, via
// `include/module.modulemap`. The podspec's `prepare_command` downloads that
// xcframework and `s.vendored_frameworks` links it, so the C symbols the
// wrapper calls resolve inside the pod.

import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '..');
const outDir = join(repoRoot, 'react-native-genesisdb', 'ios', 'vendor');

const BANNER = `// GENERATED FILE — DO NOT EDIT.
//
// Copied from %SRC% by scripts/vendor-rn-ios-sdk.mjs.
// Edit the original there; CI job \`rn-ios-vendor-freshness\` fails if this
// copy drifts. See that script's header for why the copy exists at all.

`;

/**
 * Each entry: source path, output name, and the exact import-line rewrites to
 * apply. Rewrites are asserted to match exactly once — a silent no-op here
 * would ship a pod that cannot compile, which is the very bug this fixes.
 */
const FILES = [
  {
    src: 'ios/genesisdb/Sources/GenesisDBTypes/Types.swift',
    out: 'GenesisDBTypes.swift',
    rewrites: [],
  },
  {
    src: 'ios/genesisdb/Sources/GenesisDB/GenesisDB.swift',
    out: 'GenesisDB.swift',
    rewrites: [
      ['import CGenesisDBFFI\nimport Foundation\nimport GenesisDBTypes\n', 'import Foundation\nimport GenesisBlockDB\n'],
    ],
  },
];

mkdirSync(outDir, { recursive: true });

for (const { src, out, rewrites } of FILES) {
  // Normalise to LF on read: this repo is developed on Windows with
  // autocrlf, so the working tree can hold CRLF while git stores LF.
  // Matching import blocks (and the committed output) must not depend on
  // which platform ran the script, or the freshness gate flakes — the same
  // reason .gitattributes pins include/genesisdb.h to eol=lf.
  let text = readFileSync(join(repoRoot, src), 'utf8').split('\r\n').join('\n');

  for (const [from, to] of rewrites) {
    const hits = text.split(from).length - 1;
    if (hits !== 1) {
      console.error(
        `error: expected exactly 1 occurrence of the import block in ${src}, found ${hits}.\n` +
          `The SPM package's imports changed — update FILES[] in scripts/vendor-rn-ios-sdk.mjs.`
      );
      process.exit(1);
    }
    text = text.replace(from, to);
  }

  writeFileSync(join(outDir, out), BANNER.replace('%SRC%', src) + text, { encoding: 'utf8' });
  console.log(`wrote react-native-genesisdb/ios/vendor/${out}  (from ${src})`);
}
