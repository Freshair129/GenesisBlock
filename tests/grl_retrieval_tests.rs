use genesis_block_native::{ContextPackage, EdgeInput, NodeInput, OpenOptions, Storage};
use serde_json::json;
use tempfile::tempdir;

#[test]
fn test_grl_context_retrieval_tiered() {
    let dir = tempdir().unwrap();
    let storage = Storage::open(OpenOptions {
        path: dir.path().to_str().unwrap().to_string(),
        page_cache_mb: Some(64),
        read_only: Some(false),
        vector_dim: Some(1536),
    })
    .unwrap();

    // 1. Setup a small graph: A -> B -> C
    storage
        .add_node(NodeInput {
            id: Some("A".to_string()),
            labels: vec!["USER".to_string()],
            props: Some(json!({"text": "Node A content"})),
            embedding: None,
            lang: None,
            valid_from: None,
            caused_by: None,
            ttl: None,
            collection: None,
        })
        .unwrap();

    storage
        .add_node(NodeInput {
            id: Some("B".to_string()),
            labels: vec!["USER".to_string()],
            props: Some(json!({"text": "Node B content"})),
            embedding: None,
            lang: None,
            valid_from: None,
            caused_by: None,
            ttl: None,
            collection: None,
        })
        .unwrap();

    storage
        .add_node(NodeInput {
            id: Some("C".to_string()),
            labels: vec!["USER".to_string()],
            props: Some(json!({"text": "Node C content"})),
            embedding: None,
            lang: None,
            valid_from: None,
            caused_by: None,
            ttl: None,
            collection: None,
        })
        .unwrap();

    storage
        .add_edge(EdgeInput {
            id: None,
            from: "A".to_string(),
            to: "B".to_string(),
            rel: "knows".to_string(),
            props: None,
            valid_from: None,
            supersede: None,
            impact: None,
            caused_by: None,
        })
        .unwrap();

    storage
        .add_edge(EdgeInput {
            id: None,
            from: "B".to_string(),
            to: "C".to_string(),
            rel: "knows".to_string(),
            props: None,
            valid_from: None,
            supersede: None,
            impact: None,
            caused_by: None,
        })
        .unwrap();

    // 2. Test TIER H0 (Self Only)
    let ctx_h0 = storage.retrieve_context("A", "H0", None, false).unwrap();
    assert_eq!(ctx_h0.nodes.len(), 1);
    assert_eq!(ctx_h0.nodes[0].id, "A");
    assert_eq!(ctx_h0.edges.len(), 0);

    // 3. Test TIER H1 (Neighbors)
    let ctx_h1 = storage.retrieve_context("A", "H1", None, false).unwrap();
    // Should include A and B (neighbor)
    assert!(ctx_h1.nodes.iter().any(|n| n.id == "A"));
    assert!(ctx_h1.nodes.iter().any(|n| n.id == "B"));
    assert!(!ctx_h1.nodes.iter().any(|n| n.id == "C"));

    // 4. Test TIER H2 (Feature)
    let ctx_h2 = storage.retrieve_context("A", "H2", None, false).unwrap();
    // Should include A, B, and C
    assert_eq!(ctx_h2.nodes.len(), 3);
}

#[test]
fn test_grl_budget_compression() {
    let dir = tempdir().unwrap();
    let storage = Storage::open(OpenOptions {
        path: dir.path().to_str().unwrap().to_string(),
        page_cache_mb: Some(64),
        read_only: Some(false),
        vector_dim: Some(1536),
    })
    .unwrap();

    // Add some nodes and metadata to allow SuperNode generation
    storage
        .add_node(NodeInput {
            id: Some("A".to_string()),
            labels: vec!["USER".to_string()],
            props: Some(json!({"large": "x".repeat(100)})),
            embedding: Some(vec![0.1; 1536]),
            lang: None,
            valid_from: None,
            caused_by: None,
            ttl: None,
            collection: None,
        })
        .unwrap();

    storage.detect_communities().unwrap();
    storage.generate_meta_graph().unwrap();

    // Test with low budget (triggering compression)
    let ctx_low = storage
        .retrieve_context("A", "H2", Some(10), false)
        .unwrap();
    assert!(
        ctx_low.nodes.is_empty(),
        "Nodes should be pruned due to budget"
    );
    assert!(
        !ctx_low.super_nodes.is_empty(),
        "SuperNodes should be returned as fallback"
    );
    println!("Budget fallback triggered correctly.");
}

#[test]
fn test_hql_context_command() {
    let dir = tempdir().unwrap();
    let storage = Storage::open(OpenOptions {
        path: dir.path().to_str().unwrap().to_string(),
        page_cache_mb: Some(64),
        read_only: Some(false),
        vector_dim: Some(1536),
    })
    .unwrap();

    storage
        .add_node(NodeInput {
            id: Some("Target".to_string()),
            labels: vec!["USER".to_string()],
            props: None,
            embedding: None,
            lang: None,
            valid_from: None,
            caused_by: None,
            ttl: None,
            collection: None,
        })
        .unwrap();

    // Test HQL CONTEXT
    let hql_res = storage
        .execute_hql("CONTEXT FOR Target TIER H1 BUDGET 5000")
        .unwrap();
    let ctx: ContextPackage = serde_json::from_value(hql_res).unwrap();

    assert_eq!(ctx.nodes.len(), 1);
    assert_eq!(ctx.nodes[0].id, "Target");
    assert!(ctx.reasoning_path.contains("H1"));

    println!("HQL CONTEXT command verified.");
}

// ---------------------------------------------------------------------------
// H6 tier + CoverageReport
// ---------------------------------------------------------------------------

fn coverage_storage(dir: &tempfile::TempDir) -> Storage {
    Storage::open(OpenOptions {
        path: dir.path().to_str().unwrap().to_string(),
        page_cache_mb: Some(64),
        read_only: Some(false),
        vector_dim: Some(1536),
    })
    .unwrap()
}

fn add_plain_node(storage: &Storage, id: &str) {
    storage
        .add_node(NodeInput {
            id: Some(id.to_string()),
            labels: vec!["USER".to_string()],
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

fn link(storage: &Storage, from: &str, to: &str) {
    storage
        .add_edge(EdgeInput {
            id: None,
            from: from.to_string(),
            to: to.to_string(),
            rel: "knows".to_string(),
            props: None,
            valid_from: None,
            supersede: None,
            impact: None,
            caused_by: None,
        })
        .unwrap();
}

/// Builds the chain N0 -> N1 -> ... -> N{len} (len edges).
fn chain(storage: &Storage, len: usize) {
    for i in 0..=len {
        add_plain_node(storage, &format!("N{i}"));
    }
    for i in 0..len {
        link(storage, &format!("N{i}"), &format!("N{}", i + 1));
    }
}

#[test]
fn test_grl_tier_h6_ceiling() {
    // H6 resolves exactly 6 hops, no more. Chain N0..N6 (6 edges): H6 reaches
    // N6, H5 stops at N5.
    let dir = tempdir().unwrap();
    let storage = coverage_storage(&dir);
    chain(&storage, 6);

    let ctx_h5 = storage.retrieve_context("N0", "H5", None, false).unwrap();
    assert!(ctx_h5.nodes.iter().any(|n| n.id == "N5"));
    assert!(!ctx_h5.nodes.iter().any(|n| n.id == "N6"));
    assert_eq!(ctx_h5.coverage.hops_requested, 5);
    assert_eq!(ctx_h5.coverage.hops_served, 5);
    // The chain continues past hop 5 (N6 exists), so this IS a real ceiling.
    assert!(ctx_h5.coverage.ceiling_hit);

    let ctx_h6 = storage.retrieve_context("N0", "H6", None, false).unwrap();
    assert!(ctx_h6.nodes.iter().any(|n| n.id == "N6"));
    assert_eq!(ctx_h6.nodes.len(), 7);
    assert_eq!(ctx_h6.coverage.hops_requested, 6);
    assert_eq!(ctx_h6.coverage.hops_served, 6);

    // Regression: N6 sits exactly at the boundary but is a leaf — the graph
    // does NOT continue past it, so this is complete coverage, not a truncated
    // frontier. A frontier check that only asks "was this node expanded?"
    // reports true here, which is wrong.
    assert!(
        !ctx_h6.coverage.ceiling_hit,
        "boundary leaf must not report a ceiling: the chain ends at N6"
    );

    assert!(storage.execute_hql("CONTEXT FOR N0 TIER H6").is_ok());
}

#[test]
fn test_grl_ceiling_signal_deep_graph() {
    // A chain longer than the requested tier reports ceiling_hit=true.
    let dir = tempdir().unwrap();
    let storage = coverage_storage(&dir);
    chain(&storage, 7);

    let ctx = storage.retrieve_context("N0", "H3", None, false).unwrap();
    assert_eq!(ctx.coverage.hops_requested, 3);
    assert_eq!(ctx.coverage.hops_served, 3);
    assert!(
        ctx.coverage.ceiling_hit,
        "expected ceiling_hit=true: chain extends past the H3 boundary"
    );
}

#[test]
fn test_grl_ceiling_signal_shallow_graph() {
    // A subgraph exhausted before the requested tier must not report a ceiling.
    let dir = tempdir().unwrap();
    let storage = coverage_storage(&dir);
    add_plain_node(&storage, "A");
    add_plain_node(&storage, "B");
    link(&storage, "A", "B");

    let ctx = storage.retrieve_context("A", "H5", None, false).unwrap();
    assert!(
        !ctx.coverage.ceiling_hit,
        "expected ceiling_hit=false: 2-node graph is exhausted before the H5 boundary"
    );
    assert!(ctx.coverage.hops_served <= 1);
}

#[test]
fn test_grl_coverage_h0_isolated_node_reports_complete() {
    // Regression, zero-hop: at H0 the target is popped at depth 0 >= hops 0 and
    // is never expanded. An isolated node has no graph beyond it, so coverage is
    // complete. Reporting ceiling_hit here would contradict the field's own
    // documented meaning.
    let dir = tempdir().unwrap();
    let storage = coverage_storage(&dir);
    add_plain_node(&storage, "LONE");

    let ctx = storage.retrieve_context("LONE", "H0", None, false).unwrap();
    assert_eq!(ctx.coverage.hops_requested, 0);
    assert_eq!(ctx.coverage.hops_served, 0);
    assert!(
        !ctx.coverage.ceiling_hit,
        "isolated node at H0 has no graph beyond it"
    );
    assert!(!ctx.coverage.truncated);
}

#[test]
fn test_grl_coverage_h0_with_neighbor_reports_ceiling() {
    // The other half of the zero-hop case: H0 on a node that does have an
    // unreturned neighbour is a genuine ceiling. Pairing this with the isolated
    // case is what pins the distinction between "stopped" and "truncated".
    let dir = tempdir().unwrap();
    let storage = coverage_storage(&dir);
    add_plain_node(&storage, "A");
    add_plain_node(&storage, "B");
    link(&storage, "A", "B");

    let ctx = storage.retrieve_context("A", "H0", None, false).unwrap();
    assert_eq!(ctx.coverage.hops_served, 0);
    assert!(
        ctx.coverage.ceiling_hit,
        "H0 on a connected node must report the frontier it did not cross"
    );
}

#[test]
fn test_grl_coverage_ignores_disconnected_components() {
    // An unrelated component elsewhere in the store is not "graph beyond the
    // boundary" for this target — it was never reachable.
    let dir = tempdir().unwrap();
    let storage = coverage_storage(&dir);
    add_plain_node(&storage, "A");
    add_plain_node(&storage, "B");
    link(&storage, "A", "B");
    add_plain_node(&storage, "X");
    add_plain_node(&storage, "Y");
    link(&storage, "X", "Y");

    let ctx = storage.retrieve_context("A", "H4", None, false).unwrap();
    assert!(!ctx.coverage.ceiling_hit);
    assert!(!ctx.nodes.iter().any(|n| n.id == "X"));
}

#[test]
fn test_grl_coverage_reports_truncation_under_budget() {
    // `truncated` tracks budget/SuperNode compression, independently of the
    // tier-boundary signal.
    let dir = tempdir().unwrap();
    let storage = coverage_storage(&dir);
    for i in 0..12 {
        storage
            .add_node(NodeInput {
                id: Some(format!("B{i}")),
                labels: vec!["USER".to_string()],
                props: Some(json!({ "blob": "x".repeat(2000) })),
                embedding: None,
                lang: None,
                valid_from: None,
                caused_by: None,
                ttl: None,
                collection: None,
            })
            .unwrap();
    }
    for i in 0..11 {
        link(&storage, &format!("B{i}"), &format!("B{}", i + 1));
    }

    let ctx = storage
        .retrieve_context("B0", "H3", Some(10), false)
        .unwrap();
    assert!(
        ctx.coverage.truncated,
        "budget compression must be reported in coverage.truncated"
    );
}
