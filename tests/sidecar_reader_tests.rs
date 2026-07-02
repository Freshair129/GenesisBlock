// P0-T1/T2 acceptance tests for the on-disk rerank sidecar reader
// (docs/PLAN--VECTOR-QUANTIZATION-REFINEMENT.md, ADR--GENESISDB-ONDISK-RERANK-SIDECAR).
//
// SidecarReader is a positioned-read view over an fvec_<name>.bin file: row
// d_id lives at byte offset d_id*dim*4 as `dim` little-endian f32s. These
// tests write a small fixture file directly (no Storage involved — this is a
// pure I/O + LRU cache unit) and exercise:
//   T1: exact row decode, OOB -> None, len_rows().
//   T2: LRU cache hit correctness + eviction-then-refetch-from-disk.

use genesis_block_native::{SidecarReader, SIDECAR_CACHE_ROWS};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::Path;

fn fixture_path(name: &str) -> String {
    let p = format!("{}/{}", env!("CARGO_TARGET_TMPDIR"), name);
    if Path::new(&p).exists() {
        fs::remove_file(&p).unwrap();
    }
    p
}

/// Write `rows` (each `dim` f32s) as little-endian bytes to `path`.
fn write_fvec(path: &str, rows: &[Vec<f32>]) {
    let mut f = File::create(path).unwrap();
    for row in rows {
        for &v in row {
            f.write_all(&v.to_le_bytes()).unwrap();
        }
    }
    f.flush().unwrap();
}

fn open_reader(path: &str, dim: usize) -> SidecarReader {
    let file = OpenOptions::new().read(true).write(true).open(path).unwrap();
    SidecarReader::new(file, dim)
}

#[test]
fn row_reads_exact_values_and_reports_len() {
    let path = fixture_path("sidecar_basic.bin");
    let dim = 4;
    let rows: Vec<Vec<f32>> = (0..5)
        .map(|i| {
            let base = i as f32;
            vec![base, base + 0.5, base + 0.25, base + 0.125]
        })
        .collect();
    write_fvec(&path, &rows);

    let reader = open_reader(&path, dim);

    assert_eq!(reader.len_rows(), 5);

    let row0 = reader.row(0).expect("row 0 should exist");
    assert_eq!(row0, rows[0]);

    let row4 = reader.row(4).expect("row 4 should exist");
    assert_eq!(row4, rows[4]);

    // Out of range (only 5 rows, indices 0..=4).
    assert!(reader.row(5).is_none());
}

#[test]
fn cache_hit_returns_same_value_and_survives_eviction() {
    let path = fixture_path("sidecar_cache.bin");
    let dim = 2;
    // One more row than the cache capacity so the first row is guaranteed to
    // be evicted once we've touched every other distinct row.
    let n_rows = SIDECAR_CACHE_ROWS + 1;
    let rows: Vec<Vec<f32>> = (0..n_rows)
        .map(|i| vec![i as f32, (i as f32) * 2.0 + 1.0])
        .collect();
    write_fvec(&path, &rows);

    let reader = open_reader(&path, dim);

    // Read row 0 twice; both reads must be identical (second may be a cache hit).
    let first = reader.row(0).expect("row 0 should exist");
    let second = reader.row(0).expect("row 0 should exist on re-read");
    assert_eq!(first, second);
    assert_eq!(first, rows[0]);

    // Read SIDECAR_CACHE_ROWS distinct *other* rows (1..=SIDECAR_CACHE_ROWS),
    // which is enough to push row 0 out of the bounded LRU.
    for i in 1..=SIDECAR_CACHE_ROWS {
        let got = reader.row(i).expect("row should exist");
        assert_eq!(got, rows[i]);
    }

    // Row 0 was evicted from the cache; re-reading it must still return the
    // correct value, served from disk this time.
    let refetched = reader.row(0).expect("row 0 should still be readable from disk");
    assert_eq!(refetched, rows[0]);
}
