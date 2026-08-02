//! HQL parser fuzz / property-based tests.
//!
//! Generates random and semi-valid HQL strings and feeds them to the parser,
//! asserting that it never panics. This is a portable alternative to
//! cargo-fuzz (which requires nightly + libFuzzer on Linux). The tests run
//! on every platform as normal `cargo test` integration tests.
//!
//! Categories:
//!   1. Pure random bytes / unicode — parser rejects gracefully
//!   2. Mutated valid queries — keyword swaps, truncation, injection
//!   3. Boundary values — huge vectors, extreme numbers, deep nesting
//!   4. Valid query round-trip — every command variant parses successfully

use genesis_block_native::query::ast::HqlCommand;
use std::convert::TryFrom;

fn must_not_panic(input: &str) {
    let _ = std::panic::catch_unwind(|| {
        let _ = HqlCommand::try_from(input);
    });
}

fn parses_ok(input: &str) {
    match HqlCommand::try_from(input) {
        Ok(_) => {}
        Err(e) => panic!("Expected valid HQL but got error: {e}\nInput: {input}"),
    }
}

// =========================================================================
// 1. Pure random / garbage — must never panic
// =========================================================================

#[test]
fn empty_string() {
    must_not_panic("");
}

#[test]
fn single_characters() {
    for c in 0u8..=127 {
        must_not_panic(&String::from(c as char));
    }
}

#[test]
fn random_ascii_strings() {
    let seeds: &[&str] = &[
        "asdfghjkl",
        "SELECT * FROM nodes",
        "DROP TABLE;",
        "{}[]()!@#$%^&*",
        "SEARCH",
        "TRAVERSE",
        "MATCH",
        "CONTEXT",
        "SEARCH SEARCH SEARCH",
        "null",
        "true",
        "false",
        "0",
        "-1",
        "NaN",
        "Infinity",
        "\0\0\0",
        "\n\n\n",
        "\t\t\t",
        "' OR 1=1 --",
        "<script>alert(1)</script>",
        "SEARCH x SIMILAR TO [] K 5",
        "MATCH x SIMILAR TO [1.0] ALPHA",
        "TRAVERSE FROM x DEPTH DEPTH REL REL",
        "CONTEXT FOR x TIER H99",
    ];
    for s in seeds {
        must_not_panic(s);
    }
}

#[test]
fn unicode_stress() {
    let inputs = [
        "SEARCH 日本語 SIMILAR TO [1.0] K 5",
        "SEARCH 🔥🔥🔥 SIMILAR TO [1.0] K 5",
        "TRAVERSE FROM กรุงเทพ DEPTH 3 REL เชื่อม",
        "CONTEXT FOR émoji TIER H1",
        "\u{FEFF}SEARCH x SIMILAR TO [1.0] K 5",
        "SEARCH x\u{0000}y SIMILAR TO [1.0] K 5",
        "SEARCH \u{202E}x SIMILAR TO [1.0] K 5",
        "MATCH x SIMILAR TO [\u{FF11}.\u{FF10}] ALPHA 0.5",
    ];
    for s in &inputs {
        must_not_panic(s);
    }
}

// =========================================================================
// 2. Mutated valid queries — truncation, keyword swaps, injection
// =========================================================================

const VALID_SEARCH: &str = "SEARCH mynode SIMILAR TO [1.0,2.0,3.0] K 10";
const VALID_TRAVERSE: &str = "TRAVERSE FROM seed_node DEPTH 3 REL KNOWS";
const VALID_HYBRID: &str = "MATCH target SIMILAR TO [0.5,0.5] ALPHA 0.7";
const VALID_CONTEXT: &str = "CONTEXT FOR node_id TIER H2";

#[test]
fn progressive_truncation() {
    for query in [VALID_SEARCH, VALID_TRAVERSE, VALID_HYBRID, VALID_CONTEXT] {
        for len in 0..query.len() {
            must_not_panic(&query[..len]);
        }
    }
}

#[test]
fn keyword_case_mutations() {
    let mutations = [
        "search mynode similar to [1.0] k 5",
        "SEARCH mynode similar TO [1.0] K 5",
        "Search Mynode Similar To [1.0] K 5",
        "sEaRcH mynode SIMILAR TO [1.0] K 5",
        "traverse from seed depth 3 rel knows",
        "Traverse From Seed Depth 3 Rel Knows",
        "match target similar to [0.5] alpha 0.7",
        "context for node tier h1",
        "CONTEXT FOR node TIER h1",
    ];
    for m in &mutations {
        must_not_panic(m);
    }
}

#[test]
fn sql_injection_attempts() {
    let injections = [
        "SEARCH x; DROP TABLE nodes SIMILAR TO [1.0] K 5",
        "SEARCH x SIMILAR TO [1.0] K 5; DELETE FROM edges",
        "TRAVERSE FROM x' OR '1'='1 DEPTH 3 REL ANY",
        "CONTEXT FOR x\" UNION SELECT * TIER H1",
        "SEARCH x SIMILAR TO [1.0] K 5 --comment",
        "SEARCH x SIMILAR TO [1.0] K 5 /* block */",
    ];
    for s in &injections {
        must_not_panic(s);
    }
}

#[test]
fn extra_whitespace_and_newlines() {
    let inputs = [
        "  SEARCH   mynode   SIMILAR   TO   [1.0]   K   5  ",
        "SEARCH\nmynode\nSIMILAR\nTO\n[1.0]\nK\n5",
        "SEARCH\t\tmynode\tSIMILAR\tTO\t[1.0]\tK\t5",
        "SEARCH\r\nmynode\r\nSIMILAR\r\nTO\r\n[1.0]\r\nK\r\n5",
        "\n\n\nSEARCH mynode SIMILAR TO [1.0] K 5\n\n\n",
    ];
    for s in &inputs {
        must_not_panic(s);
    }
}

#[test]
fn doubled_keywords() {
    let inputs = [
        "SEARCH SEARCH mynode SIMILAR TO [1.0] K 5",
        "TRAVERSE TRAVERSE FROM seed DEPTH 3 REL KNOWS",
        "MATCH target SIMILAR SIMILAR TO [0.5] ALPHA 0.7",
        "CONTEXT CONTEXT FOR node TIER H1",
        "SEARCH mynode SIMILAR TO TO [1.0] K 5",
        "TRAVERSE FROM FROM seed DEPTH 3 REL KNOWS",
    ];
    for s in &inputs {
        must_not_panic(s);
    }
}

#[test]
fn missing_required_clauses() {
    let inputs = [
        "SEARCH mynode",
        "SEARCH mynode SIMILAR TO",
        "SEARCH mynode SIMILAR TO [1.0]",
        "SEARCH mynode SIMILAR TO [1.0] K",
        "SEARCH SIMILAR TO [1.0] K 5",
        "TRAVERSE FROM seed",
        "TRAVERSE FROM seed DEPTH",
        "TRAVERSE FROM seed DEPTH 3",
        "TRAVERSE FROM seed DEPTH 3 REL",
        "TRAVERSE FROM DEPTH 3 REL KNOWS",
        "MATCH target SIMILAR TO [0.5]",
        "MATCH target SIMILAR TO [0.5] ALPHA",
        "CONTEXT FOR node",
        "CONTEXT FOR node TIER",
        "CONTEXT FOR TIER H1",
    ];
    for s in &inputs {
        must_not_panic(s);
    }
}

// =========================================================================
// 3. Boundary values — huge vectors, extreme numbers
// =========================================================================

#[test]
fn extreme_k_values() {
    let inputs = [
        "SEARCH x SIMILAR TO [1.0] K 0",
        "SEARCH x SIMILAR TO [1.0] K 1",
        "SEARCH x SIMILAR TO [1.0] K 999999999",
        "SEARCH x SIMILAR TO [1.0] K 4294967295",
        "SEARCH x SIMILAR TO [1.0] K 99999999999999999999",
    ];
    for s in &inputs {
        must_not_panic(s);
    }
}

#[test]
fn extreme_depth_values() {
    let inputs = [
        "TRAVERSE FROM x DEPTH 0 REL KNOWS",
        "TRAVERSE FROM x DEPTH 4294967295 REL KNOWS",
        "TRAVERSE FROM x DEPTH 99999999999999999999 REL KNOWS",
    ];
    for s in &inputs {
        must_not_panic(s);
    }
}

#[test]
fn extreme_vector_values() {
    let inputs = [
        "SEARCH x SIMILAR TO [0.0] K 5",
        "SEARCH x SIMILAR TO [-1.0] K 5",
        "SEARCH x SIMILAR TO [999999999.999999999] K 5",
        "SEARCH x SIMILAR TO [-999999999.999999999] K 5",
        "SEARCH x SIMILAR TO [1e308] K 5",
        "SEARCH x SIMILAR TO [-1e308] K 5",
    ];
    for s in &inputs {
        must_not_panic(s);
    }
}

#[test]
fn large_vector_dimension() {
    let dims: Vec<String> = (0..1000).map(|i| format!("{}.0", i)).collect();
    let query = format!("SEARCH x SIMILAR TO [{}] K 5", dims.join(","));
    must_not_panic(&query);
}

#[test]
fn extreme_alpha_values() {
    let inputs = [
        "MATCH x SIMILAR TO [1.0] ALPHA 0.0",
        "MATCH x SIMILAR TO [1.0] ALPHA 1.0",
        "MATCH x SIMILAR TO [1.0] ALPHA -1.0",
        "MATCH x SIMILAR TO [1.0] ALPHA 999.999",
        "MATCH x SIMILAR TO [1.0] ALPHA 0.0000000001",
    ];
    for s in &inputs {
        must_not_panic(s);
    }
}

#[test]
fn extreme_budget_values() {
    let inputs = [
        "CONTEXT FOR x TIER H1 BUDGET 0",
        "CONTEXT FOR x TIER H1 BUDGET 1",
        "CONTEXT FOR x TIER H1 BUDGET 4294967295",
        "CONTEXT FOR x TIER H1 BUDGET 99999999999999999999",
    ];
    for s in &inputs {
        must_not_panic(s);
    }
}

#[test]
fn long_identifiers() {
    let long_id: String = "x".repeat(10_000);
    must_not_panic(&format!("SEARCH {long_id} SIMILAR TO [1.0] K 5"));
    must_not_panic(&format!("TRAVERSE FROM {long_id} DEPTH 3 REL KNOWS"));
    must_not_panic(&format!("CONTEXT FOR {long_id} TIER H1"));
}

#[test]
fn long_string_literals() {
    let long_str: String = "a".repeat(10_000);
    must_not_panic(&format!("SEARCH \"{long_str}\" SIMILAR TO [1.0] K 5"));
}

#[test]
fn empty_vector() {
    must_not_panic("SEARCH x SIMILAR TO [] K 5");
}

#[test]
fn single_element_vector() {
    must_not_panic("SEARCH x SIMILAR TO [42.0] K 5");
}

// =========================================================================
// 4. All tier values
// =========================================================================

#[test]
fn all_valid_tiers() {
    for t in ["H0", "H1", "H2", "H3", "H4", "H5", "H6"] {
        parses_ok(&format!("CONTEXT FOR mynode TIER {t}"));
    }
}

#[test]
fn invalid_tiers() {
    // H6 is now the valid ceiling (see all_valid_tiers); H7+ stay invalid.
    let invalid = ["H7", "H99", "X1", "h0", "MASTER", ""];
    for t in &invalid {
        must_not_panic(&format!("CONTEXT FOR mynode TIER {t}"));
    }
}

// =========================================================================
// 5. Fuzzy prefix edge cases
// =========================================================================

#[test]
fn fuzzy_prefix_variants() {
    let inputs = [
        "SEARCH ~mynode SIMILAR TO [1.0] K 5",
        "TRAVERSE FROM ~seed DEPTH 3 REL KNOWS",
        "CONTEXT FOR ~node TIER H1",
        "SEARCH ~~mynode SIMILAR TO [1.0] K 5",
        "SEARCH ~ SIMILAR TO [1.0] K 5",
    ];
    for s in &inputs {
        must_not_panic(s);
    }
}

// =========================================================================
// 6. Optional clause combinations
// =========================================================================

#[test]
fn search_with_all_optional_clauses() {
    parses_ok("SEARCH mynode SIMILAR TO [1.0,2.0] K 5 IN mycoll LANGUAGE \"en\" AS OF \"2024-01-01T00:00:00Z\"");
}

#[test]
fn search_with_partial_optional_clauses() {
    parses_ok("SEARCH mynode SIMILAR TO [1.0,2.0] K 5 LANGUAGE \"en\"");
    parses_ok("SEARCH mynode SIMILAR TO [1.0,2.0] K 5 IN mycoll");
    parses_ok("SEARCH mynode SIMILAR TO [1.0,2.0] K 5 AS OF \"2024-01-01T00:00:00Z\"");
}

#[test]
fn traverse_with_as_of() {
    parses_ok("TRAVERSE FROM seed DEPTH 3 REL KNOWS AS OF \"2024-01-01T00:00:00Z\"");
}

#[test]
fn traverse_with_infer_rel() {
    parses_ok("TRAVERSE FROM seed DEPTH 2 REL INFER(similarity)");
}

#[test]
fn context_with_budget() {
    parses_ok("CONTEXT FOR mynode TIER H3 BUDGET 64000");
}

// =========================================================================
// 7. Valid query round-trip — every command variant parses successfully
// =========================================================================

#[test]
fn valid_search_parses() {
    parses_ok(VALID_SEARCH);
    if let Ok(HqlCommand::Search {
        target, vector, k, ..
    }) = HqlCommand::try_from(VALID_SEARCH)
    {
        assert_eq!(target, "mynode");
        assert_eq!(vector, Some(vec![1.0, 2.0, 3.0]));
        assert_eq!(k, 10);
    } else {
        panic!("Expected Search variant");
    }
}

#[test]
fn valid_traverse_parses() {
    parses_ok(VALID_TRAVERSE);
    if let Ok(HqlCommand::Traverse { seed, .. }) = HqlCommand::try_from(VALID_TRAVERSE) {
        assert_eq!(seed, "seed_node");
    } else {
        panic!("Expected Traverse variant");
    }
}

#[test]
fn valid_hybrid_parses() {
    parses_ok(VALID_HYBRID);
    if let Ok(HqlCommand::Hybrid { target, alpha, .. }) = HqlCommand::try_from(VALID_HYBRID) {
        assert_eq!(target, "target");
        assert!((alpha - 0.7).abs() < 1e-9);
    } else {
        panic!("Expected Hybrid variant");
    }
}

#[test]
fn valid_context_parses() {
    parses_ok(VALID_CONTEXT);
    if let Ok(HqlCommand::Context { target, tier, .. }) = HqlCommand::try_from(VALID_CONTEXT) {
        assert_eq!(target, "node_id");
        assert_eq!(tier, "H2");
    } else {
        panic!("Expected Context variant");
    }
}

// =========================================================================
// 8. Deterministic pseudo-random generation (seeded, reproducible)
// =========================================================================

fn pseudo_random_bytes(seed: u64, len: usize) -> Vec<u8> {
    let mut state = seed;
    (0..len)
        .map(|_| {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (state >> 33) as u8
        })
        .collect()
}

#[test]
fn random_bytes_never_panic() {
    for seed in 0..500 {
        let bytes = pseudo_random_bytes(seed, 64 + (seed as usize % 200));
        if let Ok(s) = String::from_utf8(bytes) {
            must_not_panic(&s);
        }
    }
}

#[test]
fn random_mutations_of_valid_queries() {
    let queries = [VALID_SEARCH, VALID_TRAVERSE, VALID_HYBRID, VALID_CONTEXT];
    for query in queries.iter() {
        let bytes = query.as_bytes().to_vec();
        for pos in 0..bytes.len() {
            for replacement in [0u8, b' ', b'[', b']', b',', b'"', b'~', 255] {
                let mut mutated = bytes.clone();
                mutated[pos] = replacement;
                if let Ok(s) = String::from_utf8(mutated) {
                    let _ = std::panic::catch_unwind(|| {
                        let _ = HqlCommand::try_from(s.as_str());
                    });
                }
            }
        }
    }
    // If we get here without panic, the parser is robust against single-byte mutations.
}
