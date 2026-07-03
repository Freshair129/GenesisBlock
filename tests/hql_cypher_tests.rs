//! HQL Cypher-style graph pattern matching (path 1) integration tests.
//!
//! Exercises the `MATCH (a)-[r]->(b) ...` command added in
//! ADR--GENESISDB-HQL-CYPHER-PATTERNS: linear path expansion, node/edge
//! constraints, direction, variable-qualified WHERE/ORDER BY/LIMIT/RETURN,
//! keyword disambiguation vs. the existing hybrid `MATCH ... SIMILAR`, and
//! bitemporal `AS OF`.

use genesis_block_native::{EdgeInput, NodeInput, OpenOptions, Storage};
use serde_json::{json, Value};
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

fn node(s: &Storage, id: &str, labels: Vec<&str>, props: Option<Value>) {
    s.add_node(NodeInput {
        id: Some(id.to_string()),
        labels: labels.into_iter().map(|l| l.to_string()).collect(),
        props,
        embedding: None,
        lang: None,
        valid_from: None,
        caused_by: None,
        ttl: None,
        collection: None,
    })
    .unwrap();
}

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

/// Collect a projected result array's rows as objects.
fn rows(v: &Value) -> &Vec<Value> {
    v.as_array().expect("result must be a JSON array")
}

// -----------------------------------------------------------------------
// 1. Single-hop pattern with rel type + projection.
// -----------------------------------------------------------------------
#[test]
fn cypher_single_hop() {
    let p = fresh("cypher_single_hop");
    let s = open(&p);
    node(&s, "A", vec![], None);
    node(&s, "B", vec![], None);
    node(&s, "C", vec![], None);
    edge(&s, "A", "B", "KNOWS");
    edge(&s, "B", "C", "LIKES"); // different rel, must be excluded

    let res = s
        .execute_hql("MATCH (a)-[:KNOWS]->(b) RETURN a.id, b.id")
        .unwrap();
    let r = rows(&res);
    assert_eq!(r.len(), 1, "exactly one KNOWS edge matches");
    assert_eq!(r[0]["a.id"], json!("A"));
    assert_eq!(r[0]["b.id"], json!("B"));
}

// -----------------------------------------------------------------------
// 2. Two-hop chain: (a)-[:KNOWS]->(b)-[:KNOWS]->(c).
// -----------------------------------------------------------------------
#[test]
fn cypher_two_hop_chain() {
    let p = fresh("cypher_two_hop_chain");
    let s = open(&p);
    for id in ["A", "B", "C", "D"] {
        node(&s, id, vec![], None);
    }
    edge(&s, "A", "B", "KNOWS");
    edge(&s, "B", "C", "KNOWS");
    edge(&s, "C", "D", "LIKES");

    let res = s
        .execute_hql("MATCH (a)-[:KNOWS]->(b)-[:KNOWS]->(c) RETURN a.id, b.id, c.id")
        .unwrap();
    let r = rows(&res);
    assert_eq!(r.len(), 1, "only A->B->C is a two-hop KNOWS chain");
    assert_eq!(r[0]["a.id"], json!("A"));
    assert_eq!(r[0]["b.id"], json!("B"));
    assert_eq!(r[0]["c.id"], json!("C"));
}

// -----------------------------------------------------------------------
// 3. Label constraint on anchor + far node.
// -----------------------------------------------------------------------
#[test]
fn cypher_label_constraints() {
    let p = fresh("cypher_label_constraints");
    let s = open(&p);
    node(&s, "u1", vec!["User"], None);
    node(&s, "u2", vec!["User"], None);
    node(&s, "m1", vec!["Message"], None);
    node(&s, "x1", vec!["Other"], None);
    edge(&s, "u1", "m1", "SENT");
    edge(&s, "u2", "x1", "SENT"); // target not a Message

    let res = s
        .execute_hql("MATCH (a:User)-[:SENT]->(m:Message) RETURN a.id, m.id")
        .unwrap();
    let r = rows(&res);
    assert_eq!(r.len(), 1, "only u1->m1 satisfies both labels");
    assert_eq!(r[0]["a.id"], json!("u1"));
    assert_eq!(r[0]["m.id"], json!("m1"));
}

// -----------------------------------------------------------------------
// 4. Incoming direction: (x)<-[:KNOWS]-(y) binds the edge's source as y.
// -----------------------------------------------------------------------
#[test]
fn cypher_incoming_direction() {
    let p = fresh("cypher_incoming_direction");
    let s = open(&p);
    node(&s, "A", vec![], None);
    node(&s, "B", vec![], None);
    edge(&s, "A", "B", "KNOWS"); // A KNOWS B

    let res = s
        .execute_hql("MATCH (x)<-[:KNOWS]-(y) RETURN x.id, y.id")
        .unwrap();
    let r = rows(&res);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0]["x.id"], json!("B"), "x is the edge target");
    assert_eq!(r[0]["y.id"], json!("A"), "y is the edge source");
}

// -----------------------------------------------------------------------
// 5. Both-direction edge matches either orientation.
// -----------------------------------------------------------------------
#[test]
fn cypher_both_direction() {
    let p = fresh("cypher_both_direction");
    let s = open(&p);
    node(&s, "A", vec![], None);
    node(&s, "B", vec![], None);
    edge(&s, "A", "B", "KNOWS");

    // Anchored on A, an undirected hop reaches B; anchored on B it reaches A.
    let res = s
        .execute_hql("MATCH (a {id:\"A\"})-[:KNOWS]-(b) RETURN b.id")
        .unwrap();
    let r = rows(&res);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0]["b.id"], json!("B"));
}

// -----------------------------------------------------------------------
// 6. Inline property constraint `{k:v}` on a node.
// -----------------------------------------------------------------------
#[test]
fn cypher_inline_prop_constraint() {
    let p = fresh("cypher_inline_prop_constraint");
    let s = open(&p);
    node(&s, "t1", vec!["Msg"], Some(json!({ "side": "them" })));
    node(&s, "t2", vec!["Msg"], Some(json!({ "side": "me" })));
    node(&s, "root", vec![], None);
    edge(&s, "root", "t1", "HAS");
    edge(&s, "root", "t2", "HAS");

    let res = s
        .execute_hql("MATCH (r)-[:HAS]->(m {side:\"them\"}) RETURN m.id")
        .unwrap();
    let r = rows(&res);
    assert_eq!(r.len(), 1, "only the side=them node matches");
    assert_eq!(r[0]["m.id"], json!("t1"));
}

// -----------------------------------------------------------------------
// 7. WHERE over a bound variable's prop (numeric) + label membership.
// -----------------------------------------------------------------------
#[test]
fn cypher_where_prop_and_label() {
    let p = fresh("cypher_where_prop_and_label");
    let s = open(&p);
    node(&s, "root", vec![], None);
    node(&s, "m1", vec!["Message"], Some(json!({ "n": 10 })));
    node(&s, "m2", vec!["Message"], Some(json!({ "n": 2 })));
    node(&s, "z1", vec!["Note"], Some(json!({ "n": 99 })));
    edge(&s, "root", "m1", "HAS");
    edge(&s, "root", "m2", "HAS");
    edge(&s, "root", "z1", "HAS");

    let res = s
        .execute_hql(
            "MATCH (r)-[:HAS]->(m) WHERE m.label = \"Message\" AND m.prop.n > 5 RETURN m.id",
        )
        .unwrap();
    let r = rows(&res);
    assert_eq!(r.len(), 1, "only m1 is a Message with n>5");
    assert_eq!(r[0]["m.id"], json!("m1"));
}

// -----------------------------------------------------------------------
// 8. ORDER BY + LIMIT over pattern rows.
// -----------------------------------------------------------------------
#[test]
fn cypher_order_by_limit() {
    let p = fresh("cypher_order_by_limit");
    let s = open(&p);
    node(&s, "root", vec![], None);
    for (id, n) in [("a", 3), ("b", 1), ("c", 2)] {
        node(&s, id, vec![], Some(json!({ "n": n })));
        edge(&s, "root", id, "HAS");
    }

    let res = s
        .execute_hql("MATCH (r)-[:HAS]->(x) ORDER BY x.prop.n DESC LIMIT 2 RETURN x.id, x.prop.n")
        .unwrap();
    let r = rows(&res);
    assert_eq!(r.len(), 2, "LIMIT 2");
    assert_eq!(r[0]["x.id"], json!("a"), "highest n first (DESC)");
    assert_eq!(r[1]["x.id"], json!("c"));
}

// -----------------------------------------------------------------------
// 9. Edge variable binding: RETURN the edge's rel + a projected edge field.
// -----------------------------------------------------------------------
#[test]
fn cypher_edge_variable() {
    let p = fresh("cypher_edge_variable");
    let s = open(&p);
    node(&s, "A", vec![], None);
    node(&s, "B", vec![], None);
    edge(&s, "A", "B", "KNOWS");

    let res = s
        .execute_hql("MATCH (a)-[r:KNOWS]->(b) RETURN r.label, r.id")
        .unwrap();
    let r = rows(&res);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0]["r.label"], json!("KNOWS"), "edge .label = rel type");
    assert!(r[0]["r.id"].is_string(), "edge id present");
}

// -----------------------------------------------------------------------
// 10. Default RETURN (no clause) yields one object per row keyed by variable.
// -----------------------------------------------------------------------
#[test]
fn cypher_default_return_shape() {
    let p = fresh("cypher_default_return_shape");
    let s = open(&p);
    node(&s, "A", vec![], None);
    node(&s, "B", vec![], None);
    edge(&s, "A", "B", "KNOWS");

    let res = s.execute_hql("MATCH (a)-[:KNOWS]->(b)").unwrap();
    let r = rows(&res);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0]["a"]["id"], json!("A"), "bare var → full node object");
    assert_eq!(r[0]["b"]["id"], json!("B"));
}

// -----------------------------------------------------------------------
// 11. Anchor-only pattern (no hops): list nodes by label.
// -----------------------------------------------------------------------
#[test]
fn cypher_anchor_only() {
    let p = fresh("cypher_anchor_only");
    let s = open(&p);
    node(&s, "u1", vec!["User"], None);
    node(&s, "u2", vec!["User"], None);
    node(&s, "m1", vec!["Message"], None);

    let res = s.execute_hql("MATCH (u:User) RETURN u.id").unwrap();
    let mut ids: Vec<String> = rows(&res)
        .iter()
        .map(|row| row["u.id"].as_str().unwrap().to_string())
        .collect();
    ids.sort();
    assert_eq!(ids, vec!["u1".to_string(), "u2".to_string()]);
}

// -----------------------------------------------------------------------
// 12. Disambiguation: the existing hybrid `MATCH <t> SIMILAR TO [..] ALPHA`
//     still parses and runs (must NOT be captured by the pattern rule), while
//     `MATCH (n)` routes to the new pattern command.
// -----------------------------------------------------------------------
#[test]
fn cypher_does_not_break_hybrid_match() {
    let p = fresh("cypher_does_not_break_hybrid_match");
    // Open with a 2-dim default collection so the hybrid vector query is valid.
    let s = Storage::open(OpenOptions {
        path: p.to_string(),
        page_cache_mb: Some(32),
        read_only: Some(false),
        vector_dim: Some(2),
    })
    .unwrap();
    s.add_node(NodeInput {
        id: Some("n".to_string()),
        labels: vec![],
        props: None,
        embedding: Some(vec![0.1, 0.2]),
        lang: None,
        valid_from: None,
        caused_by: None,
        ttl: None,
        collection: None,
    })
    .unwrap();
    s.flush_index();

    // Hybrid form: identifier target + SIMILAR TO — parses as `hybrid`, returns array.
    let hybrid = s
        .execute_hql("MATCH n SIMILAR TO [0.1, 0.2] ALPHA 0.5")
        .unwrap();
    assert!(
        hybrid.is_array(),
        "hybrid MATCH still returns a result array"
    );

    // Pattern form: `(n)` routes to the new command.
    let pattern = s.execute_hql("MATCH (x) RETURN x.id").unwrap();
    assert_eq!(rows(&pattern).len(), 1);
    assert_eq!(rows(&pattern)[0]["x.id"], json!("n"));
}

// -----------------------------------------------------------------------
// 13. Bitemporal AS OF: a pattern honours node/edge validity at a timestamp.
//     An edge created "now" is invisible to an AS OF in the past.
// -----------------------------------------------------------------------
#[test]
fn cypher_as_of_temporal() {
    let p = fresh("cypher_as_of_temporal");
    let s = open(&p);
    node(&s, "A", vec![], None);
    node(&s, "B", vec![], None);
    edge(&s, "A", "B", "KNOWS");

    // Current view: the edge is visible.
    let now = s
        .execute_hql("MATCH (a)-[:KNOWS]->(b) RETURN a.id, b.id")
        .unwrap();
    assert_eq!(rows(&now).len(), 1, "edge visible in current view");

    // As of the epoch (before anything existed): nothing matches.
    let past = s
        .execute_hql("MATCH (a)-[:KNOWS]->(b) AS OF \"1970-01-01T00:00:00Z\" RETURN a.id, b.id")
        .unwrap();
    assert_eq!(rows(&past).len(), 0, "nothing valid before it existed");
}
