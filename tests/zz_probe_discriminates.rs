//! Temporary experiment. Does a self-recall probe discriminate a bad HNSW graph
//! from a good one? If a bad graph still returns every vector as its own
//! nearest neighbour, the probe is useless and the feature must not be built on it.

use genesis_block_native::{HybridSearchInput, NodeInput, OpenOptions, Storage};
use std::fs;
use std::path::Path;

fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}
fn coord(state: &mut u64) -> f64 {
    let u = (splitmix64(state) >> 11) as f64 / (1u64 << 53) as f64;
    u * 2.0 - 1.0
}

#[test]
fn probe_vs_recall() {
    const BUILDS: usize = 60;
    const PROBE_K: usize = 32;

    for build in 0..BUILDS {
        let p = format!("{}/zz_probe_{build}", env!("CARGO_TARGET_TMPDIR"));
        if Path::new(&p).exists() {
            fs::remove_dir_all(&p).unwrap();
        }
        let s = Storage::open(OpenOptions {
            path: p.clone(),
            page_cache_mb: Some(64),
            read_only: Some(false),
            vector_dim: Some(8),
            retention: None,
        })
        .unwrap();

        let mut seed = 0x1234_5678_9ABC_DEF0u64;
        let mut vectors: Vec<Vec<f64>> = Vec::with_capacity(1000);
        for _ in 0..1000 {
            vectors.push((0..8).map(|_| coord(&mut seed)).collect());
        }
        for (i, emb) in vectors.iter().enumerate() {
            s.add_node(NodeInput {
                id: Some(format!("r{i}")),
                labels: vec!["V".into()],
                props: None,
                embedding: Some(emb.clone()),
                lang: None,
                valid_from: None,
                caused_by: None,
                ttl: None,
                collection: None,
            })
            .unwrap();
        }
        s.flush_index();

        // GROUND TRUTH: the same recall@10 the guard measures.
        let queries: Vec<Vec<f64>> = (0..50)
            .map(|_| (0..8).map(|_| coord(&mut seed)).collect())
            .collect();
        let l2 = |a: &[f64], b: &[f64]| -> f64 {
            a.iter().zip(b).map(|(x, y)| (x - y) * (x - y)).sum::<f64>()
        };
        let mut total = 0.0;
        for q in &queries {
            let mut d: Vec<(usize, f64)> = vectors
                .iter()
                .enumerate()
                .map(|(i, v)| (i, l2(q, v)))
                .collect();
            d.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
            let truth: Vec<String> = d.iter().take(10).map(|(i, _)| format!("r{i}")).collect();
            let got = s
                .hybrid_search(HybridSearchInput {
                    query_vector: q.clone(),
                    k: 10,
                    alpha: Some(0.0),
                    lang: None,
                    as_of: None,
                    collection: None,
                    ef_search: Some(200),
                    oversample: None,
                })
                .unwrap();
            total += got.iter().filter(|n| truth.contains(&n.node.id)).count() as f64 / 10.0;
        }
        let recall = total / queries.len() as f64;

        // THE PROBE UNDER TEST: ask k indexed vectors to find themselves.
        // Cheap (k searches), needs no ground truth, and targets navigability
        // rather than completeness - which is what actually fails.
        let step = vectors.len() / PROBE_K;
        let mut self_hits = 0usize;
        let mut self_top1 = 0usize;
        for j in 0..PROBE_K {
            let i = j * step;
            let got = s
                .hybrid_search(HybridSearchInput {
                    query_vector: vectors[i].clone(),
                    k: 10,
                    alpha: Some(0.0),
                    lang: None,
                    as_of: None,
                    collection: None,
                    ef_search: Some(64),
                    oversample: None,
                })
                .unwrap();
            let me = format!("r{i}");
            if got.iter().any(|n| n.node.id == me) {
                self_hits += 1;
            }
            if got.first().map(|n| n.node.id.as_str()) == Some(me.as_str()) {
                self_top1 += 1;
            }
        }
        println!(
            "PROBE build={build} recall={recall:.4} self_in_top10={}/{PROBE_K} self_is_top1={}/{PROBE_K}",
            self_hits, self_top1
        );
    }
}
