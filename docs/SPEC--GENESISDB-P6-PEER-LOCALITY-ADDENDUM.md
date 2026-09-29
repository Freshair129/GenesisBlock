---
doc_id: SPEC--GENESISDB-P6-PEER-LOCALITY-ADDENDUM
owner: GenesisBlockDB Engineering
version: 0.1.0b
created_at: "2026-09-24T03:25:29+07:00,ATHER,4010702"
last_update: "2026-09-24T09:40:19+07:00,Codex"
status: beta
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
  approval: owner-approved-2026-09-24
---

# P6 Addendum — Local-only generation and access-policy metadata

## Status and authority

This candidate supplements the approved P6 generation/lease/ACL contract at
docs/SPEC--GENESISDB-P6-GENERATIONS-LEASES-ACL.md. On 2026-09-24 the owner delegated the pending
peer-authority architecture choice to Astra 6. Astra selected P6-PEER-AUTHORITY-LOCAL-ONLY:
GenerationPublished and AccessPolicyChanged remain local durability/recovery records and are not
replicated to peers. The delegated choice resolves which architecture option to pursue. The owner
approved this implementation contract on 2026-09-24; source changes proceed within this scope.

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
   event-only events_since_seq return value never grants cursor progress.
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
      -> requester verifies trusted responder + exact request
      -> accept events -> monotonically advance that responder cursor

    Incoming peer event / consensus proposal
      -> verify ordinary peer envelope
      -> recursively preflight entire payload for P6 control events
          -> P6 found: reject before WAL, projection or memory mutation
          -> none: continue existing acceptance path

The cursor receipt is a sync-protocol envelope, not a WAL Event. If a trusted responder key and
request binding cannot be established, no filtered-only acknowledgement may advance a cursor.

## Security and compatibility boundaries

- Each replica owns its policy and generation. An imported graph change does not import the
  source GenerationPublished or access policy; the receiving replica publishes its own generation
  when its local frontier changes.
- This does not make ordinary graph/vector/relational data sent by transport sync subject to the
  source's read ACL. Each replica configures its own policy; transport data filtering and write
  authorization remain separate work.
- This does not establish Byzantine-safe consensus, a new peer enrollment protocol, or
  trust-on-first-use safety. Cursor receipts rely on an already trusted peer key; absent that
  prerequisite, fail without cursor advancement.
- Historic stores may already contain peer-origin P6 events. Recovery must detect identifiable
  contamination and fail closed. This change cannot reconstruct provenance erased by previous
  local compaction and must not claim those stores have been repaired.

## Acceptance tests and gates

Add a disjoint target tests/p6_peer_authority_tests.rs. Each behavior must first be observed RED
against current source, then GREEN after the serialized src/lib.rs owner implements it. Do not
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
| Cursor authenticity | Valid receipts bind trusted responder, requester, nonce and exact cursor range; tampered, replayed, mismatched, self-advertised-key, or unauthenticated receipts leave the cursor unchanged. Legacy unsigned progress never advances it. |
| Recovery | Local policy/generation survives WAL-only replay, snapshot load, fold and reopen. Identifiable foreign or invalidly signed direct/nested P6 fails with RECOVERY_REQUIRED, without ACL reset. A user property named AccessPolicyChanged is ordinary data. |
| Existing behavior | Non-P6 sync, collection dependency ordering, transaction receipts, Wave A/temporal/governance behavior and consensus without P6 remain unchanged. |

Machine-checkable commands after approval and implementation:

    cargo test --no-default-features --test p6_peer_authority_tests
    cargo test --no-default-features --test p6_generation_tests --test p6_lease_tests --test p6_visibility_tests --test wave_a_commit_tests --test temporal_queries_tests --test tx_as_of_wp22_tests --test governance_tests --test wave_b_sync_tests --test crdt_sync_tests --test consensus_commit_tests --test consensus_vote_sig_tests --test consensus_checkpoint_race_tests
    cargo check --no-default-features
    git diff --check

Also run the existing journal-format/migration and P5 transaction regression targets after their
exact names are confirmed from the current test tree. Review the known Wave B test expectation
schema_version 4 against SCHEMA_VERSION 5 before calling the expanded suite green; this is a
verification inconsistency, not a pass. Then repeat Luna Max Verify, independent Review, and Final
gates. Hosted CI, release, deployment, and external readiness remain unproven.

## Scope and approval

This addendum is HIGH risk / C-3 and is limited to local-only P6 control metadata, recursive sync
classification, safe progress receipts, and fail-closed recovery. It does not authorize P7,
merge, deployment, schema migration, entity namespace work, or unrelated transport redesign.
The owner approval and Astra decision together authorize implementation within this scope; no
additional choice among the three architecture alternatives is required.

## CHANGELOG

| Version | Date | Status | Summary | Commit | Agent |
|---|---|---|---|---|---|
| 0.1.0b | 2026-09-24 | candidate | Owner-delegated Astra decision selects local-only P6 metadata; specifies recursive ingress/egress rules, authenticated cursor progress, recovery provenance and regression gates. | working-tree | ATHER |
| 0.1.0b | 2026-09-24 | beta | Owner approved the addendum; implementation and local regression gates completed. | working-tree | Codex |
