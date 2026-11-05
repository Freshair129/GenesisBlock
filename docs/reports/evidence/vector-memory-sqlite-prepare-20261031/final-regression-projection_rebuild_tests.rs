use super::*;

fn fixture() -> (tempfile::TempDir, Storage) {
    let directory = tempfile::tempdir().unwrap();
    let storage = Storage::open(OpenOptions {
        path: directory.path().display().to_string(),
        page_cache_mb: Some(16),
        read_only: Some(false),
        vector_dim: Some(4),
        retention: Some("full".into()),
    })
    .unwrap();
    storage
        .register_relational_schema(RelationalSchemaPackage {
            namespace: "preserve".into(),
            schema_version: 1,
            previous_version: None,
            package_id: "00000000-0000-4000-8000-000000000001".into(),
            schema_hash: String::new(),
            tables: vec![RelationalTable {
                name: "notes".into(),
                columns: vec![
                    RelationalColumn::required("id", RelationalColumnType::Text),
                    RelationalColumn::required("text", RelationalColumnType::Text),
                ],
                primary_key: vec!["id".into()],
                foreign_keys: vec![],
                indexes: vec![],
            }],
            named_queries: vec![],
        })
        .unwrap();
    storage
        .apply_relational_rows(
            "preserve",
            vec![RelationalRowMutation {
                table: "notes".into(),
                kind: RelationalMutationKind::Upsert,
                values: serde_json::json!({"id":"existing", "text":"ห้ามข้อมูลหาย"}),
                key: None,
            }],
        )
        .unwrap();
    storage.save_state().unwrap();
    (directory, storage)
}

fn rows(storage: &Storage) -> Vec<Value> {
    storage
        .query_relational(RelationalQuery {
            namespace: "preserve".into(),
            table: "notes".into(),
            columns: vec!["notes.id".into(), "notes.text".into()],
            joins: vec![],
            filters: vec![],
            limit: Some(10),
            offset: None,
        })
        .unwrap()
}

#[test]
fn failed_replay_keeps_original_projection_and_connection() {
    let (_directory, storage) = fixture();
    let baseline = rows(&storage);
    let registered = storage.get_relational_schema("preserve").unwrap();
    let mut invalid = registered.clone().unwrap();
    invalid.schema_version = 3;
    invalid.previous_version = Some(2);
    invalid.schema_hash.clear();
    // Deliberate invalid journal entry; public registration correctly rejects it.
    storage
        .append_wal_event(&Event::RelationalSchema(invalid))
        .unwrap();
    let error = storage.projection_rebuild_from_wal().unwrap_err();
    assert!(
        error.to_string().contains("REL_SCHEMA_VERSION_CONFLICT"),
        "expected schema-version conflict from replay, got: {error}"
    );
    assert_eq!(
        storage.get_relational_schema("preserve").unwrap(),
        registered
    );
    assert_eq!(rows(&storage), baseline);
}

#[test]
fn busy_publication_keeps_original_projection_and_connection() {
    let (directory, storage) = fixture();
    let baseline = rows(&storage);
    let other = Connection::open(directory.path().join("projection.sqlite")).unwrap();
    other.execute_batch("BEGIN IMMEDIATE;").unwrap();
    let result = storage.projection_rebuild_from_wal();
    other.execute_batch("ROLLBACK;").unwrap();
    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("PROJECTION_REBUILD_BUSY"),
        "expected busy error during publication, got: {error}"
    );
    assert_eq!(rows(&storage), baseline);
    storage.projection_rebuild_from_wal().unwrap();
    assert_eq!(rows(&storage), baseline);
    storage.save_state().unwrap();
    drop(storage);
    drop(other);
    let reopened = Storage::open(OpenOptions {
        path: directory.path().display().to_string(),
        page_cache_mb: Some(16),
        read_only: Some(false),
        vector_dim: Some(4),
        retention: Some("full".into()),
    })
    .unwrap();
    assert_eq!(rows(&reopened), baseline);
}
use std::{
    ffi::c_void,
    os::raw::{c_char, c_int},
    sync::{
        atomic::{AtomicI32, AtomicUsize, Ordering},
        Arc,
    },
};

const PUBLICATION_HOOK_THRESHOLD: c_int = 1000;
const PUBLICATION_PROPS_NODE: u32 = 5932;
const PUBLICATION_PADDING_ROWS: usize = 2048;

struct PublicationWalHookState {
    calls: AtomicUsize,
    frame_count: AtomicI32,
    log_frames: AtomicI32,
    checkpointed_frames: AtomicI32,
}

unsafe extern "C" fn test_default_autocheckpoint_hook(
    user_data: *mut c_void,
    database: *mut rusqlite::ffi::sqlite3,
    database_name: *const c_char,
    frame_count: c_int,
) -> c_int {
    if !user_data.is_null()
        && !database.is_null()
        && !database_name.is_null()
        && frame_count >= PUBLICATION_HOOK_THRESHOLD
    {
        let state = unsafe { &*user_data.cast::<PublicationWalHookState>() };
        let mut log_frames = -1;
        let mut checkpointed_frames = -1;
        let _ = unsafe {
            rusqlite::ffi::sqlite3_wal_checkpoint_v2(
                database,
                database_name,
                rusqlite::ffi::SQLITE_CHECKPOINT_PASSIVE,
                &mut log_frames,
                &mut checkpointed_frames,
            )
        };
        state.frame_count.store(frame_count, Ordering::SeqCst);
        state.log_frames.store(log_frames, Ordering::SeqCst);
        state
            .checkpointed_frames
            .store(checkpointed_frames, Ordering::SeqCst);
        state.calls.fetch_add(1, Ordering::SeqCst);
    }
    rusqlite::ffi::SQLITE_OK
}

struct PublicationWalHook<'a> {
    storage: &'a Storage,
    state: Arc<PublicationWalHookState>,
    user_data: *mut c_void,
}

impl<'a> PublicationWalHook<'a> {
    fn install(storage: &'a Storage) -> Self {
        let state = Arc::new(PublicationWalHookState {
            calls: AtomicUsize::new(0),
            frame_count: AtomicI32::new(-1),
            log_frames: AtomicI32::new(-1),
            checkpointed_frames: AtomicI32::new(-1),
        });
        let user_data = Arc::into_raw(Arc::clone(&state)) as *mut c_void;
        {
            let connection = storage.projection_db.lock();
            unsafe {
                rusqlite::ffi::sqlite3_wal_hook(
                    connection.handle(),
                    Some(test_default_autocheckpoint_hook),
                    user_data,
                );
            }
        }
        Self {
            storage,
            state,
            user_data,
        }
    }
}

impl Drop for PublicationWalHook<'_> {
    fn drop(&mut self) {
        // Do not retain a borrowed Connection across publication. Unregister
        // under the storage mutex, then release callback userdata only after
        // SQLite can no longer call it.
        let connection = self.storage.projection_db.lock();
        unsafe {
            rusqlite::ffi::sqlite3_wal_autocheckpoint(
                connection.handle(),
                PUBLICATION_HOOK_THRESHOLD,
            );
        }
        drop(connection);
        unsafe {
            drop(Arc::from_raw(
                self.user_data.cast::<PublicationWalHookState>(),
            ));
        }
    }
}

fn padded_candidate(storage: &Storage, payload_json: &str) -> ProjectionCandidate {
    let mut candidate = storage.projection_candidate_from_current().unwrap();
    let connection = candidate.connection.as_mut().unwrap();
    connection
        .pragma_update(None, "wal_autocheckpoint", 0)
        .unwrap();
    connection
        .execute_batch("CREATE TABLE wal_padding (payload BLOB NOT NULL);")
        .unwrap();
    {
        let payload = vec![0x5a_u8; 4096];
        let transaction = connection.transaction().unwrap();
        for _ in 0..PUBLICATION_PADDING_ROWS {
            transaction
                .execute(
                    "INSERT INTO wal_padding(payload) VALUES (?1)",
                    [payload.as_slice()],
                )
                .unwrap();
        }
        transaction
            .execute(
                "INSERT INTO props(node_u32, payload, clock_time, clock_peer, valid_from, valid_to)
                 VALUES (?1, ?2, 1, 'projection-rebuild-test', '2026-10-31T00:00:00Z', NULL)
                 ON CONFLICT(node_u32) DO UPDATE SET payload=excluded.payload",
                rusqlite::params![PUBLICATION_PROPS_NODE, payload_json],
            )
            .unwrap();
        transaction.commit().unwrap();
    }
    let page_count: i64 = connection
        .query_row("PRAGMA page_count", [], |row| row.get(0))
        .unwrap();
    assert!(
        page_count > PUBLICATION_HOOK_THRESHOLD as i64,
        "padding candidate must exceed the 1000-frame autocheckpoint threshold: {page_count} pages"
    );
    candidate
}

fn assert_padding_survived(storage: &Storage) {
    let padding_rows: i64 = storage
        .projection_db
        .lock()
        .query_row("SELECT COUNT(*) FROM wal_padding", [], |row| row.get(0))
        .unwrap();
    assert_eq!(padding_rows, PUBLICATION_PADDING_ROWS as i64);
}

#[test]
fn first_read_does_not_run_deferred_publication_checkpoint() {
    let (_directory, storage) = fixture();
    let candidate = padded_candidate(&storage, r#"{"marker":"published"}"#);
    // Install on the real destination connection only after fixture and source
    // setup, immediately before the publication under test.
    let hook = PublicationWalHook::install(&storage);
    storage.projection_publish_candidate(&candidate).unwrap();
    let publication_hook_calls = hook.state.calls.load(Ordering::SeqCst);
    let publication_hook_frames = hook.state.frame_count.load(Ordering::SeqCst);
    let publication_log_frames = hook.state.log_frames.load(Ordering::SeqCst);
    let publication_checkpointed_frames = hook.state.checkpointed_frames.load(Ordering::SeqCst);
    hook.state.calls.store(0, Ordering::SeqCst);

    let props = storage
        .projection_props(PUBLICATION_PROPS_NODE)
        .unwrap()
        .unwrap();
    assert_eq!(props, serde_json::json!({"marker": "published"}));
    let first_read_checkpoint_calls = hook.state.calls.load(Ordering::SeqCst);
    eprintln!("publication_hook_calls={publication_hook_calls} publication_frames={publication_hook_frames} publication_log_frames={publication_log_frames} publication_checkpointed_frames={publication_checkpointed_frames} first_read_hook_calls={first_read_checkpoint_calls}");
    assert_eq!(
        first_read_checkpoint_calls, 0,
        "first projection read ran the deferred default WAL checkpoint"
    );
    assert!(
        publication_hook_calls > 0,
        "publication did not invoke the 1000-frame WAL hook"
    );
    assert!(publication_hook_frames >= PUBLICATION_HOOK_THRESHOLD);
    assert!(publication_log_frames > PUBLICATION_HOOK_THRESHOLD);
    assert_eq!(publication_checkpointed_frames, publication_log_frames);
    assert_padding_survived(&storage);
}

#[test]
fn passive_publication_checkpoint_preserves_active_reader_snapshot() {
    let (_directory, storage) = fixture();
    {
        let connection = storage.projection_db.lock();
        connection
            .execute(
                "INSERT INTO props(node_u32, payload, clock_time, clock_peer, valid_from, valid_to)
                 VALUES (?1, ?2, 1, 'projection-rebuild-test', '2026-10-30T00:00:00Z', NULL)",
                rusqlite::params![PUBLICATION_PROPS_NODE, r#"{"marker":"old"}"#],
            )
            .unwrap();
    }

    let mut reader =
        Connection::open_with_flags(&storage.projection_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let reader_tx = reader.transaction().unwrap();
    let old_before: String = reader_tx
        .query_row(
            "SELECT payload FROM props WHERE node_u32 = ?1",
            [PUBLICATION_PROPS_NODE],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(old_before, r#"{"marker":"old"}"#);

    let candidate = padded_candidate(&storage, r#"{"marker":"new"}"#);
    let hook = PublicationWalHook::install(&storage);
    storage.projection_publish_candidate(&candidate).unwrap();

    let (checkpoint_rc, log_frames, checkpointed_frames) = {
        let connection = storage.projection_db.lock();
        let mut log_frames = -1;
        let mut checkpointed_frames = -1;
        let checkpoint_rc = unsafe {
            rusqlite::ffi::sqlite3_wal_checkpoint_v2(
                connection.handle(),
                c"main".as_ptr(),
                rusqlite::ffi::SQLITE_CHECKPOINT_PASSIVE,
                &mut log_frames,
                &mut checkpointed_frames,
            )
        };
        (checkpoint_rc, log_frames, checkpointed_frames)
    };
    assert_eq!(checkpoint_rc, rusqlite::ffi::SQLITE_OK);
    assert!(
        log_frames > PUBLICATION_HOOK_THRESHOLD,
        "publication WAL should exceed 1000 frames; got {log_frames}"
    );
    assert!(
        checkpointed_frames < log_frames,
        "PASSIVE checkpoint should stop at the active reader snapshot: checkpointed {checkpointed_frames} of {log_frames} frames"
    );

    let old_after: String = reader_tx
        .query_row(
            "SELECT payload FROM props WHERE node_u32 = ?1",
            [PUBLICATION_PROPS_NODE],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(old_after, r#"{"marker":"old"}"#);
    drop(reader_tx);

    hook.state.calls.store(0, Ordering::SeqCst);
    let new_live = storage
        .projection_props(PUBLICATION_PROPS_NODE)
        .unwrap()
        .unwrap();
    assert_eq!(new_live, serde_json::json!({"marker": "new"}));
    assert_eq!(
        hook.state.calls.load(Ordering::SeqCst),
        0,
        "first live read after releasing the old snapshot ran a deferred checkpoint"
    );

    let fresh_reader =
        Connection::open_with_flags(&storage.projection_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let new_from_fresh_reader: String = fresh_reader
        .query_row(
            "SELECT payload FROM props WHERE node_u32 = ?1",
            [PUBLICATION_PROPS_NODE],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(new_from_fresh_reader, r#"{"marker":"new"}"#);
    assert_padding_survived(&storage);
}

#[test]
fn checkpoint_failure_reports_already_committed_publication() {
    use rusqlite::hooks::{AuthAction, AuthContext, Authorization};

    let (_directory, storage) = fixture();
    let candidate = padded_candidate(&storage, r#"{"marker":"committed"}"#);
    storage
        .projection_db
        .lock()
        .authorizer(Some(|context: AuthContext<'_>| match context.action {
            AuthAction::Pragma { pragma_name, .. }
                if pragma_name.eq_ignore_ascii_case("wal_checkpoint") =>
            {
                Authorization::Deny
            }
            _ => Authorization::Allow,
        }));
    let result = storage.projection_publish_candidate(&candidate);
    storage
        .projection_db
        .lock()
        .authorizer(None::<fn(AuthContext<'_>) -> Authorization>);
    let error = result.unwrap_err().to_string();
    assert!(
        error.starts_with("PROJECTION_CHECKPOINT_FAILED: projection publication committed;"),
        "post-commit maintenance failure was misreported: {error}"
    );
    let observer =
        Connection::open_with_flags(&storage.projection_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let payload: String = observer
        .query_row(
            "SELECT payload FROM props WHERE node_u32 = ?1",
            [PUBLICATION_PROPS_NODE],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(payload, r#"{"marker":"committed"}"#);
}
