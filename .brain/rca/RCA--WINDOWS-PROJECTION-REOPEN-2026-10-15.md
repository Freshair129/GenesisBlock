---
version: "0.1.0b"
status: candidate
created_at: "2026-10-15,Codex"
scope: Windows relational projection recovery for FUNG issue 41
---

# Windows projection reopen

Parent: `docs/MASTER-SPEC--GENESIS-DB.md` unified storage boundary and
`docs/SPEC--WAVE-A-COMMIT-CORRECTNESS.md`. This bounded C-2 repair is authorized
by the user's request to repair FUNG #41 without data loss and test reopening.

## Evidence before repair

FUNG's copied existing profile has a valid SQLite projection and v13 schema.
`schema_reopen_existing_profile_probe` fails at `Storage::open`, before adapter
installation, with `REL_SCHEMA_VERSION_CONFLICT`. Its retained journal includes
earlier versions. The fallback deletes the projection while its connection is
open and ignores the Windows deletion error, then opens the same old database.

## Repair contract

Keep the original connection/file. Replay into a temporary engine-owned SQLite
database (empty filename; automatically cleaned on close). Only successful replay
may publish via SQLite's transactional backup API. Restore the original handle
on both success and error. Never weaken schema identity or version validation.
No application-side SQLite mutation, database reset, or journal-format change.

## Acceptance

- Retained v1→v2 upgrade journal plus existing v2 projection reopens repeatedly.
- Rows before/after upgrade and edits after reopen survive subsequent boots.
- Failed rebuild retains the original projection and reports the error.
- Existing FUNG profile copy opens, then actual normal-profile app closes and
  reopens, with offline table snapshots/asset hashes compared against backup.
- Preserve the exact FUNG engine baseline; no unrelated engine upgrades.

Test results and limitations are recorded after execution in the companion report.
