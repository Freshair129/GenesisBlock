# HQL2 fixture provenance

The `blueprint/` directory vendors the owner-provided UEE-HQL2 Blueprint
`0.1.0-proposed` (prepared 2026-09-22). These are language/contract/reference
fixtures, not engine conformance results.

Source: the attachment path recorded in
`docs/adr/ADR--GENESISDB-HQL2-EXECUTION-BOUNDARY.md`.
Source manifest SHA-256:
`AA80EF1ACB238B0EF2F4F37CE78B7BCD2486A8C8EB0228125985C8C22B9C4BB3`.

Vendored here: 35 `.hql` examples, five example JSON payloads, the Lark grammar,
positive/negative syntax manifests and the two original Python semantic oracle
files. Source bytes are normalized to repository newlines when vendored; this
copy is not advertised as byte-identical. The original report files whose
hashes differ from their manifest are deliberately not used as local results.

Python reference helpers retain their original limited scope. P7's Rust
test-only interpreter lives under `tests/support/`, with direct expected
results and no calls to the production engine. Syntax acceptance, reference
semantics and production execution must be reported separately. Fixtures are
repository-local; no test requires the attachment directory at runtime.

Four additional normative query contracts are vendored for the P8 review.
Their original SHA-256 values (before newline normalization) are:

| File under `blueprint/contracts/` | Original SHA-256 |
|---|---|
| query-ir.schema.json | 523703BA41BC1A906CBDF7614ADFA1350F755D9AF3241A5B38828164D832B1F1 |
| query-request.schema.json | DE0A978FDDC97B29745154DDCBB226622C503CC5D326418F03AE18F5B963D053 |
| query-result.schema.json | A0FD56A9EB3734999E27DD14507D513F69AC2F0CBF5B72E92E4461B00900CB90 |
| error.schema.json | 53440EAC0B42BD36E6E0355F85BCC5781A01DF6E3524176949F2F65EA506037E |
