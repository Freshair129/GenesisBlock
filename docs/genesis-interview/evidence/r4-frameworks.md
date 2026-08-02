# Agent-Memory Layer Landscape — what an orchestrator builder reaches for instead of a new memory engine
Research date: 2026-07-06. Skeptical framing: "STORE underneath" is the structural question — nearly every player here is a **framework/service over someone else's store**, not an engine. That is both the threat (they absorb the buyer's attention) and the opening (they all need a store; almost none ships an embedded one with time-travel).

---

## 1. Mem0

**What it is.** The most-adopted standalone memory *framework*: LLM-based fact extraction + add/search/update API over pluggable stores. ~48K GitHub stars, $24M Series A (Oct 2025), Apache-2.0 OSS + hosted platform ([RockB 2026 comparison](https://baeseokjae.github.io/posts/best-ai-agent-memory-frameworks-2026/), accessed 2026-07-06; [mem0ai/mem0](https://github.com/mem0ai/mem0)).

**Store underneath.** Framework-over-store, explicitly. OSS supports 20+ vector backends (Qdrant, Chroma, Milvus, Pinecone, pgvector...); hosted Mem0 Cloud runs Qdrant (vectors) + Neo4j (graph) + Redis (KV) ([Mem0 self-host guide](https://mem0.ai/blog/self-host-mem0-docker), accessed 2026-07-06). Notable structural shift: the **OSS v3 migration removes external graph-store support (Neo4j/Memgraph) and replaces it with built-in "entity linking"** running natively with no external dependency ([docs.mem0.ai/migration/oss-v2-to-v3](https://docs.mem0.ai/migration/oss-v2-to-v3), accessed 2026-07-06) — i.e., Mem0 itself is internalizing a lightweight embedded graph rather than depending on a graph DB. That is a direct move *toward* the embedded-engine space.

**Local/embedded/offline.** Yes, but scrappy: fully offline with Ollama LLM/embedder + local Qdrant/Chroma; default OSS mode uses an in-process vector store persisted to a **SQLite** file plus a SQLite history/audit DB — with known path-handling bugs (historyDbPath ignored, cwd-relative `vector_store.db`) ([issue #4290](https://github.com/mem0ai/mem0/issues/4290); [Local AI Master guide](https://localaimaster.com/blog/local-ai-agent-memory-mem0), accessed 2026-07-06). Not an engineered embedded store; a convenience default.

**Multi-agent shared state.** user_id/agent_id/run_id scoping, native multi-user isolation (a selling point vs CrewAI's built-in memory) ([Mem0 CrewAI blog](https://mem0.ai/blog/crewai-memory-production-setup-with-mem0), accessed 2026-07-06). No shared *state graph* semantics — it's memories, not orchestrator state.

**Temporal/versioning.** Weak. A SQLite history table (audit trail of memory ops) exists in OSS; no point-in-time queries, no bitemporal model, no run replay. UNVERIFIED beyond the history-DB existence.

**OpenMemory MCP.** Mem0's local-first MCP memory server: SSE transport, **Postgres (metadata) + Qdrant (vectors)** locally, audit logs on every read/write ([tensakulabs/mem0-mcp](https://github.com/tensakulabs/mem0-mcp); [ChatForest review](https://chatforest.com/reviews/mem0-mcp-server/), accessed 2026-07-06). Note a separate, unaffiliated CaviraOSS/OpenMemory project exists — naming collision ([CaviraOSS/OpenMemory](https://github.com/CaviraOSS/OpenMemory)).

**Verdict.** **Both, tilting competitor.** As a framework it could sit on any store (channel: a Mem0 `vector_store`/graph provider for GenesisBlockDB is plausible). But v3's internalized entity-graph shows they'd rather own the store layer than integrate. No tier-B time-travel.

---

## 2. Letta (MemGPT)

**What it is.** Stateful-agent *server* (agents as persistent DB-backed objects with self-editing core memory + archival memory). Apache-2.0, ~23K stars, $10M seed, Letta Cloud hosted ([letta-ai/letta](https://github.com/letta-ai/letta); [BigDATAwire funding](https://www.hpcwire.com/bigdatawire/this-just-in/letta-emerges-from-stealth-with-10m-to-build-ai-agents-with-advanced-memory/), accessed 2026-07-06).

**Store underneath.** Framework-over-store: pip default **SQLite (+Chroma)**; production = **Postgres + pgvector** (Docker default). Crucially, **DB migrations are unsupported on SQLite** — the local/embedded path is second-class and upgrade-fragile ([letta on PyPI](https://pypi.org/project/letta/0.6.0/); [Supabase setup guide](https://medium.com/@sdptd20/how-to-set-up-letta-memgpt-with-supabase-7ae09928e401), accessed 2026-07-06).

**Local mode.** Runs as a local server (localhost:8283) fully self-hosted; usable offline with local LLMs. But it is a *server process*, not an embeddable library.

**Multi-agent shared state.** Yes and improving: Conversations API (Jan 2026) = shared memory across parallel user experiences; shared memory blocks between agents ([Letta blog](https://www.letta.com/blog/), accessed 2026-07-06).

**Temporal/versioning — the standout.** **"Context Repositories: Git-based Memory" (Feb 2026)** — agent memory rebuilt on git-based versioning ([Letta blog](https://www.letta.com/blog/), accessed 2026-07-06; implementation depth UNVERIFIED). This is the closest anyone in this list comes to tier-B versioned state, and it validates the demand — but git-over-files is not point-in-time *query* over a shared graph.

**Verdict.** **Competitor for tier-B mindshare** (it wants to *be* the agent state server), **weak channel** (Postgres-coupled ORM; swapping the store is not a supported extension point). Momentum strong (Letta Code #1 open-source agent on Terminal-Bench per their blog — vendor claim).

---

## 3. Zep / Graphiti

**What it is.** Graphiti = MIT-licensed temporal knowledge-graph library (episodes → LLM-extracted entities/edges), 20K+ stars; powers Zep Cloud ([getzep/graphiti](https://github.com/getzep/graphiti); [Zep Graphiti page](https://www.getzep.com/platform/graphiti/), accessed 2026-07-06).

**OSS/cloud split.** Zep **deprecated Community Edition April 2025**; self-hosting now means raw Graphiti + your own graph DB. Zep Cloud is credit-metered (free tier ~1K credits/mo; Flex from $125/mo per third-party comparisons), SOC2/HIPAA ([Zep OSS-strategy post](https://blog.getzep.com/announcing-a-new-direction-for-zeps-open-source-strategy/); [vectorize.io Zep alternatives](https://vectorize.io/articles/zep-alternatives), accessed 2026-07-06).

**Store underneath.** Framework-over-store, hard requirement: Neo4j, **FalkorDB** (Redis-based; FalkorDB contributed the driver and markets it for multi-agent workloads — [Zep blog](https://blog.getzep.com/graphiti-knowledge-graphs-falkordb-support/), [FalkorDB blog](https://www.falkordb.com/blog/graphiti-falkordb-multi-agent-performance/)), **Kuzu (embedded)** — note Kuzu the company is dead (2025 Apple acqui-hire, per prior competitive research), so the "embedded" backend rides an unmaintained engine/its LadybugDB successor. **Issue #1240** proposes **FalkorDB Lite** (embedded subprocess, file-based, zero-config, Python 3.12+) precisely because "requiring an external graph DB server is a barrier for local-first agents, CLI tools, personal assistants" ([graphiti#1240](https://github.com/getzep/graphiti/issues/1240), accessed 2026-07-06). **This issue is the single clearest market signal that Graphiti users want exactly what an embedded engine ships.**

**Temporal.** Genuine **bitemporal edges** (valid_at/invalid_at + ingest time), point-in-time queries, edge invalidation on contradiction — confirms your memory note: Graphiti already ships bitemporal; it is NOT your moat ([Zep docs overview](https://help.getzep.com/graphiti/getting-started/overview), accessed 2026-07-06). But: bitemporality applies to *extracted facts*, not orchestrator run state; no run replay/audit-of-failed-runs primitive.

**Verdict.** **Both — and the best CHANNEL candidate in the list.** Graphiti's driver architecture (thin driver subclasses; FalkorDB Lite would be "a thin subclass of FalkorDriver taking a file path") means a Graphiti driver over an embedded engine is a well-defined integration surface. Competitor at the Zep Cloud level only.

---

## 4. cognee

**What it is.** OSS "memory control plane": ECL pipelines (extract-cognify-load) building a knowledge graph + vectors + relational metadata. Apache-2.0, ~14–17.6K stars, **$7.5M seed**, claims 1M+ pipelines/month, users incl. Bayer ([topoteretes/cognee](https://github.com/topoteretes/cognee); [WeavAI review](https://weavai.app/blog/en/2026/05/09/cognee-2026-review-graphrag-ontology-ai-memory-layer/), accessed 2026-07-06; funding figure from secondary source — UNVERIFIED against primary).

**Store underneath.** Framework-over-store with **fully embedded defaults: Kuzu (graph) + LanceDB (vector) + SQLite (relational)** — zero infrastructure, file-based; swappable to Neo4j/FalkorDB/Neptune/Memgraph, Qdrant/pgvector/Redis/Chroma, Postgres ([cognee setup docs](https://docs.cognee.ai/setup-configuration/overview); [cognee blog](https://www.cognee.ai/blog/guides/open-source-memory-frameworks-llm-agents), accessed 2026-07-06). Fully local with Ollama ([dev.to walkthrough](https://dev.to/chinmay_bhosale_9ceed796b/cognee-with-ollama-3pp8)).

**Structural note.** cognee's default stack = *three* embedded stores glued together — and its graph default (Kuzu) is orphaned by the Apple acqui-hire. A single embedded engine doing graph+vector+relational is a direct substitution argument, and cognee's pluggable adapter layer is a channel surface.

**Multi-agent / temporal.** Dataset/user permissioning; no bitemporal model, no time-travel, no run audit (temporal awareness limited to extracted event metadata — UNVERIFIED depth).

**Verdict.** **Both.** Competitor for "local-first knowledge memory" positioning; channel via its graph/vector adapter interfaces (it already carries 8+ backend adapters).

---

## 5. LangGraph persistence (checkpointers + BaseStore)

**What it is.** Not a memory product — orchestrator-native persistence built into the dominant agent framework (LangChain reports 50M+ monthly downloads across the ecosystem; MIT license; LangChain raised $125M at $1.25B, Oct 2025 — funding from prior knowledge, UNVERIFIED here).

**What ships out of the box (this is the tier-B benchmark):**
- **Checkpointers**: every super-step of a graph run is snapshotted per-thread with monotonically increasing checkpoint IDs → **threads, fault-tolerant resume, human-in-the-loop, and true TIME-TRAVEL: fork/replay from any historical checkpoint** ([LangGraph memory docs](https://docs.langchain.com/oss/python/langgraph/add-memory); [checkpoint reference](https://reference.langchain.com/python/langgraph/checkpoints), accessed 2026-07-06). This is exactly the "versioned state + audit of failed runs" tier-B feature — already free for anyone on LangGraph.
- **Backends**: in-memory, `langgraph-checkpoint-sqlite` (local/embedded), `langgraph-checkpoint-postgres` (production), `langgraph-checkpoint-redis` (Redis-authored) ([LangGraph v0.2 post](https://blog.langchain.com/langgraph-v0-2/); [Redis blog](https://redis.io/blog/langgraph-redis-build-smarter-ai-agents-with-memory-persistence/)).
- **BaseStore**: cross-thread, namespaced (e.g. user-scoped) long-term KV+vector memory alongside checkpoints ([docs](https://docs.langchain.com/oss/python/langgraph/add-memory)).

**Caveats a skeptic notes.** Checkpoints are opaque serialized blobs — no queryable shared *graph* across agents, no semantic fusion, no bitemporal fact model; cross-agent memory is roll-your-own on BaseStore. Security surface is real: a 2026 Check Point advisory showed SQLi→RCE via the checkpointer ([research.checkpoint.com](https://research.checkpoint.com/2026/from-sqli-to-rce-exploiting-langgraphs-checkpointer/), accessed 2026-07-06).

**Verdict.** **The strongest incumbent competitor for tier-B** (time-travel + threads ship free), and simultaneously the **cleanest channel**: `BaseCheckpointSaver`/`BaseStore` are small interfaces — a GenesisBlockDB checkpointer/store package would let LangGraph users get queryable, bitemporal, fused state without leaving their framework.

---

## 6. CrewAI + AutoGen/AG2

**CrewAI.** Built-in memory = short-term (ChromaDB RAG), long-term (**SQLite**), entity, contextual; newer default **LanceDB** at `./.crewai/memory` ([CrewAI changelog](https://docs.crewai.com/en/changelog); [discussion #1125](https://github.com/crewAIInc/crewai/discussions/1125), accessed 2026-07-06). Known production failures: no per-user isolation, machine-bound paths, Chroma locking under concurrency — which is why Mem0/Zep sell "fix CrewAI memory" ([Mem0 blog](https://mem0.ai/blog/crewai-memory-production-setup-with-mem0)). No time-travel, no versioning. **Verdict: channel-shaped** — CrewAI demonstrably outsources memory to external providers; an embedded engine could be one. Weak competitor.

**AutoGen/AG2.** microsoft/autogen is in **maintenance mode** (Microsoft Agent Framework is the successor; migration guide live at [learn.microsoft.com](https://learn.microsoft.com/en-us/agent-framework/migration-guide/from-autogen/)); the community **AG2** fork is active (v0.12.2 May 2026, async rewrite, path to 1.0) ([AG2 review](https://chatforest.com/reviews/ag2-autogen-multi-agent-framework/); [autogen discussion #7066](https://github.com/microsoft/autogen/discussions/7066), accessed 2026-07-06). Memory in both is a thin `Memory` protocol with ListMemory/ChromaDB reference impls plus Mem0/Zep extension packages — pure framework-over-store, no temporal features. **Verdict: channel; near-zero competitive weight** — but the AutoGen deprecation churn means low integration ROI; AG2's memory protocol is trivial to implement over.

---

## 7. Redis as agent memory

**redis/agent-memory-server — it exists and is real.** Apache-2.0, v0.15.2 (Apr 2026), 21 releases but only **288 stars** — corporate-backed, low community pull ([repo](https://github.com/redis/agent-memory-server), accessed 2026-07-06). Two-tier design: session-scoped **working memory** → LLM-extracted persistent **long-term memory** (semantic/keyword/hybrid search via RedisVL+RediSearch), configurable extraction strategies, **dual REST + MCP interface**, namespaces/sessions/user scoping, LiteLLM multi-provider incl. Ollama; Redis productizes it as managed "Redis Agent Memory" ([redis.io/agent-memory](https://redis.io/agent-memory/); [docs](https://redis.github.io/agent-memory-server/), accessed 2026-07-06).

**Store underneath.** Redis, obviously — a server dependency, not embedded (no offline single-file story; "local" means running Redis locally). Redis also owns `langgraph-checkpoint-redis` and partners with cognee ([Redis-cognee blog](https://redis.io/blog/build-faster-ai-memory-with-cognee-and-redis/)) — a "be the store under every framework" strategy, i.e., **the same substrate play GenesisBlockDB is making, from the opposite (server) end**.

**Temporal.** None meaningful — TTL/recency, not versioning or point-in-time.

**Verdict.** **Competitor (as substrate), not channel.** Redis will not sit on your engine; it *is* the rival store. Its weakness for your tiers: server process, RAM-priced, no bitemporal, nothing embedded/on-device.

---

## Cross-cutting answers

**Who has tier-B time-travel/versioned state/audit today?**
1. **LangGraph checkpointers** — real per-step replay/fork of runs (opaque blobs, per-thread only). The only one with *run* time-travel.
2. **Graphiti/Zep** — real bitemporal *facts* with point-in-time queries (not run state).
3. **Letta Context Repositories** — git-based memory versioning (new, Feb 2026, depth UNVERIFIED).
4. Mem0 — SQLite op-history audit only. cognee, CrewAI, AG2, Redis AMS — effectively none.
**Nobody combines** run-level replay + bitemporal shared graph + audit in one queryable store — that combination is still open.

**Competitor / channel matrix (embedded-engine perspective):**
| Player | Competitor? | Channel? | Embedded/offline today |
|---|---|---|---|
| Mem0 | Yes (v3 internal graph) | Possible (vector/graph provider API) | Partial (SQLite defaults, buggy) |
| Letta | Yes (tier-B server) | Weak (Postgres-coupled) | Server-local only; SQLite unsupported for migration |
| Zep/Graphiti | Cloud only | **Best** (driver interface; #1240 proves demand) | Wants it, doesn't have it (Kuzu orphaned) |
| cognee | Yes (local-first positioning) | Yes (adapter layer) | Yes — Kuzu+LanceDB+SQLite glue |
| LangGraph | **Strongest tier-B incumbent** | **Cleanest** (checkpointer/BaseStore ifaces) | SQLite checkpointer |
| CrewAI / AG2 | No | Yes (proven outsourcing pattern) | File-local defaults, production-broken |
| Redis AMS | Yes (substrate rival) | No | No (server) |

**Structural takeaway.** Every framework here is a channel *in principle* because none is an engine — they all bottom out in SQLite/Chroma/LanceDB/Kuzu/Neo4j/Redis/Postgres. The two highest-leverage integration surfaces are (a) a **Graphiti driver** (issue #1240 is an explicit request for embedded zero-config graph storage, and their embedded option just lost its vendor) and (b) a **LangGraph checkpointer + BaseStore** (rides 50M downloads/mo and instantly inherits the time-travel narrative — but there you're competing on being a *better store for existing semantics*, since LangGraph already ships the replay UX over SQLite for free).

*All URLs accessed 2026-07-06. Star counts, funding figures, and pricing are as reported by the cited sources on that date; several (cognee funding, Letta Context Repositories depth, LangGraph download totals) are secondary-sourced and marked UNVERIFIED where noted.*