# G4 annotation diagnostic fixture omitted Annotation(Read)

## Symptom

PR #251 head `17f456937b8f4606d1cec785a8a2f1bddb8070bf`, macOS job
118419562108 (run 39358720097), failed the new
source_diagnostics_respect_explicit_actor_authority test. The intended allowed
reader returned FORBIDDEN/authorize/access_denied before normal rows.

## Evidence

- Job log lines 1044-1066: two other source tests passed; the actor test failed
  in tests/support/hql2_diagnostics.rs at the normal-query unwrap.
- The new policy grants source-reader Namespace(Read), omitting Annotation(Read).
- src/lib.rs authorize_annotation_namespace_read requires the explicit
  Annotation namespace grant. hql2_scan enforces this for AnnotationScan;
  hql2_annotation_lookup also enforces it for lookup.
- SPEC--GENESISDB-HQL2-P8-TYPED-BOUNDARY records missing Annotation(Read) as
  FORBIDDEN separately from row-bag filtering. This is existing behavior.

## Root cause

The fixture incorrectly treated Namespace(Read) as sufficient for annotation
data reads. The implementation correctly denied access. No runtime defect is
demonstrated. The change is confined to tests and evidence.

## Why detection failed

Static Clippy checks do not execute authorization. Local test execution was
blocked by Windows Application Control (4551), correctly recorded as NOT_RUN.
Hosted macOS provided the first runtime result for this test. Existing peer
tests/contracts showed the required annotation grant but the new fixture did
not reproduce it.

## Correction and prevention

Grant source-reader both Namespace(Read) and Annotation(Read). Keep the
untrusted-reader denial in all modes. Add namespace-only-reader as a negative
control for normal/ANALYZE data execution. Its EXPLAIN remains catalog-only and
may return a plan without actual counters; it does not demonstrate data access.
Keep all exact result/counter assertions and production authorization intact.
Re-run strict static gates and require fresh hosted runtime CI for the amended
head before merge. Do not reuse parent-head CI or claim local runtime success.
