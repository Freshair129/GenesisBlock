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
fn test_grl_tier_h6_ceiling() {
    // H6 is the single-agent context ceiling: exactly 6 hops, no more.
    // Build a 7-node chain N0 -> N1 -> ... -> N6 (6 edges) and assert H6
    // reaches N6 while H5 stops at N5.
    let dir = tempdir().unwrap();
    let storage = Storage::open(OpenOptions {
        path: dir.path().to_str().unwrap().to_string(),
        page_cache_mb: Some(64),
        read_only: Some(false),
        vector_dim: Some(1536),
    })
    .unwrap();

    for i in 0..=6 {
        storage
            .add_node(NodeInput {
                id: Some(format!("N{i}")),
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
    for i in 0..6 {
        storage
            .add_edge(EdgeInput {
                id: None,
                from: format!("N{i}"),
                to: format!("N{}", i + 1),
                rel: "knows".to_string(),
                props: None,
                valid_from: None,
                supersede: None,
                impact: None,
                caused_by: None,
            })
            .unwrap();
    }

    // H5 reaches N5 (5 hops) but not the 6th-hop node N6.
    let ctx_h5 = storage.retrieve_context("N0", "H5", None, false).unwrap();
    assert!(ctx_h5.nodes.iter().any(|n| n.id == "N5"));
    assert!(!ctx_h5.nodes.iter().any(|n| n.id == "N6"));

    // H6 reaches the full 6-hop radius including N6 (the ceiling).
    let ctx_h6 = storage.retrieve_context("N0", "H6", None, false).unwrap();
    assert!(ctx_h6.nodes.iter().any(|n| n.id == "N6"));
    assert_eq!(ctx_h6.nodes.len(), 7);

    // HQL surface: TIER H6 parses and executes.
    assert!(storage.execute_hql("CONTEXT FOR N0 TIER H6").is_ok());
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

#[test]
fn test_grl_ceiling_signal_deep_graph() {
    // A chain longer than the requested tier must report ceiling_hit=true:
    // the BFS frontier still has an undiscovered/unexpanded neighbor sitting
    // right at the tier boundary. Build an 8-node chain N0->N1->...->N7 (7
    // edges) and query TIER H3 (3 hops) from N0 — the chain continues well
    // past hop 3.
    let dir = tempdir().unwrap();
    let storage = Storage::open(OpenOptions {
        path: dir.path().to_str().unwrap().to_string(),
        page_cache_mb: Some(64),
        read_only: Some(false),
        vector_dim: Some(1536),
    })
    .unwrap();

    for i in 0..=7 {
        storage
            .add_node(NodeInput {
                id: Some(format!("N{i}")),
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
    for i in 0..7 {
        storage
            .add_edge(EdgeInput {
                id: None,
                from: format!("N{i}"),
                to: format!("N{}", i + 1),
                rel: "knows".to_string(),
                props: None,
                valid_from: None,
                supersede: None,
                impact: None,
                caused_by: None,
            })
            .unwrap();
    }

    let ctx = storage.retrieve_context("N0", "H3", None, false).unwrap();
    assert_eq!(ctx.hops_requested, 3);
    assert_eq!(ctx.hops_served, 3);
    assert!(
        ctx.ceiling_hit,
        "expected ceiling_hit=true: chain extends past the H3 boundary"
    );
}

#[test]
fn test_grl_ceiling_signal_shallow_graph() {
    // A subgraph that's exhausted well before the requested tier must NOT
    // report ceiling_hit: there's no more graph beyond what was returned.
    // A -> B is a 2-node graph; TIER H5 (5 hops) has nothing left to expand
    // after depth 1.
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
            id: Some("A".to_string()),
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
    storage
        .add_node(NodeInput {
            id: Some("B".to_string()),
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

    let ctx = storage.retrieve_context("A", "H5", None, false).unwrap();
    assert!(
        !ctx.ceiling_hit,
        "expected ceiling_hit=false: 2-node graph is exhausted before the H5 boundary"
    );
    assert!(ctx.hops_served <= 1);
}
