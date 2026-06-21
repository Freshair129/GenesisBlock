# GenesisDB — Canonical Version (SSOT)

This file is the **single source of truth for version/status**. Per-document
`version:` frontmatter across `docs/` is historically inconsistent (whitepaper
"v2.0.0", GEMINI "1.2.0", AGENT "0.2.2b", C4 "0.1.2b", registry "0.2.1b") — treat
those as legacy labels superseded by this file.

| Field | Value |
|---|---|
| **Engine crate** (`Cargo.toml`, `package.json`) | `0.0.1` |
| **Product milestone** | **Mark XII** — benchmark evidence & hardening |
| **Status** | Advanced prototype (durable, benchmarked, suite green) |
| **Evidence baseline** | 2026-06-21 — audits P14–P25, `REPORT--2026-06-21-PERFORMANCE-AND-COMPETITIVE.md` |
| **Positioning** | Embedded analytics / agent-memory graph + vector engine |

**Versioning policy (going forward):** the crate version in `Cargo.toml` is
authoritative for the build; `ROADMAP.md` Mark-N is the product milestone; this
file pins both. New/updated docs should reference this file rather than restate a
version. Legacy frontmatter is not retroactively edited (see `DOC-STATUS.md`).
