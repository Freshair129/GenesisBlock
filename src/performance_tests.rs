#[cfg(test)]
mod performance_tests {
    use crate::*;
    use std::time::Instant;
    use tempfile::tempdir;

    fn setup_large_graph(storage: &mut Storage, node_count: usize) {
        for i in 0..node_count {
            let id = format!("CONCEPT--NODE-{}", i);
            storage.add_node(NodeInput {
                id: Some(id),
                labels: vec!["Test".to_string()],
                props: Some(serde_json::json!({"status": "active"})),
            }).unwrap();
        }

        // Create a tree structure where each node i points to 2*i + 1 and 2*i + 2
        for i in 0..(node_count / 2) {
            let from = format!("CONCEPT--NODE-{}", i);
            let left = 2 * i + 1;
            let right = 2 * i + 2;

            if left < node_count {
                storage.add_edge(EdgeInput {
                    id: None,
                    from: from.clone(),
                    to: format!("CONCEPT--NODE-{}", left),
                    rel: "depends_on".to_string(),
                    props: None,
                    valid_from: None,
                    supersede: None,
                    impact: None,
                }).unwrap();
            }
            if right < node_count {
                storage.add_edge(EdgeInput {
                    id: None,
                    from,
                    to: format!("CONCEPT--NODE-{}", right),
                    rel: "depends_on".to_string(),
                    props: None,
                    valid_from: None,
                    supersede: None,
                    impact: None,
                }).unwrap();
            }
        }
    }

    #[test]
    fn test_incremental_impact_performance() {
        let dir = tempdir().unwrap();
        let opts = OpenOptions {
            path: dir.path().to_str().unwrap().to_string(),
            page_cache_mb: Some(64),
            read_only: Some(false),
        };
        let mut storage = Storage::open(opts).unwrap();

        let node_count = 2000;
        setup_large_graph(&mut storage, node_count);

        // Baseline: Full refresh
        let start_full = Instant::now();
        storage.refresh_impacts(None);
        let duration_full = start_full.elapsed();
        println!("Full refresh ({} nodes): {:?}", node_count, duration_full);

        // Incremental: Leaf node change (should affect very few/no downstream nodes in this tree)
        // In our tree, nodes point to children, so a leaf has no out-edges.
        // Wait, impact calculation (compute_impact) depends on incoming edges (calculate_dd).
        // If we change a node, we need to refresh ITS impact, and its DOWNSTREAM dependents.
        // In our tree (from -> to), if we change Node 0, it affects everyone.
        // If we change a leaf (e.g. Node 1999), it affects ONLY itself.
        
        let target_node = format!("CONCEPT--NODE-{}", node_count - 1);
        let start_inc = Instant::now();
        storage.refresh_impacts(Some(vec![target_node]));
        let duration_inc = start_inc.elapsed();
        println!("Incremental refresh (leaf node): {:?}", duration_inc);

        assert!(duration_inc < duration_full, "Incremental leaf refresh ({:?}) should be faster than full refresh ({:?})", duration_inc, duration_full);
    }

    #[test]
    fn test_axiomatic_guard_enforcement() {
        let dir = tempdir().unwrap();
        let opts = OpenOptions {
            path: dir.path().to_str().unwrap().to_string(),
            page_cache_mb: Some(64),
            read_only: Some(false),
        };
        let mut storage = Storage::open(opts).unwrap();

        // 1. Setup tiers
        storage.add_node(NodeInput {
            id: Some("MASTER--ROOT".to_string()),
            labels: vec!["Master".to_string()],
            props: None,
        }).unwrap();

        storage.add_node(NodeInput {
            id: Some("CONCEPT--IDEA".to_string()),
            labels: vec!["Concept".to_string()],
            props: None,
        }).unwrap();

        // 2. Test: Low tier (Concept) cannot supersede high tier (Master)
        let result = storage.add_edge(EdgeInput {
            id: None,
            from: "CONCEPT--IDEA".to_string(),
            to: "MASTER--ROOT".to_string(),
            rel: "supersedes".to_string(),
            props: None,
            valid_from: None,
            supersede: Some(true),
            impact: None,
        });

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Axiomatic Guard"));

        // 3. Test: Master CAN supersede Concept
        let result_ok = storage.add_edge(EdgeInput {
            id: None,
            from: "MASTER--ROOT".to_string(),
            to: "CONCEPT--IDEA".to_string(),
            rel: "supersedes".to_string(),
            props: None,
            valid_from: None,
            supersede: Some(true),
            impact: None,
        });

        assert!(result_ok.is_ok());
    }
}
