#!/usr/bin/env python3
"""cost_ledger.py — aggregate a per-call cost ledger for the tiered-swarm skill.

Thin wrapper over cost_estimate.py: reads a JSONL ledger (one logged call per line),
applies the two-way cost formula per row, and prints a breakdown plus a LOCAL-vs-BILLED
split so the "local saved tokens" claim is auditable (design invariant #4).

Each JSONL row is one call:
    {"tier":"verify","model":"aroow-rust-coder-9b",
     "in_uncached":1843,"in_cached":0,"cache_write":0,"out":412}
Optional per-row "cache_write_ttl": "5min"|"1hr" (default 5min).
Local calls: log the Ollama prompt_eval_count -> in_uncached and eval_count -> out
(scripts/ollama_route.sh prints both); rate is 0.0 but the COUNT must be recorded.

USAGE:
    python cost_ledger.py ledger.jsonl
    cat ledger.jsonl | python cost_ledger.py -

Verify every billed `PRICING` entry against its provider's official source before billing.
The current Claude profile carries a 2026-06-04 UNCERTAINTY FLAG. Dependency-free (stdlib only).
"""

import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import cost_estimate as ce  # noqa: E402


def load_rows(path):
    raw = sys.stdin.read() if path == "-" else open(path, "r", encoding="utf-8").read()
    rows = []
    for line in raw.splitlines():
        line = line.strip()
        if line and not line.startswith("#"):
            rows.append(json.loads(line))
    return rows


def is_local(model):
    """A row counts toward the local split if its rate vector is all-zero."""
    r = ce.rates_for(model)
    return r.get("in", 0) == 0 and r.get("out", 0) == 0


def main(argv):
    if not argv:
        sys.stderr.write(__doc__)
        sys.exit(2)
    rows = load_rows(argv[0])
    if not rows:
        sys.stderr.write("cost_ledger.py: empty ledger.\n")
        sys.exit(2)

    try:
        per_row, total = ce.cost_breakdown(rows)
    except ValueError as exc:
        sys.stderr.write(f"cost_ledger.py: {exc}\n")
        sys.exit(2)
    ce._fmt(per_row, total, "tiered-swarm cost ledger")

    # local vs billed split
    local_tok = billed_tok = 0
    billed_cost = 0.0
    for r in per_row:
        tok = (r.get("in_uncached", 0) + r.get("in_cached", 0)
               + r.get("cache_write", 0) + r.get("out", 0))
        if is_local(r["model"]):
            local_tok += tok
        else:
            billed_tok += tok
            billed_cost += r["cost_usd"]
    all_tok = local_tok + billed_tok or 1
    print("\n--- local vs billed split ---")
    print(f"  local tokens   : {local_tok:>12,}  ({100*local_tok/all_tok:5.1f}%)  cost $0.0000")
    print(f"  billed tokens  : {billed_tok:>12,}  ({100*billed_tok/all_tok:5.1f}%)  "
          f"cost ${billed_cost:.4f}")
    print(f"  TOTAL          : {all_tok:>12,}                 cost ${total:.4f}")
    print("\nREMINDER: verify every billed rate in PRICING against its provider's official source.")


if __name__ == "__main__":
    main(sys.argv[1:])
