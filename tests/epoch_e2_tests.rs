// E2 (SPEC--GENESISDB-EPOCH-HNSW §3.1/§3.3/§3.4): vector time-travel.
// Epoch stamps on vector metadata (created_seq/retired_seq, meta mv:2/GBP2),
// tx_as_of on the SEARCH path (epoch candidates: retracted nodes resurrect,
// not-yet-committed nodes drop, re-embeds resolve to the historical
// embedding), and horizon-aware compaction (history-retaining profiles keep
// retired rows; frontier_only reduces to the old live-set filter).

use genesis_block_native::{EdgeInput, HybridSearchInput, NodeInput, OpenOptions, Storage};
use serde_json::json;
use std::fs;
use std::path::Path;

fn fresh(name: &str) -> String {
    let p = format!("{}/{}", env!("CARGO_TARGET_TMPDIR"), name);
    if Path::new(&p).exists() {
        fs::remove_dir_all(&p).unwrap();
    }
    p
}

fn open_with(path: &str, retention: &str) -> Storage {
    Storage::open(OpenOptions {
        path: path.to_string(),
        page_cache_mb: Some(32),
        read_only: Some(false),
        vector_dim: Some(4),
        retention: Some(retention.to_string()),
    })
    .unwrap()
}

fn add_emb(s: &Storage, id: &str, v: i64, emb: Vec<f64>) {
    s.add_node(NodeInput {
        id: Some(id.to_string()),
        labels: vec!["DOC".to_string()],
        props: Some(json!({ "v": v })),
        embedding: Some(emb),
        lang: Some("en".to_string()),
        valid_from: Some("2020-01-01T00:00:00Z".to_string()),
        caused_by: None,
        ttl: None,
        collection: None,
    })
    .unwrap();
}

/// Query-IR vector search; `t = Some(seq)` adds `temporal.tx_as_of`.
fn search(s: &Storage, q: Vec<f64>, k: u32, t: Option<u64>) -> Vec<serde_json::Value> {
    s.flush_index();
    let mut req = json!({
        "contract_version": "query-ir.v1",
        "request_id": "e2",
        "operation": {
            "kind": "search",
            "mode": "vector",
            "query_vector": q,
            "k": k
        }
    });
    if let Some(seq) = t {
        req["temporal"] = json!({ "tx_as_of": seq });
    }
    s.execute_query_ir_json(req).unwrap()["data"]
        .as_array()
        .unwrap()
        .clone()
}

fn ids(rows: &[serde_json::Value]) -> Vec<&str> {
    rows.iter()
        .map(|r| r["node"]["id"].as_str().unwrap())
        .collect()
}

/// The vector quadrant pair in one corpus: a node retracted AFTER t must
/// resurrect (with its t-version fields), and a node first committed AFTER t
/// must not appear — while the current view shows exactly the inverse.
#[test]
fn vector_tx_resurrects_and_drops_not_yet_committed() {
    let path = fresh("e2_quadrants");
    let s = open_with(&path, "full");
    add_emb(&s, "old", 1, vec![1.0, 0.0, 0.0, 0.0]);
    let before = s.stable_frontier();
    s.retract_node("old").unwrap();
    add_emb(&s, "newcomer", 2, vec![0.9, 0.1, 0.0, 0.0]);

    let now_rows = search(&s, vec![1.0, 0.0, 0.0, 0.0], 5, None);
    assert_eq!(
        ids(&now_rows),
        vec!["newcomer"],
        "current view: retracted node hidden, newcomer served"
    );

    let then_rows = search(&s, vec![1.0, 0.0, 0.0, 0.0], 5, Some(before));
    assert_eq!(
        ids(&then_rows),
        vec!["old"],
        "belief at t: retracted node resurrects, newcomer not yet committed"
    );
    assert_eq!(
        then_rows[0]["node"]["props"]["v"], 1,
        "resurrected node carries its chain-resolved fields"
    );
}

/// Re-embedding orphans the previous vector WITH an epoch stamp: at t before
/// the re-embed, the search must rank by the OLD embedding (the one that was
/// current then), not the new one — the dedupe-by-node-id is epoch-correct.
#[test]
fn vector_tx_resolves_historical_embedding() {
    let path = fresh("e2_reembed");
    let s = open_with(&path, "full");
    add_emb(&s, "doc", 1, vec![1.0, 0.0, 0.0, 0.0]);
    add_emb(&s, "decoy", 0, vec![0.8, 0.2, 0.0, 0.0]);
    let before = s.stable_frontier();
    // Re-embed doc away from the query axis.
    s.add_vector(
        "doc".to_string(),
        "default".to_string(),
        vec![0.0, 1.0, 0.0, 0.0],
    )
    .unwrap();

    let now_top = search(&s, vec![1.0, 0.0, 0.0, 0.0], 1, None);
    assert_eq!(
        ids(&now_top),
        vec!["decoy"],
        "current view ranks by the NEW embedding — decoy is nearest now"
    );

    let then_top = search(&s, vec![1.0, 0.0, 0.0, 0.0], 1, Some(before));
    assert_eq!(
        ids(&then_top),
        vec!["doc"],
        "belief at t ranks by the embedding that was current then"
    );
}

/// Stamps must survive a snapshot instant-load: meta_<name>.bin round-trips
/// through the v2 (GBP2) container, and the resurrection still answers after
/// reopen. Also pins the on-disk magic byte-for-byte.
#[test]
fn vector_tx_survives_snapshot_reopen() {
    let path = fresh("e2_reopen_snapshot");
    let before = {
        let s = open_with(&path, "full");
        add_emb(&s, "doc", 1, vec![1.0, 0.0, 0.0, 0.0]);
        let before = s.stable_frontier();
        s.retract_node("doc").unwrap();
        s.flush_index();
        s.save_state().unwrap(); // full profile: checkpoint without fold
        before
    };

    let bytes = fs::read(Path::new(&path).join("meta_default.bin")).unwrap();
    assert_eq!(&bytes[..4], b"GBP2", "current snapshots are GBP2-tagged");

    let s = open_with(&path, "full");
    let rows = search(&s, vec![1.0, 0.0, 0.0, 0.0], 5, Some(before));
    assert_eq!(
        ids(&rows),
        vec!["doc"],
        "stamps survive the snapshot: {rows:?}"
    );
    assert_eq!(rows[0]["node"]["props"]["v"], 1);
    assert!(
        search(&s, vec![1.0, 0.0, 0.0, 0.0], 5, None).is_empty(),
        "current view stays empty after reopen"
    );
}

/// No snapshot at all: full journal replay must rebuild the stamps from the
/// frames' own seqs (created_seq from the Node frame, retired_seq from the
/// NodeRetract frame).
#[test]
fn vector_tx_survives_journal_replay() {
    let path = fresh("e2_reopen_replay");
    let before = {
        let s = open_with(&path, "full");
        add_emb(&s, "doc", 1, vec![1.0, 0.0, 0.0, 0.0]);
        let before = s.stable_frontier();
        s.retract_node("doc").unwrap();
        before
        // dropped WITHOUT save_state → next open replays the journal
    };

    let s = open_with(&path, "full");
    let rows = search(&s, vec![1.0, 0.0, 0.0, 0.0], 5, Some(before));
    assert_eq!(
        ids(&rows),
        vec!["doc"],
        "journal replay must rebuild the epoch stamps: {rows:?}"
    );
}

/// §3.4 under a history-retaining profile: compaction preserves the retired
/// row (it is still inside retained history), so compact-then-query still
/// resurrects — compaction no longer races ahead of the journal's horizon.
#[test]
fn compaction_preserves_history_under_full() {
    let path = fresh("e2_compact_full");
    let s = open_with(&path, "full");
    add_emb(&s, "doc", 1, vec![1.0, 0.0, 0.0, 0.0]);
    add_emb(&s, "other", 0, vec![0.0, 0.0, 1.0, 0.0]);
    let before = s.stable_frontier();
    s.retract_node("doc").unwrap();
    s.perform_index_compaction().unwrap();

    let coll = s.collections.get("default").unwrap();
    assert_eq!(
        coll.metadata.read().len(),
        2,
        "full profile: the retired row survives compaction"
    );
    drop(coll);

    let rows = search(&s, vec![1.0, 0.0, 0.0, 0.0], 1, Some(before));
    assert_eq!(
        ids(&rows),
        vec!["doc"],
        "compact-then-query resurrects: {rows:?}"
    );
    let now_ids = ids(&search(&s, vec![1.0, 0.0, 0.0, 0.0], 5, None))
        .into_iter()
        .map(String::from)
        .collect::<Vec<_>>();
    assert!(
        !now_ids.contains(&"doc".to_string()),
        "current view never serves the retired row: {now_ids:?}"
    );
}

/// §3.4 under frontier_only (the default): the checkpoint folds, the horizon
/// advances past the retirement, and compaction drops the row — exactly the
/// old live-set filter (C4). The tx question itself fails loudly at the
/// horizon, never silently empty.
#[test]
fn compaction_drops_history_under_frontier_only() {
    let path = fresh("e2_compact_frontier");
    let s = open_with(&path, "frontier_only");
    add_emb(&s, "doc", 1, vec![1.0, 0.0, 0.0, 0.0]);
    let before = s.stable_frontier();
    s.retract_node("doc").unwrap();
    s.flush_index();
    s.save_state().unwrap(); // frontier_only: this checkpoint folds
    s.perform_index_compaction().unwrap();

    let coll = s.collections.get("default").unwrap();
    assert_eq!(
        coll.metadata.read().len(),
        0,
        "frontier_only: fold + compaction reclaim the retired row"
    );
    drop(coll);

    let err = s
        .execute_query_ir_json(json!({
            "contract_version": "query-ir.v1",
            "request_id": "e2-horizon",
            "operation": {
                "kind": "search", "mode": "vector",
                "query_vector": [1.0, 0.0, 0.0, 0.0], "k": 1
            },
            "temporal": { "tx_as_of": before }
        }))
        .unwrap_err();
    assert!(
        err.to_string().contains("beyond_horizon"),
        "pre-fold tx question fails loudly, not empty: {err}"
    );
}

/// A v1 (GBP1 postcard, no stamps) snapshot migrates on load: rows come back
/// with zeroed stamps ("always existed"), search works, and the next save
/// rewrites the file as GBP2 — no dedicated migration step.
#[test]
fn meta_v1_gbp1_snapshot_migrates() {
    let path = fresh("e2_v1_migration");
    {
        let s = open_with(&path, "full");
        add_emb(&s, "BIG", 1, vec![1.0, 1.0, 1.0, 1.0]);
        add_emb(&s, "SMALL", 2, vec![0.2, 0.2, 0.2, 0.2]);
        s.flush_index();
        s.save_state().unwrap();
    }

    // Rewrite meta_default.bin as a byte-accurate v1 fixture: decode the GBP2
    // body the engine just wrote, strip the epoch stamps, and re-encode the
    // 8-field v1 layout (postcard serializes struct fields positionally, so a
    // tuple with the same field order is byte-identical to the old struct).
    let meta_path = Path::new(&path).join("meta_default.bin");
    let gbp2 = fs::read(&meta_path).unwrap();
    assert_eq!(&gbp2[..4], b"GBP2");
    let current: Vec<genesis_block_native::NodeMetadata> =
        postcard::from_bytes(&gbp2[4..]).unwrap();
    assert!(current.iter().all(|m| m.created_seq > 0));
    #[allow(clippy::type_complexity)]
    let v1: Vec<(u32, u32, u64, u16, u64, Vec<u8>, String, u32)> = current
        .iter()
        .map(|m| {
            (
                m.arena_id,
                m.node_u32,
                m.timestamp,
                m.vector_dim,
                m.embedding_offset,
                m.gks_attributes.clone(),
                m.lang.clone(),
                m.cluster_id,
            )
        })
        .collect();
    let mut v1_bytes = b"GBP1".to_vec();
    v1_bytes.extend(postcard::to_allocvec(&v1).unwrap());
    fs::write(&meta_path, &v1_bytes).unwrap();

    let s = open_with(&path, "full");
    {
        let coll = s.collections.get("default").unwrap();
        let meta = coll.metadata.read();
        assert_eq!(meta.len(), 2);
        assert!(
            meta.iter()
                .all(|m| m.created_seq == 0 && m.retired_seq == 0),
            "migrated v1 rows carry zeroed stamps (pre-epoch: always existed)"
        );
    }
    // Zero-stamped rows behave as "always existed": current AND tx views serve
    // them (within the horizon), and search still resolves the right node.
    let top = s
        .hybrid_search(HybridSearchInput {
            query_vector: vec![0.2, 0.2, 0.2, 0.2],
            k: 1,
            alpha: Some(0.0),
            lang: None,
            as_of: None,
            collection: None,
            ef_search: None,
            oversample: None,
        })
        .unwrap();
    assert_eq!(top[0].node.id, "SMALL");

    s.save_state().unwrap();
    let rewritten = fs::read(&meta_path).unwrap();
    assert_eq!(
        &rewritten[..4],
        b"GBP2",
        "the next save rewrites a migrated v1 snapshot as GBP2"
    );
}

/// tx_as_of on SEARCH composes with the graph side (E1): the same selector
/// answers both "which vectors did we believe in" and "which edges" — the
/// two-axis engine surface stays consistent across operations.
#[test]
fn vector_and_graph_tx_agree() {
    let path = fresh("e2_cross_op");
    let s = open_with(&path, "full");
    add_emb(&s, "hub", 0, vec![0.0, 0.0, 0.0, 1.0]);
    add_emb(&s, "doc", 1, vec![1.0, 0.0, 0.0, 0.0]);
    s.add_edge(EdgeInput {
        id: Some("e1".to_string()),
        from: "hub".to_string(),
        to: "doc".to_string(),
        rel: "KNOWS".to_string(),
        props: None,
        valid_from: Some("2020-01-01T00:00:00Z".to_string()),
        supersede: None,
        impact: None,
        caused_by: None,
    })
    .unwrap();
    let before = s.stable_frontier();
    s.retract_node("doc").unwrap();

    let vec_rows = search(&s, vec![1.0, 0.0, 0.0, 0.0], 1, Some(before));
    assert_eq!(ids(&vec_rows), vec!["doc"]);

    let trav = s
        .execute_query_ir_json(json!({
            "contract_version": "query-ir.v1",
            "request_id": "e2-trav",
            "operation": {
                "kind": "traverse", "seed_id": "hub", "depth": 1,
                "relations": ["KNOWS"], "direction": "out"
            },
            "temporal": { "tx_as_of": before }
        }))
        .unwrap()["data"]
        .as_array()
        .unwrap()
        .clone();
    assert_eq!(
        trav[0]["node"]["id"], "doc",
        "SEARCH and TRAVERSE agree on the belief at t"
    );
    assert_eq!(
        vec_rows[0]["node"]["props"]["v"], trav[0]["node"]["props"]["v"],
        "both paths resolve through the same version chain"
    );
}
