---
status: historical
---

# AUDIT--HQL-FUZZ: HQL Parser Fuzz / Property-Based Tests

**Date:** 2026-06-28
**Suite:** `tests/hql_fuzz_tests.rs`
**Result:** 34/34 PASS — zero panics across all input categories

## Motivation

The HQL parser (`src/query/hql.pest` + `src/query/ast.rs`) is the only
user-facing text→command boundary in GenesisBlockDB. A panic or hang in the
parser is a denial-of-service vector in any deployment that accepts untrusted
queries (REST API, MCP server, SDK clients).

These tests are a portable alternative to `cargo-fuzz` (which requires nightly
+ libFuzzer on Linux). They run as normal `cargo test` integration tests on
every platform and exercise the parser against ~5,000+ distinct inputs.

## Test Categories

| # | Category | Tests | Inputs | Purpose |
|---|----------|-------|--------|---------|
| 1 | Pure random/garbage | 4 | ~700 | Empty, single chars, random ASCII, random bytes |
| 2 | Unicode stress | 1 | 8 | Thai, emoji, BOM, null bytes, RTL override, fullwidth |
| 3 | Mutated valid queries | 5 | ~200 | Truncation, keyword case, SQL injection, doubled keywords, missing clauses |
| 4 | Whitespace variants | 1 | 5 | Tabs, newlines, CRLF, leading/trailing |
| 5 | Boundary values | 7 | ~30 | Extreme K/depth/alpha/budget, 1000-dim vector, 10k-char identifiers |
| 6 | Tier validation | 2 | 13 | All valid H0–H5 + invalid tiers |
| 7 | Fuzzy prefix | 1 | 5 | `~` prefix edge cases |
| 8 | Optional clause combos | 5 | 7 | All permutations of LANGUAGE/IN/AS OF/BUDGET |
| 9 | Round-trip assertions | 4 | 4 | Parsed fields match expected values |
| 10 | Pseudo-random mutation | 2 | ~4,000 | 500 random byte strings + single-byte mutations of all valid queries |

## Key Findings

### Parser is robust
- **Zero panics** across all tested inputs including null bytes, 10k-char identifiers,
  1000-dimensional vectors, and single-byte mutations of every valid query position.
- The `pest` grammar + `unwrap_or()` fallbacks in `ast.rs` handle malformed input
  gracefully — invalid queries return `Err(String)`, never panic.
- SQL injection attempts are rejected by the grammar (no semicolons, comments, or
  SQL keywords in the grammar rules).

### Noted quirk
- `TRAVERSE FROM seed_node DEPTH 3 REL KNOWS`: depth parses as 1 (the default)
  rather than 3. The `depth = { ASCII_DIGIT+ }` rule may interact with pest's
  implicit whitespace handling in a surprising way. Not a crash, but worth
  investigating for correctness.

## Run Command

```bash
cargo test --no-default-features --test hql_fuzz_tests
```

## Future: cargo-fuzz target

For continuous fuzzing on Linux CI, a `cargo-fuzz` target can be added under
`fuzz/fuzz_targets/` using the same `HqlCommand::try_from()` entry point.
The portable tests above provide baseline coverage on all platforms.
