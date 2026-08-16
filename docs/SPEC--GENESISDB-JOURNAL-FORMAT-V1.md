---
status: draft
---

# SPEC — Journal Format v1 (framed WAL, sealed segments, commit-seq cursor)

**Status:** Draft (2026-08-17) · **Work packet:** WP-1.2 of [PLAN--GNSE-REMEDIATION-MULTIAGENT](PLAN--GNSE-REMEDIATION-MULTIAGENT.md) · **Semantics authority:** [ADR--GENESISDB-JOURNAL-HISTORY](adr/ADR--GENESISDB-JOURNAL-HISTORY.md) (accepted 2026-08-17 — this spec elaborates D1/D2/D4/D5 + I1/I2/I4/I7/I9; it decides bytes and mechanisms, never semantics)
**Complexity:** C-3 (Text → Doc → **this Diagram/Spec** → Code) · **Scope (H-lock):** `src/lib.rs` (WAL writer thread, `persist`/`persist_signed`, `events_since`, recovery, `save_state`), `docs/`, `tests/` · **Out of scope:** fold-based retention + bootstrap transfer channel (WP-1.3), `node_versions` (WP-2.1)

## 1. Frame format (active file and history segments)

The payload of every frame is the **unmodified serde_json `SignedEvent` bytes** exactly as today — I4 requires original bytes (verification over stored bytes; `events_since` serves them verbatim). Framing wraps; it never rewrites.

```
Frame (little-endian):
┌───────────────┬───────────────┬───────────────┬──────────────────────────┐
│ u32 len       │ u64 commit_seq│ u32 crc32c    │ payload[len]             │
│ (payload only)│ (unsigned hdr,│ (over seq +   │ = SignedEvent JSON bytes │
│               │  writer stamp)│  payload)     │   (unchanged)            │
└───────────────┴───────────────┴───────────────┴──────────────────────────┘
```

- `commit_seq` lives in the frame header, **outside signed bytes** (ADR D2): peer frames keep original signatures; `tx_from` = this stamp, always.
- CRC covers `commit_seq || payload` so a torn/bit-rotted tail is detected frame-precise; recovery truncates at the first bad frame of the **active** file (a bad frame inside a sealed segment is corruption → segment-level error, journal-only recovery falls back to the previous segment boundary + snapshot).
- The WAL-writer thread stamps `commit_seq` from one monotonic counter and the `WalMsg::Append` ack changes `Sender<bool>` → `Sender<Result<u64>>` (the stamp), so `CommitResult` and the projection consume the local frame seq (ADR D2.4).

## 2. File formats

```
wal/active.gwal                        journal/J000042.gseg
┌──────────────────────┐               ┌──────────────────────────────┐
│ FileHeader           │               │ SegHeader                    │
│  magic  "GWA1"       │               │  magic "GSG1"                │
│  u16 format_version=1│               │  u16 format_version = 1      │
│  u64 first_seq       │    seal       │  u8  kind (1=history,        │
├──────────────────────┤   ──────▶     │      2=base, 3=legacy_jsonl) │
│ Frame …              │               │  u8  codec (0=none, 1=zstd,  │
│ Frame …              │               │      2=lz4)                  │
│ (uncompressed,       │               │  u64 min_seq / u64 max_seq   │
│  append + group-     │               │  u32 frame_count             │
│  commit fsync)       │               ├──────────────────────────────┤
└──────────────────────┘               │ body: codec-compressed       │
                                       │   concatenation of frames    │
                                       ├──────────────────────────────┤
                                       │ Footer: sha256(uncompressed  │
                                       │  body) · u64 body_len ·      │
                                       │  u32 crc32c(header+footer)   │
                                       └──────────────────────────────┘
```

- Active file is uncompressed (append + fsync path unchanged: 1024/5 ms group commit). Compression happens once, at seal, over the whole body — zstd level capped on mobile profiles (encoder memory single-digit MB, ADR D1). Codec byte makes `lz4_flex` a non-breaking fallback if `zstd-sys` fails mobile CI.
- **Seal procedure (I9, executes on the WAL-writer thread via a new `WalMsg::Seal`):** write `JNNN.gseg.tmp` → fsync → rename → best-effort dir fsync → only then rotate `active.gwal` (close, create fresh with next `first_seq`), reusing PR #28's close/reopen dance. Manifest advance strictly after seal durability (I7). Startup deletes `*.tmp`, adopts-or-cleans orphan segments, truncates active-file overlap (sealed wins).
- Seal triggers: active file ≥ seal threshold (default 64 MiB desktop / 16 MiB mobile) or checkpoint.

## 3. Manifest (`state.json` additions)

```json
"journal": {
  "frontier": { "commit_seq": 18934102, "segment_id": 42, "offset": 12888 },
  "segments": [ { "id": 0, "kind": "legacy_jsonl", "min_seq": 0, "max_seq": 0, "bytes": 104857600 },
                { "id": 41, "kind": "history", "min_seq": 1, "max_seq": 90210, "bytes": 8388608 } ],
  "tx_epoch_start": 1,
  "format_version": 1
}
```

Replaces the landed byte-positional `wal_frontier {bytes, sha256}` (PR #100) as recovery cursor. **The prefix-integrity fallback is kept:** each manifest records the sha256 footer of the frontier segment; mismatch ⇒ distrust the frontier ⇒ full journal replay (ADR D5). Atomicity: existing `state.json` rename-last contract, unchanged.

## 4. Clock plumbing (ADR D2 mechanism decisions)

1. Counter re-seeds on open from max frame seq across manifest + active tail (no longer from `projection_state`).
2. `GenesisTransactionEvent.commit_sequence` → renamed field `origin_commit_seq` (serde alias `commit_sequence` for wire/legacy compat); both reconcile `fetch_max` sites retired; `applied_transactions` uniqueness re-keys to the **local frame seq** column (migration: `ALTER TABLE` add column, drop UNIQUE on origin value).
3. **`stable_frontier()` = last durable frame seq** (redefined per ADR). New accessor `txn_frontier()` = frame seq of the last `Event::Transaction`. `expected_frontier` CAS in `commit_transaction` is **re-based to `txn_frontier`** (transaction-lineage CAS survives interleaved ordinary writes; a frame-level CAS would spuriously fail on any concurrent add_node). REST `GET /v1/frontier` returns `{ "frame": u64, "txn": u64 }` (additive JSON; SDKs read `txn` where they read the old scalar — GBP1 note in CHANGELOG). Napi mirrors both.
4. Surfaces to update (enumerated by panel): `tests/unified_transaction_u3_tests.rs`, `tests/backup_restore_u9_tests.rs`, `tests/studio_contract_tests.rs`, REST `/v1/frontier`, napi `stable_frontier`, `CommitResult.commit_sequence` (now the frame stamp).

## 5. Sync wire (ADR D4 — cursor only; bootstrap channel is WP-1.3)

- `GossipMessage::PullRequest` gains `#[serde(default)] from_commit_seq: Option<u64>` beside `from_clock` (old peers deserialize fine; new responders serve whichever cursor is present, preferring `from_commit_seq`).
- `events_since` reads **across sealed history segments + active file** in frame order, filtering by `commit_seq` when the new cursor is given (Lamport filtering retained for legacy cursor). Relational-only transactions become servable (they now have a frame seq — closes the `event_time = None` gap).
- New response variant `GossipMessage::BeyondHorizon { horizon: u64 }` — emitted when `from_commit_seq < history_horizon`. Until WP-1.3 lands fold/retention, the horizon never advances past segment continuity, so this variant is wire-defined but not yet triggerable; the requester's handling (abandon delta-pull, mark peer needs-bootstrap) ships now, the bootstrap transfer itself in WP-1.3.

## 6. Migration (ADR §4)

On first open at bumped `SCHEMA_VERSION`: (1) verify/produce a durable snapshot at the current frontier; (2) stream-seal the existing `genesis-graph.wal` bytes as segment 0 `kind=legacy_jsonl` (bounded memory; compression deferrable under low storage; **never folded/dropped before that snapshot is verified** ); (3) create fresh `wal/active.gwal` at `first_seq = 1`; (4) write manifest with `tx_epoch_start = 1`. Segment 0 is recovery-only (replayed via the retained JSONL line parser, ≥ 2 releases), not `tx_as_of`-addressable. Lower-version engine opening a migrated DB fails closed: `SCHEMA_VERSION_UNSUPPORTED`.

## 7. Verify gate (WP-1.2 `verify_command` set)

| Check | Command |
|---|---|
| Frame/segment round-trip incl. torn-tail truncation, orphan adoption, overlap rule (I9 crash matrix) | `cargo test --test journal_format_tests` |
| Legacy JSONL DB opens + migrates + reopens identically | `cargo test --test journal_migration_tests` |
| Tail replay still closes acked-write loss on the new cursor | `cargo test --test wal_tail_replay_tests` |
| Checkpoint/compaction behavior on new format | `cargo test --test wal_compaction_tests` |
| Sync serves frames across segments; legacy Lamport cursor still works; BeyondHorizon handling | `cargo test --test crdt_sync_tests` |
| Frontier surfaces (frame/txn split) | `cargo test --test unified_transaction_u3_tests --test backup_restore_u9_tests --test studio_contract_tests --test rest_api_tests --test napi_rest_parity_tests` |
| Everything else unbroken | `cargo test` + `npm test` |
| Mobile targets build with zstd in tree (**merge-blocking**, ADR D1) | `mobile-build.yml` green on all targets |
| Compression ratio measured vs JSONL baseline (ADR D3 estimate → number) | `cargo run --release --features bins --bin industrial-audit` + journal size report in PR |

## CHANGELOG

| Version | Date | Summary |
|---|---|---|
| draft-1 | 2026-08-17 | Initial format spec elaborating accepted ADR--GENESISDB-JOURNAL-HISTORY |
