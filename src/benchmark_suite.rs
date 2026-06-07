// benchmark_suite.rs
// GenesisDB Production-Grade Benchmark Suite (Mark VII)
// รวม Memory & Vector Index Efficiency + End-to-End GKS Workload

use crate::*;
use serde_json::json;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::time::{Duration, Instant};
use tempfile::tempdir;

#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    pub category: String,
    pub name: String,
    pub node_count: usize,
    pub duration_ms: u128,
    pub throughput: f64, // ops/sec
    pub memory_mb: f64,
    pub vector_memory_mb: f64,
    pub notes: String,
}

pub struct GenesisBenchmark {
    pub results: Vec<BenchmarkResult>,
}

impl GenesisBenchmark {
    pub fn new() -> Self {
        Self {
            results: Vec::new(),
        }
    }

    fn setup_db(size_hint: usize) -> (Storage, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let opts = OpenOptions {
            path: dir.path().to_str().unwrap().to_string(),
            page_cache_mb: Some(512),
            read_only: Some(false),
        };
        let storage = Storage::open(opts).unwrap();
        (storage, dir)
    }

    pub fn run_full_benchmark_suite(&mut self) {
        println!("\n🚀 GenesisDB Full Production Benchmark Suite");
        println!("============================================\n");

        let sizes = vec![1_000, 5_000, 10_000];

        for &size in &sizes {
            println!("\n📊 Testing with {} nodes...", size);
            self.benchmark_ingestion(size);
            self.benchmark_memory_vector_efficiency(size);
            self.benchmark_end_to_end_gks_workload(size);
            self.benchmark_hybrid_search(size);
            self.benchmark_traversal(size);
            self.benchmark_autonomic(size);
            self.benchmark_temporal(size);
        }

        self.generate_markdown_report();
    }

    // === MEMORY & VECTOR INDEX EFFICIENCY ===
    fn benchmark_memory_vector_efficiency(&mut self, node_count: usize) {
        let (mut storage, _dir) = Self::setup_db(node_count);
        let before_mem = storage.nodes.len() as f64 * 0.1; // Approximate

        let start = Instant::now();
        let _ = self.setup_large_graph(&mut storage, node_count);
        let duration = start.elapsed();

        let vector_count = node_count / 8;
        let vector_mem_mb = (vector_count as f64 * 1536.0 * 4.0) / (1024.0 * 1024.0); // f32 = 4 bytes

        self.results.push(BenchmarkResult {
            category: "Memory".to_string(),
            name: "Vector Index + Arena Efficiency".to_string(),
            node_count,
            duration_ms: duration.as_millis(),
            throughput: 0.0,
            memory_mb: vector_mem_mb * 1.5, // Estimated overhead
            vector_memory_mb: vector_mem_mb,
            notes: format!("{} vectors (1536-dim) | HNSW Index", vector_count),
        });

        println!(
            "   Memory/Vector: {:.1} MB vectors | {:.1} MB est. total",
            vector_mem_mb,
            vector_mem_mb * 1.5
        );
    }

    // === END-TO-END GKS WORKLOAD ===
    fn benchmark_end_to_end_gks_workload(&mut self, node_count: usize) {
        let (mut storage, _dir) = Self::setup_db(node_count);
        let start = Instant::now();

        // 1. Ingest GenesisBlock-style data
        let node_ids = self.setup_large_graph(&mut storage, node_count);

        // 2. Add GKS metadata (H0-H5 tiers)
        for (i, id) in node_ids.iter().enumerate() {
            let tier = match i % 6 {
                0 => "H0",
                1 => "H1",
                2 => "H2",
                3 => "H3",
                4 => "H4",
                _ => "H5",
            };
            let _ = storage
                .supersede_node(
                    id.clone(),
                    Some(json!({"context_scaling_tier": tier, "role": "gks_node"})),
                    Some("gks-workload".to_string()),
                )
                .unwrap();
        }

        // 3. Hybrid Search + Traversal
        let _ = storage
            .hybrid_search(HybridSearchInput {
                query_vector: vec![0.1; 1536],
                k: 15,
                alpha: Some(0.4),
                lang: Some("th".to_string()),
                as_of: None,
            })
            .unwrap();

        let _ = storage
            .neighbors(
                node_ids[0].clone(),
                NeighborInput {
                    depth: Some(4),
                    rel: None,
                    direction: Some("out".to_string()),
                    ..Default::default()
                },
                false,
            )
            .unwrap();

        // 4. Autonomic
        let _ = storage.perform_autonomic_optimization();

        let duration = start.elapsed();

        self.results.push(BenchmarkResult {
            category: "End-to-End",
            name: "Full GKS Workload (Ingest → Search → Traverse → Autonomic)".to_string(),
            node_count,
            duration_ms: duration.as_millis(),
            throughput: node_count as f64 / duration.as_secs_f64(),
            memory_mb: 0.0,
            vector_memory_mb: 0.0,
            notes: "Simulates real Genesis Knowledge System workflow with H0-H5 tiers".to_string(),
        });

        println!(
            "   End-to-End GKS Workload: {:?} ({:.1} TPS)",
            duration,
            node_count as f64 / duration.as_secs_f64()
        );
    }

    // === ส่วนอื่นๆ (เดิม) ===
    fn benchmark_ingestion(&mut self, node_count: usize) { /* ... (เหมือนเดิม) */
    }
    fn benchmark_hybrid_search(&mut self, node_count: usize) { /* ... */
    }
    fn benchmark_traversal(&mut self, node_count: usize) { /* ... */
    }
    fn benchmark_autonomic(&mut self, node_count: usize) { /* ... */
    }
    fn benchmark_temporal(&mut self, node_count: usize) { /* ... */
    }

    fn setup_large_graph(&self, storage: &mut Storage, node_count: usize) -> Vec<String> {
        let mut node_ids = Vec::with_capacity(node_count);
        for i in 0..node_count {
            let id = format!("GKS-NODE-{:06}", i);
            storage
                .add_node(NodeInput {
                    id: Some(id.clone()),
                    labels: vec!["GKS".to_string(), "Benchmark".to_string()],
                    props: Some(json!({"tier": format!("H{}", i % 6), "seq": i})),
                    embedding: if i % 8 == 0 {
                        Some(vec![0.05; 1536])
                    } else {
                        None
                    },
                    lang: Some("th".to_string()),
                    ..Default::default()
                })
                .unwrap();
            node_ids.push(id);
        }

        for i in 0..(node_count / 5) {
            let from = &node_ids[i];
            if 2 * i + 1 < node_count {
                storage
                    .add_edge(EdgeInput {
                        from: from.clone(),
                        to: node_ids[2 * i + 1].clone(),
                        rel: "depends_on".to_string(),
                        ..Default::default()
                    })
                    .unwrap();
            }
        }
        node_ids
    }

    fn generate_markdown_report(&self) {
        let mut file = File::create("genesisdb_benchmark_report.md").unwrap();
        writeln!(file, "# GenesisDB Production Benchmark Report").unwrap();
        writeln!(file, "Generated: {}", chrono::Utc::now()).unwrap();
        writeln!(file, "\n## Summary\n").unwrap();

        writeln!(
            file,
            "| Category | Benchmark | Nodes | Time (ms) | TPS | Vector Mem (MB) | Notes |"
        )
        .unwrap();
        writeln!(
            file,
            "|----------|-----------|-------|-----------|-----|-----------------|-------|"
        )
        .unwrap();

        for r in &self.results {
            writeln!(
                file,
                "| {} | {} | {} | {} | {:.1} | {:.1} | {} |",
                r.category,
                r.name,
                r.node_count,
                r.duration_ms,
                r.throughput,
                r.vector_memory_mb,
                r.notes
            )
            .unwrap();
        }

        println!("\n✅ Benchmark Report generated: genesisdb_benchmark_report.md");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_production_benchmark_suite() {
        let mut bench = GenesisBenchmark::new();
        bench.run_full_benchmark_suite();
    }
}
