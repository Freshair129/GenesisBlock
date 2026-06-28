# GenesisBlockDB — Canonical Version (SSOT)

This file is the **single source of truth for version/status**. Per-document
`version:` frontmatter across `docs/` is historically inconsistent (whitepaper
"v2.0.0", GEMINI "1.2.0", AGENT "0.2.2b", C4 "0.1.2b", registry "0.2.1b") — treat
those as legacy labels superseded by this file.

| Field | Value |
|---|---|
| **Engine crate** (`Cargo.toml`, `package.json`, `modules.json`) | `0.2.0` |
| **Product milestone** | **Mark XVI** — mobile SDK & embedded app (Phase 0 foundation) |
| **Status** | Advanced prototype (durable, benchmarked, suite green) |
| **Evidence baseline** | 2026-06-21 — audits P14–P30, `REPORT--2026-06-21-PERFORMANCE-AND-COMPETITIVE.md` |
| **Positioning** | Embedded analytics / agent-memory graph + vector engine |

**Version history:** `0.0.1` → `0.1.0-beta.1` (first beta, WAL compaction) →
`0.1.0-beta.2` (CI gates, core/napi split) → **`0.2.0`** (mobile foundation:
`mobile`/`ffi` Cargo features, C FFI layer, cross-compile CI). The crate version
is kept in lock-step across `Cargo.toml`, `package.json`, and `modules.json` by
`scripts/version.mjs` (`npm run version:check` is a CI gate).

**Versioning policy (going forward):** the crate version in `Cargo.toml` is
authoritative for the build; `ROADMAP.md` Mark-N is the product milestone; this
file pins both. New/updated docs should reference this file rather than restate a
version. Legacy frontmatter is not retroactively edited (see `DOC-STATUS.md`).
