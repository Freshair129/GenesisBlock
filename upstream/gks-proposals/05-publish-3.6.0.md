# 🔵 Proposal 05 — Publish `@freshair129/gks@3.6.0` to npm

**Filed upstream**: [Freshair129/GksV3#28](https://github.com/Freshair129/GksV3/issues/28) (2026-05-07)

## Why

`Freshair129/GksV3` repo at HEAD is `package.json: "version": "3.6.0"` and the CHANGELOG documents new features (`createNomicEmbedder`, `OBSIDIAN_URL` env convention, `.env.example` updates). But **npm registry latest is still 3.5.6** — `npm view @freshair129/gks dist-tags` returns `{ latest: '3.5.6' }`, and the 3.6.0 source is unavailable to consumers.

MSP's PR #9 / spec 2.0.1 references 3.6.0 features (locked canonical model = `nomic-embed-text-v1.5`). Consumers have to either:

- Pin a git URL (`@freshair129/gks#main`) — fragile, no dep resolution
- Wait for publish — current state, blocking M7c retrieval orchestration

Most concretely: `ADR--EMBEDDING-MODEL-PARITY` had to add a "Status note" marking the nomic claims as **aspirational** until 3.6.0 publishes. That note can be removed once this lands.

## What

Publish `3.6.0` to npm:

```
git checkout main           # in Freshair129/GksV3
npm version 3.6.0           # if package.json isn't already at 3.6.0 (it is)
npm publish --access public
```

Optionally tag the release on GitHub:
```
git tag v3.6.0
git push origin v3.6.0
gh release create v3.6.0 --notes-file CHANGELOG-3.6.0.md
```

## Compat

- Additive minor bump (per CHANGELOG): new `createNomicEmbedder()`, `.env.example` additions.
- Existing consumers on 3.5.x: `^3.5.6` semver auto-picks 3.6.0 on next `npm install`. Their existing code keeps working (Ollama / OpenAI / mock providers preserved).
- No API-shape breaks for the `MemoryStore` / `InboundQueue` / `EpisodicLayer` interfaces.

## Test (pre-publish checklist)

```
npm run build           # tsc clean
npm test                # vitest green
npm pack                # inspect tarball: 
                        #   - dist/src/memory/vector/embedder.js exports createNomicEmbedder
                        #   - .env.example contains OBSIDIAN_URL
                        #   - package.json version: "3.6.0"
```

## Downstream impact

| Consumer | Change after 3.6.0 publishes |
|---|---|
| MSP (this project) | `npm install` auto-picks 3.6.0 via `^3.5.6`. Remove "Status note" from ADR--EMBEDDING-MODEL-PARITY. Remove fallback table. M7c can use `createNomicEmbedder` directly. |
| EVA (memory_os) | Same — auto-picks. |
| New consumers | Get nomic by default, no Ollama prerequisite. |

## Atom reference

- MSP: `gks/adr/ADR--EMBEDDING-MODEL-PARITY.md` (this PR — has Status note pending this proposal)
- MSP: `gks/audit/AUDIT--TWO-REPO-VALIDATION.md` (this PR — records the discrepancy)
- MSP: `gks/concept/CONCEPT--EMBEDDING-STRATEGY.md` (referenced from)

## Drafted

2026-05-04, M7-prep follow-up + two-repo validation audit.
