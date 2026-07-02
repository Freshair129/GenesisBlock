// ADR--GENESISDB-HNSW-CAPACITY: a freshly-created collection's HNSW index must
// not eagerly reserve ~100+ MB. hnsw_rs sizes its per-layer pointer tables from
// `max_elements`; the old hardcoded `1_000_000` compounded across layers to
// >100 MB *per index*. With many collections (multi-collection) or many DBs open
// at once those reservations stacked and aborted the process on OOM
// (Windows: 0xC0000409). The fix sizes the reservation to the data.
//
// This is a deterministic reproduction: opening N collections in ONE process,
// each with its own index, allocated N × ~130 MB before the fix — a guaranteed
// OOM well before N=64 — and is trivially cheap after it.

use genesis_block_native::{HybridSearchInput, NodeInput, OpenOptions, Storage};
use std::fs;
use std::path::Path;

fn fresh(name: &str) -> String {
    let p = format!("{}/{}", env!("CARGO_TARGET_TMPDIR"), name);
    if Path::new(&p).exists() {
        fs::remove_dir_all(&p).unwrap();
    }
    p
}

fn open_dim(path: &str, dim: u32) -> Storage {
    Storage::open(OpenOptions {
        path: path.to_string(),
        page_cache_mb: Some(64),
        read_only: Some(false),
        vector_dim: Some(dim),
    })
    .unwrap()
}

/// 64 collections, each with its own index + one vector, coexist in a single
/// process and all stay searchable. Pre-fix this OOM-aborted from stacked
/// ~130 MB-per-index reservations (~8 GB); post-fix each index reserves the
/// floor (1024 elements) and grows on demand.
#[test]
fn many_collection_indexes_do_not_oom() {
    let dim = 8usize;
    let s = open_dim(&fresh("test_hnsw_cap_many"), dim as u32);

    for i in 0..64 {
        let name = format!("c{i}");
        s.create_collection(
            name.clone(),
            "m".to_string(),
            dim as u32,
            None,
            None,
            None,
            None,
        )
        .unwrap();
        // A distinct one-hot vector per collection so we can verify isolation.
        let mut emb = vec![0.0f64; dim];
        emb[i % dim] = 1.0 + i as f64; // unique direction/magnitude
        s.add_node(NodeInput {
            id: Some(format!("n{i}")),
            labels: vec![],
            props: None,
            embedding: Some(emb),
            lang: None,
            valid_from: None,
            caused_by: None,
            ttl: None,
            collection: Some(name),
        })
        .unwrap();
    }
    s.flush_index();

    // Every collection's index built and is searchable in isolation.
    for i in 0..64 {
        let mut q = vec![0.0f64; dim];
        q[i % dim] = 1.0 + i as f64;
        let hits = s
            .hybrid_search(HybridSearchInput {
                query_vector: q,
                k: 1,
                alpha: Some(0.0),
                lang: None,
                as_of: None,
                collection: Some(format!("c{i}")),
                ef_search: None,
                oversample: None,
            })
            .unwrap();
        assert_eq!(hits.len(), 1, "collection c{i} returns its single vector");
        assert_eq!(hits[0].node.id, format!("n{i}"));
    }
}

/// A deterministic embedding for id `i` (no RNG: reproducible).
fn emb_for(i: u32, dim: usize) -> Vec<f64> {
    (0..dim)
        .map(|j| {
            (((i.wrapping_mul(2_654_435_761)
                .wrapping_add(j as u32 * 40_503))
                % 1000) as f64)
                / 1000.0
        })
        .collect()
}

/// An index lazily created at the floor (1024) still grows correctly well past
/// it — guards the "size to data, grow on demand" contract: every vector stays
/// inserted+indexed after the index outgrows its initial reservation, and the
/// grown index is still queryable (doesn't corrupt / lose points on realloc).
#[test]
fn index_grows_past_initial_floor() {
    let dim = 8usize;
    let s = open_dim(&fresh("test_hnsw_cap_grow"), dim as u32);
    let n = 1500u32; // > HNSW_MIN_CAP (1024): forces growth beyond the reservation
                     // Batch insert (one WAL append + parallel_insert) — exercises growth via the
                     // bulk path and avoids per-node fsync (fast even on a spinning-disk test dir).
    let nodes: Vec<NodeInput> = (0..n)
        .map(|i| NodeInput {
            id: Some(format!("g{i}")),
            labels: vec![],
            props: None,
            embedding: Some(emb_for(i, dim)),
            lang: None,
            valid_from: None,
            caused_by: None,
            ttl: None,
            collection: None, // default collection
        })
        .collect();
    s.bulk_add_nodes(nodes).unwrap();
    s.flush_index();

    // All vectors made it into the (grown) index — nothing lost on realloc.
    let count = s
        .list_collections()
        .into_iter()
        .find(|c| c.name == "default")
        .unwrap()
        .count;
    assert_eq!(count, n, "every vector inserted past the floor is indexed");

    // The grown index is still functional: a query returns the requested k
    // neighbors. (Exact identity isn't asserted — the synthetic vectors collide,
    // and HNSW is approximate; correctness of recall is covered elsewhere.)
    let hits = s
        .hybrid_search(HybridSearchInput {
            query_vector: emb_for(n - 1, dim),
            k: 5,
            alpha: Some(0.0),
            lang: None,
            as_of: None,
            collection: None,
            ef_search: None,
            oversample: None,
        })
        .unwrap();
    assert_eq!(hits.len(), 5, "grown index returns k neighbors");
}
