use genesis_block_native::{
    EdgeInput, NeighborInput, NeighborOutput, NodeInput, OpenOptions, QueryInput, Storage,
};
use serde_json::json;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

// ── helpers ──────────────────────────────────────────────────────────────

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

fn ids(out: &[NeighborOutput]) -> HashSet<String> {
    out.iter().map(|n| n.node.id.clone()).collect()
}

fn nb(s: &Storage, id: &str, depth: u32, dir: &str) -> Vec<NeighborOutput> {
    s.neighbors(
        id.to_string(),
        NeighborInput {
            depth: Some(depth),
            rel: None,
            rels: None,
            direction: Some(dir.to_string()),
            as_of: None,
            include_invalid: None,
            limit: None,
        },
        false,
    )
    .unwrap()
}

// ── tests ────────────────────────────────────────────────────────────────

#[test]
fn traverse_depth_chain() {
    let p = fresh("gt_depth_chain");
    let s = open(&p);
    node(&s, "A");
    node(&s, "B");
    node(&s, "C");
    node(&s, "D");
    edge(&s, "ab", "A", "B", "LINK");
    edge(&s, "bc", "B", "C", "LINK");
    edge(&s, "cd", "C", "D", "LINK");

    let d1 = nb(&s, "A", 1, "out");
    assert_eq!(ids(&d1), ["B"].into_iter().map(String::from).collect());

    let d2 = nb(&s, "A", 2, "out");
    assert_eq!(
        ids(&d2),
        ["B", "C"]
            .into_iter()
            .map(String::from)
            .collect::<HashSet<_>>()
    );

    let d3 = nb(&s, "A", 3, "out");
    assert_eq!(
        ids(&d3),
        ["B", "C", "D"]
            .into_iter()
            .map(String::from)
            .collect::<HashSet<_>>()
    );
}

#[test]
fn traverse_depth_0_returns_empty() {
    // depth=0 means "zero hops" — the BFS skips expansion when curr_depth >= depth,
    // so no neighbors are collected.
    let p = fresh("gt_depth0");
    let s = open(&p);
    node(&s, "A");
    node(&s, "B");
    edge(&s, "ab", "A", "B", "LINK");

    let d0 = nb(&s, "A", 0, "out");
    assert!(
        d0.is_empty(),
        "depth=0 should return no neighbors (got {})",
        d0.len()
    );
}

#[test]
fn relationship_filter() {
    let p = fresh("gt_rel_filter");
    let s = open(&p);
    node(&s, "A");
    node(&s, "B");
    node(&s, "C");
    edge(&s, "ab", "A", "B", "KNOWS");
    edge(&s, "ac", "A", "C", "BOUGHT");

    let res = s
        .neighbors(
            "A".to_string(),
            NeighborInput {
                depth: Some(1),
                rel: Some("KNOWS".to_string()),
                rels: None,
                direction: Some("out".to_string()),
                as_of: None,
                include_invalid: None,
                limit: None,
            },
            false,
        )
        .unwrap();

    assert_eq!(ids(&res), ["B"].into_iter().map(String::from).collect());
}

#[test]
fn direction_outgoing() {
    let p = fresh("gt_dir_out");
    let s = open(&p);
    node(&s, "A");
    node(&s, "B");
    node(&s, "C");
    edge(&s, "ab", "A", "B", "LINK");
    edge(&s, "ca", "C", "A", "LINK");

    let res = nb(&s, "A", 1, "out");
    assert_eq!(ids(&res), ["B"].into_iter().map(String::from).collect());
}

#[test]
fn direction_incoming() {
    let p = fresh("gt_dir_in");
    let s = open(&p);
    node(&s, "A");
    node(&s, "B");
    node(&s, "C");
    edge(&s, "ab", "A", "B", "LINK");
    edge(&s, "ca", "C", "A", "LINK");

    let res = nb(&s, "A", 1, "in");
    assert_eq!(ids(&res), ["C"].into_iter().map(String::from).collect());
}

#[test]
fn direction_both() {
    let p = fresh("gt_dir_both");
    let s = open(&p);
    node(&s, "A");
    node(&s, "B");
    node(&s, "C");
    edge(&s, "ab", "A", "B", "LINK");
    edge(&s, "ca", "C", "A", "LINK");

    let res = nb(&s, "A", 1, "both");
    assert_eq!(
        ids(&res),
        ["B", "C"]
            .into_iter()
            .map(String::from)
            .collect::<HashSet<_>>()
    );
}

#[test]
fn cycle_safety() {
    let p = fresh("gt_cycle");
    let s = open(&p);
    node(&s, "A");
    node(&s, "B");
    node(&s, "C");
    edge(&s, "ab", "A", "B", "LINK");
    edge(&s, "bc", "B", "C", "LINK");
    edge(&s, "ca", "C", "A", "LINK");

    let res = nb(&s, "A", 10, "out");

    // Must terminate (we are here). The visited set prevents revisits, so the
    // result should contain exactly B and C (A is the seed, never returned).
    assert!(res.len() <= 3, "cycle must not explode; got {}", res.len());
    let found = ids(&res);
    assert!(found.contains("B"));
    assert!(found.contains("C"));
    // No duplicate node ids
    assert_eq!(found.len(), res.len(), "no duplicate nodes in result");
}

#[test]
fn limit_honored() {
    let p = fresh("gt_limit");
    let s = open(&p);
    node(&s, "hub");
    for i in 0..100 {
        let nid = format!("n{}", i);
        node(&s, &nid);
        let eid = format!("e{}", i);
        edge(&s, &eid, "hub", &nid, "HAS");
    }

    let res = s
        .neighbors(
            "hub".to_string(),
            NeighborInput {
                depth: Some(1),
                rel: None,
                rels: None,
                direction: Some("out".to_string()),
                as_of: None,
                include_invalid: None,
                limit: Some(10),
            },
            false,
        )
        .unwrap();

    assert!(
        res.len() <= 10,
        "limit=10 must be honored; got {}",
        res.len()
    );
    assert!(!res.is_empty(), "should return some results with limit=10");
}

#[test]
fn edge_retract_hides_from_current() {
    let p = fresh("gt_retract_hide");
    let s = open(&p);
    node(&s, "A");
    node(&s, "B");
    edge(&s, "e1", "A", "B", "LINK");

    // Confirm edge is visible before retraction
    let before = nb(&s, "A", 1, "out");
    assert_eq!(before.len(), 1);

    s.retract_edge("e1".to_string(), None).unwrap();

    // After retraction, default neighbors should hide the edge
    let after = nb(&s, "A", 1, "out");
    assert!(
        after.is_empty(),
        "retracted edge should be hidden; got {} results",
        after.len()
    );
}

#[test]
fn edge_retract_visible_with_include_invalid() {
    let p = fresh("gt_retract_visible");
    let s = open(&p);
    node(&s, "A");
    node(&s, "B");
    edge(&s, "e1", "A", "B", "LINK");

    s.retract_edge("e1".to_string(), None).unwrap();

    // query() returns all edges matching from/to regardless of retraction
    // (the query implementation scans without filtering valid_to).
    // We verify the edge still exists and has valid_to set.
    let edges = s
        .query(QueryInput {
            from: Some("A".to_string()),
            to: None,
            rel: None,
            as_of: None,
            include_invalid: Some(true),
            limit: None,
        })
        .unwrap();

    assert!(
        !edges.is_empty(),
        "retracted edge should still exist in storage"
    );
    let e1 = edges.iter().find(|e| e.id == "e1");
    assert!(e1.is_some(), "edge e1 must be present");
    assert!(
        e1.unwrap().valid_to.is_some(),
        "retracted edge must have valid_to set"
    );
}

#[test]
fn edge_properties_preserved() {
    let p = fresh("gt_edge_props");
    let s = open(&p);
    node(&s, "A");
    node(&s, "B");

    let props = json!({"weight": 0.5, "tag": "test"});
    s.add_edge(EdgeInput {
        id: Some("ep1".to_string()),
        from: "A".to_string(),
        to: "B".to_string(),
        rel: "WEIGHTED".to_string(),
        props: Some(props.clone()),
        valid_from: None,
        supersede: None,
        impact: None,
        caused_by: None,
    })
    .unwrap();

    let edges = s
        .query(QueryInput {
            from: Some("A".to_string()),
            to: Some("B".to_string()),
            rel: None,
            as_of: None,
            include_invalid: None,
            limit: None,
        })
        .unwrap();

    let found = edges
        .iter()
        .find(|e| e.id == "ep1")
        .expect("edge ep1 must exist");
    assert_eq!(found.props["weight"], json!(0.5));
    assert_eq!(found.props["tag"], json!("test"));
}

#[test]
fn edge_properties_survive_reopen() {
    let p = fresh("gt_edge_props_reopen");
    {
        let s = open(&p);
        node(&s, "A");
        node(&s, "B");
        s.add_edge(EdgeInput {
            id: Some("ep2".to_string()),
            from: "A".to_string(),
            to: "B".to_string(),
            rel: "WEIGHTED".to_string(),
            props: Some(json!({"weight": 0.5, "tag": "test"})),
            valid_from: None,
            supersede: None,
            impact: None,
            caused_by: None,
        })
        .unwrap();
        s.save_state().unwrap();
        // Storage dropped here
    }

    let s2 = open(&p);
    let edges = s2
        .query(QueryInput {
            from: Some("A".to_string()),
            to: Some("B".to_string()),
            rel: None,
            as_of: None,
            include_invalid: None,
            limit: None,
        })
        .unwrap();

    let found = edges
        .iter()
        .find(|e| e.id == "ep2")
        .expect("edge ep2 must survive reopen");
    assert_eq!(found.props["weight"], json!(0.5));
    assert_eq!(found.props["tag"], json!("test"));
}

#[test]
fn large_fanout_stress() {
    let p = fresh("gt_fanout_stress");
    let s = open(&p);
    node(&s, "hub");
    for i in 0..100 {
        let nid = format!("fan{}", i);
        node(&s, &nid);
        let eid = format!("fe{}", i);
        edge(&s, &eid, "hub", &nid, "FAN");
    }

    let res = s
        .neighbors(
            "hub".to_string(),
            NeighborInput {
                depth: Some(1),
                rel: None,
                rels: None,
                direction: Some("out".to_string()),
                as_of: None,
                include_invalid: None,
                limit: Some(50),
            },
            false,
        )
        .unwrap();

    assert_eq!(
        res.len(),
        50,
        "limit=50 over 100-fanout should yield exactly 50; got {}",
        res.len()
    );
}

#[test]
fn multi_hop_path_tracking() {
    let p = fresh("gt_path_track");
    let s = open(&p);
    node(&s, "A");
    node(&s, "B");
    node(&s, "C");
    edge(&s, "ab", "A", "B", "STEP");
    edge(&s, "bc", "B", "C", "STEP");

    let res = nb(&s, "A", 2, "out");
    let c_entry = res.iter().find(|n| n.node.id == "C");
    assert!(c_entry.is_some(), "C must be reachable at depth 2");

    let c = c_entry.unwrap();
    assert_eq!(c.depth, 2, "C should be at depth 2");
    assert_eq!(
        c.path.len(),
        2,
        "path to C should have 2 edges; got {}",
        c.path.len()
    );
    // Verify the path edges are in order: A->B then B->C
    assert_eq!(c.path[0].from, "A");
    assert_eq!(c.path[0].to, "B");
    assert_eq!(c.path[1].from, "B");
    assert_eq!(c.path[1].to, "C");
}

#[test]
fn self_loop_does_not_infinite() {
    let p = fresh("gt_self_loop");
    let s = open(&p);
    node(&s, "A");
    edge(&s, "aa", "A", "A", "SELF");

    // A self-loop edge from A to A: the BFS visited set starts with A,
    // so the edge's far endpoint (A) is already visited and is never enqueued.
    // Traversal should terminate immediately with no results.
    let res = nb(&s, "A", 5, "out");
    assert!(
        res.len() <= 1,
        "self-loop must not cause infinite expansion; got {}",
        res.len()
    );
}
