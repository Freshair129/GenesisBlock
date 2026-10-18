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
- Preserve one decoded copy of retained events during ordering preflight. The
  preflight still scales with retained-journal length; a source review identified
  bounded-memory preflight as a low-severity follow-up for very large
  full-retention databases. No threshold was measured.

## Outcome

The final focused recovery, gossip membership, backup-restore, and P6 run passed
(48 tests total). `cargo build --no-default-features --features bins`,
`cargo fmt --all`, and `git diff --check` passed. The full Rust suite compiled
but its run was interrupted during `concurrency::bulk_add_edges_1000` after the
process exceeded 4 GiB working set without reporting progress; full-suite
execution is not claimed as passing. A fresh independent source-only review
found no remaining authorization bypass and confirmed one conditional P3
memory-scaling concern described above. CI and merge remain pending.
