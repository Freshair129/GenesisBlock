//! HQL (Hybrid Query Language) integration tests.
//!
//! Exercises the `execute_hql` entry-point for TRAVERSE and CONTEXT commands,
//! including edge cases around quoting, Unicode, depth bounds, and parse errors.

use genesis_block_native::{
    ContextPackage, EdgeInput, NeighborOutput, NodeInput, OpenOptions, Storage,
};
use serde_json::{from_value, json};
use std::fs;
use std::path::Path;

fn fresh(name: &str) -> String {
    let db_path = format!("{}/{}", env!("CARGO_TARGET_TMPDIR"), name);
    if Path::new(&db_path).exists() {
        fs::remove_dir_all(&db_path).unwrap();
    }
    db_path
}

fn open(path: &str) -> Storage {
    Storage::open(OpenOptions {
        path: path.to_string(),
        page_cache_mb: Some(32),
        read_only: Some(false),
        vector_dim: None,
    })
    .unwrap()
}

/// Helper: add a bare node (no embedding, no special props).
fn node(s: &Storage, id: &str) {
    s.add_node(NodeInput {
        id: Some(id.to_string()),
        labels: vec![],
        props: None,
        embedding: None,
        lang: None,
        valid_from: None,
        caused_by: None,
        ttl: None,
        collection: None,
    })
    .unwrap();
}

/// Helper: add a bare node with labels.
fn node_with_labels(s: &Storage, id: &str, labels: Vec<&str>) {
    s.add_node(NodeInput {
        id: Some(id.to_string()),
        labels: labels.into_iter().map(|l| l.to_string()).collect(),
        props: None,
        embedding: None,
        lang: None,
        valid_from: None,
        caused_by: None,
        ttl: None,
        collection: None,
    })
    .unwrap();
}

/// Helper: add a directed edge with a given rel type.
fn edge(s: &Storage, from: &str, to: &str, rel: &str) {
    s.add_edge(EdgeInput {
        id: None,
        from: from.to_string(),
        to: to.to_string(),
        rel: rel.to_string(),
        props: None,
        valid_from: None,
        supersede: None,
        impact: None,
        caused_by: None,
    })
    .unwrap();
}

// -----------------------------------------------------------------------
// 1. TRAVERSE valid: A->B->C, depth 2
//
// The `DEPTH N` parser bug (where the non-atomic `depth` rule let trailing
// whitespace creep into the span, failing `.parse()` and defaulting to 1) is
// fixed — the digit rules are now atomic (`@{ ASCII_DIGIT+ }`). HQL DEPTH 2
// now reaches both B (1-hop) and C (2-hop).
// -----------------------------------------------------------------------
#[test]
fn hql_traverse_valid() {
    let p = fresh("hql_traverse_valid");
    let s = open(&p);

    node(&s, "A");
    node(&s, "B");
    node(&s, "C");
    edge(&s, "A", "B", "KNOWS");
    edge(&s, "B", "C", "KNOWS");

    // HQL TRAVERSE depth 2 — now honored (digit-rule whitespace bug fixed).
    let res = s.execute_hql("TRAVERSE FROM A DEPTH 2 REL KNOWS").unwrap();
    let neighbors: Vec<NeighborOutput> = from_value(res).unwrap();
    let ids: Vec<&str> = neighbors.iter().map(|n| n.node.id.as_str()).collect();
    assert!(ids.contains(&"B"), "depth-1 neighbor B must appear");
    assert!(ids.contains(&"C"), "depth-2 neighbor C must appear (DEPTH now honored)");

    // Cross-check the direct `neighbors()` API agrees.
    let direct = s
        .neighbors(
            "A".to_string(),
            genesis_block_native::NeighborInput {
                depth: Some(2),
                rel: Some("KNOWS".to_string()),
                rels: None,
                direction: Some("out".to_string()),
                as_of: None,
                include_invalid: Some(false),
                limit: None,
            },
            false,
        )
        .unwrap();
    let direct_ids: Vec<&str> = direct.iter().map(|n| n.node.id.as_str()).collect();
    assert!(
        direct_ids.contains(&"B"),
        "direct: B must appear at depth 1"
    );
    assert!(
        direct_ids.contains(&"C"),
        "direct: C must appear at depth 2"
    );
}

// -----------------------------------------------------------------------
// 2. TRAVERSE depth 1: only immediate neighbor
// -----------------------------------------------------------------------
#[test]
fn hql_traverse_depth_1() {
    let p = fresh("hql_traverse_depth_1");
    let s = open(&p);

    node(&s, "A");
    node(&s, "B");
    node(&s, "C");
    edge(&s, "A", "B", "KNOWS");
    edge(&s, "B", "C", "KNOWS");

    let res = s.execute_hql("TRAVERSE FROM A DEPTH 1 REL KNOWS").unwrap();
    let neighbors: Vec<NeighborOutput> = from_value(res).unwrap();

    let ids: Vec<&str> = neighbors.iter().map(|n| n.node.id.as_str()).collect();
    assert!(ids.contains(&"B"), "depth-1 neighbor B must appear");
    assert!(
        !ids.contains(&"C"),
        "depth-2 neighbor C must NOT appear at depth 1"
    );
}

// -----------------------------------------------------------------------
// 3. CONTEXT H0: self-only context package
// -----------------------------------------------------------------------
#[test]
fn hql_context_h0() {
    let p = fresh("hql_context_h0");
    let s = open(&p);

    s.add_node(NodeInput {
        id: Some("target".to_string()),
        labels: vec!["USER".to_string()],
        props: Some(json!({"text": "hello"})),
        embedding: None,
        lang: None,
        valid_from: None,
        caused_by: None,
        ttl: None,
        collection: None,
    })
    .unwrap();

    let res = s.execute_hql("CONTEXT FOR target TIER H0").unwrap();
    let ctx: ContextPackage = from_value(res).unwrap();

    assert_eq!(ctx.nodes.len(), 1, "H0 returns only the target node");
    assert_eq!(ctx.nodes[0].id, "target");
    assert_eq!(ctx.edges.len(), 0, "H0 returns no edges");
}

// -----------------------------------------------------------------------
// 4. Invalid HQL command word -> parse error
// -----------------------------------------------------------------------
#[test]
fn hql_invalid_command() {
    let p = fresh("hql_invalid_command");
    let s = open(&p);

    let res = s.execute_hql("FOOBAR xyz");
    assert!(res.is_err(), "unknown command must return Err");
}

// -----------------------------------------------------------------------
// 5. Malformed syntax (TRAVERSE without seed) -> parse error
// -----------------------------------------------------------------------
#[test]
fn hql_malformed_syntax() {
    let p = fresh("hql_malformed_syntax");
    let s = open(&p);

    let res = s.execute_hql("TRAVERSE FROM");
    assert!(res.is_err(), "incomplete TRAVERSE must return Err");
}

// -----------------------------------------------------------------------
// 6. Quoted id with special characters
// -----------------------------------------------------------------------
#[test]
fn hql_quoted_id_with_special_chars() {
    let p = fresh("hql_quoted_special");
    let s = open(&p);

    // The identifier grammar allows alphanumeric, _, - but the colon (:)
    // requires quoting via string_lit.
    s.add_node(NodeInput {
        id: Some("node:special-1".to_string()),
        labels: vec![],
        props: None,
        embedding: None,
        lang: None,
        valid_from: None,
        caused_by: None,
        ttl: None,
        collection: None,
    })
    .unwrap();

    // Use quoted form since colon is not in `identifier` charset.
    // No outgoing KNOWS edges, so result should be empty but must not crash.
    let res = s.execute_hql(r#"TRAVERSE FROM "node:special-1" DEPTH 1 REL KNOWS"#);
    assert!(
        res.is_ok(),
        "quoted special-char id must parse and execute without error: {:?}",
        res.err()
    );

    let neighbors: Vec<NeighborOutput> = from_value(res.unwrap()).unwrap();
    // No edges were added, so empty is expected.
    assert!(neighbors.is_empty(), "no edges => empty result");
}

// -----------------------------------------------------------------------
// 7. Unicode (Thai) in labels — no encoding corruption
// -----------------------------------------------------------------------
#[test]
fn hql_unicode_thai_in_labels() {
    let p = fresh("hql_unicode_thai");
    let s = open(&p);

    node_with_labels(
        &s,
        "thai-node",
        vec!["\u{0e17}\u{0e14}\u{0e2a}\u{0e2d}\u{0e1a}"],
    );
    node(&s, "thai-peer");
    edge(&s, "thai-node", "thai-peer", "KNOWS");

    let res = s
        .execute_hql("TRAVERSE FROM thai-node DEPTH 1 REL KNOWS")
        .unwrap();
    let neighbors: Vec<NeighborOutput> = from_value(res).unwrap();
    assert_eq!(neighbors.len(), 1);
    assert_eq!(neighbors[0].node.id, "thai-peer");

    // Verify the source node's labels survived round-trip without corruption.
    let u32_id = s.get_u32("thai-node").expect("thai-node must be interned");
    let stored = s.nodes.get(&u32_id).unwrap();
    assert_eq!(
        stored.labels,
        vec!["\u{0e17}\u{0e14}\u{0e2a}\u{0e2d}\u{0e1a}".to_string()],
        "Thai label must survive without encoding corruption"
    );
}

// -----------------------------------------------------------------------
// 8. TRAVERSE DEPTH 0 — document actual behaviour
// -----------------------------------------------------------------------
#[test]
fn hql_depth_0() {
    let p = fresh("hql_depth_0");
    let s = open(&p);

    node(&s, "A");
    node(&s, "B");
    edge(&s, "A", "B", "KNOWS");

    // DEPTH 0 is syntactically valid (grammar accepts any ASCII_DIGIT+) and now
    // parses correctly to 0 (digit-rule whitespace bug fixed) — no expansion.
    let res = s.execute_hql("TRAVERSE FROM A DEPTH 0 REL KNOWS");
    assert!(res.is_ok(), "DEPTH 0 must not crash: {:?}", res.err());

    let neighbors: Vec<NeighborOutput> = from_value(res.unwrap()).unwrap();
    assert!(
        neighbors.is_empty(),
        "DEPTH 0 expands nothing, got {} neighbor(s)",
        neighbors.len()
    );
}
