# RCA: annotation ACL fixture depends on an unrelated query budget

## Symptom and evidence

PR #247 Windows job `117894437637` in run `39201232913`, head `3372c1d`,
fails three annotation ACL tests at `tests/hql2_annotation_acl_tests.rs:126`.
All three failures are `QUERY_BUDGET_EXCEEDED / execute / query_limit` from
the shared `database_id` setup helper. The target finishes 9 passed / 3 failed.
The same target passes all 12 cases locally within the 547-test regression.
The generic error does not identify the budget dimension; runner contention
or elapsed-time exhaustion is a hypothesis, not a reproduced root cause.

## Immediate cause and contract evidence

The fixture executes a complete budgeted Values query solely to obtain the
database identity before exercising annotation behavior. Resource exhaustion
in that setup dependency prevents the annotation assertions from running.
The P8 typed boundary already defines database identity as lowercase SHA-256
of `genesis.api.v2.database:` followed by persisted public verifying-key bytes
(sections defining RecordRefV2 and the snapshot identity). The engine's private
`database_id_for_verifying_key` implements that contract, and `local_identity`
already exposes the same public bytes as hex. No private key is needed.

## Why detection failed

Fast isolated/local runs did not exhaust the setup query's budget. Historical
Windows budget failures were not enough to make fixture metadata independent
of query scheduling. The production budget correctly fails closed; changing
that limit would not address this unnecessary fixture dependency.

## Remediation and prevention

Compute the fixture's expected database ID from the existing public identity
contract. Preserve all annotation authorization, lineage, revision, persistence
and negative assertions. Production query code, budget limits, CI scheduling
and the substantive test expectations stay as defined by their current
contracts. Keep the original failing job log; require fresh focused tests and
green hosted checks at the updated PR heads.

## Validation

The focused annotation ACL (12), annotation lookup (6) and G4 contract (2)
targets pass 20/20 after the fixture change. Strict all-target Clippy passes
in both core and default/NAPI configurations; formatting and diff checks pass.
Production source and build inputs are unchanged from the earlier successful
default/server builds and 547-test regression. Fresh hosted qualification at
the updated heads is required. The initial hosted failure remains recorded
even if later checks pass.
