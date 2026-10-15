# RCA: tiered-swarm Model ID Configuration and Pricing Fallback

## Status / Date

- Status: confirmed
- Date: 2026-10-08

## Symptom

The `.agents` skill advertised invalid `Codex-sonnet-4-6` and `Codex-opus-4-8` identifiers,
while the router independently hard-coded Claude model IDs. The estimator also treated an
unlisted `:cloud` model as a guessed rate and any other unlisted model as local cost $0.
Consequently, host dispatch could disagree with the selected model and a billed model ID
could be recorded with a false price.

## Evidence

- At pre-fix revision `3a1af93`, `.agents/skills/tiered-swarm/SKILL.md:67-68` contained the
  `Codex-*` IDs, while `.agents/skills/tiered-swarm/scripts/route.py:42-43` assigned T2/T3
  to `claude-sonnet-4-6` and `claude-opus-4-8`.
- At that revision, `.agents/skills/tiered-swarm/scripts/cost_estimate.py:63,81-94` had a
  generic `__local__` zero-rate entry, guessed a rate for unknown `:cloud` tags, and sent
  other unknown IDs to the zero-rate entry.
- The initial regression-test run reproduced the defect: 5 tests produced 14 expected
  failures for ignored T2/T3 environment values and silent fallback pricing.
- After the fix, the router resolves T2/T3 through
  `.claude/skills/tiered-swarm/scripts/route.py:48-50,93-104`; unknown IDs raise in
  `scripts/cost_estimate.py:65-75`; and the ledger exits 2 on that error in
  `scripts/cost_ledger.py:56-60`.

## Root Cause

Tier-to-model routing was encoded as provider-specific constants in the router and copied
into host-specific skill text. The model-pricing lookup used name-pattern heuristics and a
generic local-zero fallback instead of treating `PRICING` as an exact allowlist. The
provider-specific copies were not covered by tests that exercised arbitrary configured IDs
and arbitrary unknown billed IDs.

## Why the Issue Escaped Detection

Existing cost checks covered a known Claude price and a misspelled Claude tag, but not an
unlisted billed ID with a different provider name or an unknown cloud tag. Existing agent
validation checked the agent registry and policy mirrors, not tiered-swarm model routing.
The skill copies could therefore contain inconsistent IDs without a failing check.

## Fix (Decided)

- Resolve T2 and T3 from `TIERED_SWARM_T2_MODEL_ID` and
  `TIERED_SWARM_T3_MODEL_ID`; provide no default IDs and exit 2 when a routed tier is
  unconfigured.
- Accept only exact model IDs present in `PRICING`; keep zero rates only for explicitly
  listed local models and propagate unknown-ID errors through the ledger CLI.
- Update the shared skill, specification, blueprint, and references in both host copies;
  retain only host-native dispatch wording in `SKILL.md`.
- Add regression coverage for both copies, configured T2/T3 CLI output, missing config,
  local routing, and unknown-price CLI failure.

## Outcome (Measured)

- Both skill-copy regression suites passed: 8/8 tests each.
- `npm run agents:validate` passed: 6 agents, 12 routes; shared policy mirrors matched.
- `scripts/validate_doc_status.py docs` passed directly with the bundled Python runtime:
  0 violations in 241 files. The `npm run docs:validate` launcher first emitted a warning
  because `py.exe` found no installed Python; the direct validator command produced the
  measured PASS result.
- Shared content parity passed across 11 paired skill files; `git diff --check` passed.
- No rate values or cost-savings claims were changed in this slice. Separate cost-evidence
  and hosted-review findings remain outside its scope.

## Proposed Prevention

- Keep T2/T3 IDs configurable and test both host copies with sentinel model IDs.
- Require exact, sourced `PRICING` entries; never infer billed versus local status from an
  ID prefix or suffix.
- Keep the regression suite in both skill copies and include it in their reproduction spec.
