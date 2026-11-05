# Seeded real BGE before/after evidence (2026-11-02)

Four complete paired runs use the same 11,266-vector, 1,024-dimension real BGE-M3 repository fixture, 24 fixed queries, F32/no quantization, no rerank, ef=200, and the original 5,000 ms query deadline. Every pair used fresh, separate run directories and matching corpus/query/truth/meta hashes. There was no warm-up query. The first query recorded after the same-process reopen is retained in each memory.json; the separate fresh-child check records open/coverage only and does not execute a query.

Before binary SHA-256: a209694310b0f08265f681a299d3d26edc7f36da74b780843c1039f8241d965f  
After binary SHA-256: e046276f03755e99737b13dc4d5dcfab8f4b5a6b610513dbd618c5dde51e327d

Each pair directory contains a comparator receipt, and before/after provenance, per-query metrics and exact query IDs, fresh-reopen coverage, and gzip-compressed topology snapshots. provenance.json records raw topology SHA-256, source input hashes, binary identity, deadline/resource limits, admission samples, guard result, and process-tree working-set peak. The .txt.gz graphs decompress to the exact captured UTF-8 topology bytes. No vector database, full corpus, or executable binary is included.

comparison.json compares initial graph and post-reopen graph bytes and node-level topology, each query's returned IDs, recall, and latency. cross-seed-comparison.json checks repeatability across the two seed 43 repetitions and counts returned-ID/truth-hit differences between seed 43 and seeds 44/45. The RSS values are sampled process-tree working-set peaks, not exact allocations and can vary with shared pages. Small 24-query p95/p99 changes are descriptive single-run observations, not statistical non-regression proof.

Labels nov02-seed43d and nov02-seed43f are independent repetitions at seed 43; nov02-seed44a and nov02-seed45a use seeds 44 and 45. Seed 42's earlier pair is documented in the main audit report and its previously committed evidence.
