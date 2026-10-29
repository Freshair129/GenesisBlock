# H1-T: gossip datagram capacity investigation

Status: diagnostic stage, HIGH/C-3, owner-approved H1-T ceiling15 files. No capacity repair or macOS acceptance claimed yet.

## Symptom and preserved evidence

PR278 head15574fcd8eef83faa1a08e31938c9ade05abdabf failed actual macOS Rust job123693025312, run40943595313. `peer_complete_prefix_budget_and_retry_boundaries` passed filtered-only/trailing/same-sequence controls, then timed out on the first required prefix-budget response containing a30,000-byte signed event. Its unchanged three-second guard returned None and the required reply unwrap failed. The other13 peer tests, including nested graph/reopen, passed. Later54KB/55KB cases were not reached on that runner.

Raw evidence: external p6-h1-boundary-qualification-20261029/ci/FAIL-MACOS-JOB-123693025312.log, SHA256 a595272695944c382de0f6eaa960524e04b24fa96a6993764d539afb4a930ded; CI-MACOS-FAILURE.json and CI-MACOS-FAILURE-INVESTIGATION.md retain runner/job/source details. The graph RCA and old local qualification remain separate, historical evidence.

## Confirmed source facts and missing evidence

`Storage::start_gossip_manager` binds the Tokio socket successfully before publishing gossip_port. The failing case logged a bound port; a pre-bind readiness race is not supported. The socket has no explicit send-capacity setup. The authenticated PushDelta send result was discarded, so the original timeout cannot establish an actual OS errno, effective SO_SNDBUF or exact encoded wrapper length.

Leading hypothesis: an atomic datagram larger than the default Darwin source socket send capacity is rejected by the OS, and discarding that error causes the requester timeout. This is not yet a confirmed runner diagnosis. [Apple XNU atomic send handling](https://github.com/apple-oss-distributions/xnu/blob/main/bsd/kern/uipc_socket.c#L1824-L1826) and [historical UDP allocation](https://github.com/apple-oss-distributions/xnu/blob/f6217f891ac0bb64f3d375211650a4c1ff8ca1ea/bsd/netinet/udp_usrreq.c#L2221-L2286) support the mechanism, not the actual runner capacity.

## Diagnostic design before behavioral repair

Promote the already-locked socket2=0.6.4 to a direct dependency without resolution churn. Borrow the actual bound Tokio socket through safe SockRef and report effective capacity. Report authenticated PushDelta encoded length, actual send result and raw OS error without payload, keys or signatures. Add a controlled default-capacity numeric-loopback probe with128/30,000/55,000/65,507 byte sends; successful sends must roundtrip exact bytes, large-send errors are observations. The original required prefix reply and all budget/state/receipt assertions remain unchanged. No setsockopt/capacity setter occurs in this diagnostic candidate.

Actual macOS source capacity, encoded length, send errno and probe evidence must be frozen before any behavioral correction. Different cause requires a different concrete scope; no guessed fix. If insufficient capacity is confirmed, the approved next stage sets and verifies sufficient per-socket capacity before readiness, without global sysctl or protocol changes.

## Why earlier qualification missed it

Author and independent execution were on the same Windows host and did not measure Darwin socket defaults. Their graph and finite P6 results are valid at their recorded bytes but do not imply macOS transport acceptance. The original code hid send failures. Fresh cross-platform CI found the gap; no timeout increase, size reduction, skip or blind rerun is an acceptable substitute.

## Prevention and remaining gates

Preserve original54,000/54,001 event and55,000/55,001 vector boundaries, prefix continuation/retry and bounded oversized-group controls. Require actual macOS RED→GREEN after evidence-supported repair, fresh current-source local/independent execution and whole-branch review, and all applicable new-head CI. A Cargo change activates the additional non-publication Release PR path; release/tag/manual-only publication jobs remain excluded and unrun.

Broad P6/G4/P8/H2-H4/P9-P16, non-graph composition correctness, direct-Edge historical replay, oversized positive-group sync liveness, universal no-write, other-machine/new-oracle/performance/device/release gates remain open. Devices are unavailable. No customer database repair or broad closure claim.
