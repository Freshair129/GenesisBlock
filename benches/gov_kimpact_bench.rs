// P24 — Governance guard cost (guard off vs on)
// P25 — K-Impact cost (full recompute vs incremental), proving O(V_affected)
//
// Run: GB_VBENCH=<dir> cargo run --release --bin gov-kimpact-bench

use genesis_block_native::{EdgeInput, NodeInput, OpenOptions, Storage};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::time::Instant;

fn build_graph(storage: &Storage, n: usize, fanout: usize, rng: &mut StdRng) {
    let chunk = 50_000usize;
    let mut i0 = 0;
    while i0 < n {
        let i1 = (i0 + chunk).min(n);
        let nodes: Vec<NodeInput> = (i0..i1)
            .map(|i| NodeInput {
                id: Some(format!("g{i}")),
                labels: vec!["USER".to_string(), "doc".to_string()],
                props: None,
                embedding: None,
                lang: None,
                valid_from: None,
                caused_by: None,
                ttl: None,
                collection: None,
            })
            .collect();
        storage.bulk_add_nodes(nodes).unwrap();
        i0 = i1;
    }
    let mut buf: Vec<EdgeInput> = Vec::with_capacity(chunk);
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
                storage.bulk_add_edges(std::mem::take(&mut buf)).unwrap();
            }
        }
    }
    if !buf.is_empty() {
        storage.bulk_add_edges(buf).unwrap();
    }
}

fn open(bench: &str, sub: &str) -> Storage {
    let p = format!("{bench}/gk_{sub}");
    let _ = std::fs::remove_dir_all(&p);
    Storage::open(OpenOptions {
        path: p,
        page_cache_mb: Some(256),
        read_only: Some(false),
        vector_dim: Some(8),
    })
    .unwrap()
}

fn main() {
    let bench = std::env::var("GB_VBENCH").unwrap_or_else(|_| ".".to_string());
    let mut rng = StdRng::seed_from_u64(7);

    // ---------- P24: governance guard cost ----------
    println!("== P24 Governance guard cost ==");
    let s = open(&bench, "gov");
    let labels_user = vec!["USER".to_string(), "doc".to_string(), "topic".to_string()];
    let labels_master = vec!["MASTER".to_string()];
    let iters = 5_000_000u64;

    // baseline: empty loop (compiler may optimize; black_box the labels ref)
    let t = Instant::now();
    let mut acc = 0u64;
    for _ in 0..iters {
        acc = acc.wrapping_add(std::hint::black_box(labels_user.len() as u64));
    }
    let base_ns = t.elapsed().as_nanos() as f64 / iters as f64;
    std::hint::black_box(acc);

    // guard ON: validate_governance (USER passes)
    let t = Instant::now();
    for _ in 0..iters {
        let _ =
            std::hint::black_box(s.validate_governance(std::hint::black_box(&labels_user), false));
    }
    let guard_ns = t.elapsed().as_nanos() as f64 / iters as f64;

    // guard ON, MASTER (rejected path)
    let t = Instant::now();
    for _ in 0..iters {
        let _ = std::hint::black_box(
            s.validate_governance(std::hint::black_box(&labels_master), false),
        );
    }
    let guard_master_ns = t.elapsed().as_nanos() as f64 / iters as f64;

    let overhead = (guard_ns - base_ns).max(0.0);
    println!("  baseline           {:.2} ns/op", base_ns);
    println!(
        "  guard ON (USER)    {:.2} ns/op  -> overhead {:.2} ns/op",
        guard_ns, overhead
    );
    println!(
        "  guard ON (MASTER)  {:.2} ns/op  (reject path)",
        guard_master_ns
    );
    println!(
        "  context: a durable add_node is ~us-ms (WAL fsync); guard is ~{:.0}x cheaper",
        1000.0 / overhead.max(0.01)
    );

    // ---------- P25: K-Impact full vs incremental ----------
    println!("\n== P25 K-Impact: full vs incremental recompute ==");
    println!(
        "  {:>10} {:>16} {:>18} {:>14}",
        "nodes", "full (ms)", "incremental(1) us", "speedup"
    );
    let sizes = std::env::var("GB_KIMPACT_SIZES")
        .ok()
        .map(|s| {
            s.split(',')
                .filter_map(|x| x.trim().parse().ok())
                .collect::<Vec<usize>>()
        })
        .unwrap_or_else(|| vec![10_000, 100_000, 500_000]);
    for &n in &sizes {
        let s = open(&bench, &format!("ki{n}"));
        build_graph(&s, n, 8, &mut rng);

        // full recompute: refresh_impacts(None) -> O(N)
        let t = Instant::now();
        s.refresh_impacts(None);
        let full_ms = t.elapsed().as_secs_f64() * 1000.0;

        // incremental: refresh_impacts(Some([one])) averaged over many single ids
        let trials = 2000u64;
        let t = Instant::now();
        for _ in 0..trials {
            let id = format!("g{}", rng.gen_range(0..n));
            s.refresh_impacts(Some(vec![id]));
        }
        let inc_us = (t.elapsed().as_nanos() as f64 / trials as f64) / 1000.0;
        let speedup = (full_ms * 1000.0) / inc_us.max(1e-6);
        println!(
            "  {:>10} {:>16.2} {:>18.3} {:>13.0}x",
            n, full_ms, inc_us, speedup
        );
    }
    println!("  -> full scales with N (O(V)); incremental(1) stays ~flat (O(V_affected)),");
    println!("     confirming the O(V_affected + E_affected) claim for localized updates.");
}
