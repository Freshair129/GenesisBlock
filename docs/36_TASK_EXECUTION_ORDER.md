# 36 — Task Execution Order (Waves)

> Machine SSOT: `queue/PROJECT_GRAPH.json` (`waves[]`). This doc is the human-readable view.

## Ordering rules
1. **W1 is unconditional** — publish is nearly free; do it.
2. **GATE-DEMAND-1** sits between W1 and W2/W3/W4 — owner decides after ~1 month based on install signal.
3. **Within W1**, run TASK-0002/0003 (LOCAL_SAFE, no deps) in parallel with TASK-0001 (CLOUD_REQUIRED). Then TASK-0004 (gated on 0001–0003). Then TASK-0005.
4. **W3 does NOT depend on W2** — after the gate they can run in parallel.
5. **W4 further gated** on TASK-0016 verdict (adapter vs translator vs infeasible).

## Wave order
```
W1  ─┐
     ├─▶ GATE-DEMAND-1 (owner) ─┬─▶ W2
     │                          ├─▶ W3
     │                          └─▶ W4 (also gated on TASK-0016 verdict)
```

## Parallelizable now (Wave W1 start)
- **Local track (any LOCAL_SAFE agent):** TASK-0002, TASK-0003.
- **Cloud/human track:** TASK-0001 (needs repo/CI/secrets access).
- **Also local-ready but touches later waves' scope:** TASK-0009, TASK-0011, TASK-0012, TASK-0014 — hold unless there's spare local capacity; they're gated on the demand signal for merge/publish, but their code can be authored now.

## Wave exit criteria (short form; full form in `33_TASK_BREAKDOWN.md`)
- **W1:** `npm install @freshair129/gks-genesis-block-native` on a clean machine → smoke passes.
- **W2:** adapter published + `npx` MCP server starts and serves a client.
- **W3:** `docker run` REST server → authed SDK CRUD green.
- **W4:** Graphiti driver conformance + example runs against a running REST server.
