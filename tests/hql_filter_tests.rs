//! HQL WHERE / ORDER BY / LIMIT / RETURN clause tests.
//! Covers ADR--GENESISDB-HQL-FILTER-PROJECTION: post-process filtering,
//! projection, ordering (nulls last), SQL-style null/type-mismatch = false,
//! and the `score` field (present on SEARCH, null on TRAVERSE).

use genesis_block_native::{NeighborOutput, NodeInput, OpenOptions, Storage};
use serde_json::{from_value, json, Value};
use std::fs;
use std::path::Path;

fn fresh(name: &str) -> String {
    let p = format!("{}/{}", env!("CARGO_TARGET_TMPDIR"), name);
    if Path::new(&p).exists() {
        fs::remove_dir_all(&p).unwrap();
    }
    p
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

fn node(s: &Storage, id: &str, labels: &[&str], props: Value) {
    s.add_node(NodeInput {
        id: Some(id.to_string()),
        labels: labels.iter().map(|l| l.to_string()).collect(),
        props: Some(props),
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
    s.add_edge(genesis_block_native::EdgeInput {
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

/// Build a fixture: seed `user5` SENT five messages with assorted labels/props,
/// including deliberately-missing fields to exercise null handling.
fn chat_fixture(name: &str) -> Storage {
    let s = open(&fresh(name));
    node(&s, "user5", &[], json!({}));
    node(
        &s,
        "msg1",
        &["Message"],
        json!({"text": "hello world", "side": "them", "time": 300}),
    );
    node(
        &s,
        "msg2",
        &["Message"],
        json!({"text": "bye now", "side": "me", "time": 100}),
    );
    node(
        &s,
        "msg3",
        &["Notification"],
        json!({"text": "weather alert", "side": "them", "time": 200}),
    );
    node(
        &s,
        "msg4",
        &["Message"],
        json!({"side": "them", "time": 50}),
    ); // no text
    node(&s, "msg5", &["Message"], json!({"text": "orphan"})); // no side, no time
    for m in ["msg1", "msg2", "msg3", "msg4", "msg5"] {
        edge(&s, "user5", m, "SENT");
    }
    s
}

fn run(s: &Storage, q: &str) -> Value {
    s.execute_hql(q).unwrap()
}

/// Collect the `id` field from a projected (RETURN) result array.
fn ids(v: &Value) -> Vec<String> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|o| o.get("id").unwrap().as_str().unwrap().to_string())
        .collect()
}

// --- WHERE ---------------------------------------------------------------

#[test]
fn where_label_membership() {
    let s = chat_fixture("hqlf_label");
    let v = run(
        &s,
        r#"TRAVERSE FROM user5 DEPTH 1 REL SENT WHERE label = "Message" RETURN id"#,
    );
    let mut got = ids(&v);
    got.sort();
    assert_eq!(got, vec!["msg1", "msg2", "msg4", "msg5"]); // not msg3 (Notification)
}

#[test]
fn where_prop_eq_and_projects_null_for_missing_field() {
    let s = chat_fixture("hqlf_prop_eq");
    let v = run(
        &s,
        r#"TRAVERSE FROM user5 DEPTH 1 REL SENT WHERE prop.side = "them" RETURN id, prop.text"#,
    );
    let arr = v.as_array().unwrap();
    let mut got = ids(&v);
    got.sort();
    assert_eq!(got, vec!["msg1", "msg3", "msg4"]); // msg5 excluded (no side), msg2 is "me"
                                                   // msg4 has side=them but no text -> projected as null, must not panic/omit.
    let msg4 = arr.iter().find(|o| o["id"] == json!("msg4")).unwrap();
    assert_eq!(msg4.get("text"), Some(&Value::Null));
}

#[test]
fn where_numeric_excludes_missing_and_smaller() {
    let s = chat_fixture("hqlf_num");
    let v = run(
        &s,
        r#"TRAVERSE FROM user5 DEPTH 1 REL SENT WHERE prop.time > 150 RETURN id"#,
    );
    let mut got = ids(&v);
    got.sort();
    // msg1=300, msg3=200 pass; msg2=100, msg4=50 fail; msg5 (no time) excluded.
    assert_eq!(got, vec!["msg1", "msg3"]);
}

#[test]
fn where_ne_excludes_missing_sql_semantics() {
    let s = chat_fixture("hqlf_ne");
    let v = run(
        &s,
        r#"TRAVERSE FROM user5 DEPTH 1 REL SENT WHERE prop.side != "me" RETURN id"#,
    );
    let mut got = ids(&v);
    got.sort();
    // them: msg1, msg3, msg4 pass; msg2 (me) fails; msg5 (no side) EXCLUDED (SQL NULL).
    assert_eq!(got, vec!["msg1", "msg3", "msg4"]);
}

#[test]
fn where_contains() {
    let s = chat_fixture("hqlf_contains");
    let v = run(
        &s,
        r#"TRAVERSE FROM user5 DEPTH 1 REL SENT WHERE prop.text CONTAINS "weather" RETURN id"#,
    );
    assert_eq!(ids(&v), vec!["msg3"]);
}

#[test]
fn where_and_conjunction() {
    let s = chat_fixture("hqlf_and");
    let v = run(
        &s,
        r#"TRAVERSE FROM user5 DEPTH 1 REL SENT WHERE label = "Message" AND prop.side = "them" RETURN id"#,
    );
    let mut got = ids(&v);
    got.sort();
    assert_eq!(got, vec!["msg1", "msg4"]); // Message AND them
}

// --- ORDER BY / LIMIT ----------------------------------------------------

#[test]
fn order_by_prop_desc_nulls_last_then_limit() {
    let s = chat_fixture("hqlf_order");
    // Message msgs by time: msg1=300, msg2=100, msg4=50, msg5=null.
    let v = run(
        &s,
        r#"TRAVERSE FROM user5 DEPTH 1 REL SENT WHERE label = "Message" ORDER BY prop.time DESC RETURN id"#,
    );
    assert_eq!(ids(&v), vec!["msg1", "msg2", "msg4", "msg5"]); // null (msg5) sinks last

    let v2 = run(
        &s,
        r#"TRAVERSE FROM user5 DEPTH 1 REL SENT WHERE label = "Message" ORDER BY prop.time DESC LIMIT 2 RETURN id"#,
    );
    assert_eq!(ids(&v2), vec!["msg1", "msg2"]);
}

#[test]
fn order_by_asc_default() {
    let s = chat_fixture("hqlf_order_asc");
    let v = run(
        &s,
        r#"TRAVERSE FROM user5 DEPTH 1 REL SENT WHERE prop.time < 1000 ORDER BY prop.time RETURN id, prop.time"#,
    );
    // present time asc: msg4=50, msg2=100, msg3=200, msg1=300 (msg5 null excluded by < filter)
    assert_eq!(ids(&v), vec!["msg4", "msg2", "msg3", "msg1"]);
}

// --- RETURN --------------------------------------------------------------

#[test]
fn return_star_keeps_full_shape() {
    let s = chat_fixture("hqlf_star");
    let v = run(&s, r#"TRAVERSE FROM user5 DEPTH 1 REL SENT RETURN *"#);
    let rows: Vec<NeighborOutput> = from_value(v).unwrap();
    assert_eq!(rows.len(), 5);
}

#[test]
fn no_clause_is_backward_compatible() {
    let s = chat_fixture("hqlf_noclause");
    let v = run(&s, r#"TRAVERSE FROM user5 DEPTH 1 REL SENT"#);
    let rows: Vec<NeighborOutput> = from_value(v).unwrap();
    assert_eq!(rows.len(), 5);
}

// --- score (null on TRAVERSE, present on SEARCH) -------------------------

#[test]
fn score_is_null_on_traverse() {
    let s = chat_fixture("hqlf_score_null");
    // WHERE score > 0.8 excludes everything (traverse rows have no score).
    let v = run(
        &s,
        r#"TRAVERSE FROM user5 DEPTH 1 REL SENT WHERE score > 0.8 RETURN id"#,
    );
    assert_eq!(v.as_array().unwrap().len(), 0);

    // RETURN score -> null, no panic.
    let v2 = run(
        &s,
        r#"TRAVERSE FROM user5 DEPTH 1 REL SENT RETURN id, score"#,
    );
    let first = &v2.as_array().unwrap()[0];
    assert_eq!(first.get("score"), Some(&Value::Null));
}

#[test]
fn search_order_by_score_desc_projects_numeric_score() {
    let s = Storage::open(OpenOptions {
        path: fresh("hqlf_search_score"),
        page_cache_mb: Some(32),
        read_only: Some(false),
        vector_dim: Some(2),
    })
    .unwrap();
    for (id, emb) in [
        ("a", vec![1.0, 0.0]),
        ("b", vec![0.8, 0.6]),
        ("c", vec![0.0, 1.0]),
    ] {
        s.add_node(NodeInput {
            id: Some(id.to_string()),
            labels: vec!["Doc".to_string()],
            props: Some(json!({})),
            embedding: Some(emb),
            lang: None,
            valid_from: None,
            caused_by: None,
            ttl: None,
            collection: None,
        })
        .unwrap();
    }
    s.flush_index();

    let v = run(
        &s,
        r#"SEARCH q SIMILAR TO [1.0,0.0] K 5 ORDER BY score DESC RETURN id, score"#,
    );
    let arr = v.as_array().unwrap();
    // Assert the feature's guarantees (score projected as numeric + ORDER BY
    // score DESC), which hold regardless of HNSW recall under parallel load.
    assert!(!arr.is_empty(), "search returns at least one hit");
    let scores: Vec<f64> = arr
        .iter()
        .map(|o| {
            o["score"]
                .as_f64()
                .expect("score projected as numeric, not null")
        })
        .collect();
    for w in scores.windows(2) {
        assert!(w[0] >= w[1], "ORDER BY score DESC: {scores:?}");
    }
    // "a" is identical to the query, so when it is in the set it must rank first.
    if arr.iter().any(|o| o["id"] == json!("a")) {
        assert_eq!(arr[0]["id"], json!("a"), "nearest (a) ranks first");
    }
}

// --- additional coverage (from adversarial review) -----------------------

#[test]
fn where_startswith() {
    let s = chat_fixture("hqlf_startswith");
    let v = run(
        &s,
        r#"TRAVERSE FROM user5 DEPTH 1 REL SENT WHERE prop.text STARTSWITH "hello" RETURN id"#,
    );
    assert_eq!(ids(&v), vec!["msg1"]); // "hello world" starts with hello; not CONTAINS-style
}

#[test]
fn where_numeric_boundary_inclusive_vs_exclusive() {
    let s = chat_fixture("hqlf_bound");
    // time: msg1=300, msg2=100, msg3=200, msg4=50, msg5=null
    let ge = run(
        &s,
        r#"TRAVERSE FROM user5 DEPTH 1 REL SENT WHERE prop.time >= 200 ORDER BY prop.time ASC RETURN id"#,
    );
    assert_eq!(ids(&ge), vec!["msg3", "msg1"]); // 200 included
    let gt = run(
        &s,
        r#"TRAVERSE FROM user5 DEPTH 1 REL SENT WHERE prop.time > 200 RETURN id"#,
    );
    assert_eq!(ids(&gt), vec!["msg1"]); // 200 excluded
    let le = run(
        &s,
        r#"TRAVERSE FROM user5 DEPTH 1 REL SENT WHERE prop.time <= 100 ORDER BY prop.time ASC RETURN id"#,
    );
    assert_eq!(ids(&le), vec!["msg4", "msg2"]); // 100 included
}

#[test]
fn order_by_asc_nulls_last() {
    let s = chat_fixture("hqlf_asc_nulls");
    // Message msgs: msg1=300, msg2=100, msg4=50, msg5=null. ASC -> 50,100,300, null LAST.
    let v = run(
        &s,
        r#"TRAVERSE FROM user5 DEPTH 1 REL SENT WHERE label = "Message" ORDER BY prop.time ASC RETURN id"#,
    );
    assert_eq!(ids(&v), vec!["msg4", "msg2", "msg1", "msg5"]);
}

#[test]
fn return_star_equals_no_return() {
    let s = chat_fixture("hqlf_star_eq");
    // Both paths emit the full NeighborOutput shape (no projection). Traversal
    // order is not part of the contract without ORDER BY (neighbors() iterates a
    // HashSet), so compare the deserialized rows as a set by id + same shape.
    let none: Vec<NeighborOutput> =
        from_value(run(&s, r#"TRAVERSE FROM user5 DEPTH 1 REL SENT"#)).unwrap();
    let star: Vec<NeighborOutput> =
        from_value(run(&s, r#"TRAVERSE FROM user5 DEPTH 1 REL SENT RETURN *"#)).unwrap();
    let mut a: Vec<String> = none.iter().map(|n| n.node.id.clone()).collect();
    let mut b: Vec<String> = star.iter().map(|n| n.node.id.clone()).collect();
    a.sort();
    b.sort();
    assert_eq!(
        a, b,
        "RETURN * yields the same NeighborOutput set as no RETURN clause"
    );
    assert_eq!(a.len(), 5);
}

#[test]
fn return_key_collision_last_wins() {
    let s = open(&fresh("hqlf_collision"));
    node(&s, "user5", &[], json!({}));
    // node whose props.id differs from its node id
    node(&s, "msgX", &["Message"], json!({"id": "PROP_ID_VALUE"}));
    edge(&s, "user5", "msgX", "SENT");
    let v = run(
        &s,
        r#"TRAVERSE FROM user5 DEPTH 1 REL SENT RETURN id, prop.id"#,
    );
    let obj = &v.as_array().unwrap()[0];
    // Both project to key "id"; documented contract = last field wins (prop.id).
    assert_eq!(obj.get("id"), Some(&json!("PROP_ID_VALUE")));
    assert_eq!(
        obj.as_object().unwrap().len(),
        1,
        "collided key appears once"
    );
}

#[test]
fn where_label_ne_membership() {
    let s = chat_fixture("hqlf_label_ne");
    let v = run(
        &s,
        r#"TRAVERSE FROM user5 DEPTH 1 REL SENT WHERE label != "Notification" RETURN id"#,
    );
    let mut got = ids(&v);
    got.sort();
    assert_eq!(got, vec!["msg1", "msg2", "msg4", "msg5"]); // only msg3 (Notification) excluded
}

#[test]
fn where_type_mismatch_excludes() {
    let s = chat_fixture("hqlf_typemismatch");
    // string literal vs numeric prop -> excluded
    let a = run(
        &s,
        r#"TRAVERSE FROM user5 DEPTH 1 REL SENT WHERE prop.time = "300" RETURN id"#,
    );
    assert_eq!(a.as_array().unwrap().len(), 0);
    // numeric literal vs string prop -> excluded
    let b = run(
        &s,
        r#"TRAVERSE FROM user5 DEPTH 1 REL SENT WHERE prop.text = 100 RETURN id"#,
    );
    assert_eq!(b.as_array().unwrap().len(), 0);
}

#[test]
fn where_three_way_and_with_missing_conjunct() {
    let s = chat_fixture("hqlf_3way");
    let hit = run(
        &s,
        r#"TRAVERSE FROM user5 DEPTH 1 REL SENT WHERE label = "Message" AND prop.time > 40 AND prop.text CONTAINS "world" RETURN id"#,
    );
    assert_eq!(ids(&hit), vec!["msg1"]);
    // a conjunct on a missing field excludes the row (AND short-circuits to false)
    let none = run(
        &s,
        r#"TRAVERSE FROM user5 DEPTH 1 REL SENT WHERE label = "Message" AND prop.nope = "x" RETURN id"#,
    );
    assert_eq!(none.as_array().unwrap().len(), 0);
}

#[test]
fn prop_accessor_is_case_insensitive() {
    let s = chat_fixture("hqlf_propcase");
    let v = run(
        &s,
        r#"TRAVERSE FROM user5 DEPTH 1 REL SENT WHERE PROP.side = "them" RETURN id"#,
    );
    let mut got = ids(&v);
    got.sort();
    assert_eq!(got, vec!["msg1", "msg3", "msg4"]); // same as lowercase prop.side
}

#[test]
fn order_by_label_does_not_panic() {
    let s = chat_fixture("hqlf_orderlabel");
    // label is multi-valued; ORDER BY label must not panic and must keep all rows.
    let v = run(
        &s,
        r#"TRAVERSE FROM user5 DEPTH 1 REL SENT ORDER BY label RETURN id"#,
    );
    assert_eq!(v.as_array().unwrap().len(), 5);
}

#[test]
fn hybrid_match_carries_clauses() {
    let s = Storage::open(OpenOptions {
        path: fresh("hqlf_hybrid_clauses"),
        page_cache_mb: Some(32),
        read_only: Some(false),
        vector_dim: Some(2),
    })
    .unwrap();
    for (id, emb) in [
        ("a", vec![1.0, 0.0]),
        ("b", vec![0.8, 0.6]),
        ("c", vec![0.0, 1.0]),
    ] {
        s.add_node(NodeInput {
            id: Some(id.to_string()),
            labels: vec!["Doc".to_string()],
            props: Some(json!({})),
            embedding: Some(emb),
            lang: None,
            valid_from: None,
            caused_by: None,
            ttl: None,
            collection: None,
        })
        .unwrap();
    }
    s.flush_index();
    // MATCH (hybrid) must accept the same trailing clauses as SEARCH.
    let v = run(
        &s,
        r#"MATCH q SIMILAR TO [1.0,0.0] ALPHA 0.5 ORDER BY score DESC LIMIT 2 RETURN id, score"#,
    );
    let arr = v.as_array().unwrap();
    assert!(
        !arr.is_empty() && arr.len() <= 2,
        "LIMIT 2 caps MATCH result"
    );
    let scores: Vec<f64> = arr
        .iter()
        .map(|o| o["score"].as_f64().expect("score projected as numeric"))
        .collect();
    for w in scores.windows(2) {
        assert!(w[0] >= w[1], "scores DESC on MATCH: {scores:?}");
    }
}

#[test]
fn search_no_return_keeps_neighboroutput_shape() {
    let s = Storage::open(OpenOptions {
        path: fresh("hqlf_search_noret"),
        page_cache_mb: Some(32),
        read_only: Some(false),
        vector_dim: Some(2),
    })
    .unwrap();
    s.add_node(NodeInput {
        id: Some("a".into()),
        labels: vec!["Doc".into()],
        props: Some(json!({})),
        embedding: Some(vec![1.0, 0.0]),
        lang: None,
        valid_from: None,
        caused_by: None,
        ttl: None,
        collection: None,
    })
    .unwrap();
    s.flush_index();
    let v = run(&s, r#"SEARCH q SIMILAR TO [1.0,0.0] K 5"#);
    let rows: Vec<NeighborOutput> = from_value(v).unwrap(); // full shape, no projection
    assert!(!rows.is_empty(), "search returns the indexed doc");
    assert!(
        rows.iter().all(|r| r.score.is_some()),
        "SEARCH rows carry a score"
    );
}

#[test]
fn context_rejects_trailing_clauses() {
    let s = chat_fixture("hqlf_ctx_reject");
    assert!(s.execute_hql("CONTEXT FOR msg1 TIER H0 LIMIT 1").is_err());
    assert!(s
        .execute_hql(r#"CONTEXT FOR msg1 TIER H0 WHERE id = "x""#)
        .is_err());
    assert!(s.execute_hql("CONTEXT FOR msg1 TIER H0 RETURN id").is_err());
    // but plain CONTEXT (with optional BUDGET) still works
    assert!(s.execute_hql("CONTEXT FOR msg1 TIER H0 BUDGET 100").is_ok());
}

// --- AST-level guards for the atomic-digit fix (K / DEPTH / BUDGET / LIMIT) ---

#[test]
fn digit_rules_parse_exact_values() {
    use genesis_block_native::query::ast::{HqlClauses, HqlCommand};
    use std::convert::TryFrom;

    let depth = HqlCommand::try_from("TRAVERSE FROM a DEPTH 3 REL R").unwrap();
    assert!(matches!(depth, HqlCommand::Traverse { depth: 3, .. }));

    let k = HqlCommand::try_from("SEARCH q SIMILAR TO [1.0] K 7").unwrap();
    assert!(matches!(k, HqlCommand::Search { k: 7, .. }));

    let budget = HqlCommand::try_from("CONTEXT FOR x TIER H2 BUDGET 64000").unwrap();
    assert!(matches!(
        budget,
        HqlCommand::Context {
            budget: Some(64000),
            ..
        }
    ));

    let limit = HqlCommand::try_from("TRAVERSE FROM a DEPTH 1 REL R LIMIT 9").unwrap();
    let cl: HqlClauses = match limit {
        HqlCommand::Traverse { clauses, .. } => clauses,
        _ => panic!("expected Traverse"),
    };
    assert_eq!(cl.limit, Some(9));
}

#[test]
fn limit_overflow_saturates_not_drops() {
    use genesis_block_native::query::ast::HqlCommand;
    use std::convert::TryFrom;
    let q = "TRAVERSE FROM a DEPTH 1 REL R LIMIT 99999999999999999999999999999";
    let cmd = HqlCommand::try_from(q).unwrap();
    let limit = match cmd {
        HqlCommand::Traverse { clauses, .. } => clauses.limit,
        _ => panic!(),
    };
    assert_eq!(
        limit,
        Some(usize::MAX),
        "oversized LIMIT saturates, not dropped to None"
    );
}

#[test]
fn strict_numeric_parse_errors_surface_in_ast() {
    use genesis_block_native::query::ast::HqlCommand;
    use std::convert::TryFrom;

    let k_err = HqlCommand::try_from("SEARCH q SIMILAR TO [1.0] K 99999999999999999999")
        .unwrap_err();
    assert!(k_err.contains("K value out of range"));

    let depth_err =
        HqlCommand::try_from("TRAVERSE FROM a DEPTH 99999999999999999999 REL R").unwrap_err();
    assert!(depth_err.contains("DEPTH value out of range"));

    let alpha_err = HqlCommand::try_from(
        "MATCH q SIMILAR TO [1.0] ALPHA 9999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999",
    )
    .unwrap_err();
    assert!(alpha_err.contains("ALPHA value out of range"));

    let budget_err =
        HqlCommand::try_from("CONTEXT FOR x TIER H1 BUDGET 99999999999999999999").unwrap_err();
    assert!(budget_err.contains("BUDGET value out of range"));
}

#[test]
fn hybrid_and_search_parse_new_exposed_knobs() {
    use genesis_block_native::query::ast::HqlCommand;
    use std::convert::TryFrom;

    let search = HqlCommand::try_from("SEARCH q K 7 EF 128 OVERSAMPLE 9").unwrap();
    assert!(matches!(
        search,
        HqlCommand::Search {
            k: 7,
            ef_search: Some(128),
            oversample: Some(9),
            vector: None,
            ..
        }
    ));

    let hybrid =
        HqlCommand::try_from("MATCH q ALPHA 0.5 K 50 EF 256 OVERSAMPLE 11").unwrap();
    assert!(matches!(
        hybrid,
        HqlCommand::Hybrid {
            k: 50,
            ef_search: Some(256),
            oversample: Some(11),
            vector: None,
            ..
        }
    ));
}

#[test]
fn traverse_parse_direction_and_rel_union() {
    use genesis_block_native::query::ast::{HqlCommand, HqlRel};
    use std::convert::TryFrom;

    let cmd =
        HqlCommand::try_from("TRAVERSE FROM a DEPTH 1 REL KNOWS|LIKES DIRECTION both").unwrap();
    match cmd {
        HqlCommand::Traverse {
            rel,
            rels,
            direction,
            ..
        } => {
            assert!(matches!(rel, HqlRel::Physical(ref r) if r == "KNOWS"));
            assert_eq!(rels, Some(vec!["KNOWS".to_string(), "LIKES".to_string()]));
            assert_eq!(direction, Some("both".to_string()));
        }
        _ => panic!("expected Traverse"),
    }
}
