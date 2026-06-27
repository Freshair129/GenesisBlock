// Concurrent write safety: multiple OS threads calling add_node on the same
// Arc<Storage> must not panic, deadlock, or silently drop nodes. Storage uses
// DashMap for node/edge maps and a Mutex-wrapped WAL writer — both are designed
// for concurrent access — but this test exercises that contract end-to-end.

use genesis_block_native::{Storage, OpenOptions, NodeInput};
use std::sync::Arc;
use std::thread;
use std::fs;
use std::path::Path;

fn fresh(name: &str) -> String {
    let p = format!("{}/{}", env!("CARGO_TARGET_TMPDIR"), name);
    if Path::new(&p).exists() { fs::remove_dir_all(&p).unwrap(); }
    p
}

fn open(path: &str) -> Arc<Storage> {
    Arc::new(Storage::open(OpenOptions {
        path: path.to_string(),
        page_cache_mb: Some(64),
        read_only: Some(false),
        vector_dim: None,
    }).unwrap())
}

/// Eight threads each write 25 unique nodes concurrently; every node must be
/// findable in the interning table afterwards.
#[test]
fn concurrent_add_nodes_no_panic_or_data_loss() {
    const THREADS: usize = 8;
    const PER_THREAD: usize = 25;

    let storage = open(&fresh("test_concurrent_add"));

    let handles: Vec<_> = (0..THREADS).map(|t| {
        let s = Arc::clone(&storage);
        thread::spawn(move || {
            for i in 0..PER_THREAD {
                s.add_node(NodeInput {
                    id: Some(format!("n-{t}-{i}")),
                    labels: vec!["Concurrent".to_string()],
                    props: None,
                    embedding: None,
                    lang: None,
                    valid_from: None,
                    caused_by: None,
                    ttl: None,
                    collection: None,
                }).expect("add_node must not error under concurrent writes");
            }
        })
    }).collect();

    for h in handles {
        h.join().expect("writer thread panicked");
    }

    // Every written node must be retrievable via the interning table.
    let found = (0..THREADS)
        .flat_map(|t| (0..PER_THREAD).map(move |i| format!("n-{t}-{i}")))
        .filter(|id| storage.get_u32(id).is_some())
        .count();

    assert_eq!(
        found,
        THREADS * PER_THREAD,
        "all concurrently-written nodes must be retrievable"
    );
}

/// Concurrent writes across different relation types then an edge traversal —
/// edges created by multiple threads must all appear in the adjacency index.
#[test]
fn concurrent_add_edges_all_indexed() {
    const THREADS: usize = 4;
    const EDGES_PER: usize = 10;

    let storage = open(&fresh("test_concurrent_edges"));

    // Pre-create a shared hub and spoke nodes so threads only add edges.
    storage.add_node(NodeInput {
        id: Some("hub".to_string()),
        labels: vec![],
        props: None,
        embedding: None,
        lang: None,
        valid_from: None,
        caused_by: None,
        ttl: None,
        collection: None,
    }).unwrap();

    for t in 0..THREADS {
        for i in 0..EDGES_PER {
            storage.add_node(NodeInput {
                id: Some(format!("spoke-{t}-{i}")),
                labels: vec![],
                props: None,
                embedding: None,
                lang: None,
                valid_from: None,
                caused_by: None,
                ttl: None,
                collection: None,
            }).unwrap();
        }
    }

    let handles: Vec<_> = (0..THREADS).map(|t| {
        let s = Arc::clone(&storage);
        thread::spawn(move || {
            for i in 0..EDGES_PER {
                s.add_edge(genesis_block_native::EdgeInput {
                    id: Some(format!("e-{t}-{i}")),
                    from: "hub".to_string(),
                    to: format!("spoke-{t}-{i}"),
                    rel: "CONNECTS".to_string(),
                    props: None,
                    valid_from: None,
                    supersede: None,
                    impact: None,
                    caused_by: None,
                }).expect("add_edge must not error under concurrent writes");
            }
        })
    }).collect();

    for h in handles {
        h.join().expect("edge writer thread panicked");
    }

    // All edges from hub must be traversable.
    use genesis_block_native::NeighborInput;
    let neighbors = storage.neighbors(
        "hub".to_string(),
        NeighborInput {
            depth: Some(1),
            rel: Some("CONNECTS".to_string()),
            rels: None,
            direction: Some("out".to_string()),
            as_of: None,
            include_invalid: None,
            limit: None,
        },
        false,
    ).unwrap();

    assert_eq!(
        neighbors.len(),
        THREADS * EDGES_PER,
        "all concurrently-added edges must appear in the adjacency index"
    );
}
