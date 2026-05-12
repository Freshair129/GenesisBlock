---
id: ADR--CODEGEN-POST-PROCESSING
phase: 2
type: adr
status: stable
vault_id: default
tier: genesis
source_type: axiomatic
title: Codegen post-processing — strip wrapping before pattern checks
tags:
  - msp
  - codegen
  - post-processing
  - slm
crosslinks: {"references":["CONCEPT--CODEGEN-MICROTASK-CONTRACT"]}
created_at: 2026-05-03T15:08:41.716+07:00
---

# ADR — codegen post-processing

## Context

T1 SLMs (Qwen 2.5 Coder, Llama local) wrap their output in ways that break downstream tooling: markdown fences around code, "Sure, here's the implementation:" preambles, trailing explanations, mixed line endings from Windows-trained models. We can either teach every check downstream to ignore these, or strip them once at the boundary.

## Decision

Run a deterministic post-processor on every SLM completion, before any other check. Four strip operations, in order:

| Step | Setting | What |
|---|---|---|
| 1 | `strip_markdown_fences: true` | remove ` ``` `, ` ```ts `, ` ``` ` wrappers |
| 2 | `strip_leading_commentary: true` | drop everything before the first `import`, `export`, or `const` line |
| 3 | `strip_trailing_commentary: true` | drop everything after the last top-level `}` |
| 4 | `normalize_line_endings: "\n"` | convert CRLF / mixed → LF |

Order matters — fence-stripping first reveals the real first/last code lines for steps 2/3.

## Algorithm sketch

```ts
function postProcess(raw: string): string {
  let s = raw
  s = stripMarkdownFences(s)        // /^[`~]{3,}.*$\n?/ at start, /\n?[`~]{3,}\s*$/ at end
  s = stripLeadingCommentary(s)     // find first /^(import|export|const|function|class)/m
  s = stripTrailingCommentary(s)    // find last balanced `}` and truncate after
  s = s.replace(/\r\n?/g, '\n')
  return s
}
```

## Consequences

**Positive**
- One place to fix SLM wrapping artefacts.
- Pattern checks downstream see clean code regardless of model.
- Deterministic — same input → same output, no LLM in the loop.

**Negative**
- `strip_trailing_commentary` is heuristic. If a snippet legitimately ends mid-block (e.g. an unfinished helper), we'll truncate too aggressively. Acceptable — the acceptance test will catch the resulting compile error.
- Fence stripping doesn't handle nested fences in markdown-as-string content. SLMs don't typically emit such, so accepting the risk.

## Alternatives considered

1. **Train SLMs not to wrap.** Out of scope — we don't own the model weights.
2. **Run prettier instead.** Rejected — prettier doesn't strip preambles, and adding it as a runtime dep widens the security surface.
3. **Make agents resubmit if wrapping detected.** Rejected — wastes a retry; post-processing is free.

## Source

`msp_spec.md` §5.1 (Post-processing).
