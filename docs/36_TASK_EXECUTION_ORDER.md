# 36 — Task Execution Order (Waves)

> Machine SSOT: `queue/PROJECT_GRAPH.json` (`waves[]`). This doc is the human-readable view.

## Ordering rules
1. **W0 first, strictly ordered** — TASK-0000a MUST run on clean `main`, THEN TASK-0000b, THEN TASK-0000c. Baseline window closes on next merge to main.
2. **W1 depends on W0 exit** — no publish until W0 has landed baselines and P0 fixes.
3. **GATE-DEMAND-1** sits between W1 and W2/W3/W4 — owner decides after ~1 month based on install signal.
4. **Within W1**, TASK-0002/0003 (LOCAL_SAFE, no deps) can be authored in parallel with W0; only TASK-0001+ blocked on TASK-0000c.
5. **W3 does NOT depend on W2** — after the gate they can run in parallel.
6. **W4 further gated** on TASK-0016 verdict (adapter vs translator vs infeasible).

## Wave order
```
W0 (0000a → 0000b → 0000c) ─▶ W1 ─┐
                                   ├─▶ GATE-DEMAND-1 (owner) ─┬─▶ W2
                                   │                          ├─▶ W3
                                   │                          └─▶ W4 (also gated on TASK-0016 verdict)
```

## Parallelizable now
- **W0 critical path:** TASK-0000a — start immediately on clean main.
- **Local track (can run alongside W0, no baseline hazard):** TASK-0002, TASK-0003 (touch package.json/QUICKSTART/SECURITY — unrelated to HQL).
- **Also local-ready but touches later waves:** TASK-0009, TASK-0011, TASK-0012, TASK-0014 — can be authored on side branches; merge waits on gate/dependencies.
- **Cloud/human track:** TASK-0001 — dry-run CI (can proceed but blocked from proceeding to TASK-0004 until TASK-0000c commits).

## Wave exit criteria (short form; full form in `33_TASK_BREAKDOWN.md`)
- **W0:** v1 + post-P0 baselines committed under `benches/baselines/`; P0 fixes merged; no regression >5%.
- **W1:** `npm install @freshair129/gks-genesis-block-native` on a clean machine → smoke passes.
- **W2:** adapter published + `npx` MCP server starts and serves a client.
- **W3:** `docker run` REST server → authed SDK CRUD green.
- **W4:** Graphiti driver conformance + example runs against a running REST server.
