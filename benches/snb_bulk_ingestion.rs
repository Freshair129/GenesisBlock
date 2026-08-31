//! Bulk ingestion harness. CLAUDE.md names it as one of the runs to make
//! "before claiming no perf regression on storage/index/HQL changes".
//!
//! It could not do that job for edges. The edge phase was never timed — it ran
//! `bulk_add_edges` and printed "Bulk Chain Linking Complete.", so the whole
//! measurement was node-side. That blindness was not theoretical: the edge
//! projection added a SQLite write per edge and moved edge ingestion from 91 to
//! 240 us/edge, and this harness reported nothing.
//!
//! Sizes are env-overridable (`SNB_NODES`, `SNB_EDGES`) so the noise floor can
//! be characterised without a rebuild — a rebuild here costs ~13 minutes, which
//! is enough to stop anyone from characterising anything. The defaults come
//! from that measurement; see the note above EDGES_DEFAULT.
//!
//! One result worth carrying: edge id generation dominates this benchmark.
//! Same build, same harness, only `SNB_EDGE_IDS` differing, n=8 each -
//! random `Uuid::new_v4()` ids cost 310-378 us/edge while sorted ids cost
//! 135-152. The edges table has a TEXT primary key, so sorted ids append while
//! random ids scatter the B-tree. Against the ~50 us/edge baseline that is
//! 6.9x for the default a caller gets and 2.8x if they supply sorted ids.

use genesis_block_native::{EdgeInput, NodeInput, OpenOptions, Storage};
use std::time::Instant;

const NODES_DEFAULT: usize = 5_000;

/// 40,000, not the original 4,999. Chosen from measurement, not preference.
///
/// At 4,999 edges this harness's own edge rate spans 192.7-354.5 us/edge across
/// five runs of one build — a 162 us spread, wider than the ~149 us effect it
/// would need to resolve. It cannot separate a real regression from its own
/// noise, and would report "no regression" for one that is there.
///
/// At 40,000, once the machine is settled, ten runs span 326.6-431.2 us/edge
/// (sd ~35). Against the planted control below that is complete separation.
///
/// PROVEN by planting the defect: built with the `Event::Edge` arms of
/// `projection_apply_event` removed - the pre-#163 node-only projection - the
/// same harness reports 40.6-60.2 us/edge over ten runs. The two ranges do not
/// touch; the worst run without the projection is 5.4x faster than the best run
/// with it. That is the separation this size exists to buy.
///
/// The first runs after a heavy build are NOT usable: an earlier five-run
/// sample read 1685, 1231, 847, 771, 521 us/edge, monotonically settling. Those
/// are the machine, not the engine. Discard until the numbers stop trending.
const EDGES_DEFAULT: usize = 40_000;

fn env_size(key: &str, default: usize) -> usize {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn main() {
    let nodes_n = env_size("SNB_NODES", NODES_DEFAULT);
    let edges_n = env_size("SNB_EDGES", EDGES_DEFAULT);

    let db_path = ".brain/snb_bulk_db";
    if std::path::Path::new(db_path).exists() {
        let _ = std::fs::remove_dir_all(db_path);
    }

    let storage = Storage::open(OpenOptions {
        path: db_path.to_string(),
        page_cache_mb: Some(1024),
        read_only: Some(false),
        vector_dim: None,
        retention: None,
    })
    .expect("Failed to open storage");

    println!("SNB BULK INGESTION: Processing {} nodes...", nodes_n);

    let mut buffer = Vec::with_capacity(nodes_n);
    for i in 0..nodes_n {
        buffer.push(NodeInput {
            id: Some(format!("B-{}", i)),
            labels: vec!["Entity".to_string()],
            props: Some(serde_json::json!({"val": i})),
            embedding: None,
            lang: None,
            valid_from: None,
            caused_by: None,
            ttl: None,
            collection: None,
        });
    }

    let start = Instant::now();
    storage.bulk_add_nodes(buffer).unwrap();
    let duration = start.elapsed();

    println!(
        "Bulk Ingestion Rate: {:.2} nodes/sec ({:.1} us/node)",
        nodes_n as f64 / duration.as_secs_f64(),
        duration.as_secs_f64() * 1e6 / nodes_n as f64
    );

    // The buffer is built OUTSIDE the timed region: allocating 40,000 structs
    // with formatted ids is not ingestion, and including it would dilute the
    // very signal this phase exists to show.
    // `SNB_EDGE_IDS=seq` supplies sorted ids instead of letting the engine mint
    // a random `Uuid::new_v4()`. This is not a knob for tuning the result: the
    // edges table has a TEXT primary key, so sorted ids insert append-mostly
    // while random ones scatter across the B-tree, and the two differ enough
    // that a benchmark which quietly picks one is reporting a case rather than
    // the cost. The DEFAULT stays random, because that is what a caller who
    // does not pass an id actually gets.
    let seq_ids = std::env::var("SNB_EDGE_IDS").as_deref() == Ok("seq");
    let mut edge_buffer = Vec::with_capacity(edges_n);
    for i in 0..edges_n {
        edge_buffer.push(EdgeInput {
            id: seq_ids.then(|| format!("E-{i:09}")),
            from: format!("B-{}", i % nodes_n),
            to: format!("B-{}", (i + 1) % nodes_n),
            rel: "CHAIN".to_string(),
            props: None,
            valid_from: None,
            supersede: None,
            impact: None,
            caused_by: None,
        });
    }

    let start = Instant::now();
    storage.bulk_add_edges(edge_buffer).unwrap();
    let duration = start.elapsed();

    println!(
        "Bulk Chain Linking Rate: {:.2} edges/sec ({:.1} us/edge)",
        edges_n as f64 / duration.as_secs_f64(),
        duration.as_secs_f64() * 1e6 / edges_n as f64
    );
}
