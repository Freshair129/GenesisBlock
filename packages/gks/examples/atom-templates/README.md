# Atom templates

Starter `.md` templates per prefix — a **representative subset** of the full taxonomy
in [`docs/KNOWLEDGE-TYPES.md`](../../docs/KNOWLEDGE-TYPES.md) (~30 atom types). The
templates below are the most commonly authored types; for the complete prefix list
including IDEA, CONCEPT, FRAME, MASTER, PROTO, MOD, PROTOCOL (interaction-contract),
API/ENDPOINT/ENTRYPOINT, ENTITY, PARAMS, PERSONA, POLICY, REQ, CONSTRAINT, etc.,
see KNOWLEDGE-TYPES.md.

Each template is the **minimum viable shape** for that atom type:
required frontmatter + recommended frontmatter + body skeleton with
section headings. **Workflow:** copy → fill in → propose via the `msp_candidate`
MCP tool which writes to `.brain/msp/projects/<ns>/candidates/`, then human PR
review promotes to `gks/<type>/`. (For ISSUE-- — light-governance tier per
ADR-012 — you can write directly into `gks/issues/` without the candidates step.)

> **Migration note:** Legacy `gks propose-inbound` / `msp:propose` CLI commands
> were removed in Phase 3 (2026-05-09); use `msp_candidate` MCP tool instead.

## Available templates

### Implementation flow

- [`ADR--TEMPLATE.md`](./ADR--TEMPLATE.md) — architecture decision
- [`FEAT--TEMPLATE.md`](./FEAT--TEMPLATE.md) — feature spec
- [`ALGO--TEMPLATE.md`](./ALGO--TEMPLATE.md) — algorithm
- [`FLOW--TEMPLATE.md`](./FLOW--TEMPLATE.md) — data / UI flow
- [`ENTITY--TEMPLATE.md`](./ENTITY--TEMPLATE.md) — data schema
- [`BLUEPRINT.yaml`](./BLUEPRINT.yaml) — implementation plan (YAML variant; MSP also accepts `.md` with YAML frontmatter)
- [`AUDIT--TEMPLATE.md`](./AUDIT--TEMPLATE.md) — verification report

### Agent governance

- [`SKILL--TEMPLATE.md`](./SKILL--TEMPLATE.md) — agent capability
- [`PROTOCOL--TEMPLATE.md`](./PROTOCOL--TEMPLATE.md) — interaction contract (HTTP/MCP-style, multi-step handshake)
- [`GUARD--TEMPLATE.md`](./GUARD--TEMPLATE.md) — runtime-enforced policy

### Requirements engineering

- [`FR--TEMPLATE.md`](./FR--TEMPLATE.md) — functional requirement
- [`NFR--TEMPLATE.md`](./NFR--TEMPLATE.md) — non-functional requirement

### Ops governance

- [`INC--TEMPLATE.md`](./INC--TEMPLATE.md) — incident post-mortem
- [`ISSUE--TEMPLATE.md`](./ISSUE--TEMPLATE.md) — live issue (light-governance)
- [`HOTFIX--TEMPLATE.md`](./HOTFIX--TEMPLATE.md) — hotfix escape hatch (48h backfill window, ADR-014)
- [`RISK--TEMPLATE.md`](./RISK--TEMPLATE.md) — risk + mitigation
- [`RUNBOOK--TEMPLATE.md`](./RUNBOOK--TEMPLATE.md) — operational response guide ("if you see X, do Y")
- [`SLO--TEMPLATE.md`](./SLO--TEMPLATE.md) — service-level objective

### Types without bundled templates

For these, create the file by hand following the schema in KNOWLEDGE-TYPES.md:
IDEA, CONCEPT, FRAME, MASTER (root policy), PROTO (machine-enforced invariant),
MOD, API, ENDPOINT, ENTRYPOINT, PARAMS, PERSONA, POLICY, REQ, CONSTRAINT,
INSIGHT, FACT, RULE (auto-derived).

## Conventions

- **`id`** must match `^[A-Z][A-Z0-9_]*--[A-Z0-9][A-Z0-9_-]*$`
- **`phase`** must be an integer 0-6 (validator-enforced range). P7 (Ops) is discussed conceptually but isn't an atom phase
- **`type`** must match the prefix lowercased (`ADR-- → type: adr`)
- **`status`** must be one of: `stub` / `raw` / `draft` / `active` / `stable` / `deprecated` / `superseded` / `partial` (validator-enforced via `src/validator/rules/phase-status.ts`)
  - Atom lifecycle: `draft` → `stable` (after PR review merge) → optionally `superseded` or `deprecated` later
  - Issues (light-governance): use ISSUE-- workflow statuses `open` → `triaged` → `in_progress` → `closed` (stored in atom body, distinct from atom-level `status`)
- **`linked_symbols`** + **`geography`** — see ADR-010 for cross-reference semantics

## See also

- [`docs/KNOWLEDGE-TYPES.md`](../../docs/KNOWLEDGE-TYPES.md) — full reference
- [`docs/adr/012-extended-taxonomy.md`](../../docs/adr/012-extended-taxonomy.md) — why this list exists
- [`docs/adr/010-reverse-citation-lookup.md`](../../docs/adr/010-reverse-citation-lookup.md) — `linked_symbols` semantics
