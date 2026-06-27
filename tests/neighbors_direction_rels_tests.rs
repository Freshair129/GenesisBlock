use genesis_block_native::{EdgeInput, NeighborInput, NodeInput, OpenOptions, Storage};
use std::collections::HashSet;
use std::fs;
use std::path::Path;

fn setup_db(name: &str) -> Storage {
    let db_path = format!("{}/{}", env!("CARGO_TARGET_TMPDIR"), name);
    if Path::new(&db_path).exists() {
        fs::remove_dir_all(&db_path).unwrap();
    }
    Storage::open(OpenOptions {
        path: db_path,
        page_cache_mb: Some(64),
        read_only: Some(false),
        vector_dim: None,
    })
    .unwrap()
}

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

fn edge(s: &Storage, eid: &str, from: &str, to: &str, rel: &str) {
    s.add_edge(EdgeInput {
        id: Some(eid.to_string()),
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

/// Build:
///     B --LIKES--> A
///     A --KNOWS--> C
///     A --LIKES--> D
///     E --FOLLOWS--> A
fn seed(s: &Storage) {
    for n in ["A", "B", "C", "D", "E"] {
        node(s, n);
    }
    edge(s, "e1", "B", "A", "LIKES");
    edge(s, "e2", "A", "C", "KNOWS");
    edge(s, "e3", "A", "D", "LIKES");
    edge(s, "e4", "E", "A", "FOLLOWS");
}

fn default_args() -> NeighborInput {
    NeighborInput {
        depth: Some(1),
        rel: None,
        rels: None,
        direction: None,
        as_of: None,
        include_invalid: None,
        limit: None,
    }
}

fn ids(out: &[genesis_block_native::NeighborOutput]) -> HashSet<String> {
    out.iter().map(|n| n.node.id.clone()).collect()
}

#[test]
fn direction_defaults_to_out_for_backcompat() {
    let s = setup_db("test_neighbors_dir_default");
    seed(&s);
    // No direction => current/legacy behavior = out only
    let res = s.neighbors("A".to_string(), default_args(), false).unwrap();
    assert_eq!(ids(&res), HashSet::from(["C".to_string(), "D".to_string()]));
}

#[test]
fn direction_in_returns_predecessors_only() {
    let s = setup_db("test_neighbors_dir_in");
    seed(&s);
    let mut a = default_args();
    a.direction = Some("in".to_string());
    let res = s.neighbors("A".to_string(), a, false).unwrap();
    assert_eq!(ids(&res), HashSet::from(["B".to_string(), "E".to_string()]));
}

#[test]
fn direction_both_returns_union() {
    let s = setup_db("test_neighbors_dir_both");
    seed(&s);
    let mut a = default_args();
    a.direction = Some("both".to_string());
    let res = s.neighbors("A".to_string(), a, false).unwrap();
    assert_eq!(
        ids(&res),
        HashSet::from([
            "B".to_string(),
            "C".to_string(),
            "D".to_string(),
            "E".to_string(),
        ])
    );
}

#[test]
fn direction_is_case_insensitive() {
    let s = setup_db("test_neighbors_dir_case");
    seed(&s);
    let mut a = default_args();
    a.direction = Some("BOTH".to_string());
    let res = s.neighbors("A".to_string(), a, false).unwrap();
    assert_eq!(res.len(), 4);
}

#[test]
fn rels_set_filters_to_chosen_types() {
    let s = setup_db("test_neighbors_rels_set");
    seed(&s);
    let mut a = default_args();
    a.direction = Some("both".to_string());
    a.rels = Some(vec!["LIKES".to_string(), "FOLLOWS".to_string()]);
    let res = s.neighbors("A".to_string(), a, false).unwrap();
    // LIKES: out -> D, in -> B ; FOLLOWS: in -> E. Excludes C (KNOWS).
    assert_eq!(
        ids(&res),
        HashSet::from(["B".to_string(), "D".to_string(), "E".to_string()])
    );
}

#[test]
fn rels_overrides_rel_when_both_given() {
    let s = setup_db("test_neighbors_rels_overrides_rel");
    seed(&s);
    let mut a = default_args();
    a.direction = Some("out".to_string());
    a.rel = Some("KNOWS".to_string());
    a.rels = Some(vec!["LIKES".to_string()]);
    let res = s.neighbors("A".to_string(), a, false).unwrap();
    // rels overrides → only LIKES out from A = D
    assert_eq!(ids(&res), HashSet::from(["D".to_string()]));
}

#[test]
fn rel_any_means_no_filter() {
    let s = setup_db("test_neighbors_rel_any");
    seed(&s);
    let mut a = default_args();
    a.direction = Some("out".to_string());
    a.rel = Some("ANY".to_string());
    let res = s.neighbors("A".to_string(), a, false).unwrap();
    assert_eq!(ids(&res), HashSet::from(["C".to_string(), "D".to_string()]));
}

#[test]
fn limit_still_honored_with_direction_both() {
    let s = setup_db("test_neighbors_limit_both");
    seed(&s);
    let mut a = default_args();
    a.direction = Some("both".to_string());
    a.limit = Some(2);
    let res = s.neighbors("A".to_string(), a, false).unwrap();
    assert_eq!(res.len(), 2);
}

#[test]
fn empty_rels_falls_back_to_rel() {
    let s = setup_db("test_neighbors_empty_rels");
    seed(&s);
    let mut a = default_args();
    a.direction = Some("out".to_string());
    a.rel = Some("KNOWS".to_string());
    a.rels = Some(vec![]); // empty → should not override; fall back to rel
    let res = s.neighbors("A".to_string(), a, false).unwrap();
    assert_eq!(ids(&res), HashSet::from(["C".to_string()]));
}
