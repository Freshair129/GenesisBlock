# Embedder compatibility with browse-side surfaces

## TL;DR

GKS 3.6.0+ (current: 3.7.0) ships `createNomicEmbedder()` using
`nomic-ai/nomic-embed-text-v1.5` (768-dim, Thai+English, fully local).
If your project also runs a Memory OS that exposes the same vault to an
in-Obsidian browse plugin (e.g. Smart Connections), configure that plugin
to use the **same model**. Otherwise the vault is embedded twice into
incompatible vector spaces.

> **Monorepo note (2026-05-11+)**: GKS now ships from
> `packages/gks/` of the `cognitive_system` monorepo. Run package-scoped
> commands via `npm run <script> --workspace=packages/gks` from the repo
> root (e.g. `npm run re-embed --workspace=packages/gks`). The CLI is
> still callable as `gks <cmd>` once `packages/gks` is built and linked,
> matching the pre-monorepo invocation shown below.

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
- 2× storage (`.brain/.../vector/` + `.smart-connections/`)
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
# Monorepo (current):
npm run re-embed --workspace=packages/gks   # GKS rebuilds .brain/.../vector/

# Or, if `gks` CLI is on PATH (after `npm link`):
gks re-embed

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
