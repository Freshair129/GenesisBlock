---
proposed_id: ADR--GENESISDB-JOURNAL-HISTORY
type: adr
status: accepted
tier: strategy
cluster: implementation_flow
role: "ADR — resolve WAL-compaction vs version-history vs projection-rebuildability; journal segments + fold-based retention + history-horizon contract; authority table + invariants I1–I9"
date: 2026-08-17
deciders: Boss
related:
  - adr/ADR--GENESISDB-EMBEDDED-SQLITE-SUBSTRATE
  - adr/ADR--GENESISDB-TEMPORAL-MODEL
  - PLAN--GNSE-REMEDIATION-MULTIAGENT
  - BENCH-SPEC--HQL-MOAT-AND-EXPRESSIVENESS
  - SPEC--GENESISDB-UNIFIED-OPERATIONAL-BOUNDARY-V1
---

# ADR: Journal & History (one log, sealed segments, an explicit history horizon)

**Status:** Accepted (2026-08-17) · **Deciders:** Boss · **Work packet:** WP-0.1 of [PLAN--GNSE-REMEDIATION-MULTIAGENT](../PLAN--GNSE-REMEDIATION-MULTIAGENT.md)

## 1. Context

Three committed decisions are mutually inconsistent, and the interview PROPOSAL already flagged the conflict as "unresolvable without an ADR":

1. **WAL is the only durability authority** — asserted by five committed docs (SQLITE-SUBSTRATE ADR §2.2, SPEC--SQLITE-SUBSTRATE-S0-S1 §3.1, UNIFIED-OPERATIONAL-BOUNDARY §5/§7, MASTER-SPEC 2.1.0, C4 C1).
2. **WAL compaction truncates to live state** (PR #28, wired into `save_state`) — built to fix a measured ~20 GB WAL @ 1M nodes, and required on mobile (foreground compaction bounds on-device growth).
3. **Bitemporal correctness is the honest moat** (interview R2) and the BENCH-SPEC makes a **transaction-time probe a mandatory gate** — which requires history that decision 2 destroys.

Verified code facts (8-agent fact-check 2026-08-17, re-verified by the review panel) that force the resolution:

- `compact_unlocked`/`build_compacted_wal` keeps only `valid_to == None` nodes/edges, **re-signs peer events with the local key**, and reduces transactions to empty identity stubs. Superseded node versions exist *only* as WAL lines — no read API reaches them — so every checkpoint permanently destroys version history and commit order.
- Plain WAL lines carry **no sequence number** (`SignedEvent { event, signature, signer_peer_id }`); three clocks exist (u64 `commit_sequence` on the transaction path only, u32 Lamport on ordinary mutations, per-replica file order). Note: journal frame order is a **per-replica recovery order only** — gossip convergence is keyed on the order-independent state Merkle root, never on frame order.
- The acked-write-loss recovery window (either/or snapshot-vs-replay) is **already fixed on main** (`4072cc9`, RCA--WAL-TAIL-REPLAY / WP-1.1): `open()` replays the WAL tail past a byte-positional `wal_frontier` (bytes + sha256) recorded in `state.json`. This ADR builds on that landed cursor and migrates it (D5).
- CRDT sync reads the WAL file directly (`events_since` → `PushDelta`); the sync cursor is the **Lamport u32** (`from_clock`), gossip is single-UDP-datagram (~60 KB cap), and both reconcile paths currently `fetch_max` a **peer's** `commit_sequence` into the local counter — with a live collision hazard on `applied_transactions.commit_sequence UNIQUE`.

This ADR decides the semantics. Implementation lands in WP-1.2 (framed journal + commit stamps + sync cursor) and WP-1.3 (fold-based retention + probes). It stays inside the UNIFIED-OPERATIONAL-BOUNDARY line ("no new storage engine in v1"): the journal remains the existing WAL, restructured — DashMaps, SQLite projection, HNSW, snapshots all keep their roles.

## 2. Decision

> **Journal is Truth. Snapshots are Materializations. Indexes and projections are disposable.**
> Checkpoint advances a frontier; retention **folds** old history into a materialized base segment — it never leaves the journal unable to recover current state on its own. How much *history* survives is an explicit, per-deployment retention profile — and every query surface can see the horizon.

### D1 — Journal lifecycle: active WAL + sealed immutable segments

```
                writes (group-commit fsync, unchanged)
                     │
                     ▼
            wal/active.gwal            ← the only mutable journal file
                     │ seal (size threshold, or at checkpoint) — on the WAL-writer
                     ▼                   thread via WalMsg (only the handle owner
        journal/J000042.gseg             rotates the file; reuses PR #28's
                     │                   close/reopen dance)
                     │ retention (D3): FOLD, never bare-drop
                     ▼
        journal/B000007.gseg           ← base segment: materialized live state
                                         as-of an old frontier (derived, §D3)
```

- **Two segment kinds.** *History segments* (`J*.gseg`) store original event bytes — byte-immutable, original signatures preserved (I4). *Base segments* (`B*.gseg`) are materializations produced by folding — locally signed, marked `derived`; they are journal members for recovery but are **not** original history.
- **Sealing order & atomicity (I9):** write `JNNN.gseg.tmp` → fsync → rename → fsync directory (best-effort on Windows, which has no directory fsync). The active file is not rotated until the sealed segment is durable; **manifest advance always comes after seal durability** (rename-last preserved — a crash between seal and manifest leaves a harmless orphan segment, cleaned or re-adopted by the startup scan; `*.tmp` files are deleted on startup; if an active file overlaps a sealed segment's `commit_seq` range, the sealed segment wins and the active file is truncated to the segment's end).
- All journal files live under the DB root (`OpenOptions.path`); no engine-owned file escapes it (one-bundle backup contract kept).
- `WalMsg::Checkpoint` in its truncate-and-rewrite form is retired. Checkpoint = seal + record `(frontier_commit_seq, segment_id, offset)` in the snapshot manifest + enforce retention (D3). Snapshot files remain materialized state; they are no longer written *into* the journal as pseudo-events. The mobile **foreground hook maps to this checkpoint** (seal + fold) — SPEC--MOBILE-SDK's risk-register row "WAL grows unbounded → compaction on foreground" is superseded by this ADR (spec update owned by WP-1.3).
- **Compression codec:** zstd via `zstd`/`zstd-sys` — the second C dependency after bundled SQLite — as an **unconditional storage-core dependency** (sealing runs under `--no-default-features --features mobile`). WP-1.2 may not merge until `mobile-build.yml` is green for all mobile targets with the dep in tree; the segment header carries a codec byte so the fallback (`lz4_flex`, pure Rust) is not a format break. Encoder level/window on mobile profiles is capped so encoder memory stays single-digit MB.

### D2 — One commit clock (replica-local, frame-header, unsigned)

Every journal frame carries a **`commit_seq`**: a u64 stamped by the single WAL-writer thread from one monotonic counter, stored in the **unsigned frame header** — it cannot live inside signed bytes, because peer frames keep their original signatures (I4). Consequences, decided here:

1. **Replica-local.** `commit_seq` values are never comparable across replicas; `tx_as_of` means "what did *this replica* believe at *its* commit N." A frame ingested from a peer is stamped with a fresh local `commit_seq`. **`tx_from` = the local frame stamp, always.**
2. **`GenesisTransactionEvent.commit_sequence` is demoted to `origin_commit_seq`** (origin metadata). The receiving replica MUST NOT `fetch_max` it into its own counter (retires both reconcile-path fetch_max sites), and the projection's `applied_transactions` uniqueness re-keys on the local frame seq — eliminating the cross-replica UNIQUE-collision abort.
3. **`stable_frontier()` is REDEFINED** — from "last transaction commit_sequence" to "commit_seq of the last durable frame." This is a breaking behavior change (GBP1-noted), advancing on every mutation, with sparse transaction numbers in that domain. `expected_frontier` CAS in `commit_transaction` becomes a frame-level CAS (or is re-based per transaction — WP-1.2 decides mechanism, semantics fixed here). Known affected surfaces to update: `tests/unified_transaction_u3_tests.rs`, `tests/backup_restore_u9_tests.rs`, `tests/studio_contract_tests.rs`, REST `GET /v1/frontier`, napi `stable_frontier`, `CommitResult.commit_sequence`.
4. **Plumbing follows the stamp:** the `WalMsg::Append` ack returns the stamped seq (so `CommitResult` and the projection use it); the counter re-seeds on open from the journal's max frame seq, not from `projection_state`.
5. The Lamport `LogicalClock` remains CRDT reconciliation metadata — not a journal cursor, not a tx-time axis.

Prospective-only: history destroyed by past checkpoints is unrecoverable, and this ADR does not pretend otherwise. The **tx-time epoch begins at the migration boundary** (§4): migrated legacy data is recovery-only, not `tx_as_of`-addressable.

### D3 — Retention profiles: fold, never bare-drop (the answer to 20 GB @ 1M)

Retention is a first-class, per-database setting on `OpenOptions`, budget-based (bytes):

| Profile | Behavior | Default for |
|---|---|---|
| `full` | seal + compress; never fold history away; optional archive hook | self-host, explicit opt-in |
| `budget(N bytes)` | seal + compress; when sealed bytes > N, **fold** the oldest history segments into a new base segment (live state as-of that boundary) and advance the **history horizon** | desktop/self-host default **4 GiB** · mobile default **provisional 64–256 MiB** (WP-1.3 exit criterion — validated against the disk-growth probe *and* app-store/iCloud-backup posture, which the mobile SDK wrappers must decide explicitly: backup-exclude `journal/` or keep the budget small enough to ride in backups) |
| `frontier_only` | fold immediately at every checkpoint — the compressed equivalent of today's PR #28 behavior; forfeits tx-time, and capabilities says so | opt-in (constrained devices) |

Rules:

- **Folding, not dropping**, is what keeps D6 true: at every instant the journal alone (base segment + history segments + active file) is a complete recovery source. The snapshot never becomes the sole copy of any state. (The pinned test `wal_compaction_tests::checkpointed_wal_replays_to_identical_live_state` — delete snapshot, recover from journal alone — survives with folding; it would be falsified by bare-drop.)
- **Disk contract** = sealed budget + active-file seal threshold (part of the contract; mobile default 16–32 MiB) + one in-flight seal temp. Fold-time provenance: folded ranges lose original signatures (base segments are locally signed materializations) — same truncation today's compaction performs, now scoped and labeled.
- **Peer-aware floor:** retention MUST NOT advance the horizon past the oldest acknowledged sync cursor of a registered peer **unless** beyond-horizon re-bootstrap (D4) is in force for that peer. Silent incomplete deltas are forbidden.
- **Archive hook** (`full`/`budget`): best-effort, outside I1–I9 — archive failure never blocks folding (surfaced via status); mobile default: no archive.

### D4 — History horizon contract

`history_horizon` = the oldest `commit_seq` still covered by *history* segments (base segments are below the horizon by definition; the tx-time epoch boundary from §4 is a floor).

1. No queryable history surface (e.g., the `node_versions` projection, WP-2.1) may extend beyond the journal horizon. Projections stay strictly rebuildable.
2. tx-time queries (`tx_as_of`, WP-2.2) older than the horizon fail explicitly (`beyond_horizon`), never silently return current state. The Query IR capabilities endpoint advertises `{history_horizon, tx_epoch_start}`.
3. **Sync:** the Lamport `from_clock` cannot express the horizon (incommensurable units; ingested peer events keep original clocks, so segments do not partition Lamport space). The sync wire therefore gains a **commit_seq/segment cursor alongside `from_clock`**; a responder that detects a cursor older than its horizon answers with a distinguishable `beyond_horizon` response that triggers **explicit snapshot bootstrap over a new transfer channel** (gossip's ~60 KB UDP datagram cannot carry it — this is a new wire surface, designed in WP-1.2, not a behavior tweak). Bootstrap state is materialized and signed by the responder; provenance older than the responder's horizon is truncated — an accepted, explicit tradeoff.

### D5 — Recovery invariant

Startup = load snapshot at `frontier_commit_seq` + **replay journal frames > frontier** (sealed segments newer than the snapshot, then the active tail). The either/or loader is already retired on main (`4072cc9`): today's cursor is byte-positional `(bytes, sha256)` over the single WAL file; **WP-1.2 migrates it to `(frontier_commit_seq, segment_id, offset)`**, keeping the landed prefix-integrity fallback (mismatch ⇒ full replay). On mobile profiles, checkpoint cadence is tied to lifecycle events so the cold-start replay window stays small; WP-1.3's probe measures cold-start replay time at the mobile seal threshold.

### D6 — Authority table (locked)

| Component | Authority |
|---|---|
| Journal = active WAL + history segments + base segments | **Canonical durability authority & complete recovery source at every instant**; history segments additionally = canonical history within horizon |
| Snapshot set (`state.json`, `nodes.bin`, `edges.bin`, `vec_*/meta_*`) | Materialized state at the frontier — a cache of replay, rebuildable, never the sole copy |
| Identity dictionary (`id_to_u32`) | Canonical internal mapping (rebuilt from journal) |
| `projection.sqlite` (props, labels, app tables, `node_versions`) | Rebuildable projection — never authoritative |
| HNSW / adjacency / lexical / temporal indexes | Disposable accelerators |
| DashMaps (`nodes`/`edges`/`out_idx`/`in_idx`) | Memtable/cache |

### Invariants (testable; adopted into the engine contract)

- **I1 Durable commit** — the ack is sent only after `flush()` + `sync_all()` return for the batch containing the frame (syscall-ordering property; media durability beyond the OS is an audit assumption, not a CI-testable invariant).
- **I2 One order** — every frame written post-migration carries one `commit_seq` from one local counter (scoped: pre-migration segment-zero content is recovery-only).
- **I3 Reproducibility (per replica)** — same `(valid_at, tx_as_of)` against the same replica's journal, within horizon ⇒ same logical state. Across replicas only eventual convergence of final state holds (LWW elides/reorders per arrival); the bench probe is defined single-replica.
- **I4 Sealed immutability** — history segments are never rewritten; original event bytes and signatures preserved. Any verification of a sealed frame verifies over the **stored bytes**, never a re-serialization. The peer verifying-key registry becomes journal-durable (or frames carry signer key material) so foreign signatures remain checkable by peers that never met the signer; reconcile-derived batch-inner frames are marked `derived` (signature covers the outer batch only). Base segments are marked `derived` wholesale.
- **I5 Disposable indexes** — deleting every index/projection must be fully recoverable from the journal (+ snapshot as accelerator).
- **I6 Horizon honesty** — no surface answers history questions beyond the horizon or before the tx epoch; both are queryable via capabilities.
- **I7 Atomic manifest** — startup sees the old or the new snapshot manifest, never half; manifest advance strictly follows seal durability.
- **I8 Recovery** — snapshot@frontier + replay(>frontier) = all acked state; journal-only recovery (no snapshot) must also reach current state.
- **I9 Seal atomicity** — tmp → fsync → rename → dir-fsync (best-effort on Windows); active file remains authoritative until the sealed segment is durable; startup deletes `*.tmp`, adopts-or-cleans orphans, and on overlap the sealed segment wins; a crash at any instant leaves exactly one authoritative copy of every frame.

## 3. Options considered

| Option | Verdict | Reason |
|---|---|---|
| **Status quo** (truncate-to-live checkpoint) | rejected | Destroys the moat the strategy docs claim (tx-time probe fails deterministically); violates the WAL-authority contract five docs assert; silently re-signs peer events. |
| **Full event sourcing, never delete** | rejected | Measured ~20 GB JSONL @ 1M nodes; mobile budget 300 MB–2 GB with bounded-growth requirement. |
| **SQLite as co-authority for history** | rejected as authority | Reopens the dual-authority ambiguity substrate ADR §2.2 closed. Kept as a projection within horizon (WP-2.1). |
| **Sealed segments + bare-drop retention** | rejected (panel) | Once segments drop, the snapshot silently becomes sole authority for pre-horizon state — contradicting D6 and falsifying the pinned journal-only-recovery test. |
| **Sealed segments + fold-based retention + explicit horizon** | **CHOSEN** | One authority at every instant; today's truncation becomes the degenerate `frontier_only` profile; disk bounded by budget on every tier; history loss explicit and queryable, never silent. |

## 4. Migration & compatibility

- **One-way.** On first open at the new `SCHEMA_VERSION`, the existing JSONL WAL is sealed as **segment zero** (recovery-only; not tx-addressable — the tx-time epoch starts here). Sealing streams with bounded memory, may defer compression under low storage, and budget enforcement never folds/drops segment zero before a snapshot at the frontier is verified durable.
- **Old-engine-reads-new fails closed** with `SCHEMA_VERSION_UNSUPPORTED` — never partial read, never rewrite (Android sideload-downgrade reality).
- **New-engine-reads-old:** JSONL reader retained ≥ 2 releases (GBP1 discipline, same pattern as the bincode→postcard window). One `SCHEMA_VERSION` bump covers desktop + mobile together.

## 5. Consequences

**Positive:** tx-time implementable (WP-2.x) and the mandatory bench probe passable; peer signatures survive sealing; mobile keeps bounded disk with an explicit contract instead of an implicit truncation; PR #28's writer-thread machinery and the landed tail-replay (4072cc9) are reused, not discarded; the journal is a complete recovery source at every instant — stronger than today's contract, honestly stated.

**Negative / accepted costs:** disk floor rises on history-retaining profiles (bounded by budget); fold work at retention time; a new sync wire surface (commit_seq cursor + `beyond_horizon` + bootstrap channel) with `crdt_sync_tests` and `wal_compaction_tests` rewrites; `stable_frontier` semantics change ripples across tests/SDK surfaces (enumerated in D2); zstd-sys joins as a second C dependency gated on mobile CI; provenance is truncated at folds and at bootstrap (explicit, labeled).

**Out of scope here:** frame byte-layout, segment thresholds, zstd level (WP-1.2 spec + diagram); prev-hash chaining across frames (future hardening, needs its own sync-wire decision); native property/adjacency segment stores, page cache, epoch-HNSW (deferred backlog, evidence-gated per PLAN §8).

## 6. Acceptance for this ADR (C-3 gate)

- [x] 3-lens review panel — verdicts: code-consistency APPROVE-WITH-CHANGES · mobile/disk APPROVE-WITH-CHANGES · CRDT-sync APPROVE-WITH-CHANGES; all BLOCKER/MAJOR findings incorporated in draft-2 (replica-local commit clock, `stable_frontier` redefinition, fold-based retention, I9 seal atomicity, sync cursor/`beyond_horizon` wire surface, migration fail-closed, zstd constraint)
- [x] Lead review
- [x] USER approval (2026-08-17: replica-local tx-time semantics, `stable_frontier` redefinition, zstd-sys C dep gated on mobile CI, provisional budget defaults — all approved) → status `accepted`; WP-1.2/1.3 unlocked

## CHANGELOG

| Version | Date | Summary |
|---|---|---|
| accepted | 2026-08-17 | USER approved all four gate decisions; WP-1.2/1.3 unlocked |
| draft-2 | 2026-08-17 | Panel findings incorporated: fold-based retention replaces bare-drop; I9 added; D2 rewritten (unsigned frame header, replica-local, stable_frontier redefined, fetch_max retired); D4 sync cursor + beyond_horizon bootstrap; migration/downgrade rules; WP-1.1 marked landed (4072cc9) |
| draft-1 | 2026-08-17 | Initial draft from GNSE review + verified code facts |
