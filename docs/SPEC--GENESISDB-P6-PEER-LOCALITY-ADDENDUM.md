---
doc_id: SPEC--GENESISDB-P6-PEER-LOCALITY-ADDENDUM
owner: GenesisBlockDB Engineering
version: 0.1.15b
created_at: "2026-09-24T03:25:29+07:00,ATHER,4010702"
last_update: "2026-10-05T10:35:01+07:00,Codex"
status: active
superseded_by: null
attributes:
  doc_type: spec
  domain: replication-security
  scope: p6-peer-local-only-control-events
  parent_doc: SPEC--GENESISDB-P6-GENERATIONS-LEASES-ACL
  decision_id: P6-PEER-AUTHORITY-LOCAL-ONLY
  decision_authority: owner-delegated-to-Astra-6
  risk: HIGH
  complexity: C-3
  approval: owner-approved-2026-09-24; delegated-Astra-6-approved-2026-10-04
---

# P6 Addendum — Local-only generation and access-policy metadata

## Status and authority

This active addendum supplements the approved P6 generation/lease/ACL contract at
docs/SPEC--GENESISDB-P6-GENERATIONS-LEASES-ACL.md. On 2026-09-24 the owner delegated the pending
peer-authority architecture choice to Astra 6. Astra selected P6-PEER-AUTHORITY-LOCAL-ONLY:
GenerationPublished and AccessPolicyChanged remain local durability/recovery records and are not
replicated to peers. The delegated choice resolves which architecture option to pursue. The owner
approved v0.1.0b on 2026-09-24. After the Review Gate findings, the owner delegated the bounded
receipt-authority correction to Astra 6. On 2026-10-04 Astra approved v0.1.2b's fail-closed remote cursor
progress and clarified that neither heartbeat-learned nor pre-existing Storage.peers entries
establish trust because the map carries no provenance. The production requester supplies no
trusted responder key in this correction; remote cursor progress remains unavailable until a
separate trust contract is approved. The signed receipt must also exactly match the outer cursor.
No peer enrollment or trust-on-first-use is added. The approved correction is implemented in the
isolated worktree and the local Verify Gate passed. Review returned PASS_WITH_LIMITATIONS; the
stale Wave B note and RCA status are corrected in this documentation update set. Astra 6 Final
Gate returned PASS_WITH_LIMITATIONS and justified local sign-off; it relied on supplied local
verification evidence rather than rerunning tests. Hosted CI, release, deployment, merge, and P7
remain NOT_RUN and out of scope.

Those PASS_WITH_LIMITATIONS results cover only the bounded receipt-authority correction. The
initial broad P6 Review Gate returned FAIL on the separate temporal-frontier, snapshot ACL CAS-chain,
Plan-only temporal, and ACL revision-overflow findings recorded in the parent P6 specification and
RCAs. The approved corrections preserve Plan's no-publication/no-write behavior and reject ACL
revision overflow. Current independent Verify passed: P6 acceptance 49/49, focused remediation
34/34, `cargo check`, format, docs validation (0 violations in 240 files), and diff check. The
independent broad Review Gate returned PASS. Its bounded, non-blocking limitation is that the
stale-publication Plan fallback lacks a dedicated regression fixture; Review relied on Verify
receipts and did not rerun suites. Astra 6 Final returned PASS_WITH_LIMITATIONS, approving the
reviewed local P6 slice at HEAD `ba488dd1e22cd24fdd2c7238a1c5de34f6f9fd1e`. Astra accepted that
bounded coverage limitation because the guarded candidate-frontier path matches generation
pinning; the gate made no edits and reran no tests. Current-patch hosted/cross-platform CI and
release/deployment qualification remain NOT_RUN. The earlier 45/45, 32/32 and 130/130 counts are
historical checkpoints; the receipt-authority sign-off alone is not whole-P6 approval.
This status update does not change the approved local-only peer-control boundary or add peer
enrollment/trust behavior.

Decision evidence was reviewed against HEAD 40107027465c246e7f6c8aed3f229772e080ee6d. The
reviewed src/lib.rs SHA-256 was
7B3E14911E608938EEA37CD5A8B5048A7E75F89DF69FF4E4FF521E359AF6D4F3. That was the pre-approval
decision snapshot; implementation evidence is recorded in the RCA and regression gates below.

## Decision and rationale

Keep P6 control metadata local to the replica whose WAL, snapshot, manifest digest, and policy
authority it describes. A peer-to-principal mapping would not make origin-local generation
coordinates meaningful on a receiver, and peer signature authenticity is not ManagePolicy
authorization. Deferring the decision would leave paths available to apply remote policy and
generation events.

This is HIGH risk / C-3 because it changes sync, consensus admission, recovery provenance, and
cursor behavior. It does not add peer-to-principal authorization, authenticated transport, write
authorization, or ACL filtering of ordinary synchronized graph/vector/relational data.

## Requirements

1. **Typed recursive classification.** A P6 control event is a GenerationPublished or
   AccessPolicyChanged variant at any depth inside Event::Batch. Classify by typed event variant,
   not actor string, folded flag, signer label, or user properties containing similar text.
2. **Local durability remains enabled.** Locally authorized policy changes and locally published
   generations continue to be signed, persisted in WAL, represented in snapshots/folds, and
   replayed. Distinguish local persistence from peer-origin persistence; do not apply peer-ingress
   rejection indiscriminately to local writes or recovery.
3. **Peer ingress rejects before side effects.** Before the first WAL append or in-memory mutation,
   recursively preflight the entire reconciliation request. Reject any direct, nested, or folded
   P6 event with stable error prefix P6_LOCAL_ONLY; do not partially accept ordinary siblings.
   Apply the same rule at consensus proposal creation, incoming proposal admission, final quorum
   commit (including cached proposals), and the peer persistence sink. A claimed local peer ID or
   valid peer signature does not bypass the rule.
4. **All outbound producers exclude local P6.** Apply the recursive classifier to Lamport
   events_since, sequence events_since_seq, sync_delta, gossip response construction, and
   folded/base materializations. Never strip children from or re-sign a signed batch. A signed
   envelope containing both P6 and transferable data is not exportable as-is: fail with
   P6_MIXED_BATCH_UNEXPORTABLE, send no later frames across the blocked sequence, and do not
   acknowledge progress past it. Legacy Vec-only enumeration returns no events plus an explicit
   diagnostic for that failed request; an empty vector is not a cursor receipt.
5. **Cursor is a complete-prefix receipt.** In sequence sync, through_seq denotes the
   responder-local sequence prefix fully accounted for, not the last event emitted. Control-only
   P6 frames may be skipped within that prefix. Do not advance over a transferable frame that was
   not sent. Treat every folded/same-sequence group atomically: send every eligible member before
   acknowledging that sequence, or fail without advancing it. Respect datagram budget, dependency
   ordering, history horizon, and cursor-zero/base-sequence bootstrap semantics.
6. **Filtered progress is authenticated.** When filtering P6-only frames creates genuine cursor
   progress, return an empty-or-data PushDelta with a versioned SyncProgressReceipt, not an
   unsigned through_seq alone. The signed receipt binds protocol version, responder peer ID,
   requester peer ID, request nonce, from_seq, and through_seq. Verify against the responder key
   already trusted for that peer and the exact outstanding request before advancing that
   responder's cursor. A key advertised only in the same unauthenticated heartbeat/reply is not
   sufficient. If the peer-trust context cannot authenticate the receipt, return
   SYNC_AUTH_REQUIRED, accept no cursor progress, and do not use legacy unsigned progress. The
   event-only events_since_seq return value never grants cursor progress. Before advancing, the
   requester MUST also require the outer PushDelta.through_seq to equal the signed receipt's
   through_seq; a mismatch rejects cursor progress even when the receipt signature is valid.
7. **Recovery fails closed on foreign P6 provenance.** Verify direct and nested P6 events during
   replay and snapshot-authority validation. Distinguish locally authored durable materializations
   from peer-origin envelopes using verified provenance; do not infer locality from folded, actor,
   or claimed peer ID. If identifiable foreign or unverifiable P6 exists in historical WAL/fold/
   snapshot material, set RECOVERY_REQUIRED; do not silently drop it or reopen with Disabled
   policy. If prior compaction erased provenance, report that the store cannot be certified by
   this change; data repair is a separate recovery task.
8. **Compatibility remains explicit.** Receivers reject P6 events from older peers with
   P6_LOCAL_ONLY. Ordinary transferable events retain original signatures and ordering. This
   addendum does not claim wire compatibility with peers relying on unsigned cursor advancement;
   a peer without the signed progress receipt must not advance a filtered cursor.

## Architecture

    Local policy/generation operation
      -> local authorization / snapshot validation
      -> signed P6 WAL event -> local fold, snapshot and replay

    Peer sync request -> scan contiguous responder-local sequence groups
      -> recursively classify signed envelopes
          -> control-only P6 group: account as filtered
          -> transferable group: emit unchanged signed events
          -> mixed signed Batch or budget obstruction: fail before acknowledging that group
      -> signed SyncProgressReceipt(request nonce, from_seq, through_seq, peer IDs)
      -> production requester supplies no trusted responder key in this correction
      -> heartbeat data and Storage.peers membership are discovery only, not trust
          -> no independent key: SYNC_AUTH_REQUIRED; cursor unchanged
      -> private verifier tests supply an explicit trusted-key fixture
          -> verify signature, exact request, and outer through_seq equality
              -> valid fixture: return the signed, verified sequence
              -> any missing input or mismatch: reject
      -> production has no independent remote key; no remote cursor is advanced

    Incoming peer event / consensus proposal
      -> verify ordinary peer envelope
      -> recursively preflight entire payload for P6 control events
          -> P6 found: reject before WAL, projection or memory mutation
          -> none: continue existing acceptance path

The cursor receipt is a sync-protocol envelope, not a WAL Event. `Storage.peers` has no trust
provenance, including entries present before gossip startup. If an independent trusted responder
key and request binding cannot be established, no acknowledgement may advance a remote cursor.
The current production path has no independent remote-key source and therefore remains fail-closed.

## Security and compatibility boundaries

- Each replica owns its policy and generation. An imported graph change does not import the
  source GenerationPublished or access policy; the receiving replica publishes its own generation
  when its local frontier changes.
- This does not make ordinary graph/vector/relational data sent by transport sync subject to the
  source's read ACL. Each replica configures its own policy; transport data filtering and write
  authorization remain separate work.
- This does not establish Byzantine-safe consensus, a new peer enrollment protocol, or
  trust-on-first-use safety. No current production source provides an independent remote receipt
  key; heartbeat-advertised and pre-existing `Storage.peers` keys are insufficient. Remote cursor
  progress remains disabled until a separate trust contract supplies that authority.
- Historic stores may already contain peer-origin P6 events. Recovery must detect identifiable
  contamination and fail closed. This change cannot reconstruct provenance erased by previous
  local compaction and must not claim those stores have been repaired.

## Acceptance tests and gates

Add requester-path regressions to tests/p6_peer_authority_tests.rs and private verifier tests for
explicit trusted-key fixtures. Before implementation, the confirmed requester regressions
(heartbeat-only key trust, pre-existing peer-map trust, heartbeat key replacement, and outer/signed
cursor mismatch) must be observed RED where the current receive path accepts progress. Then run the
requester regressions GREEN after the serialized src/lib.rs owner implements the correction. Do not
inspect or touch tests/zz_probe_discriminates.rs.

| Area | Required assertion |
|---|---|
| Unauthorized policy ingress | Correctly signed peer policy changes, including folded/stale/jumped revisions and forged local signer IDs, return P6_LOCAL_ONLY; WAL bytes, frontiers, policy, data, and leases are unchanged before and after reopen. |
| Foreign generation ingress | Lower, higher, and coincident local sequence coordinates and foreign manifest digests are rejected before WAL; none becomes a reusable local generation. |
| Whole-request preflight | Direct, singly/multiply nested, folded P6 and ordinary siblings before/after it are rejected atomically by reconcile; no sibling persists or applies, including after reopen. |
| Consensus | Local proposal creation, incoming proposal admission, and final commit reject P6 even with a valid signature/quorum or cached proposal; no deferred P6 applies on replay. |
| Outbound completeness | Lamport, sequence, delta, gossip and folded producers expose no direct/nested P6; permitted signed events remain signature-equivalent and ordered. |
| Mixed signed batch | A batch containing P6 and transferable children fails with P6_MIXED_BATCH_UNEXPORTABLE; no rewritten envelope, later frame, or cursor progress crosses the obstruction. |
| Cursor progression | Test filtered-only ranges, trailing filtered frames, mixed data/P6 sequence groups, same-sequence folds, budget boundaries, base seq 0, retries, and no skipping unsent transferable frames. |
| Cursor authenticity | Private verifier tests with an explicit trusted-key fixture require valid signature, responder/requester IDs, nonce, from-sequence, non-reversed range, and exact outer/signed through-sequence equality; malformed, absent, replayed, mismatched, or invalid inputs fail. The production requester supplies no remote trust key in this correction; heartbeat-only, pre-existing peer-map, and replacement-heartbeat receipts leave the cursor unchanged. Legacy unsigned progress never advances it. |
| Recovery | Local policy/generation survives WAL-only replay, snapshot load, fold and reopen. Identifiable foreign or invalidly signed direct/nested P6 fails with RECOVERY_REQUIRED, without ACL reset. A user property named AccessPolicyChanged is ordinary data. |
| Existing behavior | Non-P6 sync, collection dependency ordering, transaction receipts, Wave A/temporal/governance behavior and consensus without P6 remain unchanged. |

Machine-checkable commands used for local verification:

    cargo test --no-default-features --test p6_peer_authority_tests
    cargo test --no-default-features --lib
    cargo test --no-default-features --test p6_generation_tests --test p6_lease_tests --test p6_visibility_tests --test wave_a_commit_tests --test temporal_queries_tests --test tx_as_of_wp22_tests --test governance_tests --test wave_b_sync_tests --test crdt_sync_tests --test consensus_commit_tests --test consensus_vote_sig_tests --test consensus_checkpoint_race_tests
    cargo check --no-default-features
    git diff --check

The supplemental compatibility targets were also run: `journal_format_tests` (10/10),
`journal_migration_tests` (4/4), and `unified_transaction_p5_tests` (4/4). The previous Wave B
schema warning was stale: the current source defines `SCHEMA_VERSION = 6`, while
`gossip_rejects_old_schema_before_sending_delta` sends version 3 and expects
`UpgradeRequired { schema_version: SCHEMA_VERSION }`; these values are consistent.

## Local verification status (2026-10-04)

| Gate | Result | Evidence |
|---|---|---|
| Pre-fix RED | PASS | Four requester assertions failed as intended on the pre-correction source; five existing assertions passed. |
| Unit and requester GREEN | PASS | Library 7/7; `p6_peer_authority_tests` 9/9. |
| Focused P6 regression matrix | PASS | 65/65 across the 12 named generation, lease, visibility, transaction, temporal, governance, sync, and consensus targets above. |
| Journal and P5 compatibility | PASS | 18/18 across journal format, journal migration, and unified P5 transaction targets. |
| Compile and repository checks | PASS | `cargo check --no-default-features`; `docs:validate` 0 violations in 240 files; rustfmt and `git diff --check`. |
| Independent Review | PASS_WITH_LIMITATIONS | No blocking code issue; its two P2 documentation findings are corrected in this update set. |
| Astra 6 Final Gate | PASS_WITH_LIMITATIONS | No blocker; local sign-off justified. Gate relied on supplied evidence and did not rerun tests. |

One initial post-correction requester test run failed with Windows OS error 112 because C: had
zero free bytes and `tempfile` used the default C: temp directory. The same target passed 9/9
after rerun with process-scoped TEMP/TMP/TMPDIR on E:. No repository change was made to address
the machine's disk-space condition.

## Scope and approval

This addendum is HIGH risk / C-3 and is limited to local-only P6 control metadata, recursive sync
classification, safe progress receipts, and fail-closed recovery. The original v0.1.0b owner
approval and Astra 6's delegated approval of the v0.1.2b correction authorize implementation
within this scope. It does not authorize peer enrollment, P7, merge, deployment, schema migration,
entity namespace work, or unrelated transport redesign; no further choice among the three
architecture alternatives is required.

## CHANGELOG

| Version | Date | Status | Summary | Commit | Agent |
|---|---|---|---|---|---|
| 0.1.15b | 2026-10-05 | active | Record Astra 6 Final PASS_WITH_LIMITATIONS for the reviewed local P6 slice; accept bounded Plan fallback fixture limitation; hosted/cross-platform CI and release/deployment remain NOT_RUN | working-tree | Astra 6 / Codex |
| 0.1.14b | 2026-10-05 | active | Record corrected-tree independent Verify PASS (P6 49/49, focused 34/34) and broad Review PASS; disclose stale-publication Plan fallback fixture limitation; Astra Final pending; local-only scope unchanged | working-tree | Codex |
| 0.1.13b | 2026-10-05 | active | Record corrected-tree independent Verify PASS after ACL overflow fix; P6 49/49, focused 34/34, check/fmt/docs/diff pass; independent Review rerun and Astra Final pending | working-tree | Codex |
| 0.1.12b | 2026-10-05 | active | Record local checked-increment fix for ACL u64::MAX overflow and green regression/suites; independent Verify and Review pending; peer-control boundary unchanged | working-tree | Codex |
| 0.1.11b | 2026-10-05 | active | Add RED evidence for the broad Review P2 ACL revision overflow; source correction and fresh Verify/Review pending; peer-control boundary unchanged | working-tree | Codex |
| 0.1.10b | 2026-10-05 | active | Record broad Review P2: saturating ACL revision increment permits changed policy at u64::MAX; contract/RCA updated, peer-control boundary unchanged; correction and gates pending | working-tree | Codex |
| 0.1.9b | 2026-10-05 | active | Record fresh current-tree independent Verify PASS after Plan-only temporal correction; P6 49/49, focused 32/32, docs/fmt/diff/check pass; Review and Astra Final pending; peer-control contract unchanged | working-tree | Codex |
| 0.1.8b | 2026-10-05 | active | Record Plan-only temporal RED/GREEN fix and current local P6 49/49, focused 32/32, and expanded matrix 130/130; fresh Verify/Review and Astra Final pending; peer-control contract unchanged | working-tree | Codex |
| 0.1.7b | 2026-10-05 | active | Record independent P6 Verify 49/49 and focused 31/31; broad Review rerun identifies a separate Plan-only temporal bound path; Astra Final remains NOT_RUN; peer-control contract unchanged | working-tree | Codex |
| 0.1.6b | 2026-10-05 | active | Sync parent P6 remediation status: temporal-frontier and snapshot ACL CAS-chain corrections pass local Verify; independent Verify, broad Review rerun and Astra 6 Final pending; peer-control contract unchanged | working-tree | Codex |
| 0.1.5b | 2026-10-05 | active | Scope the earlier receipt-authority PASS_WITH_LIMITATIONS gates to that slice; record broad P6 Review FAIL and Final NOT_RUN for temporal-frontier and ACL CAS-chain remediation | working-tree | Codex |
| 0.1.4b | 2026-10-04 | active | Records Astra 6 Final Gate PASS_WITH_LIMITATIONS and local sign-off; hosted CI/release/deployment remain NOT_RUN. | working-tree | Astra 6 / Codex |
| 0.1.3b | 2026-10-04 | active | Records local implementation and 99 passing green tests, corrects the stale Wave B schema warning, and records Review Gate status; Astra 6 Final Gate pending. | working-tree | Codex |
| 0.1.2b | 2026-10-04 | active | Astra 6 selected fail-closed progress for every remote peer: no trusted-key snapshot from Storage.peers, no production enrollment, explicit trusted-key verifier fixtures, exact outer/signed cursor equality. | working-tree | Astra 6 / Codex |
| 0.1.1b | 2026-10-04 | approved for implementation | Astra 6 approved the bounded fail-closed receipt correction under owner delegation: heartbeat-only keys do not establish trust, and outer through_seq must equal signed through_seq; requester-side non-advance regressions required. | working-tree | Astra 6 / Codex |
| 0.1.0b | 2026-09-24 | candidate | Owner-delegated Astra decision selects local-only P6 metadata; specifies recursive ingress/egress rules, authenticated cursor progress, recovery provenance and regression gates. | working-tree | ATHER |
| 0.1.0b | 2026-09-24 | beta | Owner approved the addendum; implementation and local regression gates completed. | working-tree | Codex |
