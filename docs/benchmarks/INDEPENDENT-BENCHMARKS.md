# Independent Benchmarks — Status & Credibility Levels

This page tracks how much each GenesisBlockDB performance claim has been
independently verified. It exists to keep claims honest: an internal audit on the
maintainer's machine is **not** the same as a result reproduced by strangers on
their own hardware, and this page makes that distinction explicit.

## Credibility levels

| Level | Name | What it means | Evidence required |
|-------|------|---------------|-------------------|
| **0** | Internal audit | Maintainer ran a benchmark on one machine; numbers live in `docs/AUDIT--*.md`. Useful signal, but unverified by anyone else. | A maintainer audit doc. |
| **1** | Reproducible benchmark | A fixed command in this repo produces a schema-checked `result.json` + raw logs that anyone *could* re-run. | `BENCHMARKING.md` command + `verify_report.py` PASS, committed by maintainer. |
| **2** | Community reproduced | At least one **external** person (not the maintainer) ran the official command on their own hardware and submitted a verified `result.json` + `raw.log`. | An accepted issue via the Independent Benchmark template, listed below. |
| **3** | External suite / third-party reviewed | An independent party audited the methodology and/or ran the suite as part of their own published comparison. | A linked third-party write-up. |

> **Where things stand today:** the soak/graph/vector benchmarks are **Level 1**
> (reproducible: official commands + a verifier exist). They reach **Level 2**
> only when external testers submit verified results — see "How to get to
> Level 2" below. No Level 3 claims exist.

## Current status

| Benchmark | Highest level reached | Notes |
|-----------|----------------------|-------|
| Short soak smoke | Level 1 | `benchmark/run_smoke.*` |
| 1h soak | Level 1 | `benchmark/run_soak_12h.*` with `SOAK_DURATION_SEC=3600` |
| 12h soak | Level 1 | `benchmark/run_soak_12h.*`; internal audit context in `docs/AUDIT--SOAK-TEST.md` (Level 0, light/medium profiles) |
| Graph traversal | Level 1 | `benchmark/run_graph_bench.*` |
| Vector search (recall@k) | Level 1 | `benchmark/run_vector_bench.*` |

## Submitted results

Community-reproduced runs are listed here once their issue is accepted. **Do not
add a row without an attached, verifier-PASS `result.json` from the submitter.**

| Date | Tester | Commit | Machine | Benchmark | Result | Artifacts |
| ---- | ------ | ------ | ------- | --------- | ------ | --------- |
| _(example — not a real result)_ | `EXAMPLE-tester` | `0123456` | Ryzen 9 5950X / 64 GB / NVMe / Linux | `soak_heavy_12h` | _illustrative only_ | _n/a_ |

> The single row above is a **clearly-marked example** showing the expected
> format. It is not a real benchmark result. Real rows are added only from
> accepted submissions with attached artifacts.

## How to get to Level 2 (community reproduction)

1. Pick a benchmark and run the official command from
   [`../../BENCHMARKING.md`](../../BENCHMARKING.md) on a **clean tree**.
2. Verify locally: `python benchmark/verify_report.py <dir>/result.json` → `PASS`.
3. Open an issue with the **Independent Benchmark Result** template and attach
   `result.json` + `raw.log`.
4. A maintainer confirms the artifacts verify and adds a row above with a link to
   the issue.

## Methodology guardrails

- Results come from the engine's own emitted metrics + host-captured environment;
  the benchmark code never self-reports its own hardware (`assemble_result.py`
  fills commit/dirty/env separately).
- The verifier rejects dirty trees, missing commits, failed/interrupted runs, and
  short 12h soaks — so a listed result maps to a real commit and a complete run.
- Embedded vs server, post-process filtering, OS noise, and disk differences are
  documented as caveats in `BENCHMARKING.md` §9 and must not be elided when
  quoting a number.
