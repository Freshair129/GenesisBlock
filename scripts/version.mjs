#!/usr/bin/env node
// Single-source-of-truth version control for the GenesisBlockDB multi-surface
// repo. The engine version (x.y.z[-prerelease]) lives in Cargo.toml and is
// mirrored to package.json, modules.json (engine + the npm-native-addon
// surface). This CLI keeps them in lock-step and a CI gate (`check`) fails the
// build if they ever drift.
//
//   node scripts/version.mjs get                 # print engine version
//   node scripts/version.mjs check               # verify all surfaces agree (CI gate)
//   node scripts/version.mjs set 0.1.0-beta.2    # write version to all surfaces
//   node scripts/version.mjs bump patch|minor|major|prerelease
//
// Semver: MAJOR.MINOR.PATCH with an optional -prerelease (e.g. 0.1.0-beta.2).

import { readFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');
const CARGO = join(ROOT, 'Cargo.toml');
const PKG = join(ROOT, 'package.json');
const MODULES = join(ROOT, 'modules.json');

const NPM_SURFACE = '@freshair129/gks-genesis-block-native';

// MAJOR.MINOR.PATCH(-prerelease)? — prerelease is dot-separated alnum/hyphen.
const SEMVER = /^(\d+)\.(\d+)\.(\d+)(?:-([0-9A-Za-z]+(?:\.[0-9A-Za-z]+)*))?$/;

function parseSemver(v) {
  const m = SEMVER.exec(v);
  if (!m) throw new Error(`not a valid semver (x.y.z[-prerelease]): "${v}"`);
  return { major: +m[1], minor: +m[2], patch: +m[3], pre: m[4] ?? null };
}

// --- readers (Cargo.toml is the anchor) ------------------------------------

function readCargoVersion() {
  const txt = readFileSync(CARGO, 'utf8');
  // First `version = "..."` after the [package] header.
  const pkgIdx = txt.indexOf('[package]');
  const m = /^version\s*=\s*"([^"]+)"/m.exec(pkgIdx >= 0 ? txt.slice(pkgIdx) : txt);
  if (!m) throw new Error('could not find [package] version in Cargo.toml');
  return m[1];
}

function readJson(path) {
  return JSON.parse(readFileSync(path, 'utf8'));
}

function collectVersions() {
  const cargo = readCargoVersion();
  const pkg = readJson(PKG).version;
  const mods = readJson(MODULES);
  const engine = mods.engine.version;
  const surface = (mods.surfaces.find((s) => s.name === NPM_SURFACE) || {}).version;
  return { cargo, pkg, engine, surface };
}

// --- writers ----------------------------------------------------------------

function writeCargoVersion(version) {
  const txt = readFileSync(CARGO, 'utf8');
  const pkgIdx = txt.indexOf('[package]');
  let replaced = false;
  const head = pkgIdx >= 0 ? txt.slice(0, pkgIdx) : '';
  const rest = (pkgIdx >= 0 ? txt.slice(pkgIdx) : txt).replace(
    /^(version\s*=\s*)"[^"]+"/m,
    (_, p1) => {
      replaced = true;
      return `${p1}"${version}"`;
    }
  );
  if (!replaced) throw new Error('failed to rewrite Cargo.toml [package] version');
  writeFileSync(CARGO, head + rest);
}

function writeJsonVersion(path, mutate) {
  const raw = readFileSync(path, 'utf8');
  const obj = JSON.parse(raw);
  mutate(obj);
  // Preserve trailing newline style.
  const out = JSON.stringify(obj, null, 2) + (raw.endsWith('\n') ? '\n' : '');
  writeFileSync(path, out);
}

function setVersion(version) {
  parseSemver(version); // validate or throw
  writeCargoVersion(version);
  writeJsonVersion(PKG, (o) => {
    o.version = version;
  });
  writeJsonVersion(MODULES, (o) => {
    o.engine.version = version;
    const s = o.surfaces.find((x) => x.name === NPM_SURFACE);
    if (s) {
      s.version = version;
      s.minEngineVersion = version;
    }
  });
  console.log(`✓ version set to ${version} across Cargo.toml, package.json, modules.json`);
}

function nextVersion(current, kind) {
  const v = parseSemver(current);
  switch (kind) {
    case 'major':
      return `${v.major + 1}.0.0`;
    case 'minor':
      return `${v.major}.${v.minor + 1}.0`;
    case 'patch':
      return `${v.major}.${v.minor}.${v.patch + 1}`;
    case 'prerelease': {
      if (!v.pre) return `${v.major}.${v.minor}.${v.patch}-beta.1`;
      // Bump the trailing numeric identifier (e.g. beta.1 -> beta.2).
      const parts = v.pre.split('.');
      const last = parts[parts.length - 1];
      if (/^\d+$/.test(last)) parts[parts.length - 1] = String(+last + 1);
      else parts.push('1');
      return `${v.major}.${v.minor}.${v.patch}-${parts.join('.')}`;
    }
    default:
      throw new Error(`unknown bump kind "${kind}" (use major|minor|patch|prerelease)`);
  }
}

// --- commands ---------------------------------------------------------------

function cmdCheck() {
  const v = collectVersions();
  const all = [v.cargo, v.pkg, v.engine, v.surface];
  for (const ver of all) parseSemver(ver); // each must be valid semver
  const agree = all.every((x) => x === v.cargo);
  if (!agree) {
    console.error('✗ version drift detected:');
    console.error(`    Cargo.toml [package].version : ${v.cargo}`);
    console.error(`    package.json .version        : ${v.pkg}`);
    console.error(`    modules.json engine.version  : ${v.engine}`);
    console.error(`    modules.json npm surface     : ${v.surface}`);
    console.error('  Run `npm run version:set <x.y.z>` to resync.');
    process.exit(1);
  }
  console.log(`✓ all surfaces agree on version ${v.cargo}`);
}

const [cmd, arg] = process.argv.slice(2);
try {
  switch (cmd) {
    case 'get':
      console.log(readCargoVersion());
      break;
    case 'check':
      cmdCheck();
      break;
    case 'set':
      if (!arg) throw new Error('usage: version.mjs set <x.y.z[-prerelease]>');
      setVersion(arg);
      break;
    case 'bump':
      if (!arg) throw new Error('usage: version.mjs bump <major|minor|patch|prerelease>');
      setVersion(nextVersion(readCargoVersion(), arg));
      break;
    default:
      console.error('usage: version.mjs <get|check|set <v>|bump <kind>>');
      process.exit(2);
  }
} catch (e) {
  console.error(`✗ ${e.message}`);
  process.exit(1);
}
