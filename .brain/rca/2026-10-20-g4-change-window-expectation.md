# G4 ChangeScan window fixture counted operations rather than revisions

## Symptom and evidence

The first local focused run for the approved history/change slice executed
successfully on this host, but change_scan_matches_p7_for_exclusive_after_inclusive_through_windows
failed its newly added exact source rows assertion. The correction window
expected one emitted row while the source reported two. All 20 other tests
in the History/Change target passed; Annotation ACL 12/12, Annotation source
7/7 and EXPLAIN source 4/4 also passed. The Row target was not reached.

## Root cause

The new manual expectation counted one supersede operation as one revision
event. The existing fixture and its P7 catalog describe one insert revision,
two supersede revisions (the closed old Node revision and the new Node
revision), then one retract revision. Therefore the five named windows have
1/2/3/1/0 events. The existing semantic P7 bag is the independent expected
multiset; no runtime or source counter defect is demonstrated.

## Why detection failed

Static Clippy checks cannot validate fixture cardinality. The new budget case
already documented the two-revision supersede rule, but the separate window
count table did not apply it consistently. Earlier runtime policy blocking
was historical; this local focused attempt actually executed, so it is a
failed run rather than NOT_RUN.

## Correction and prevention

Fix the new manual counts, assert their equality to the sum of independent
P7 bag multiplicities, then compare emitted source counters to that oracle
sum. Keep all original semantic, transaction-boundary and error assertions.
Re-run focused plus shared-helper regression targets and strict static gates.
Do not derive the expected count from observed diagnostic output, and do not
change production supersede, temporal, authorization or counter behavior.
