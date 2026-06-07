#!/bin/bash
# run_benchmarks.sh — GenesisDB Production Benchmark Runner

set -e

echo "🔧 Building GenesisDB Benchmark Suite..."
cargo build --release

echo "🚀 Running Full Production Benchmark..."
cargo test --test benchmark_suite run_production_benchmark -- --nocapture

echo "📈 Generating Charts & Summary..."
# Optional: Add Python script for visualization later

echo "✅ Benchmark completed! Check benchmark_report.md"
ls -lh benchmark_report.md