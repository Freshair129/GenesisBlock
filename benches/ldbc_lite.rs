use criterion::{criterion_group, criterion_main, Criterion};
use genesis_block_native::*;
use tempfile::tempdir;
use std::hint::black_box;

fn setup_ldbc_graph(storage: &mut Storage, node_count: usize, fan_out: usize) {
    for i in 0..node_count {
        storage.add_node(NodeInput { embedding: None,
            id: Some(format!("N-{}", i)),
            labels: vec!["Entity".to_string()],
            props: None,
        }).unwrap();
    }

    for i in 0..node_count {
        for _ in 0..fan_out {
            let to = rand::random_range(0..node_count);
            storage.add_edge(EdgeInput {
                id: None,
                from: format!("N-{}", i),
                to: format!("N-{}", to),
                rel: "connects".to_string(),
                props: None,
                valid_from: None,
                supersede: None,
                impact: None,
            }).ok();
        }
    }
}

fn bench_traversal(c: &mut Criterion) {
    let dir = tempdir().unwrap();
    let opts = OpenOptions {
        path: dir.path().to_str().unwrap().to_string(),
        page_cache_mb: Some(128),
        read_only: Some(false),
    };
    let mut storage = Storage::open(opts).unwrap();
    
    let node_count = 1000; // Reduced for faster benchmarking
    setup_ldbc_graph(&mut storage, node_count, 5);

    let mut group = c.benchmark_group("LDBC-Lite Traversal");
    
    for depth in [black_box(1), black_box(2), black_box(3)] {
        group.bench_function(format!("{}-hop Latency", depth), |b| {
            b.iter(|| {
                let seed = format!("N-{}", 100);
                storage.neighbors(seed, NeighborInput {
                    depth: Some(depth),
                    rel: None,
                    rels: None,
                    direction: Some("out".to_string()),
                    as_of: None,
                    include_invalid: Some(false),
                    limit: Some(100),
                }).ok();
            });
        });
    }
    group.finish();
}

criterion_group!(benches, bench_traversal);
criterion_main!(benches);
