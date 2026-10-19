# RCA: Multi-node Recovery Replayed Journal Events Without Membership Authorization

## Status / Date

Fix implemented — 2026-10-19. Pending CI and merge.

## Risk

`HIGH` — this changes security-sensitive database recovery and can make a
multi-node database fail to open when its retained history cannot be authorized
by the configured roster.

## Symptom

After upgrading from a version that accepted gossip identities from discovery,
opening the same database with the new operator-pinned roster could replay a
previously persisted event from an identity that is no longer authorized, or a
`MASTER` event whose signer does not have `masterAuthority`.

## Evidence

- `Storage::open_with_membership` installs the pinned roster but called
  `open_internal(..., false)`, disabling startup journal authorization.
- `open_internal` only ran journal authorization when that flag was true; the
  enabled path was used by backup restore.
- `Storage::replay_journal` rejected invalid local P6 control events, then
  applied other signed events without rechecking current roster membership or
  `MASTER` authority.
- The first recovery fix only covered `openWithMembership`. An independent
  review found that callers could use plain `open()`, replay history, then call
  `configure_peer_membership()` and start gossip without revalidation.
- Existing ingress tests covered `reconcile_state`, and backup restore had a
  journal-authorization preflight. No regression test covered opening a legacy
  journal under a newly configured membership roster.
- The accepted peer-membership ADR explicitly said WAL replay was not
  retroactively filtered against the current roster, documenting the behavior
  that allowed this upgrade-state gap.

## Root Cause

The membership security boundary was applied to live reconciliation and
membership-aware backup restore, but not to normal multi-node startup recovery.
Startup reused `open_internal` with journal authentication disabled, so the
replay loop trusted records persisted under an older authorization policy. The
public storage API also allowed roster configuration after that unverified
replay, carrying recovered state into a multi-node instance without a check.

## Why Detection Failed

The formal review and regression suite focused on new network ingress,
consensus membership, and backup restore. They did not exercise a database
containing a signed legacy journal from a formerly accepted peer and then reopen
it under a different pinned roster. The ADR also treated historical replay as
an intentional compatibility behavior, so the upgrade-state impact was not
tracked as a separate security boundary.

## Prevention

- Preflight the complete retained journal with the installed roster before
  `openWithMembership` replays it; fail closed before replay when signatures,
  membership, or `MASTER` authority do not validate.
- When `configure_peer_membership()` is called after recovery, stage the
  proposed roster, preflight retained history, roll staged state back on
  failure, and enable membership only after validation. Gossip startup also
  requires the configured-state flag.
- Keep plain single-node `open()` self-only; do not permit late membership to
  start gossip around the multi-node recovery check.
- Document the upgrade compatibility rule and the need to preserve historical
  signer/proof provenance until an operator-led trusted migration completes.
- Add positive and negative recovery tests for legacy JSONL, late configuration,
  and compacted MASTER history: authorized signers succeed and unrostered or
  non-MASTER signers fail.
- Keep backup restore and normal multi-node startup on the same authorization
  contract, with context-specific error codes.
- Before allocating the journal-wide authorization vector, count aggregate
  uncompressed event payload bytes and top-level records across legacy JSONL,
  framed legacy/history/base segments, and the active WAL. Reuse the repository's
  existing fail-closed 64 MiB / 100,000-record consensus-ledger bounds. Reject
  oversized file/body metadata before whole-file reads or decompression; allow
  16 bytes of framing per record, the fixed 72-byte segment envelope, and up to
  1 MiB of per-segment LZ4 container overhead. Bound decompression by the
  declared body length, count actual payloads before vector growth, and reject
  oversized active WAL and legacy JSONL files before reading them. Preserve
  `RECOVERY_AUTH_REQUIRED` for startup/late configuration and
  `BACKUP_AUTH_REQUIRED` for restore. Do not claim a peak-RSS bound: event JSON
  shapes can deserialize to different heap sizes, and no memory amplification
  benchmark exists. This policy trades multi-node availability for fail-closed
  bounded input if retained history exceeds the caps; issue #245 tracks the
  follow-up and its compatibility limit.

## Outcome

The final focused recovery, gossip membership, backup-restore, and P6 run passed
(48 tests total). `cargo build --no-default-features --features bins`,
`cargo fmt --all`, and `git diff --check` passed. The full Rust suite compiled
but its run was interrupted during `concurrency::bulk_add_edges_1000` after the
process exceeded 4 GiB working set without reporting progress; full-suite
execution is not claimed as passing. A fresh independent source-only review
found no remaining authorization bypass and confirmed one conditional P3
memory-scaling concern described above. CI and merge remain pending.

Issue #245 resource-bound follow-up: the membership journal preflight now checks
source metadata, aggregate uncompressed payload bytes, and record count before
allocating its authorization vectors. Review clarified that late configuration
follows local-only replay, so an over-budget rejection preserves that local
state while rolling back the staged roster. Integration review also found that
the two collection-definition scans inside `preflight_journal` re-read the same
sources; they now use the same size and exact event-budget checks on every pass,
including after the initial count pass. Regression tests cover growth beyond
the file bound and growth in event count while remaining within the byte bound.
The specs state these lifecycle and recheck rules explicitly. On Windows,
`cargo test --lib --no-default-features --features bins` passed 40/40, including
all seven new budget tests; `cargo build --no-default-features --features bins`,
`cargo check --no-default-features --features mobile`, `cargo fmt --all -- --check`,
and `git diff --check` passed. The build completed in 3m01s and the mobile check
in 36.5s. CI and merge remain pending.
