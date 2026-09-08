---
doc_id: CONTRACT--WAVE-E-CLIENT-CAPABILITY
version: "0.1.0b"
created_at: "2026-09-08T21:44:00+07:00,ATHER"
last_update: "2026-09-08T21:44:00+07:00,ATHER"
status: beta
superseded_by: null
attributes:
  domain: client-capability
  scope: wave-e-r12
---

# Wave E client capability matrix

This matrix records the local implementation slice. It is not a hosted package
or production acceptance claim.

| Surface | Query IR search/traverse | Query IR target-id context | Capabilities | Auth/timeout evidence | Local verification |
|---|---|---|---|---|---|
| Rust core | implemented | implemented | implemented | n/a | Rust integration tests |
| REST | implemented | implemented | implemented | bearer gate + admission | REST integration tests |
| N-API | implemented | implemented | implemented | native caller boundary | NAPI tests |
| C FFI | JSON passthrough | JSON passthrough | JSON passthrough | caller-owned | host compile + parity |
| Android JNI | JSON passthrough | JSON passthrough | JSON passthrough | caller-owned | host compile + parity |
| Python SDK | typed methods | typed methods | typed method | finite timeout + bearer key | mocked client tests |
| Go SDK | typed methods | typed methods | typed method | finite timeout + bearer key | mocked client tests when Go is available |
| MCP | compatibility tools | `query_ir` tool | via N-API | process-local | MCP integration tests |

Explicit boundaries for this matrix:

- typed metadata filters are `unsupported`;
- lexical BM25/RRF fusion is `planned`;
- query-vector and temporal context are `unsupported`;
- HQL remains a compatibility surface and is not a fallback for unsupported
  typed capabilities.

The six Wave D recall-floor findings remain a separate quality-envelope issue;
this matrix does not promote a recall or production-readiness claim.
