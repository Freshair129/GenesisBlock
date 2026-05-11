# `examples/` — supporting setup, hooks, starter content

Things that ship alongside MSP for users to copy, adapt, or read. Nothing
under this tree is required for the validator / writers / MCP server to
run; everything here is opt-in scaffolding.

## Contents

| Path | What | When you need it |
|------|------|------------------|
| [`hooks/`](./hooks/README.md) | git pre-commit + pre-push hooks (bash) that run `msp:validate` and `gks verify-flow` | First time setting up a worktree; see [`hooks/README.md`](./hooks/README.md) for install / behaviour. |
| [`setup/smart-connections-config.md`](./setup/smart-connections-config.md) | Step-by-step: install Smart Connections, pick `nomic-embed-text-v1.5`, re-index, verify, troubleshoot. | Before you expect `msp_recall` to do semantic retrieval. |
| [`setup/obsidian-local-rest-api.md`](./setup/obsidian-local-rest-api.md) | Companion to the above: install Local REST API plugin, generate the API key, set `OBSIDIAN_HOST` / `OBSIDIAN_API_KEY`, verify with `curl`. | Whenever MSP should talk to a running Obsidian instead of file-only fallback. |
| [`identity/sample-profile.yaml`](./identity/sample-profile.yaml) | Annotated starter identity profile (M7e). Heavy comments — copy and trim. | When the M7e identity layer lands; until then, read it as a schema reference. |

## Reading order for a new user

1. **Get hooks installed** — [`hooks/README.md`](./hooks/README.md). Five
   minutes; idempotent installer; commits/pushes get validated locally
   before CI sees them.
2. **Get Obsidian wired up** — first
   [`setup/obsidian-local-rest-api.md`](./setup/obsidian-local-rest-api.md)
   (so MSP can reach the vault), then
   [`setup/smart-connections-config.md`](./setup/smart-connections-config.md)
   (so semantic recall works once M7c lands).
3. **Plan your identity** — skim
   [`identity/sample-profile.yaml`](./identity/sample-profile.yaml). The
   profile won't be loaded by anything until M7e merges, but the shape is
   stable and the comments explain what each field will gate.

## Background atoms

The configuration choices in this directory are recorded in the canonical
knowledge graph under `gks/`:

| Topic | Atom |
|-------|------|
| Why MSP doesn't ship its own embedder | [`gks/concept/CONCEPT--EMBEDDING-STRATEGY.md`](../gks/concept/CONCEPT--EMBEDDING-STRATEGY.md) |
| Decision: semantic search via Smart Connections | [`gks/adr/ADR--SEMANTIC-SEARCH-VIA-SMART-CONNECTIONS.md`](../gks/adr/ADR--SEMANTIC-SEARCH-VIA-SMART-CONNECTIONS.md) |
| Obsidian as the runtime layer | [`gks/concept/CONCEPT--OBSIDIAN-AS-RUNTIME.md`](../gks/concept/CONCEPT--OBSIDIAN-AS-RUNTIME.md) |
| REST primary / file fallback | [`gks/adr/ADR--MSP-OBSIDIAN-INTEGRATION.md`](../gks/adr/ADR--MSP-OBSIDIAN-INTEGRATION.md) |
| Pre-commit hook design | [`gks/adr/ADR--MSP-PRECOMMIT-HOOK.md`](../gks/adr/ADR--MSP-PRECOMMIT-HOOK.md) |
| Pre-push hook design | [`gks/adr/ADR--MSP-PREPUSH-HOOK.md`](../gks/adr/ADR--MSP-PREPUSH-HOOK.md) |
| Identity / soul layer scope | `msp_spec.md` §7e + roadmap M7e |

## Out of scope for `examples/`

- **No source code.** Anything that needs to be imported lives under `src/`.
- **No atoms.** This tree is supporting documentation; durable knowledge
  goes in `gks/<type>/` via the inbound queue (`npm run msp:propose`).
- **No spec changes.** Adjustments to the Smart Connections / Obsidian
  contract belong in the relevant ADR, not in these guides.
