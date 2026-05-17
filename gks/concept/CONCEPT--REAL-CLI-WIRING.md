---
id: CONCEPT--REAL-CLI-WIRING
phase: 1
type: concept
status: stable
vault_id: default
tier: process
source_type: axiomatic
title: Real CLI wiring for T1/T2/T3 tier adapters — invocation patterns + opt-in
  integration tests
tags: &a1
  - msp
  - phase-e1
  - tier-adapters
  - cli
  - integration-test
crosslinks: &a2
  references:
    - AUDIT--PHASE-E1-REAL-CLI-WIRING
    - CONCEPT--TIER-3-DEFERRED
created_at: 2026-05-14T04:00:00.000+07:00
aliases: &a3
  - CONCEPT
  - implementation_flow
  - Strategic intent / PRD
cluster: implementation_flow
role: Strategic intent / PRD
attributes:
  id: CONCEPT--REAL-CLI-WIRING
  phase: 1
  type: concept
  status: stable
  vault_id: default
  tier: process
  source_type: axiomatic
  title: Real CLI wiring for T1/T2/T3 tier adapters — invocation patterns + opt-in
    integration tests
  tags: *a1
  crosslinks: *a2
  created_at: 2026-05-14T04:00:00.000+07:00
  aliases: *a3
  cluster: implementation_flow
  role: Strategic intent / PRD
  attributes:
    id: CONCEPT--REAL-CLI-WIRING
    phase: 1
    type: concept
    status: stable
    vault_id: default
    tier: process
    source_type: axiomatic
    title: Real CLI wiring for T1/T2/T3 tier adapters — invocation patterns + opt-in
      integration tests
    tags: *a1
    crosslinks: *a2
    created_at: 2026-05-14T04:00:00.000+07:00
    aliases: *a3
    cluster: implementation_flow
    role: Strategic intent / PRD
    attributes:
      domain: concept
    domain: concept
    language: markdown
    is_test: false
    is_entrypoint: false
    has_secret: true
    secret_type: high_entropy_string
    leak_risk: high
    encryption_level: none
  domain: concept
  language: markdown
  is_test: false
  is_entrypoint: false
  has_secret: true
  secret_type: high_entropy_string
  leak_risk: high
  encryption_level: none
---

# CONCEPT — Real CLI wiring for T1/T2/T3 adapters

## Why this atom exists

Phase D shipped tier adapters (`qwen.ts` / `gemini.ts` / `claude.ts`) that
`spawn` real CLI binaries through `spawn-helper.ts`. Unit tests mock
`runCli()` so CI passes without binaries installed.

This atom records (a) the **actual** invocation pattern each adapter uses,
verified against the installed CLIs on 2026-05-14, and (b) the convention
for opt-in real-CLI integration tests that exercise the wiring locally.

## Invocation patterns (verified 2026-05-14)

### T1 — Qwen (`packages/msp/src/agents/tiers/qwen.ts`)

```
qwen <prompt-positional>
```

**Binary**: `qwen` (Python Ollama-wrapper CLI; `packages/qwen-cli/qwen.py`,
installed as `qwen.exe` shim under Python Scripts/).

**Notes**:
- The CLI takes the prompt as **positional** args (joined with spaces).
  There is NO `--prompt` flag. The earlier `qwen.md` doc referred to an
  aspirational pattern that the installed binary does not implement.
- Does NOT support `--version` (exits non-zero with "unrecognized
  arguments"). Healthcheck uses `--help` instead.
- Backend: Ollama at `http://localhost:11434`. If Ollama is not running,
  the CLI exits 1 with "ERROR: Ollama not running" on stderr — adapter
  surfaces this through `result.ok === false`.

### T2 — Gemini (`packages/msp/src/agents/tiers/gemini.ts`)

```
gemini --approval-mode yolo -p "<prompt>"
```

**Binary**: `gemini` (npm package `@google/gemini-cli`, install globally
via `npm i -g @google/gemini-cli`; verified against v0.42.0).

**Flag rationale**:
- `-p / --prompt`: non-interactive (headless) mode.
- `--approval-mode yolo`: auto-approve all tool calls — required for
  agent-driven invocation; without it the CLI would prompt interactively.
- Healthcheck: `gemini --version` (supported, exits 0).

### T3 — Claude (`packages/msp/src/agents/tiers/claude.ts`)

```
claude --print "<prompt>"
```

**Binary**: `claude` (npm package `@anthropic-ai/claude-code`, install
globally via `npm i -g @anthropic-ai/claude-code`; verified against
v2.1.140).

**Flag rationale**:
- `-p / --print`: print response and exit (non-interactive mode); the
  workspace-trust dialog is skipped when stdout is not a TTY.
- Healthcheck: `claude --version` (supported, exits 0).

## Install pointers

Each CLI has its own installer & auth flow. This atom does not embed URLs
(deliberate — they rot). Look them up:

- **qwen**: see `packages/qwen-cli/setup.py` for Python deps; needs
  `ollama serve` running locally with `qwen2.5-coder:14b` model pulled.
- **gemini**: npm package `@google/gemini-cli`. Auth via Google account
  on first run (`gemini` command opens browser flow).
- **claude**: npm package `@anthropic-ai/claude-code`. Auth via
  `claude /login` (subscription) or `ANTHROPIC_API_KEY` env var.

## Healthcheck semantics (contract)

Each adapter's `healthcheck()`:

1. **Returns `false`** when the binary is missing on PATH — **never
   throws**. The spawn-helper translates ENOENT (POSIX, or `shell: false`
   on Windows) into `exit_code: -1`. On Windows with `shell: true`,
   cmd.exe surfaces "not recognized" as a non-zero exit instead. Either
   way, `exit_code !== 0` → healthcheck returns false.
2. **Returns `true`** when the binary responds successfully to the
   probe (`--version` for gemini / claude, `--help` for qwen). This
   only verifies the binary is callable — NOT that the upstream LLM
   service (Ollama, Gemini API auth, Claude auth) is reachable.
3. **Has a 3 s timeout** (`HEALTHCHECK_TIMEOUT_MS`). A hung binary is
   treated as unhealthy.

## Opt-in integration test convention

`packages/msp/test/agents/integration/real-cli.test.ts` exercises each
adapter against the real CLI binaries. The entire suite is **gated** on:

```
process.env.MSP_TEST_REAL_CLIS === '1'
```

`describe.skipIf(!ENABLED)` ensures the suite is silently skipped in CI.
Developers opt in locally via:

```
MSP_TEST_REAL_CLIS=1 npm test --workspace=packages/msp -- test/agents/integration/
```

**Per-adapter behaviour when enabled**:

- `healthcheck()` is called first; the test only asserts that it returns
  a boolean (never throws).
- `run('echo "test"', { timeout_ms: 30000, capture_stderr: true })` is
  invoked. The test asserts the **shape** of the `RunResult` (typeof
  ok/output/exit_code), NOT that `ok === true` — because backend
  outages (Ollama down, API key missing) are environmental, not adapter
  bugs. A non-zero exit is logged with context.
- If `healthcheck()` is false, the run-test is silently skipped with a
  console message — developers with only one or two CLIs installed can
  still run the suite.

## Adapter prompt-arg safety

`spawn-helper.ts` uses `shell: true` for bare binary names on Windows so
that `.cmd` shims resolve via PATHEXT. Node's DEP0190 warns that args are
concatenated without escaping under shell mode — this is a real risk for
**untrusted** prompts.

**Mitigation in current design**: prompts are treated as
**trusted-internal** input. They originate from the MSP orchestrator
(`packages/msp/src/master/`), never from raw external user input. This
matches the threat model in `[[BLUEPRINT--AGENT-DISPATCHER]]`. If/when MSP
ever forwards untrusted prompts to a tier CLI, that BLUEPRINT must be
amended and `spawn-helper.ts` hardened (e.g. resolve full binary paths
upfront so `shell: false` can be used everywhere).

## Invariants

- Adapters MUST NOT throw from `healthcheck()` on missing binary — they
  return false.
- Adapters MUST forward `RunOpts.timeout_ms` and `capture_stderr` to
  `runCli`.
- The `TierAdapter` interface (`name` / `healthcheck` / `run`) is
  stable and not changed by this concept.
- Integration tests stay SKIP in CI; only run with explicit env opt-in.

## Source

- Direct inspection of `qwen --help`, `gemini --help`, `claude --help`
  on the developer machine 2026-05-14.
- `packages/qwen-cli/qwen.py` source for T1 confirmation.
- Cross-referenced against `qwen.md`, `GEMINI.md` (root docs — qwen.md
  was stale re: `--prompt` flag and is now superseded by this atom's
  positional-arg pattern).

## Connections
- [[AUDIT--PHASE-E1-REAL-CLI-WIRING]]
- [[CONCEPT--TIER-3-DEFERRED]]

