// `retract_edge` — bitemporal soft-delete (MARK XIV P3). Retraction sets the
// edge's `valid_to`; the edge stays in the log (history preserved) but
// `neighbors` hides it from the current view unless `include_invalid = true`.
// Retraction timestamps are explicit (2025) and valid_from is in the past
// (2024), so these assertions are independent of the wall clock.

use genesis_block_native::{Storage, OpenOptions, NodeInput, EdgeInput, NeighborInput};
use std::fs;
use std::path::Path;

fn fresh(name: &str) -> String {
    let p = format!("G:/GenesisBlock_Dev/GenesisBlock/tests/{}", name);
    if Path::new(&p).exists() { fs::remove_dir_all(&p).unwrap(); }
    p
}

fn open(path: &str) -> Storage {
    Storage::open(OpenOptions {
        path: path.to_string(), page_cache_mb: Some(32), read_only: Some(false), vector_dim: None,
    }).unwrap()
}

fn add_node(s: &Storage, id: &str) {
    s.add_node(NodeInput {
        id: Some(id.to_string()), labels: vec![], props: None, embedding: None, lang: None,
        valid_from: Some("2024-01-01T00:00:00Z".to_string()), caused_by: None, ttl: None, collection: None,
    }).unwrap();
}

fn add_edge(s: &Storage, id: &str, from: &str, to: &str) {
    s.add_edge(EdgeInput {
        id: Some(id.to_string()), from: from.to_string(), to: to.to_string(), rel: "REL".to_string(),
        props: None, valid_from: Some("2024-02-01T00:00:00Z".to_string()), supersede: None, impact: None, caused_by: None,
    }).unwrap();
}

fn neighbor_ids(s: &Storage, seed: &str, as_of: Option<&str>, include_invalid: Option<bool>) -> Vec<String> {
    s.neighbors(seed.to_string(), NeighborInput {
        depth: Some(1), rel: None, rels: None, direction: Some("out".to_string()),
        as_of: as_of.map(|x| x.to_string()), include_invalid, limit: None,
    }, false).unwrap().into_iter().map(|n| n.node.id).collect()
}

fn seed(s: &Storage) {
    add_node(s, "A");
    add_node(s, "B");
    add_edge(s, "E1", "A", "B");
}

/// A live edge is traversable; after retraction it disappears from the current view.
#[test]
fn retract_hides_from_current_view() {
    let s = open(&fresh("test_retract_current"));
    seed(&s);
    assert_eq!(neighbor_ids(&s, "A", None, None), vec!["B".to_string()]);

    let retracted = s.retract_edge("E1".to_string(), Some("2025-01-01T00:00:00Z".to_string())).unwrap();
    assert!(retracted.is_some(), "retract returns the edge");
    assert_eq!(retracted.unwrap().valid_to, Some("2025-01-01T00:00:00Z".to_string()));

    assert!(neighbor_ids(&s, "A", None, None).is_empty(), "retracted edge hidden from current view");
}

/// History is preserved: a time-travel query before the retraction still sees the edge.
#[test]
fn retracted_visible_as_of_before_retraction() {
    let s = open(&fresh("test_retract_asof"));
    seed(&s);
    s.retract_edge("E1".to_string(), Some("2025-01-01T00:00:00Z".to_string())).unwrap();

    // Before the retraction (2024-06): edge still live.
    assert_eq!(neighbor_ids(&s, "A", Some("2024-06-01T00:00:00Z"), None), vec!["B".to_string()]);
    // After the retraction (2025-06): edge gone.
    assert!(neighbor_ids(&s, "A", Some("2025-06-01T00:00:00Z"), None).is_empty());
}

/// `include_invalid = true` surfaces a retracted edge in the current view.
#[test]
fn include_invalid_surfaces_retracted() {
    let s = open(&fresh("test_retract_include"));
    seed(&s);
    s.retract_edge("E1".to_string(), Some("2025-01-01T00:00:00Z".to_string())).unwrap();

    assert!(neighbor_ids(&s, "A", None, None).is_empty(), "hidden by default");
    assert_eq!(neighbor_ids(&s, "A", None, Some(true)), vec!["B".to_string()], "shown with include_invalid");
}

/// Retracting a non-existent edge is a no-op returning `None` (not an error).
#[test]
fn retract_missing_returns_none() {
    let s = open(&fresh("test_retract_missing"));
    seed(&s);
    assert!(s.retract_edge("does-not-exist".to_string(), None).unwrap().is_none());
}

/// Retraction survives a save + reopen (the valid_to is durable via WAL).
#[test]
fn retraction_persists_across_reopen() {
    let path = fresh("test_retract_persist");
    {
        let s = open(&path);
        seed(&s);
        s.retract_edge("E1".to_string(), Some("2025-01-01T00:00:00Z".to_string())).unwrap();
        s.save_state().unwrap();
    }
    let s2 = open(&path);
    assert!(neighbor_ids(&s2, "A", None, None).is_empty(), "retraction durable across reopen");
    assert_eq!(neighbor_ids(&s2, "A", None, Some(true)), vec!["B".to_string()]);
}
