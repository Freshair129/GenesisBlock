#!/usr/bin/env python3
"""Generate external diagnostic-only SQLite prepare variants."""
from __future__ import annotations
import difflib, hashlib, json
from pathlib import Path

ROOT = Path(r"C:\Users\suanr\genesis-sqlite-prepare-20261031")
BASE = Path(r"C:\Users\suanr\genesis-vector-memory-followup-20261031\projection-props-split-20261031T151315\patched-querytrace-lib.rs")
PROPOSED = ROOT / "proposed-publication-lib.rs"
BASE_SHA256 = "D64EC0AC47F35CCC70F8A4251FC5DBAC763BC556F7085049104C8D760795CA7B"
PROPOSED_SHA256 = "5B8BB9BED5DDE5FA6E7D4D255DA3ACCE6C1BFE892092456133ADB65A78998FDD"

def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest().upper()

def extract_function(source: str, name: str, next_name: str):
    start = source.index(f"    fn {name}")
    end = source.index(f"    fn {next_name}", start)
    return start, end, source[start:end].rstrip("\n")

def replace_function(source: str, name: str, next_name: str, replacement: str) -> str:
    start, end, _ = extract_function(source, name, next_name)
    return source[:start] + replacement.rstrip("\n") + "\n\n" + source[end:]

def source_slice(source: str, start: str, end: str) -> str:
    if start not in source or end not in source:
        raise ValueError(f"region markers missing: start={start!r} present={start in source}; end={end!r} present={end in source}")
    return source[source.index(start):source.index(end, source.index(start))]

def normalize_variant_bytes(source: str) -> bytes:
    return source.replace("\r\n", "\n").replace("\n", "\r\n").encode("utf-8")

HELPERS = r'''fn projection_query_diag_emit(event: &str, fields: serde_json::Value) {
    if std::env::var_os("GB_VECTOR_QUERY_DIAGNOSTICS").is_none() {
        return;
    }
    let mut fields = match fields {
        serde_json::Value::Object(fields) => fields,
        other => {
            let mut fields = serde_json::Map::new();
            fields.insert("details".to_string(), other);
            fields
        }
    };
    fields.insert("event".to_string(), serde_json::Value::String(event.to_string()));
    eprintln!("GB_VECTOR_QUERY_DIAG {}", serde_json::Value::Object(fields));
}

fn projection_query_diag_snapshot(event: &str, path: &Path, fields: serde_json::Value) {
    if std::env::var_os("GB_VECTOR_QUERY_DIAGNOSTICS").is_none() {
        return;
    }
    let sidecar_path = |suffix: &str| {
        let mut path = path.as_os_str().to_os_string();
        path.push(suffix);
        PathBuf::from(path)
    };
    let wal_path = sidecar_path("-wal");
    let shm_path = sidecar_path("-shm");
    let main_bytes = fs::metadata(path).ok().map(|metadata| metadata.len());
    let wal_bytes = fs::metadata(&wal_path).ok().map(|metadata| metadata.len());
    let shm_bytes = fs::metadata(&shm_path).ok().map(|metadata| metadata.len());
    let shm_header = (|| -> std::io::Result<[u8; 100]> {
        let mut bytes = [0_u8; 100];
        File::open(&shm_path)?.read_exact(&mut bytes)?;
        Ok(bytes)
    })();
    let (mx_frame, n_backfill, shm_header_error) = match shm_header {
        Ok(bytes) => (
            Some(u32::from_le_bytes(bytes[16..20].try_into().unwrap())),
            Some(u32::from_le_bytes(bytes[96..100].try_into().unwrap())),
            None,
        ),
        Err(error) => (None, None, Some(error.to_string())),
    };
    let mut fields = match fields {
        serde_json::Value::Object(fields) => fields,
        other => {
            let mut fields = serde_json::Map::new();
            fields.insert("details".to_string(), other);
            fields
        }
    };
    fields.insert("main_bytes".to_string(), serde_json::json!(main_bytes));
    fields.insert("wal_bytes".to_string(), serde_json::json!(wal_bytes));
    fields.insert("shm_bytes".to_string(), serde_json::json!(shm_bytes));
    fields.insert("shm_mx_frame_le_u32_at_16".to_string(), serde_json::json!(mx_frame));
    fields.insert("shm_n_backfill_le_u32_at_96".to_string(), serde_json::json!(n_backfill));
    fields.insert("shm_header_error".to_string(), serde_json::json!(shm_header_error));
    projection_query_diag_emit(event, serde_json::Value::Object(fields));
}
'''

SYNC_CANDIDATE = r'''    fn projection_sync_candidate(
        &self,
        candidate: &mut ProjectionCandidate,
        journal_fingerprint: &str,
    ) -> Result<()> {
        let staged = candidate
            .connection
            .take()
            .ok_or_else(|| Error::from_reason("projection rebuild candidate is unavailable"))?;
        let original = {
            let mut projection = self.projection_db.lock();
            std::mem::replace(&mut *projection, staged)
        };

        let diagnostics = std::env::var_os("GB_VECTOR_QUERY_DIAGNOSTICS").is_some();
        let replay_started = diagnostics.then(Instant::now);
        let replay_result = self.projection_replay_wal();
        if let Some(started) = replay_started {
            projection_query_diag_emit("projection_sync_candidate_stage", serde_json::json!({
                "stage": "projection_replay_wal",
                "elapsed_us": started.elapsed().as_micros(),
                "status": if replay_result.is_ok() { "ok" } else { "error" },
            }));
        }
        let history_result = replay_result.and_then(|()| {
            let started = diagnostics.then(Instant::now);
            let result = self.rebuild_edge_history();
            if let Some(started) = started {
                projection_query_diag_emit("projection_sync_candidate_stage", serde_json::json!({
                    "stage": "rebuild_edge_history",
                    "elapsed_us": started.elapsed().as_micros(),
                    "status": if result.is_ok() { "ok" } else { "error" },
                }));
            }
            result
        });
        let result = history_result.and_then(|()| {
            let started = diagnostics.then(Instant::now);
            let result = self.projection_backfill_edges();
            if let Some(started) = started {
                projection_query_diag_emit("projection_sync_candidate_stage", serde_json::json!({
                    "stage": "projection_backfill_edges",
                    "elapsed_us": started.elapsed().as_micros(),
                    "status": if result.is_ok() { "ok" } else { "error" },
                }));
            }
            result
        });

        let staged = {
            let mut projection = self.projection_db.lock();
            std::mem::replace(&mut *projection, original)
        };
        candidate.connection = Some(staged);
        result?;
        Self::projection_store_journal_fingerprint(
            candidate
                .connection
                .as_ref()
                .expect("candidate connection was restored after replay"),
            journal_fingerprint,
        )
    }
'''

SYNC_ON_OPEN = r'''    fn projection_sync_on_open(&self, force_rebuild: bool) -> Result<()> {
        let diagnostics = std::env::var_os("GB_VECTOR_QUERY_DIAGNOSTICS").is_some();
        if self.read_only {
            if diagnostics {
                projection_query_diag_emit("projection_sync_on_open", serde_json::json!({
                    "branch": if force_rebuild { "read_only_force_rebuild_rejected" } else { "read_only_no_sync" },
                    "read_only": true,
                    "force_rebuild": force_rebuild,
                    "marker_match": null,
                }));
            }
            if force_rebuild {
                return Err(Error::from_reason(
                    "RECOVERY_REQUIRED: rejected snapshot requires a writable open to rebuild the projection",
                ));
            }
            return Ok(());
        }
        let journal_fingerprint = self.projection_journal_fingerprint()?;
        if force_rebuild {
            if diagnostics {
                projection_query_diag_emit("projection_sync_on_open", serde_json::json!({
                    "branch": "force_rebuild",
                    "read_only": false,
                    "force_rebuild": true,
                    "marker_match": null,
                }));
            }
            return self
                .projection_rebuild_fresh_candidate(&journal_fingerprint, None)
                .map_err(|error| {
                    Error::from_reason(format!(
                        "RECOVERY_REQUIRED: rejected or unverifiable snapshot projection rebuild failed: {error}"
                    ))
                });
        }
        let marker_started = diagnostics.then(Instant::now);
        let marker_match = self.projection_journal_fingerprint_matches(&journal_fingerprint);
        if let Some(started) = marker_started {
            projection_query_diag_emit("projection_sync_on_open", serde_json::json!({
                "branch": if marker_match { "marker_match" } else { "marker_miss" },
                "read_only": false,
                "force_rebuild": false,
                "marker_match": marker_match,
                "marker_check_us": started.elapsed().as_micros(),
            }));
        }
        if marker_match {
            return Ok(());
        }
        // Lamport time is not a WAL cursor: distinct peers may emit equal clocks,
        // and compaction rewrites the WAL. Scan the authoritative WAL on open and
        // rely on full-clock LWW upserts for idempotence. A durable sequence cursor
        // belongs to the unified transaction protocol, not this S0/S1 projection.
        // Replay mutates SQLite event by event, so every marker miss must stay on
        // a candidate until replay, edge rebuilding, and validation all succeed.
        if self.projection_replay_needs_fresh_candidate() {
            return self.projection_rebuild_fresh_candidate(&journal_fingerprint, None);
        }
        self.projection_sync_existing_candidate(&journal_fingerprint)
    }
'''

def add_common(base: str) -> str:
    anchor = "const GOSSIP_MAX_PEERS: usize = 256;\n"
    assert base.count(anchor) == 1
    result = base.replace(anchor, HELPERS + "\n\n" + anchor, 1)
    result = replace_function(result, "projection_sync_candidate", "projection_replay_needs_fresh_candidate", SYNC_CANDIDATE)
    result = replace_function(result, "projection_sync_on_open", "projection_sync_existing_candidate", SYNC_ON_OPEN)
    return result

def instrument_before(source: str) -> str:
    _, _, f = extract_function(source, "projection_publish_candidate", "projection_backup_error")
    lock = "        let mut destination = self.projection_db.lock();\n"
    assert f.count(lock) == 1
    f = f.replace(lock, lock + "        let diagnostics = std::env::var_os(\"GB_VECTOR_QUERY_DIAGNOSTICS\").is_some();\n        let backup_started = diagnostics.then(Instant::now);\n", 1)
    done = "                rusqlite::backup::StepResult::Done => return Ok(()),"
    assert f.count(done) == 1
    f = f.replace(done, '''                rusqlite::backup::StepResult::Done => {
                    if let Some(started) = backup_started {
                        projection_query_diag_snapshot(
                            "projection_publish_backup_done",
                            &self.projection_path,
                            serde_json::json!({
                                "backup_elapsed_us": started.elapsed().as_micros(),
                                "publication_variant": "before_maintenance",
                            }),
                        );
                    }
                    return Ok(());
                },''', 1)
    return replace_function(source, "projection_publish_candidate", "projection_backup_error", f)

def instrument_after(source: str, proposed: str) -> str:
    _, _, f = extract_function(proposed, "projection_publish_candidate", "projection_backup_error")
    lock = "        let mut destination = self.projection_db.lock();\n"
    assert f.count(lock) == 1
    f = f.replace(lock, lock + "        let diagnostics = std::env::var_os(\"GB_VECTOR_QUERY_DIAGNOSTICS\").is_some();\n        let backup_started = diagnostics.then(Instant::now);\n", 1)
    done = "                    rusqlite::backup::StepResult::Done => break,"
    assert f.count(done) == 1
    f = f.replace(done, '''                    rusqlite::backup::StepResult::Done => {
                        if let Some(started) = backup_started {
                            projection_query_diag_snapshot(
                                "projection_publish_backup_done",
                                &self.projection_path,
                                serde_json::json!({
                                    "backup_elapsed_us": started.elapsed().as_micros(),
                                    "publication_variant": "after_maintenance",
                                }),
                            );
                        }
                        break;
                    },''', 1)
    original = '''        destination
            .execute_batch("PRAGMA wal_checkpoint(PASSIVE);")
            .map_err(|error| {
                Error::from_reason(format!(
                    "PROJECTION_CHECKPOINT_FAILED: projection publication committed; {error}"
                ))
            })
'''
    assert f.count(original) == 1
    timed = '''        let maintenance_started = diagnostics.then(Instant::now);
        let maintenance_result = destination.execute_batch("PRAGMA wal_checkpoint(PASSIVE);");
        if let Some(started) = maintenance_started {
            projection_query_diag_snapshot(
                "projection_publish_maintenance_done",
                &self.projection_path,
                serde_json::json!({
                    "maintenance_elapsed_us": started.elapsed().as_micros(),
                    "maintenance_status": if maintenance_result.is_ok() { "ok" } else { "error" },
                    "maintenance_error": maintenance_result.as_ref().err().map(|error| error.to_string()),
                    "publication_variant": "after_maintenance",
                }),
            );
        }
        maintenance_result.map_err(|error| {
            Error::from_reason(format!(
                "PROJECTION_CHECKPOINT_FAILED: projection publication committed; {error}"
            ))
        })
'''
    f = f.replace(original, timed, 1)
    return replace_function(source, "projection_publish_candidate", "projection_backup_error", f)

def main() -> None:
    base_bytes, proposed_bytes = BASE.read_bytes(), PROPOSED.read_bytes()
    if sha256(base_bytes) != BASE_SHA256:
        raise SystemExit(f"diagnostic base SHA mismatch: {sha256(base_bytes)}")
    if sha256(proposed_bytes) != PROPOSED_SHA256:
        raise SystemExit(f"publication proposal SHA mismatch: {sha256(proposed_bytes)}")
    base, proposed = base_bytes.decode("utf-8").replace(chr(13) + chr(10), chr(10)), proposed_bytes.decode("utf-8").replace(chr(13) + chr(10), chr(10))
    before = instrument_before(add_common(base))
    after = instrument_after(add_common(base), proposed)
    before_prefix, before_pub_tail = before.split("    fn projection_publish_candidate", 1)
    after_prefix, after_pub_tail = after.split("    fn projection_publish_candidate", 1)
    _, before_post = before_pub_tail.split("    fn projection_backup_error", 1)
    _, after_post = after_pub_tail.split("    fn projection_backup_error", 1)
    assert before_prefix == after_prefix and before_post == after_post

    regions = {
        "projection_props_unlocked": ("    fn projection_props_unlocked(", "    pub fn register_relational_schema("),
        "hybrid_search": ("    pub fn hybrid_search(&self, args: HybridSearchInput)", "    fn hybrid_search_impl("),
        "hybrid_search_impl": ("    fn hybrid_search_impl(", "    fn hybrid_search_with_budget("),
        "hybrid_search_with_budget": ("    fn hybrid_search_with_budget(", "    fn neighbors_with_budget("),
    }
    region_hashes = {}
    for name, (start, end) in regions.items():
        expected = source_slice(base, start, end)
        assert source_slice(before, start, end) == expected, f"changed region {name} in before variant"
        assert source_slice(after, start, end) == expected, f"changed region {name} in after variant"
        region_hashes[name] = sha256(expected.encode("utf-8"))

    output_hashes = {}
    variants = {"before-traced-lib.rs": before, "after-traced-lib.rs": after}
    for name, source in variants.items():
        data = normalize_variant_bytes(source)
        (ROOT / name).write_bytes(data)
        output_hashes[name] = {"sha256": sha256(data), "bytes": len(data)}

    patches = {
        "before-traced.patch": (base, before, "patched-querytrace-lib.rs", "before-traced-lib.rs"),
        "after-traced.patch": (base, after, "patched-querytrace-lib.rs", "after-traced-lib.rs"),
        "after-vs-before.patch": (before, after, "before-traced-lib.rs", "after-traced-lib.rs"),
    }
    patch_hashes = {}
    for name, (old, new, old_name, new_name) in patches.items():
        text = "".join(difflib.unified_diff(old.splitlines(keepends=True), new.splitlines(keepends=True), fromfile=old_name, tofile=new_name))
        data = text.encode("utf-8")
        (ROOT / name).write_bytes(data)
        patch_hashes[name] = {"sha256": sha256(data), "bytes": len(data)}

    script_bytes = Path(__file__).read_bytes()
    metadata = {
        "purpose": "External diagnostic variants only; no repo/C-source/database/build/runtime mutation.",
        "inputs": {
            "diagnostic_base": {"path": str(BASE), "sha256": sha256(base_bytes), "bytes": len(base_bytes)},
            "publication_proposal": {"path": str(PROPOSED), "sha256": sha256(proposed_bytes), "bytes": len(proposed_bytes)},
            "generation_script": {"path": str(Path(__file__)), "sha256": sha256(script_bytes), "bytes": len(script_bytes)},
        },
        "outputs": output_hashes,
        "patches": patch_hashes,
        "unchanged_query_props_budget_region_sha256": region_hashes,
        "diagnostics": {
            "environment_gate": "GB_VECTOR_QUERY_DIAGNOSTICS",
            "open_branches": ["force_rebuild", "marker_match", "marker_miss", "read_only_no_sync", "read_only_force_rebuild_rejected"],
            "candidate_stages": ["projection_replay_wal", "rebuild_edge_history", "projection_backfill_edges"],
            "publication_snapshot": "main/WAL/SHM sizes from filesystem metadata; read only the first 100 SHM bytes; mxFrame is LE u32 at offset 16 and nBackfill at offset 96.",
            "before_publication": "At SQLITE_DONE, log filesystem-only snapshot then immediately return; no SQLite API calls or SQL between backup completion and first props query.",
            "after_publication": "Backup scope ends before the unchanged PRAGMA wal_checkpoint(PASSIVE) SQL call; log its elapsed time and post-maintenance filesystem/SHM snapshot.",
        },
        "verification": {
            "input_hashes_verified": True,
            "variants_differ_only_in_publication_function": True,
            "query_props_and_budget_regions_byte_identical": True,
            "line_endings": "CRLF matching diagnostic base",
            "build_test_runtime": "not run by design",
        },
    }
    (ROOT / "traced-variants-metadata.json").write_text(json.dumps(metadata, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"outputs": output_hashes, "patches": patch_hashes, "metadata": str(ROOT / 'traced-variants-metadata.json')}, indent=2))

if __name__ == "__main__":
    main()
