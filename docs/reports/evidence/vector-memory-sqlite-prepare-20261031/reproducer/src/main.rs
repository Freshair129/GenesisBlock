use rusqlite::{backup::{Backup, StepResult}, Connection, OpenFlags};
use std::{env, error::Error, ffi::CStr, fs, io::{self, Write}, path::{Path, PathBuf}, time::Instant};

const SQL: &str = "SELECT payload FROM props WHERE node_u32 = ?1";
const DEFAULT_NODE: u32 = 5932;
fn say(s: impl std::fmt::Display) { println!("{s}"); let _ = io::stdout().flush(); }
fn cstr(p: *const std::os::raw::c_char) -> String { unsafe { CStr::from_ptr(p).to_string_lossy().into_owned() } }
fn size(p: &Path) -> String { fs::metadata(p).map(|m| m.len().to_string()).unwrap_or_else(|_| "missing".into()) }
fn snapshot(label: &str, db: &Path) {
    // Read-only diagnostic of pinned SQLite3.46.0 native-endian wal-index header.
    let shm = fs::read(format!("{}-shm", db.display())).ok();
    if let Some(h) = shm.as_ref().filter(|h| h.len() >= 100) {
        let frames = u32::from_ne_bytes(h[16..20].try_into().unwrap());
        let backfill = u32::from_ne_bytes(h[96..100].try_into().unwrap());
        say(format!("wal_index stage={label} mx_frame={frames} n_backfill={backfill}"));
    }
    say(format!("files stage={label} main={} wal={} shm={}", size(db), size(Path::new(&format!("{}-wal", db.display()))), size(Path::new(&format!("{}-shm", db.display())))));
}
fn query(conn: &Connection, label: &str, db: &Path, node: u32) -> Result<(), Box<dyn Error>> {
    snapshot(&format!("{label}_before_prepare"), db);
    let t = Instant::now(); let mut stmt = conn.prepare(SQL)?; let prepare = t.elapsed();
    snapshot(&format!("{label}_after_prepare"), db);
    let t = Instant::now(); let mut rows = stmt.query([node])?;
    let row = rows.next()?.ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "props row missing"))?;
    let payload: String = row.get(0)?; drop(rows); let row_time = t.elapsed();
    let t = Instant::now(); drop(stmt); let finalize = t.elapsed();
    if payload != "{}" { return Err(format!("expected payload '{{}}', got {payload:?}").into()); }
    say(format!("query={label} prepare_us={} row_us={} finalize_us={} payload_len_bytes={} payload={payload}", prepare.as_micros(), row_time.as_micros(), finalize.as_micros(), payload.len()));
    snapshot(&format!("{label}_after_query"), db); Ok(())
}
fn main() -> Result<(), Box<dyn Error>> {
    let a: Vec<String> = env::args().collect();
    if a.len() < 3 { return Err("usage: sqlite-prepare-reproducer direct SOURCE [NODE_U32] | backup|backup-noauto SOURCE DEST_COPY [NODE_U32]".into()); }
    let mode = a[1].as_str(); let src_path = PathBuf::from(&a[2]);
    let node = a.last().filter(|s| s.parse::<u32>().is_ok()).map(|s| s.parse()).transpose()?.unwrap_or(DEFAULT_NODE);
    let (db_path, mut dest) = if mode == "direct" { (src_path.clone(), None) } else if mode == "backup" || mode == "backup-noauto" || mode == "backup-checkpoint" {
        if a.len() < 4 { return Err("backup mode requires an existing disposable DEST_COPY".into()); }
        let dst = PathBuf::from(&a[3]);
        if src_path.canonicalize()? == dst.canonicalize()? { return Err("source and destination must be distinct paths".into()); }
        (dst.clone(), Some(Connection::open_with_flags(&dst, OpenFlags::SQLITE_OPEN_READ_WRITE)?))
    } else { return Err("mode must be direct, backup, or backup-noauto".into()); };
    let source = Connection::open_with_flags(&src_path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    say(format!("run mode={mode} sqlite_version={:?} sqlite_source_id={:?} source={:?} destination={:?} node_u32={node}", cstr(unsafe { rusqlite::ffi::sqlite3_libversion() }), cstr(unsafe { rusqlite::ffi::sqlite3_sourceid() }), src_path, if mode == "direct" { None::<PathBuf> } else { Some(db_path.clone()) }));
    if let Some(dst) = dest.as_mut() {
        dst.pragma_update(None, "journal_mode", "WAL")?;
        dst.pragma_update(None, "foreign_keys", true)?;
        dst.pragma_update(None, "synchronous", 1)?;
        if mode == "backup-noauto" { dst.pragma_update(None, "wal_autocheckpoint", 0)?; }
        // Match publication into a connection whose schema was already loaded.
        drop(dst.prepare(SQL)?);
        snapshot("before_backup", &db_path);
        { let backup = Backup::new(&source, dst)?; loop { match backup.step(256)? { StepResult::Done => break, StepResult::More => {}, StepResult::Busy | StepResult::Locked => return Err("backup step busy/locked".into()), _ => return Err("unexpected backup step result".into()) } } }
        snapshot("immediately_after_backup", &db_path);
        drop(source);
        snapshot("after_source_drop_before_select", &db_path);
        if mode == "backup-checkpoint" {
            let t = Instant::now();
            dst.execute_batch("PRAGMA wal_checkpoint(PASSIVE);")?;
            say(format!("publication_maintenance_us={}", t.elapsed().as_micros()));
            snapshot("after_publication_maintenance", &db_path);
        }
        query(dst, "first", &db_path, node)?;
        query(dst, "second", &db_path, node)?;
    } else {
        snapshot("direct_open", &db_path);
        query(&source, "first", &db_path, node)?;
        query(&source, "second", &db_path, node)?;
    }
    Ok(())
}


