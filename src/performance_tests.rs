#[cfg(test)]
mod performance_tests {
    use crate::*;
    use std::time::Instant;
    use tempfile::tempdir;
    use serde_json::json;

    fn setup_test_db() -> (Storage, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let opts = OpenOptions {
            path: dir.path().to_str().unwrap().to_string(),
            page_cache_mb: Some(128),
            read_only: Some(false),
        };
        let storage = Storage::open(opts).unwrap();
        (storage, dir)
    }

    fn setup_large_graph(storage: &mut Storage, node_count: usize) -> Vec<String> {
        let mut node_ids = Vec::with_capacity(node_count);
        
        // Create nodes
        for i in 0..node_count {
            let id = format!("NODE-{:06}", i);
            storage.add_node(NodeInput {
                id: Some(id.clone()),
                labels: vec!["Test".to_string(), if i % 10 == 0 { "MASTER".to_string() } else { "CONCEPT".to_string() }],
                props: Some(json!({"status": "active", "index": i})),
                embedding: if i % 5 == 0 { 
                    Some((0..1536).map(|x| (x as f64) * 0.001).collect()) 
                } else { None },
                lang: Some("th".to_string()),
                valid_from: None,
                caused_by: None,
                ttl: None,
            }).unwrap();
            node_ids.push(id);
        }

        // Create tree + some cross edges
        for i in 0..(node_count / 3) {
            let from = &node_ids[i];
            // Child edges
            if 2 * i + 1 < node_count {
                storage.add_edge(EdgeInput {
                    id: None,
                    from: from.clone(),
                    to: node_ids[2 * i + 1].clone(),
                    rel: "depends_on".to_string(),
                    props: None,
                    valid_from: None,
                    supersede: None,
                    impact: None,
                    caused_by: None,
                }).unwrap();
            }
            if 2 * i + 2 < node_count {
                storage.add_edge(EdgeInput {
                    id: None,
                    from: from.clone(),
                    to: node_ids[2 * i + 2].clone(),
                    rel: "depends_on".to_string(),
                    props: None,
                    valid_from: None,
                    supersede: None,
                    impact: None,
                    caused_by: None,
                }).unwrap();
            }
            // Some cross links
            if i % 7 == 0 && i + 100 < node_count {
                storage.add_edge(EdgeInput {
                    id: None,
                    from: from.clone(),
                    to: node_ids[i + 100].clone(),
                    rel: "related_to".to_string(),
                    props: None,
                    valid_from: None,
                    supersede: None,
                    impact: None,
                    caused_by: None,
                }).unwrap();
            }
        }
        node_ids
    }

    #[test]
    fn benchmark_scalability() {
        println!("\n=== GenesisDB Performance Benchmark (Mark VII) ===");

        let node_sizes = vec![1_000, 5_000, 10_000];
        
        for &size in &node_sizes {
            let (mut storage, _dir) = setup_test_db();
            let start = Instant::now();
            let node_ids = setup_large_graph(&mut storage, size);
            let ingest_time = start.elapsed();

            println!("{} nodes ingested in {:?}", size, ingest_time);
            println!("   TPS: {:.2}", size as f64 / ingest_time.as_secs_f64());

            // Hybrid Search
            let search_start = Instant::now();
            let results = storage.hybrid_search(HybridSearchInput {
                query_vector: vec![0.1; 1536],
                k: 10,
                alpha: Some(0.4),
                lang: Some("th".to_string()),
                as_of: None,
            }).unwrap();
            let search_time = search_start.elapsed();
            println!("   Hybrid Search (k=10): {:?}", search_time);

            // Traversal
            let trav_start = Instant::now();
            let _ = storage.neighbors(node_ids[0].clone(), NeighborInput {
                depth: Some(4),
                rel: None,
                rels: None,
                direction: Some("out".to_string()),
                as_of: None,
                include_invalid: Some(false),
                limit: Some(50),
            }, false).unwrap();
            let trav_time = trav_start.elapsed();
            println!("   Traversal depth=4: {:?}", trav_time);

            // Autonomic
            let auto_start = Instant::now();
            let _ = storage.perform_autonomic_optimization();
            let auto_time = auto_start.elapsed();
            println!("   Autonomic Optimization: {:?}", auto_time);

            println!("----------------------------------------");
        }
    }

    #[test]
    fn benchmark_fuzzy_search() {
        let (mut storage, _dir) = setup_test_db();
        setup_large_graph(&mut storage, 5000);

        let queries = vec!["NODE-01234", "NODE-04567", "CONCEPT--NODE", "NODE-99999"];

        for q in queries {
            let start = Instant::now();
            let result = storage.find_fuzzy_id(q);
            let duration = start.elapsed();
            println!("Fuzzy search '{q}' -> {:?} in {:?}", result, duration);
        }
    }

    #[test]
    fn benchmark_temporal_operations() {
        let (mut storage, _dir) = setup_test_db();
        let node_ids = setup_large_graph(&mut storage, 2000);

        // Supersede benchmark
        let start = Instant::now();
        for i in 0..100 {
            let _ = storage.supersede_node(
                node_ids[i].clone(),
                Some(json!({"status": "updated", "version": i})),
                Some("test-benchmark".to_string())
            ).unwrap();
        }
        println!("100 supersede_node operations: {:?}", start.elapsed());

        // Reconcile benchmark
        let events: Vec<Event> = (0..50).map(|i| {
            Event::Node(NodeOutput {
                id: format!("REMOTE-{}", i),
                labels: vec!["Test".to_string()],
                props: json!({"remote": true}),
                impact: Some(0.8),
                embedding: None,
                lang: Some("en".to_string()),
                valid_from: chrono::Utc::now().to_rfc3339(),
                valid_to: None,
                caused_by: None,
                expires_at: None,
                clock: LogicalClock { time: i as u32, peer_id: "peer-1".to_string() },
            })
       