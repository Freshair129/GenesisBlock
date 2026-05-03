# MSP — Memory & Soul Passport

> Gatekeeper layer that sits on top of [`@freshair129/gks`](https://github.com/Freshair129/GksV3) and enforces the schema, ID, and wikilink discipline described in [`msp_spec.md`](./msp_spec.md).

## What this repo is

GKS is a *storage engine*. MSP is the *Memory OS gatekeeper* above it — schema validation, ID-uniqueness, wikilink resolution, forbidden-field guard, and the promote workflow that turns a candidate atom in `.brain/msp/projects/evaAI/inbound/` into a stable artifact under `gks/<type>/`.

See `msp_spec.md` for the full surface (envelope, atomic write contract, codegen contract, phase governance, memory subsystem, promotion levels).

## Layout

```
msp/
├── msp_spec.md                       authoritative human-readable spec
├── gks/                              canonical atom tree (committed)
│   ├── 00_index/atomic_index.jsonl
│   ├── concept/  adr/  feat/  blueprint/  frame/
│   └── audit/  task/  issues/
├── .brain/msp/projects/evaAI/        runtime state (mostly gitignored)
│   ├── inbound/                      candidate atoms awaiting review
│   ├── audit/  session/  memory/  vector/
└── src/validator/                    MSP validator implementation (M2)
```

## Workflow (doc-to-code, P1 → P6)

```
P1 CONCEPT → P2 ADR/FEAT → P3 BLUEPRINT → P4 TASK → P5 src/ → P6 AUDIT
```

```sh
npm run msp:propose -- CONCEPT--MSP-VALIDATOR --title="..."
npm run msp:list                                    # what's in inbound
npm run msp:promote CONCEPT--MSP-VALIDATOR          # → gks/concept/
npm run msp:verify FEAT--MSP-VALIDATOR              # gate before src/
npm run msp:validate                                # MSP's own validator
```

Or use GKS directly:

```sh
npx gks new-feature msp-validator \
  --title="MSP validator pipeline" \
  --concept="why we need it" \
  --adr="forbidden fields + dangling wikilinks + ID uniqueness" \
  --blueprint-file=src/validator/index.ts
```

## Status

- [x] **M0** — Bootstrap (npm + GKS install + `gks/` tree)
- [x] **M1** — Slice `msp_spec.md` into atoms via inbound queue
- [x] **M2** — Implement validator under `src/validator/` (49/49 tests)
- [x] **M3a** — Pre-commit hook (`examples/hooks/`) — block bad atoms locally before they reach CI
- [ ] **M3b** — Load forbidden-fields list from `atomic_contract.yaml` at runtime
- [ ] **M3c** — Implement the 4 FEAT scaffolds (codegen runner + 3 memory writers)

## Pre-commit hook

```sh
bash examples/hooks/install.sh
```

After install, `git commit` blocks if any staged `.md` under `gks/` or `.brain/msp/projects/<ns>/inbound/` fails the validator. Skip with the standard `git commit --no-verify`. Full docs: [`examples/hooks/README.md`](./examples/hooks/README.md).

## License

MIT
