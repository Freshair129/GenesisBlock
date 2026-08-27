//! Integration tests for vector / HNSW / collection functionality.

use genesis_block_native::{CollectionInfo, HybridSearchInput, NodeInput, OpenOptions, Storage};
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
        retention: None,
    })
    .unwrap()
}

fn node(id: &str, emb: Option<Vec<f64>>, collection: Option<&str>) -> NodeInput {
    NodeInput {
        id: Some(id.to_string()),
        labels: vec![],
        props: None,
        embedding: emb,
        lang: None,
        valid_from: None,
        caused_by: None,
        ttl: None,
        collection: collection.map(|c| c.to_string()),
    }
}

fn search_q(
    s: &Storage,
    q: Vec<f64>,
    k: u32,
    collection: Option<&str>,
    ef: Option<u32>,
) -> Result<Vec<String>, String> {
    s.hybrid_search(HybridSearchInput {
        query_vector: q,
        k,
        alpha: Some(0.0),
        lang: None,
        as_of: None,
        collection: collection.map(|c| c.to_string()),
        ef_search: ef,
        oversample: None,
    })
    .map(|v| v.into_iter().map(|n| n.node.id).collect())
    .map_err(|e| e.to_string())
}

fn info(s: &Storage, name: &str) -> Option<CollectionInfo> {
    s.list_collections().into_iter().find(|c| c.name == name)
}

// ── 1. basic_vector_search ──────────────────────────────────────────────────

#[test]
fn basic_vector_search() {
    let p = fresh("vc_basic_search");
    let s = open_dim(&p, 4);

    s.add_node(node("a", Some(vec![1.0, 0.0, 0.0, 0.0]), None))
        .unwrap();
    s.add_node(node("b", Some(vec![0.0, 1.0, 0.0, 0.0]), None))
        .unwrap();
    s.add_node(node("c", Some(vec![0.0, 0.0, 1.0, 0.0]), None))
        .unwrap();
    s.flush_index();

    let ids = search_q(&s, vec![0.9, 0.1, 0.0, 0.0], 1, None, None).unwrap();
    assert_eq!(ids.len(), 1);
    assert_eq!(ids[0], "a", "closest to [0.9,0.1,0,0] should be node 'a'");
}

// ── 2. vector_dim_mismatch_on_insert ────────────────────────────────────────

#[test]
fn vector_dim_mismatch_on_insert() {
    let p = fresh("vc_dim_mismatch_insert");
    let s = open_dim(&p, 4);

    let before = s.nodes.len();
    let res = s.add_node(node("bad", Some(vec![1.0, 2.0, 3.0]), None)); // dim 3 != 4
    assert!(res.is_err(), "inserting wrong-dim embedding should fail");
    assert_eq!(s.nodes.len(), before, "no partial node should remain");
}

// ── 3. vector_dim_mismatch_on_search ────────────────────────────────────────

#[test]
fn vector_dim_mismatch_on_search() {
    let p = fresh("vc_dim_mismatch_search");
    let s = open_dim(&p, 4);

    s.add_node(node("ok", Some(vec![1.0, 0.0, 0.0, 0.0]), None))
        .unwrap();
    s.flush_index();

    let err = search_q(&s, vec![1.0, 0.0, 0.0], 1, None, None) // dim 3 != 4
        .expect_err("search with wrong-dim query should fail");
    let lower = err.to_lowercase();
    assert!(
        lower.contains("dim") || lower.contains("dimension") || lower.contains("mismatch"),
        "error should mention dimension; got: {err}"
    );
}

// ── 4. multi_collection_isolation ───────────────────────────────────────────

#[test]
fn multi_collection_isolation() {
    let p = fresh("vc_multi_collection");
    let s = open_dim(&p, 4);

    s.create_collection(
        "text".into(),
        "test-model".into(),
        4,
        Some("l2".into()),
        None,
        None,
        None,
    )
    .unwrap();
    s.create_collection(
        "code".into(),
        "code-model".into(),
        8,
        Some("l2".into()),
        None,
        None,
        None,
    )
    .unwrap();

    // n1 with embedding in "text" collection
    s.add_node(node("n1", Some(vec![1.0, 0.0, 0.0, 0.0]), Some("text")))
        .unwrap();

    // n2 without embedding, then add_vector to "code"
    s.add_node(node("n2", None, None)).unwrap();
    s.add_vector(
        "n2".into(),
        "code".into(),
        vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    )
    .unwrap();

    s.flush_index();

    // Search "text" (dim-4) should find n1
    let text_ids = search_q(&s, vec![0.9, 0.1, 0.0, 0.0], 5, Some("text"), None).unwrap();
    assert!(
        text_ids.contains(&"n1".to_string()),
        "text collection should contain n1"
    );
    assert!(
        !text_ids.contains(&"n2".to_string()),
        "text collection should NOT contain n2"
    );

    // Search "code" (dim-8) should find n2
    let code_ids = search_q(
        &s,
        vec![0.9, 0.1, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        5,
        Some("code"),
        None,
    )
    .unwrap();
    assert!(
        code_ids.contains(&"n2".to_string()),
        "code collection should contain n2"
    );
    assert!(
        !code_ids.contains(&"n1".to_string()),
        "code collection should NOT contain n1"
    );
}

// ── 5. read_your_write_after_flush ──────────────────────────────────────────

#[test]
fn read_your_write_after_flush() {
    let p = fresh("vc_ryw");
    let s = open_dim(&p, 4);

    s.add_node(node("v1", Some(vec![1.0, 0.0, 0.0, 0.0]), None))
        .unwrap();
    // index_lag may or may not be > 0 depending on timing; just record it
    let _lag_before = s.index_lag();

    s.flush_index();
    assert_eq!(s.index_lag(), 0, "after flush, index_lag must be 0");

    let ids = search_q(&s, vec![1.0, 0.0, 0.0, 0.0], 1, None, None).unwrap();
    assert_eq!(ids, vec!["v1"]);
}

// ── 6. snapshot_rehydrate_vector_search ─────────────────────────────────────

#[test]
fn snapshot_rehydrate_vector_search() {
    let p = fresh("vc_snapshot");

    {
        let s = open_dim(&p, 4);
        for i in 0..5u32 {
            let mut emb = vec![0.0f64; 4];
            emb[(i as usize) % 4] = 1.0;
            s.add_node(node(&format!("s{i}"), Some(emb), None)).unwrap();
        }
        s.flush_index();
        s.save_state().unwrap();
    } // drop

    // Reopen — HNSW should be rehydrated from snapshot
    let s2 = open_dim(&p, 4);
    let ids = search_q(&s2, vec![1.0, 0.0, 0.0, 0.0], 3, None, None).unwrap();
    assert!(!ids.is_empty(), "search after reopen should return results");
    assert!(
        ids.contains(&"s0".to_string()),
        "s0 should be nearest to [1,0,0,0]"
    );
}

// ── 7. recall_sanity_1000_vectors ───────────────────────────────────────────

#[test]
fn recall_sanity_1000_vectors() {
    let p = fresh("vc_recall_1000");
    let s = open_dim(&p, 8);

    // Deterministic, but NOT degenerate. The previous generator was
    //
    //     v[i % 8] = 1.0 + i as f64 * 0.001
    //
    // which produces only 8 distinct DIRECTIONS, 125 vectors each, separated
    // along an axis by 0.001. The true top-10 for an axis-aligned query is
    // then an almost arbitrary pick among ~125 near-equidistant candidates, so
    // recall@10 measured by ID equality was mostly sampling HNSW's random
    // layer assignment rather than its search quality. That is why this test
    // kept drifting below its bound (0.80 -> lowered to 0.75 in #78, then
    // observed at 0.720 anyway): the data made a high score impossible to hold.
    //
    // hnsw_rs 0.3.4 seeds its StdRng with `StdRng::from_os_rng()` internally
    // and exposes no seeding API, so the index cannot be made deterministic
    // from here. Fixing the DATA is the available fix: well-separated points
    // give an unambiguous top-10, which a working ANN index recovers reliably.
    //
    // splitmix64 keeps the corpus fixed across runs and machines without
    // pulling `rand` into the test or depending on its version-to-version
    // stream stability.
    fn splitmix64(state: &mut u64) -> u64 {
        *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = *state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    // Uniform in [-1, 1) from the top 53 bits.
    fn coord(state: &mut u64) -> f64 {
        let u = (splitmix64(state) >> 11) as f64 / (1u64 << 53) as f64;
        u * 2.0 - 1.0
    }

    let mut seed = 0x2545_F491_4F6C_DD1D_u64;
    let mut vectors: Vec<Vec<f64>> = Vec::with_capacity(1000);
    for _ in 0..1000 {
        vectors.push((0..8).map(|_| coord(&mut seed)).collect());
    }

    for (i, emb) in vectors.iter().enumerate() {
        s.add_node(node(&format!("r{i}"), Some(emb.clone()), None))
            .unwrap();
    }
    s.flush_index();

    // L2 distance helper
    fn l2(a: &[f64], b: &[f64]) -> f64 {
        a.iter().zip(b).map(|(x, y)| (x - y) * (x - y)).sum::<f64>()
    }

    // 5 query points drawn from the same distribution as the corpus, from the
    // same fixed stream. The old queries were axis-aligned unit vectors, which
    // sat on the corner of the degenerate layout above and made the near-tie
    // problem worst-case; a query representative of the data is what this
    // sanity check is actually meant to exercise.
    // 50 queries, not 5. Averaging more queries is how you make this stable
    // WITHOUT lowering the bar — the opposite of what #78 did. HNSW is
    // probabilistic: measured over 400 queries, per-query recall@10 was 10/10
    // on 354, 9/10 on 44 and 8/10 on 2. With 5 queries one unlucky query moves
    // the run average by 20%, which is what produced the occasional deep
    // outlier; with 50 it moves it by 2%. Standard error falls by sqrt(10)
    // while the thing being asserted gets STRICTER, not looser.
    let queries: Vec<Vec<f64>> = (0..50)
        .map(|_| (0..8).map(|_| coord(&mut seed)).collect())
        .collect();

    let mut total_recall = 0.0;
    for q in &queries {
        // Brute-force top-10
        let mut dists: Vec<(usize, f64)> = vectors
            .iter()
            .enumerate()
            .map(|(i, v)| (i, l2(q, v)))
            .collect();
        dists.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        let brute_top10: Vec<String> = dists
            .iter()
            .take(10)
            .map(|(i, _)| format!("r{i}"))
            .collect();

        let hnsw_ids = search_q(&s, q.clone(), 10, None, Some(200)).unwrap();
        let hits = hnsw_ids
            .iter()
            .filter(|id| brute_top10.contains(id))
            .count();
        total_recall += hits as f64 / 10.0;
    }
    // HNSW graph construction uses random layer assignment, so recall is
    // nondeterministic even on deterministic input; high ef_search + a small
    // margin keep this sanity check meaningful without flaking near the bound.
    // Bound set FROM MEASUREMENT, not chosen and hoped for.
    //
    // History of this line, because it has been moved the wrong way before:
    //   0.80   original
    //   0.75   #78 "deflake" — LOWERED, justified by "green 5/5 consecutive
    //          runs". Zero failures in 5 runs bounds the failure rate at only
    //          ~60% (rule of three), so that sample could never have shown
    //          what it was claimed to show.
    //   0.720  what CI actually measured five weeks later — below the lowered
    //          bound too. Lowering the bar did not fix anything; it deferred.
    //   0.95   here, from 40 runs of this version: mean 0.9900, sd 0.0049,
    //          min 0.9780, median 0.9920. Predicted mean 0.9880 / sd 0.0048
    //          from the per-query distribution beforehand and the measurement
    //          matched, so the model behind this number is understood and not
    //          merely fitted. 0.95 sits 8.1 sd below the mean.
    //
    // The engine was never at fault: across 400 queries every search returned
    // a full 10 results (no short result sets, i.e. none of the recall-cliff
    // signature that #103's exact-scan floor addresses), with per-query
    // recall@10 of 10/10 on 354, 9/10 on 44 and 8/10 on 2. What made the old
    // test unstable was degenerate data plus averaging only 5 queries.
    let avg_recall = total_recall / queries.len() as f64;
    assert!(
        avg_recall >= 0.95,
        "recall@10 should be >= 0.95; got {avg_recall:.3}"
    );
}

// ── 8. ef_search_parameter_works ────────────────────────────────────────────

#[test]
fn ef_search_parameter_works() {
    let p = fresh("vc_ef_search");
    let s = open_dim(&p, 4);

    // 50 deterministic embeddings
    for i in 0..50u32 {
        let mut emb = vec![0.0f64; 4];
        emb[(i as usize) % 4] = 1.0 + (i as f64) * 0.01;
        emb[((i as usize) + 1) % 4] = (i as f64) * 0.005;
        s.add_node(node(&format!("e{i}"), Some(emb), None)).unwrap();
    }
    s.flush_index();

    let q = vec![1.0, 0.0, 0.0, 0.0];
    let ids_low = search_q(&s, q.clone(), 5, None, Some(10)).unwrap();
    let ids_high = search_q(&s, q, 5, None, Some(200)).unwrap();

    assert!(!ids_low.is_empty(), "ef_search=10 should return results");
    assert!(!ids_high.is_empty(), "ef_search=200 should return results");
    // Both should return valid node IDs (start with "e")
    for id in ids_low.iter().chain(ids_high.iter()) {
        assert!(id.starts_with('e'), "unexpected node id: {id}");
    }
}

// ── 9. empty_embedding_node_exists_as_graph_node ────────────────────────────

#[test]
fn empty_embedding_node_exists_as_graph_node() {
    let p = fresh("vc_no_embedding");
    let s = open_dim(&p, 4);

    s.add_node(node("plain", None, None)).unwrap();
    assert!(
        s.get_u32("plain").is_some(),
        "node without embedding should still be interned"
    );

    // Also add a vector node so search has something to return
    s.add_node(node("vec_node", Some(vec![1.0, 0.0, 0.0, 0.0]), None))
        .unwrap();
    s.flush_index();

    // Search should not crash; plain node should not appear
    let ids = search_q(&s, vec![1.0, 0.0, 0.0, 0.0], 10, None, None).unwrap();
    assert!(
        !ids.contains(&"plain".to_string()),
        "node without embedding should not appear in vector search results"
    );
}

// ── 10. collection_listing ──────────────────────────────────────────────────

#[test]
fn collection_listing() {
    let p = fresh("vc_listing");
    let s = open_dim(&p, 4);

    s.create_collection(
        "alpha".into(),
        "model-a".into(),
        4,
        Some("l2".into()),
        None,
        None,
        None,
    )
    .unwrap();
    s.create_collection(
        "beta".into(),
        "model-b".into(),
        8,
        Some("cosine".into()),
        None,
        None,
        None,
    )
    .unwrap();
    // gamma uses SQ8 quantization + rerank to exercise the rerank flag
    s.create_collection(
        "gamma".into(),
        "model-c".into(),
        16,
        Some("l2".into()),
        Some("sq8".into()),
        None,
        Some(true),
    )
    .unwrap();

    let all = s.list_collections();
    let names: Vec<&str> = all.iter().map(|c| c.name.as_str()).collect();

    assert!(
        names.contains(&"default"),
        "default collection should always exist"
    );
    assert!(names.contains(&"alpha"));
    assert!(names.contains(&"beta"));
    assert!(names.contains(&"gamma"));

    let alpha = info(&s, "alpha").unwrap();
    assert_eq!(alpha.model, "model-a");
    assert_eq!(alpha.dim, 4);
    assert!(
        alpha.metric.eq_ignore_ascii_case("l2"),
        "alpha metric should be L2; got: {}",
        alpha.metric
    );

    let beta = info(&s, "beta").unwrap();
    assert_eq!(beta.model, "model-b");
    assert_eq!(beta.dim, 8);
    assert!(
        beta.metric.eq_ignore_ascii_case("cosine"),
        "beta metric should be cosine; got: {}",
        beta.metric
    );

    let gamma = info(&s, "gamma").unwrap();
    assert_eq!(gamma.dim, 16);
    assert!(
        gamma.rerank,
        "gamma with sq8+rerank should have rerank=true"
    );
}

// ── 11. collection_already_exists_error ─────────────────────────────────────

#[test]
fn collection_already_exists_error() {
    let p = fresh("vc_dup_collection");
    let s = open_dim(&p, 4);

    s.create_collection("test".into(), "m".into(), 4, None, None, None, None)
        .unwrap();
    let res = s.create_collection("test".into(), "m2".into(), 8, None, None, None, None);
    assert!(res.is_err(), "creating a duplicate collection should fail");
}

// ── 12. add_vector_to_nonexistent_node_errors ───────────────────────────────

#[test]
fn add_vector_to_nonexistent_node_errors() {
    let p = fresh("vc_ghost_node");
    let s = open_dim(&p, 4);

    let err = s
        .add_vector("ghost".into(), "default".into(), vec![1.0, 0.0, 0.0, 0.0])
        .expect_err("add_vector to nonexistent node should fail");
    let msg = err.to_string().to_lowercase();
    assert!(
        msg.contains("not found") || msg.contains("no node") || msg.contains("unknown"),
        "error should mention node not found; got: {err}"
    );
}

// ── 13. add_vector_wrong_dim_errors ─────────────────────────────────────────

#[test]
fn add_vector_wrong_dim_errors() {
    let p = fresh("vc_wrong_dim_add_vector");
    let s = open_dim(&p, 4);

    s.add_node(node("target", None, None)).unwrap();
    let res = s.add_vector("target".into(), "default".into(), vec![1.0, 2.0]); // dim 2 != 4
    assert!(res.is_err(), "add_vector with wrong dim should fail");
}

// ── 14. cosine_metric_collection ────────────────────────────────────────────

#[test]
fn cosine_metric_collection() {
    let p = fresh("vc_cosine");
    let s = open_dim(&p, 4);

    s.create_collection(
        "cos".into(),
        "cos-model".into(),
        4,
        Some("cosine".into()),
        None,
        None,
        None,
    )
    .unwrap();

    s.add_node(node("c1", Some(vec![1.0, 0.0, 0.0, 0.0]), Some("cos")))
        .unwrap();
    s.add_node(node("c2", Some(vec![0.0, 1.0, 0.0, 0.0]), Some("cos")))
        .unwrap();
    s.add_node(node("c3", Some(vec![0.7, 0.7, 0.0, 0.0]), Some("cos")))
        .unwrap();
    s.flush_index();

    let ids = search_q(&s, vec![1.0, 0.0, 0.0, 0.0], 3, Some("cos"), None).unwrap();
    assert!(!ids.is_empty(), "cosine search should return results");
    assert_eq!(
        ids[0], "c1",
        "cosine: [1,0,0,0] should be most similar to c1"
    );
}
