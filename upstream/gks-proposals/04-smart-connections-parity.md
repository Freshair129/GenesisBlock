# 🔵 Proposal 04 — Document Smart Connections + nomic-embed-text-v1.5 compatibility

**Filed upstream**: [Freshair129/GksV3#29](https://github.com/Freshair129/GksV3/issues/29) (2026-05-07)

## Why

GKS 3.6.0 introduced `createNomicEmbedder()` running `nomic-ai/nomic-embed-text-v1.5`
locally — 768-dim, Thai+English support, no Ollama required. Excellent default.

Memory OS layers above GKS (MSP, EVA, etc.) often use **Obsidian + Smart
Connections** as a human-facing browse path. Smart Connections embeds the
same vault — independently, with its own model choice in the GUI.

If GKS embeds with model A and Smart Connections embeds with model B, the
**same atom is embedded twice with incompatible vectors** (different
dimension, tokenizer, normalisation). MSP's `ADR--EMBEDDING-MODEL-PARITY`
locks both surfaces to `nomic-embed-text-v1.5` to avoid this.

GKS doesn't have to enforce this — it's a Memory OS concern. But a one-page
documentation note in GKS would help every Memory OS implementer make the
same decision instead of discovering the divergence the hard way.

## What

A short documentation page added to GKS — no code changes.

### Proposed file: `docs/embedder-compatibility.md`

Suggested outline (full draft below):

```
# Embedder compatibility with browse-side surfaces

## TL;DR

GKS embeds atoms via `createNomicEmbedder()` (default in 3.6.0+) using
`nomic-ai/nomic-embed-text-v1.5` — 768-dim, Thai+English. If your stack
also runs an in-Obsidian browse plugin (Smart Connections, Quartz, etc.)
that embeds the same vault, configure it to use the same model — otherwise
the same atoms are embedded twice into incompatible spaces.

## Why this matters

[explain double-embed cost + cross-surface incompatibility]

## Smart Connections (most common browse plugin)

Settings → Smart Connections → Embedding Model:
   nomic-ai/nomic-embed-text-v1.5

[screenshot or path-walk]

## Other browse plugins

[brief paragraph for each common one — Quartz, Smart View successors]

## What if you can't match?

Configurations diverge: agents see GKS-canonical results; humans see the
plugin's view. Drift visible to humans, not destructive — but neighbours
in Smart View won't match what `gks recall` returns.

## Re-embedding after a model swap

[two-line `npm run gks re-embed` + plugin re-index instructions]
```

### Full draft body

```markdown
# Embedder compatibility with browse-side surfaces

## TL;DR

GKS 3.6.0+ ships `createNomicEmbedder()` using `nomic-ai/nomic-embed-text-v1.5`
(768-dim, Thai+English, fully local). If your project also runs a Memory OS
that exposes the same vault to an in-Obsidian browse plugin (e.g. Smart
Connections), configure that plugin to use the **same model**. Otherwise
the vault is embedded twice into incompatible vector spaces.

## Why this matters

Embedding models differ in:
- **Dimension** (768 vs 384 vs 1024 vs ...)
- **Tokenizer** (BPE vs SentencePiece vs WordPiece)
- **Normalisation** (L2 vs none)

Two models = two vector spaces. Vectors from one are not comparable to
vectors from the other. Cross-surface retrieval becomes impossible without
re-embedding queries through both pipelines.

Cost when models diverge:
- 2× compute (every atom embedded twice)
- 2× storage (.brain/.../vector/ + .smart-connections/)
- Inconsistent similarity rankings between agent recall and human browse

## Smart Connections (most common Obsidian browse plugin)

Configure Smart Connections to match GKS's default:

```
Obsidian Settings → Smart Connections → Embedding Model:
   nomic-ai/nomic-embed-text-v1.5
```

Smart Connections supports model selection via its GUI. Pick the same model
GKS uses; re-index the vault after switching.

## Other browse plugins

Most Obsidian "find similar notes" plugins expose a model picker. The same
principle applies: match GKS's `createNomicEmbedder()` model.

For headless / non-Obsidian setups (CI, server, Quartz static export):
GKS's vector store is independent — Smart Connections is **not required**.
Agent-facing recall works without any Obsidian browse plugin.

## What if your project deliberately uses a different model

Allowed. Document the deviation in your project's ADRs (e.g. for a
multilingual project that prefers BGE-M3). Configure both GKS (via
`createNomicEmbedder` options or a custom `Embedder`) and the browse
plugin to use the chosen model. The ADR records why.

## Re-embedding after a model swap

If you change the canonical model:

```bash
npm run gks re-embed                   # GKS rebuilds .brain/.../vector/
# Then in Obsidian:
# Settings → Smart Connections → Re-index vault
```

Both indexes must be rebuilt; querying mid-migration returns mixed-model
results which are not useful.

## Why GKS doesn't enforce this

Embedder choice is a Memory OS / project concern. GKS provides:
- A sensible default (`nomic-embed-text-v1.5`)
- A pluggable `Embedder` interface for swap-in
- A re-embed CLI

What it does **not** do:
- Force a particular model
- Read or coordinate with the browse plugin's index
- Detect drift between GKS and plugin model choice

Drift detection (e.g. "your Smart Connections model differs from GKS's") is
a Memory OS concern. See e.g. MSP's `ADR--EMBEDDING-MODEL-PARITY` for one
such project's policy.
```

## Compat

- **Documentation only** — no code change, no test impact.
- Lives under `docs/` so existing GKS users are unaffected; new users hit it via the table-of-contents.

## Test

N/A — documentation.

## Atom reference

- MSP: `gks/adr/ADR--EMBEDDING-MODEL-PARITY.md` (this PR)
- MSP: `gks/concept/CONCEPT--EMBEDDING-STRATEGY.md` (updated this PR)
- MSP: `gks/adr/ADR--SEMANTIC-SEARCH-VIA-SMART-CONNECTIONS.md` (updated this PR)

## Drafted

2026-05-04, M7-prep follow-up audit.
