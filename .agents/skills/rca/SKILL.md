---
name: rca
description: Run a Root Cause Analysis before fixing a bug, and record it under .brain/rca/ — the default debugging workflow mandated by AGENT.md (Documentation-Driven Development + RCA). Use when the user reports a bug, a perf regression, a crash/OOM, a flaky test, or says "why is this happening", "find the root cause", "debug this", "RCA this", before writing the fix.
---

# Root Cause Analysis (RCA)

`AGENT.md` makes RCA the **default mode** for bugs: *"identify root cause with
evidence and document RCA in `.brain/rca/` before fixing."* This skill makes that
prescriptive. The goal is a fix grounded in a proven cause, not a patched symptom.

Do **not** write the fix until the root cause is evidenced. Run in order.

## 1. Capture the symptom (measured, not described)

Pin down what is actually observed, with numbers and a reproduction:
- The exact failure: error text / stack trace / the metric and its delta
  (e.g. "query P95 1.13 → 6.31 ms under concurrent ingest"), not "it's slow".
- The smallest reproduction — which harness / test / command triggers it.
  Prefer an existing harness (`shadow-sync-stress`, `hql-query-stress`, a
  `tests/*.rs` case) so the symptom is re-measurable.
- **When debugging a Rust crash via PowerShell, do NOT suppress stderr**
  (`2>$null` / `2>&1`) — it hides the real error. A past HNSW OOM
  (`memory allocation of … bytes failed`) was masked for several attempts this way.

## 2. Trace to the root cause (with file:line evidence)

Follow the call path in `src/lib.rs` and cite it. A root cause is only confirmed
when you can point at the code and explain the mechanism:
- Walk the hot path (e.g. `add_node → add_vector_internal → insert_one → hnsw.insert`)
  and identify *where* the cost / contention / incorrectness originates.
- Distinguish **storage / correctness** from **derived structures**. The engine is
  bitemporal + append-mostly with async-rebuildable indexes — a symptom on the
  HNSW/index path is often decoupled from durability (WAL carries the truth).
- Rule out the obvious-but-wrong explanation explicitly. State what you tested.

## 3. Write the RCA doc → `.brain/rca/`

Create `.brain/rca/RCA--<SHORT-SLUG>.md`. Match the style of the most recent
`.brain/rca/RCA--*.md`. Required sections:
- **Status / Date** (`confirmed` only when evidence supports it; else `suspected`).
- **Symptom** — the measured observation + reproduction from step 1.
- **Root cause** — the mechanism, with `src/lib.rs:<line>` citations.
- **Fix (decided)** — the chosen change and *why it addresses the cause*, not the
  symptom. Reference an `ADR--*` if the fix is an architectural decision.
- **Outcome (measured)** — fill in AFTER the fix lands: the same metric, re-measured.

## 4. Implement the fix, then close the loop

- Apply the fix per the RCA's "Fix (decided)" section.
- Re-run the reproduction from step 1 and record the **measured** outcome back into
  the RCA doc. A regression fix without a re-measured number is not done.
- For perf/storage/index/HQL changes, validate with the `run-bench-audit` skill
  before claiming "no regression".

## 5. Persist what was learned

- If the cause revealed a non-obvious gotcha or a corrected wrong assumption, add it
  to `.brain/memory/SELF-NOTE--*.md` (and `MEMORY.md` index if used) so it is not
  rediscovered next session — see the `end-session` skill.

## Notes

- Be honest in the record: a `suspected` cause stays `suspected` until evidenced.
  An RCA that documents a dead-end is more useful than one that looks tidy.
- DDD pairs with this: for non-trivial fixes, the doc precedes the code.
