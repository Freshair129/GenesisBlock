# Benchmark CI / Runner Plan

How the Independent Benchmark Suite maps onto CI, and why the long soak must not
run on a GitHub-hosted runner.

## Two tiers

| Tier | Runner | Benchmarks | Trigger |
|------|--------|-----------|---------|
| Fast | GitHub-hosted (`ubuntu-latest`, `windows-latest`) | smoke soak, graph, vector (small sizes) | every PR + `workflow_dispatch` |
| Long | **self-hosted** (`self-hosted, soak`) | 1h / 12h soak, large graph/vector | `workflow_dispatch` only |

## Why the 12h soak can't use a GitHub-hosted runner

- **Job time limit.** GitHub-hosted jobs are capped at **6 hours** — a 12h soak
  cannot finish, full stop.
- **No persistent fast disk.** A 12h soak ingests tens of millions of nodes and
  needs 50+ GB on a fast local disk. Hosted runners give you a smallish,
  network-backed, ephemeral disk shared with the OS.
- **Noisy neighbours.** Hosted runners are virtualized and share I/O; latency
  percentiles (especially p99) are meaningless under that jitter.
- **Cost / fairness.** A 12h job blocks a runner for half a day.

So: the smoke benchmark gates PRs; the long soak runs on hardware you control.

## Setting up a self-hosted runner

1. Provision a box you can leave running for 12h+ with **50+ GB free** on a fast
   SSD/NVMe and a stable power profile (disable sleep/throttling).
2. Install the toolchain: Rust stable 1.96+, Python 3.8+, git.
3. Register it as a repository self-hosted runner (Settings → Actions → Runners →
   New self-hosted runner) and add the labels `self-hosted` and `soak`.
4. Optionally set `SOAK_TMPDIR` on the runner to point the database at the fast
   disk (e.g. `/mnt/nvme/gsoak`).

## Triggering

```bash
# smoke (hosted) runs automatically on PRs; to run on demand:
gh workflow run independent-benchmark.yml -f benchmark=smoke

# long soak (self-hosted only):
gh workflow run independent-benchmark.yml -f benchmark=soak_12h
gh workflow run independent-benchmark.yml -f benchmark=soak_1h
```

The workflow guards the long soak behind `runs-on: [self-hosted, soak]`, so if no
such runner is online the job simply queues — it never silently falls back to a
hosted runner.

## Storing artifacts

Each job uploads the **entire run directory** (`benchmark/results/<id>/<run>/`)
as a workflow artifact: `result.json`, `raw.log`, `stderr.log`, `env.json`,
`summary.md`. Retention is set to 90 days for soak artifacts. The verifier runs
as the final step, so a failed/incomplete report fails the job (the artifact is
still uploaded for inspection).

## Relationship to existing workflows

This suite adds `independent-benchmark.yml`. It is intentionally separate from the
pre-existing `benchmarks.yml` (per-PR scientific audit) and `bench-manual.yml`
(manual soak/criterion/audit) so those are untouched. The new workflow is the
*reproducible, schema-verified* path; the older ones remain as internal audits.
