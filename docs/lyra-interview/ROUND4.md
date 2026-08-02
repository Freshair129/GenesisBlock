# ROUND4 - LYRA

## L-R4.1

**Verdict:** [measured] The claim "we already have the layer" is falsified if "layer" means a stranger-installable platform product. The repo evidence shows real internal artifacts, but GKS/MSP/GoVibe/Rwang are not yet a coherent external-adopter layer: GoVibe is private and author-path-coupled, MSP has no runtime code in the audited path, GKS is fragmented, and Rwang still carries project/path coupling. [docs/ROUND4-ENGINE-VS-PLATFORM.md:25] [docs/genesis-interview/evidence/r5-platform.md:81]

**Evidence:**

- [measured] GenesisBlockDB is real engine code, but its npm package is not currently a public install path: `package.json` names `@freshair129/gks-genesis-block-native`, and the platform audit recorded `npm view @freshair129/gks-genesis-block-native version -> E404`. [package.json:2] [docs/genesis-interview/evidence/r5-platform.md:21]
- [measured] The engine MCP surface exists, but it is a thin local server with three tools (`query_hql`, `retrieve_tiered_context`, `add_knowledge`) over a locally built addon and a default local DB path, not a standalone GKS platform. [mcp/server.js:12] [mcp/server.js:38] [mcp/server.js:49] [mcp/server.js:63]
- [measured] The local `.brain/gks/storage` artifact exists, but the audit records `genesis-graph.wal` as 0 bytes and `identity.bin` as 32 bytes; that is storage residue/identity material, not an externally usable GKS product. [docs/genesis-interview/evidence/r5-platform.md:16]
- [measured] No top-level `gks/` codebase exists in this repo, while a separate Rwang GKS module exists under `G:\GenesisBlock_Dev\Rwang_remote\gks`; the audit classifies GKS as fragmented with no stranger-install path. [docs/genesis-interview/evidence/r5-platform.md:18] [docs/genesis-interview/evidence/r5-platform.md:53] [docs/genesis-interview/evidence/r5-platform.md:61]
- [measured] MSP is not implemented as a runtime in the audited local path: `G:\msp` exists but is empty, and the audit found MSP as docs/runbooks rather than executable memory-system code. [docs/genesis-interview/evidence/r5-platform.md:19] [docs/genesis-interview/evidence/r5-platform.md:42] [docs/genesis-interview/evidence/r5-platform.md:49]
- [measured] GoVibe has runnable code, but `G:\govibe` is private, has no root README in the audit, and includes hardcoded `G:\govibe` examples/scripts; the audit found no external consumption path. [docs/genesis-interview/evidence/r5-platform.md:26] [docs/genesis-interview/evidence/r5-platform.md:27] [docs/genesis-interview/evidence/r5-platform.md:28] [docs/genesis-interview/evidence/r5-platform.md:31]
- [measured] The older `D:\GoVibe` tree is a private scaffold/tutorial-style repo with no real GenesisDB binding, so it cannot count as an adopter-ready platform layer. [docs/genesis-interview/evidence/r5-platform.md:33] [docs/genesis-interview/evidence/r5-platform.md:35]
- [measured] Rwang has code, scripts, and tests, but current repo evidence still contains author/project coupling: prompts name G-Maiden and a fixed `G:/G-Maiden` root, and GenesisDB bindings default to `G:/GenesisBlock_Dev/GenesisBlock/index.js`. [D:/rwang/RWANG/engine.mjs:408] [D:/rwang/RWANG/engine.mjs:433] [D:/rwang/RWANG/store/genesis-sidecar.mjs:18] [D:/rwang/RWANG/store/knowledge.mjs:29]
- [unknown] "external adopters = 0" is not proven globally. The local audit found no install path or consumption evidence, but no package-download telemetry, GitHub clone data, customer list, Discord/issue audit, or external deployment inventory was provided. [docs/genesis-interview/evidence/r5-platform.md:81]

**Falsifier / what would settle it:** [asserted] A clean-machine test by a non-author would falsify this negative assessment if the user can install GKS/MSP/GoVibe/Rwang from public docs, configure paths without local hardcoding, run a real workflow, and persist/replay state through GenesisBlockDB without author intervention. [docs/ROUND4-ENGINE-VS-PLATFORM.md:69]

**Open questions:** See `OPEN-QUESTIONS`.

## L-R4.2

**Verdict:** [unknown] "Come for the engine, stay for the platform" is a strategic hypothesis, not yet established repo fact. The Round 4 brief cites an engine-to-platform reconciliation pattern, but the local evidence only proves that the engine is closer to product reality than the platform layers; it does not prove that GenesisBlockDB can climb into a platform. [docs/ROUND4-ENGINE-VS-PLATFORM.md:46] [docs/genesis-interview/evidence/r5-platform.md:81]

**Evidence:**

- [measured] The engine has concrete build/test/MCP artifacts, while the audited platform layers are less productized: Rwang is coupled to local/project paths, GoVibe is private/internal, MSP lacks runtime code, and GKS has no unified installable codebase. [AGENTS.md:7] [package.json:36] [docs/genesis-interview/evidence/r5-platform.md:81]
- [asserted] The Round 4 brief names SQLite-to-Turso, DuckDB-to-MotherDuck, and Vault as precedent narratives; LYRA cannot treat those analogies as proof for this repo without a cohort/channel analysis. [docs/ROUND4-ENGINE-VS-PLATFORM.md:46] [docs/ROUND4-ENGINE-VS-PLATFORM.md:49]
- [measured] The "engine wedge" is still not fully stranger-installable because the package registry path recorded E404; therefore even the strongest layer needs packaging proof before it can validate platform pull. [docs/genesis-interview/evidence/r5-platform.md:21]

**What evidence distinguishes repeatable pattern from survivorship story:** [asserted] A repeatable pattern would show external users first adopting the engine for a painful job, then organically asking for orchestration, memory, governance, or UI because the engine creates adjacent workflow demand. A survivorship story would show no engine-led inbound demand, no integrations beyond author repos, and platform claims sustained mostly by internal methodology docs. [docs/ROUND4-ENGINE-VS-PLATFORM.md:72]

**Falsifier / test:** [asserted] Timebox an engine-wedge release: public package or binary, minimal MCP/SDK docs, one clean-machine tutorial, and a target cohort such as LangGraph/CrewAI/AutoGen users who need local-first graph+vector+temporal state. Track whether external users request platform features after engine use. If they only ask for Redis/SQLite/Kuzu/LanceDB-compatible adapters or simpler state, the platform-climb thesis weakens. [docs/ROUND4-ENGINE-VS-PLATFORM.md:48] [docs/ROUND4-ENGINE-VS-PLATFORM.md:72]

**Open questions:** See `OPEN-QUESTIONS`.

## L-R4.3

**Verdict:** [measured] The opinionated methodology is currently an adoption tax before it is proven to be a moat. The Round 4 brief itself identifies the 12-stage/H0-H6/C-0..3 method as both the biggest differentiator and the biggest adoption cost, while the local audit shows the methodology is not yet packaged into a non-author runtime. [docs/ROUND4-ENGINE-VS-PLATFORM.md:25] [docs/ROUND4-ENGINE-VS-PLATFORM.md:41] [docs/genesis-interview/evidence/r5-platform.md:81]

**Evidence:**

- [asserted] The brief positions GKS plus 12-stage top-down, 7-phase bottom-up, H0-H6, and C-0..3 as a knowledge/governance layer competing near Graphiti. That is a positioning claim, not adoption evidence. [docs/ROUND4-ENGINE-VS-PLATFORM.md:25]
- [measured] GoVibe ADR-014 explicitly constrains MSP/GKS exposure: Mission Control may display MSP/GKS as provenance/config and must not fake live execution; broader MSP capabilities are out of v1. [G:/govibe/docs/adr/ADR-014-MSP-GKS-Traceability-Gate.md:116] [G:/govibe/docs/adr/ADR-014-MSP-GKS-Traceability-Gate.md:118]
- [measured] Rwang GKS notes include planned/greenfield features such as memory OS, node DB canvas, and GenesisDB sidecar; those are not evidence that an external user can adopt the full methodology today. [D:/rwang/RWANG/gks/atoms/feature--memoryos.md:14] [D:/rwang/RWANG/gks/atoms/feature--node-db-canvas.md:14] [D:/rwang/RWANG/gks/atoms/tech_stack--genesisdb-sidecar.md:14]
- [unknown] There is no supplied evidence that non-authors understand, prefer, or pay for the 12-stage/H0-H6 methodology over simpler memory/orchestration tools. [docs/ROUND4-ENGINE-VS-PLATFORM.md:75]

**What would settle it:** [asserted] Run a usability-and-demand test with non-author teams: one cohort gets a thin engine/MCP path; another gets the full methodology. Measure setup completion, first successful workflow, retained weekly use, support burden, willingness to pay, and whether users ask for the methodology by name. If the full method improves auditability/coordination without slowing adoption, it starts looking like moat; if it increases setup/support load or users bypass it, it is tax. [docs/ROUND4-ENGINE-VS-PLATFORM.md:75]

**Open questions:** See `OPEN-QUESTIONS`.

## OPEN-QUESTIONS

- [unknown] How many external users have installed GenesisBlockDB, GoVibe, Rwang, GKS, or MSP from outside the author's machine? Evidence needed: package downloads, GitHub clones/issues from non-author accounts, customer/user list, or deployment inventory. [docs/genesis-interview/evidence/r5-platform.md:81]
- [unknown] Is there a public, maintained install path for any one of GKS/MSP/GoVibe/Rwang that succeeds on a clean machine without `G:\...` or `D:\...` assumptions? Evidence needed: README, package release, CI clean-install job, and non-author replay log. [docs/genesis-interview/evidence/r5-platform.md:28]
- [unknown] Which exact persona is willing to adopt the opinionated method: orchestration-framework builders, local-first agent app builders, regulated audit/governance buyers, or internal users only? Evidence needed: interviews or conversion data. [docs/ROUND4-ENGINE-VS-PLATFORM.md:75]
- [unknown] Do users want GenesisBlockDB as a standalone engine, as an adapter inside existing orchestrators, or as a full Rwang/GoVibe-style platform? Evidence needed: cohort tests and support/request logs after an engine-wedge release. [docs/ROUND4-ENGINE-VS-PLATFORM.md:48]

## ROUND4 recommendation

[asserted] Engine-wedge-first, then extracted MCP/SDK adapter, then platform hardening only after a non-author clean-machine workflow proves adoption; do not claim "we already have the layer" until GKS/MSP/GoVibe/Rwang are installable without author paths and have external-use evidence. [docs/genesis-interview/evidence/r5-platform.md:91] [docs/ROUND4-ENGINE-VS-PLATFORM.md:46]
