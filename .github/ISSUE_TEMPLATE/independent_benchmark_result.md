---
name: Independent Benchmark Result
about: Submit a reproduced GenesisBlockDB benchmark run for community verification
title: "[bench] <benchmark_id> on <your machine> @ <short-commit>"
labels: ["benchmark", "independent-result"]
assignees: []
---

<!--
Thank you for independently reproducing a benchmark! Please run on a CLEAN tree
(no uncommitted changes) so your result maps to a known commit, and verify it
locally before submitting:

    python benchmark/verify_report.py <run-dir>/result.json   # must print PASS

See BENCHMARKING.md for the official commands.
-->

### Confirmation

- [ ] I am **not** the project maintainer (this is an independent reproduction).
- [ ] I ran `python benchmark/verify_report.py <dir>/result.json` and it printed `PASS`.
- [ ] I am attaching `result.json` and `raw.log` from the run directory.

### Did you modify the repo?

- [ ] No — I ran an unmodified clone.
- [ ] Yes — I modified the repo (describe what and why below). _Note: modified
      runs cannot reach Level 2 community-reproduced status._

### Run details

| Field | Value |
|-------|-------|
| Benchmark (`benchmark_id`) | <!-- e.g. soak_heavy_12h / soak_smoke / graph_traversal / vector_search --> |
| Commit hash | <!-- `git rev-parse HEAD` --> |
| OS | <!-- e.g. Ubuntu 24.04 / Windows 11 / macOS 14 --> |
| CPU | <!-- e.g. Ryzen 9 5950X (16C/32T) --> |
| RAM | <!-- e.g. 64 GB --> |
| Disk | <!-- model + SSD/NVMe/HDD + free space, e.g. Samsung 990 Pro NVMe, 500 GB free --> |
| Rust version | <!-- `rustc --version` --> |
| Command used | <!-- exact command, e.g. `bash benchmark/run_soak_12h.sh` --> |
| PASS / FAIL | <!-- verifier result --> |

### Attachments

- [ ] `result.json` attached
- [ ] `raw.log` attached
- [ ] `env.json` attached (optional but helpful)

### Notes / observations

<!--
Anything notable: thermal throttling, background load, disk type quirks,
unexpected latency spikes, deviations from the documented numbers, etc.
Be specific — this is what makes an independent result valuable.
-->
