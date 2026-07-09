---
id: ADR--ENGINE-WEDGE-FIRST
type: adr
status: accepted
decided: 2026-07-07
deciders: Boss (CEO) + cross-model panel (Genesis/Fable-5, LYRA/GPT-5.5, KAIROS/Gemini-Pro-3.1) + local-tier models (Mellum2-12B worker, gemma-4-12b orchestrator, gemma-4-e2b on-device)
supersedes_framing_in: [docs/genesis-interview/PROPOSAL.md, docs/ROUND4-ENGINE-VS-PLATFORM.md]
evidence:
  - docs/genesis-interview/{PROPOSAL,ROUND2,ROUND3,ROUND4,COMPETITIVE}.md + evidence/r5-*.md (on-disk audit)
  - docs/lyra-interview/{ASSESSMENT,ROUND2,ROUND3,COMPETITIVE}.md
  - docs/kairos-interview/{ADOPTION,ROUND2,ROUND3,ROUND4,COMPETITIVE}.md
  - docs/ROUND3-LOCAL-MODEL-FINDINGS.md, docs/BENCH-SPEC--HQL-MOAT-AND-EXPRESSIVENESS.md
---

# ADR — Engine-Wedge-First (Trojan-Horse Hybrid), not Platform-First

## Context

A multi-round, cross-model, cross-consumer-tier advisory program (see `evidence`) asked two
questions: (1) what is the moat, and (2) do we sell the **engine** (substrate under other
frameworks) or the **platform** (GKS/MSP/GoVibe/Rwang stack competing with Graphiti/Mem0/
LangGraph). Prior rounds settled the moat as *the only embedded engine with enforced row-level
bitemporal + vector ANN + graph + signed governance*, and identified two real buyer tiers
(local multi-agent orchestrator; on-device mobile). Round 4 resolved the engine-vs-platform fork.

**Decisive finding (Genesis on-disk audit, 2026-07-07):** the premise "we already own a full
vertically-integrated stack" is **falsified on disk**:
- **MSP** — code NOT FOUND (`G:\msp` empty); SDD/ADR docs only. Greenfield.
- **GKS** — no unified codebase (143-line MCP shim + 0-byte WAL + Rwang's unrelated `gks/` backlog).
- **Rwang** — dashboard/dry-run clone-runs, but real agent execution is author-machine-only
  (hardcoded `G:/G-Maiden`, Thai-only prompts, Windows-only, npm E404).
- **GoVibe** — two divergent codebases; `D:\GoVibe` is a stale scaffold.
- **engine** — closest to product, but a stranger CANNOT install it today (npm unpublished/E404,
  no v0.2.0 tag, release matrix never produced an artifact, no Docker/binary). Gap = packaging (M).

Even the author's own consumer (NotiKeeper) binds the engine via a hardcoded `require("G:/…")`;
nowhere in the estate is it consumed through a path-independent SDK contract. **The "layer" is
design docs + bespoke author-machine tooling — not a product a stranger can install.** External
adopters = 0 across the entire estate. KAIROS independently: platform-first has no demonstrable
first-10-users; the 12-stage/H0-6/C-0..3 methodology is an adoption tax with no shown demand.

Critically, the fork is **not** either/or: the wedge and the platform share the same ~6k-line
Rust core; three engine work items (HQL v-next, `events_since` REST exposure, bitemporal audit
fixes) are demanded by both; and an embryonic adapter already exists (~600 lines in Rwang
`store/genesis-sidecar.mjs` + `knowledge.mjs`), trapped behind a `G:/` path.

## Decision

**Adopt engine-wedge-first (a Trojan-horse hybrid), not platform-first.** Ship the engine as the
only on-ramp that exists in code; keep the platform (GKS/MSP/GoVibe/Rwang) as the destination
narrative and as internal design programs until external demand pulls them forward.

**Sequence:**
1. **Publish the engine** — verify NPM_TOKEN, tag `v0.2.0`, fire the release matrix, fix
   QUICKSTART/dist-tag/SECURITY.md, publish prebuild `optionalDependencies`. (days–weeks)
2. **Extract Rwang's `genesis-sidecar` + `knowledge.mjs` into the first published, path-independent
   consumer package**; make `mcp/server.js` npx-able (fix `vectorDim:1536` → 1024).
3. **Ship the REST server as a binary + Docker image + SDK auth** (the named wedge blocker).
4. **Graphiti driver** on top — gated on first checking their `GraphDriver` contract fits the HQL
   subset (the issue #1240 window; the highest-leverage channel move once steps 1–3 exist).

**Brand = platform vision ("memory OS for governed agents"); on-ramp = engine + channel.** The
Graphiti/LangGraph driver is a distribution tactic, never the brand identity.

**Gate before funding either fork's expensive half:** publishing is nearly free — after step 1,
**measure a first-10-external-installs signal within ~1 month.** No external install signal ⇒ do
not spend a quarter building MSP/GKS/GoVibe for strangers.

**Deferred (internal design programs, not shipped):** MSP (greenfield), unified GKS, GoVibe
Mission Control. The 12-stage/H0-6/C-0..3 methodology stays in the unshipped layers — the thin
install-and-go slice carries **zero** methodology, by design.

## Consequences

- **Positive:** ships in weeks on the only installable layer; validates demand cheaply before big
  spend; wastes no platform work (shared core); the wedge adapter mostly exists; keeps the
  differentiated methodology out of the adoption path until asked for.
- **Negative / risk:** commoditization if we let the driver own the narrative (mitigated: brand =
  platform vision); the platform value capture is deferred, not abandoned.
- **What we explicitly refuse:** in-engine MVCC (storage tar-pit; snapshot isolation conceded to
  SQLite WAL for now — expose shipped `events_since`/Merkle/signed-events instead); new data-model
  engines (ArangoDB trap); enterprise self-host HA/multi-tenancy (different product).

## Open / to verify
- **LYRA ROUND4 pending** — falsification lens on "we already have the layer"; fold in when written.
- **Private MSP/GKS runtime repo?** The audit spot-checked D: only; if a real runtime exists
  elsewhere it could reweight the greenfield gaps.
- **Graphiti `GraphDriver` contract** must be checked before committing step 4 (adapter vs translator).
- Downstream benches still gate any HQL/G3 claim (see `BENCH-SPEC--HQL-MOAT-AND-EXPRESSIVENESS.md`).

## Related
- Moat + bench gates: [[BENCH-SPEC--HQL-MOAT-AND-EXPRESSIVENESS]]
- Substrate direction: [[ADR--GENESISDB-EMBEDDED-SQLITE-SUBSTRATE]]
- Ranking-in-app precedent: [[ADR--GENESISDB-KIMPACT-AS-SIGNAL]]
