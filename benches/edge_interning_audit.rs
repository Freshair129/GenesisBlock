// Phase-0 RCA probe — edge-id interning RAM breakdown.
// Builds N nodes then N*fanout edges, snapshotting RSS and structural counts
// at each stage so the edge-attributable cost (and where it lands:
// id_to_u32 / u32_to_id / trigram_index) is measured, not estimated.
//
// Run (C: SSD!):
//   GB_VBENCH=C:\Users\freshair\gb_vbench GB_AUDIT_N=100000 GB_AUDIT_FANOUT=8 \
//     cargo run --release --bin edge-interning-audit

use genesis_block_native::{Storage, OpenOptions, NodeInput, EdgeInput};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use std::fs;
use std::time::Instant;

fn env_usize(k: &str, d: usize) -> usize {
    std::env::var(k).ok().and_then(|s| s.parse().ok()).unwrap_or(d)
}

fn rss_mb() -> f64 {
    let mut s = sysinfo::System::new_all();
    s.refresh_all();
    sysinfo::get_current_pid()
        .ok()
        .and_then(|pid| s.process(pid).map(|p| p.memory()))
        .unwrap_or(0) as f64
        / 1024.0
        / 1024.0
}

/// Total members across all trigram_index HashSets (the real cost — each
/// tokenized id char/bigram inserts one u32 into a set keyed by that token).
fn trigram_stats(s: &Storage) -> (usize, usize) {
    let mut entries = 0usize;
    let mut members = 0usize;
    for e in s.trigram_index.iter() {
        entries += 1;
        members += e.value().len() as usize; // RoaringBitmap::len() -> u64
    }
    (entries, members)
}

fn string_bytes_keys(s: &Storage) -> usize {
    s.id_to_u32.iter().map(|e| e.key().len()).sum()
}
fn string_bytes_vals(_s: &Storage) -> usize {
    // Reverse u32->id map removed (ADR--GENESISDB-NODE-ID-INTERNING, Layer A);
    // id strings now live only in id_to_u32 keys + nodes[u32].id.
    0
}

fn main() {
    let bench = std::env::var("GB_VBENCH").unwrap_or_else(|_| ".".to_string());
    let n = env_usize("GB_AUDIT_N", 100_000);
    let fanout = env_usize("GB_AUDIT_FANOUT", 8);
    let chunk = 50_000usize;

    let dbpath = format!("{bench}/gdb_edge_audit");
    let _ = fs::remove_dir_all(&dbpath);
    let storage = Storage::open(OpenOptions {
        path: dbpath,
        page_cache_mb: Some(256),
        read_only: Some(false),
        vector_dim: Some(8),
    })
    .expect("open");

    let rss_empty = rss_mb();
    println!("=== edge-interning RCA: N={n} fanout={fanout} (edges≈{}) ===", n * fanout);
    println!("[stage 0] empty open                RSS {rss_empty:8.1} MB");

    // --- nodes ---
    let t = Instant::now();
    let mut i0 = 0;
    while i0 < n {
        let i1 = (i0 + chunk).min(n);
        let nodes: Vec<NodeInput> = (i0..i1)
            .map(|i| NodeInput {
                id: Some(format!("g{i}")),
                labels: vec!["v".to_string()],
                props: None,
                embedding: None,
                lang: None,
                valid_from: None,
                caused_by: None,
                ttl: None, collection: None,
            })
            .collect();
        storage.bulk_add_nodes(nodes).unwrap();
        i0 = i1;
    }
    let node_sec = t.elapsed().as_secs_f64();
    let rss_nodes = rss_mb();
    let id_to_u32_after_nodes = storage.id_to_u32.len();
    let (tri_entries_n, tri_members_n) = trigram_stats(&storage);
    println!(
        "[stage 1] +{n} nodes ({node_sec:.1}s)      RSS {rss_nodes:8.1} MB  (+{:.1})  id_to_u32={id_to_u32_after_nodes}  trigram: {tri_entries_n} entries / {tri_members_n} members",
        rss_nodes - rss_empty
    );

    // --- edges ---
    let mut rng = StdRng::seed_from_u64(42);
    let t = Instant::now();
    let mut buf: Vec<EdgeInput> = Vec::with_capacity(chunk);
    let mut total_edges = 0usize;
    for i in 0..n {
        for _ in 0..fanout {
            let to = rng.gen_range(0..n);
            buf.push(EdgeInput {
                id: None,
                from: format!("g{i}"),
                to: format!("g{to}"),
                rel: "LINK".to_string(),
                props: None,
                valid_from: None,
                supersede: None,
                impact: None,
                caused_by: None,
            });
            if buf.len() >= chunk {
                total_edges += buf.len();
                storage.bulk_add_edges(std::mem::take(&mut buf)).unwrap();
            }
        }
    }
    if !buf.is_empty() {
        total_edges += buf.len();
        storage.bulk_add_edges(buf).unwrap();
    }
    let edge_sec = t.elapsed().as_secs_f64();
    let rss_edges = rss_mb();

    // --- structural breakdown after edges ---
    let id_to_u32_total = storage.id_to_u32.len();
    let u32_to_id_total = 0usize; // reverse map removed (node id interning Layer A)
    let edges_map = storage.edges.len();
    let nodes_map = storage.nodes.len();
    let out_entries: usize = storage.out_idx.len();
    let out_members: usize = storage.out_idx.iter().map(|e| e.value().len()).sum();
    let in_entries: usize = storage.in_idx.len();
    let in_members: usize = storage.in_idx.iter().map(|e| e.value().len()).sum();
    let (tri_entries, tri_members) = trigram_stats(&storage);
    let key_bytes = string_bytes_keys(&storage);
    let val_bytes = string_bytes_vals(&storage);

    // edge-attributable interning (nodes already accounted in stage 1)
    let edge_ids_interned = id_to_u32_total - id_to_u32_after_nodes;
    let edge_tri_members = tri_members.saturating_sub(tri_members_n);

    println!(
        "[stage 2] +{total_edges} edges ({edge_sec:.1}s)  RSS {rss_edges:8.1} MB  (+{:.1} vs nodes)",
        rss_edges - rss_nodes
    );
    println!();
    println!("--- structural counts after full build ---");
    println!("  id_to_u32 (String->u32)   entries={id_to_u32_total:>10}  key string bytes={key_bytes:>12}");
    println!("  u32_to_id (u32->String)   entries={u32_to_id_total:>10}  val string bytes={val_bytes:>12}  (removed — node id interning Layer A)");
    println!("  trigram_index             entries={tri_entries:>10}  set members={tri_members:>12}");
    println!("  edges  (u32->EdgeOutput)  entries={edges_map:>10}");
    println!("  nodes  (u32->NodeOutput)  entries={nodes_map:>10}");
    println!("  out_idx                   entries={out_entries:>10}  members={out_members:>12}");
    println!("  in_idx                    entries={in_entries:>10}  members={in_members:>12}");
    println!();
    println!("--- edge-attributable interning (the lever) ---");
    println!("  edge UUIDs interned          : {edge_ids_interned}");
    println!("  -> id_to_u32 (reverse map removed): 1 String copy each (~36 B UUID)");
    println!("  -> trigram members from edges: {edge_tri_members}  (~{:.1} per edge UUID)",
        edge_tri_members as f64 / edge_ids_interned.max(1) as f64);
    println!("  RSS edges delta              : {:.1} MB for {total_edges} edges = {:.1} B/edge",
        rss_edges - rss_nodes, (rss_edges - rss_nodes) * 1024.0 * 1024.0 / total_edges.max(1) as f64);

    let out = serde_json::json!({
        "probe": "edge-interning-audit", "n": n, "fanout": fanout, "edges": total_edges,
        "rss_empty_mb": rss_empty, "rss_nodes_mb": rss_nodes, "rss_edges_mb": rss_edges,
        "rss_edge_delta_mb": rss_edges - rss_nodes,
        "id_to_u32_total": id_to_u32_total, "u32_to_id_total": u32_to_id_total,
        "id_key_string_bytes": key_bytes, "id_val_string_bytes": val_bytes,
        "trigram_entries": tri_entries, "trigram_members": tri_members,
        "trigram_members_nodes_only": tri_members_n,
        "edge_ids_interned": edge_ids_interned, "edge_trigram_members": edge_tri_members,
        "edges_map": edges_map, "nodes_map": nodes_map,
        "out_entries": out_entries, "out_members": out_members,
        "in_entries": in_entries, "in_members": in_members,
    });
    fs::write(
        format!("{bench}/edge_interning_audit_{n}.json"),
        serde_json::to_string_pretty(&out).unwrap(),
    )
    .unwrap();
    println!("\nwrote {bench}/edge_interning_audit_{n}.json");
}
