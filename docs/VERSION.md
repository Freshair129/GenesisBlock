---
doc_id: VERSION
status: current
version: current
owner: GenesisBlockDB Engineering
---

# GenesisBlockDB — Canonical Version (SSOT)

This file is the **canonical human-readable version/status record**. The
engine build version is anchored by `Cargo.toml`; `scripts/version.mjs`
synchronizes and checks `Cargo.lock`, `package.json`, `package-lock.json`, and
`modules.json` against it. The record in this document is maintained alongside
those files.
Per-document `version:` frontmatter across `docs/` is historically inconsistent
(whitepaper
"v2.0.0", GEMINI "1.2.0", AGENT "0.2.2b", C4 "0.1.2b", registry "0.2.1b") — treat
those as legacy labels superseded by this file.

| Field | Value |
|---|---|
| **Engine crate** (`Cargo.toml`, `Cargo.lock`, `package.json`, `package-lock.json`, `modules.json`) | `0.2.7` |
| **Product milestone** | **Mobile SDK** — Phase B (iOS/Android/React Native) shipped and published; on-device acceptance verified for iOS |
| **Status** | Advanced prototype (durable, benchmarked, suite green) |
| **Evidence baseline** | 2026-06-21 — audits P14–P30, `REPORT--2026-06-21-PERFORMANCE-AND-COMPETITIVE.md` |
| **Positioning** | Embedded analytics / agent-memory graph + vector engine |

**Version history:** `0.0.1` → `0.1.0-beta.1` (first beta, WAL compaction) →
`0.1.0-beta.2` (CI gates, core/napi split) → `0.2.0` (mobile foundation:
`mobile`/`ffi` Cargo features, C FFI layer, cross-compile CI) → `0.2.1`/`0.2.2` →
`0.2.3` (issue #125 publish cycle: `genesisdb-android` live on GitHub
Packages, `react-native-genesisdb` + the native addon's 4 platform packages
live on npm) → **`0.2.4`** (no engine change; cut to publish
`genesisdb-android` 0.1.1 with the new `x86_64` emulator ABI and
`react-native-genesisdb` 0.1.1, which finally delivers that package's Android
and iOS integration fixes — see `CHANGELOG.md`). Version `0.2.5` delivered
the Maven Central/RN distribution release; `0.2.6` delivers the typed context
query contracts and isolated GenesisRAG17 worker integration. **`0.2.7`
(2026-09-27)** publishes the main npm package and MCP CLI, standalone server
archives with SHA-256 sidecars, GHCR image, and Go SDK `v0.1.0`. The same-repo
Homebrew formula and Scoop manifest install the checksummed server archives; a
winget manifest is not published. Android SDK `0.1.2` was
published independently to Maven Central on 2026-09-27 with its public compile
dependency included; a clean x86_64 emulator consumer passed in Mobile Build
run 36316221124. Python package version
`0.1.0` is built, but the first PyPI Trusted Publishing request was rejected;
the external publisher binding or service-side OIDC state is not yet confirmed.
The crate version is kept in lock-step across
`Cargo.toml`, `Cargo.lock`, `package.json`, `package-lock.json`, and `modules.json`
by `scripts/version.mjs` (`npm run version:check` is a CI gate).

**Versioning policy (going forward):** the crate version in `Cargo.toml` is
authoritative for the build; the product milestone is a plain theme named after
the semver minor it lands in (`v0.<minor> — <theme>`) — the `MARK N` series is
frozen at XVI and is now only a historical tag, see `ROADMAP.md`'s "Milestone
naming" section; this file pins both. New/updated docs should reference this file rather than restate a
version. Legacy frontmatter is not retroactively edited (see `DOC-STATUS.md`).
