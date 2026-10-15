# RCA: Agent Instruction Policy Drift

## Status / Date

- Status: confirmed
- Date: 2026-10-07

## Symptom

The repository had overlapping agent instruction files with policy differences. The supplied audit measured a 25-line difference between `AGENTS.md` and `CLAUDE.md` after normalizing the tool name. The initial checked-in files did not mark a canonical shared policy or generated mirror sections.

## Evidence

- At the initial repository revision, `AGENTS.md`, `CLAUDE.md`, and `GEMINI.md` contained no `CANONICAL_AGENT_POLICY_START` or `GENERATED_AGENT_POLICY_START` markers.
- The initial `AGENTS.md` identified Codex in its opening guidance, while `CLAUDE.md` identified Claude Code; the instructions were maintained as separate files.
- `package.json` initially mapped `agents:validate` only to `.agents/validate-agent-registry.mjs`. No check compared shared policy text across the instruction files.
- The initial validator required the registry context files but did not require `AGENTS.md` as a global context file.

## Root Cause

Shared rules were copied and adapted manually across tool-specific instruction files. The registry validator checked agent identities and routes, not shared instruction content, so edits could drift without a deterministic failure.

## Why the Issue Escaped Detection

Existing checks validated document status and the agent registry, but did not identify one policy source, mark generated sections, or compare mirrors. Review depended on a person noticing semantic differences during a broad documentation review.

## Fix (Decided)

Use versioned `AGENTS.md` as the shared R1-R10 source. Generate marked shared-policy sections in `CLAUDE.md` and `GEMINI.md`; make `agents:validate` check mirror freshness and canonical metadata; retain tool-specific text outside those sections. Keep the Rust MSRV undeclared until a locked check passes on the exact candidate toolchain.

## Outcome (Measured)

The focused Node suite passed (5/5); `npm run agents:validate` passed for 6 agents and 12 routes and confirmed both mirrors match `AGENTS.md` v1.0.0; the documentation status validator reported 0 violations across 241 files; and `git diff --check` passed. Cargo/rustc 1.88.0 passed `cargo check --locked` with default features, and locked dependency metadata reported no package minimum above 1.88; `Cargo.toml` now declares `rust-version = "1.88"`. Rustup's proxy self-update was denied, so the installed toolchain binaries were invoked directly. Cross-target CI feature builds were not run.

## Proposed Prevention

- Run `npm run agents:validate` to fail on stale or malformed generated policy mirrors.
- Update shared policy only in `AGENTS.md`, then run `npm run agents:sync`.
- Keep tool-specific instructions outside generated markers and review version changes with the normal repository documentation workflow.
- Declare `package.rust-version` only after `cargo check --locked` passes on that exact Rust toolchain.
