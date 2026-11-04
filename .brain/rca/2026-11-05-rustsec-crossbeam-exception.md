# RCA: expired and misclassified RustSec exception

## Symptom

On PR #281 head a86a5952df21bcff23b810b41cfd51cd864c9929, Security Audit run 41880735310/job 126752244393 failed in Validate advisory exception policy. The validator reported that RUSTSEC-2025-0141 and RUSTSEC-2026-0204 had both expired on 2026-10-31. The step exited before cargo audit was invoked, so this run produced no audit result for any lockfile.

## Evidence

- The exception registry classified RUSTSEC-2026-0204 as unmaintained and listed crossbeam-epoch as a transitive dependency.
- The security audit helper reads every registry ID and passes each one as cargo audit --ignore for all three lockfiles; scripts/test_security_audit.py hard-coded those two IDs.
- Root Cargo.lock and genesisblock-mobile/src-tauri/Cargo.lock pin crossbeam-epoch 0.9.18. Studio's lockfile pins 0.9.20.
- The primary RustSec record (https://github.com/RustSec/advisory-db/blob/main/crates/crossbeam-epoch/RUSTSEC-2026-0204.md) describes an invalid pointer dereference in fmt::Pointer for crossbeam-epoch Atomic and Shared and lists >=0.9.20 as patched.
- The primary RustSec record (https://github.com/RustSec/advisory-db/blob/main/crates/bincode/RUSTSEC-2025-0141.md) marks bincode 1.3 as informational/unmaintained and states that its team considers 1.3.3 complete.
- The bincode ADR records the remaining use as a read-only legacy metadata fallback, with postcard writes and an existing removal gate at engine version 0.4.0.

## Root cause

A stale exception entry gave a vulnerability advisory the wrong kind and then caused the audit helper to suppress that advisory across every lockfile, including affected 0.9.18 selections. Separately, the legitimate informational bincode exception was not reviewed before its date expired. The validator correctly stopped the workflow before an audit could create misleading results.

## Why detection failed

The validator checked exception dates and synchronization between the registry and workflow, but did not check advisory classification or affected dependency versions. The audit unit test asserted the same hard-coded ignore list and therefore reinforced the suppression. The review deadline for bincode was not acted on before expiration.

## Corrective and preventive actions

- Remove RUSTSEC-2026-0204 from the exception registry and scheduled workflow ignore list.
- Update the root and mobile lockfiles to crossbeam-epoch 0.9.20 or later; leave Studio unchanged because it is already at 0.9.20.
- Make the audit regression test reject a crossbeam ignore and verify every audited lockfile is on a patched crossbeam-epoch version.
- Retain only the evidence-backed, time-limited bincode informational exception for the read-only fallback; refresh its review date while preserving the 0.4.0 removal gate.
- Require a fresh hosted audit to reach all three cargo audit invocations and pass them before merging. Any remaining audit finding blocks merge and requires its own evidence-backed RCA.

## Limits

No OOM or exploit was observed in this incident. This RCA establishes a policy/lockfile mismatch, not unauthenticated reachability or runtime exploitability. It does not change production Rust behavior, API, storage format, or the approved sidecar cache bound.
