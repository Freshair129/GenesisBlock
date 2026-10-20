# G4 text fixture selected a publication receipt rather than a covered frontier

## Symptom / evidence
First local focused run: EXPLAIN target5/5, P8 completion11/14. New lexical,
Context and output-budget matrices failed with BEYOND_HORIZON/bind/
retention_horizon instead of reaching their intended counters/budget assertion.
Logs are retained in g4-lexical-context-qualification/focused-tests.log.

## Root cause
The new request builders sampled stable_frontier separately for HQL and IR.
On empty storage the sampled0 is below normalize's history_horizon().max(1)
(src/query/hql2/request.rs:168; src/lib.rs:8472). After the first query publishes
a generation, stable_frontier can be its publication receipt, whereas selected
transactions must be inside history_horizon..wal_frontier (src/lib.rs:8766-8778).
Thus later builders could select a receipt outside the covered data interval.
Existing history generation-boundary tests explicitly distinguish these values.
The tests constructed invalid selectors; no lexical/context runtime defect is shown.

## Why detection failed
All-target Clippy checks types, not generation receipt semantics. The fixture
builders were reviewed as pinned selectors without applying the already tested
P6 covered-frontier distinction to empty and successive query cases.

## Correction and prevention
Publish the fixture generation before constructing either frontend request.
Capture its covered wal_frontier once and pass it to both builders. If the empty
generation has no selectable transaction (wal_frontier<history_horizon), omit
explicit tx in both frontends; still verify each mode's pinned snapshot metadata.
Keep counter/P7/error assertions intact. Retain the failed log and rerun focused,
shared-helper/P7 regressions and strict static gates. No runtime or P6 changes.

## Follow-up fixture error: duplicate P7 occurrence keys
Corrected frontier run passed EXPLAIN5/5 and completion12/14. Both remaining
failures originate from independent P7's DuplicateRowKey, not Storage or counters.
The new duplicate-bag adapter cloned Candidate.row_key as well as owner/revision.
P7 explicitly requires unique occurrence row keys (rank_reference.rs:6-7,193-194)
while allowing repeated owner identities. Static compilation and nonduplicate
fixtures did not exercise this adapter precondition. Assign deterministic unique
occurrence keys after cloning while retaining owner/revision/text and multiplicity.
Preserve all semantic and exact-counter assertions; rerun full scoped validation.
