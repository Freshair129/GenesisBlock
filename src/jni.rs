//! Android JNI bridge over the storage core (mobile SDK task B-2,
//! SPEC--MOBILE-SDK §B-2).
//!
//! This is the Android counterpart of [`crate::ffi`] (the C ABI used by the iOS
//! xcframework). It exposes the same synchronous [`Storage`] core to the JVM via
//! `System.loadLibrary("genesis_block_native")` + `external fun` declarations on
//! the `dev.genesisblock.GenesisDB` Kotlin class. The symbol names therefore
//! follow JNI's `Java_<pkg>_<Class>_<method>` mangling for that class.
//!
//! ## Contract (mirrors [`crate::ffi`])
//! - Built with napi OFF: `--no-default-features --features "mobile ffi
//!   android-jni"`. It talks to the plain-native core via the `Error`/`Result`
//!   shim, never the napi-only `GenesisDatabase` wrapper.
//! - Every entry point is wrapped in [`std::panic::catch_unwind`] so an engine
//!   panic can never unwind across the JNI boundary (UB). On a caught panic or
//!   an engine error, JSON-returning methods return a Java `null` and
//!   `nativeFlushIndex` returns a nonzero code.
//! - JSON in / JSON out uses the same serde types as REST/NAPI/FFI
//!   ([`NodeInput`], [`HybridSearchInput`], …), so those tests act as implicit
//!   contract tests for this surface too.
//! - `nativeOpen` returns a `jlong` handle that boxes an `Arc<Storage>`. The Arc
//!   keeps the engine's background threads (WAL writer, async HNSW indexer)
//!   alive for the handle's lifetime. The handle MUST be handed back to
//!   `nativeClose` exactly once; using it afterwards is undefined behaviour.
//!
//! The `jni` crate is pure Rust and compiles on any host (it only links a JVM
//! when its `invocation` feature is on, which we do not use), so this module
//! builds and type-checks on the Windows dev box even though it can only be
//! *linked into an Android `.so`* by the cross-compile CI.

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::Arc;

use jni::objects::{JClass, JString};
use jni::sys::{jint, jlong, jstring};
use jni::JNIEnv;
use serde::Deserialize;

use crate::{
    GenesisTransaction, HybridSearchInput, NamedQueryRequest, NodeInput, OpenOptions,
    RelationalMutationBatch, RelationalQuery, RelationalRowMutation, RelationalSchemaPackage,
    Storage,
};

#[derive(Deserialize)]
struct RelationalRowsInput {
    namespace: String,
    mutations: Vec<RelationalRowMutation>,
}

/// Input shape for `nativeRetrieveContext`. `Storage::retrieve_context` takes
/// scalar args rather than a struct, so the JSON contract is defined here.
/// Kept identical to the `RetrieveContextInput` in [`crate::ffi`] so the two
/// mobile surfaces accept the exact same payload.
#[derive(Deserialize)]
struct RetrieveContextInput {
    target_id: String,
    #[serde(default = "default_tier")]
    tier: String,
    #[serde(default)]
    budget: Option<u32>,
    #[serde(default)]
    fuzzy: bool,
}

fn default_tier() -> String {
    "H1".to_string()
}

// --- internal helpers -------------------------------------------------------

/// Borrow the boxed `Arc<Storage>` from a `jlong` handle without taking
/// ownership. Returns `None` for a zero/invalid handle.
///
/// # Safety
/// `handle` must be 0 or a value previously returned by `nativeOpen` and not
/// yet passed to `nativeClose`.
unsafe fn handle_storage<'a>(handle: jlong) -> Option<&'a Arc<Storage>> {
    if handle == 0 {
        return None;
    }
    Some(&*(handle as *const Arc<Storage>))
}

/// Pull a `JString` argument out into an owned Rust `String`. Returns `None` on
/// a null reference or a JVM error.
fn jstring_to_string(env: &mut JNIEnv, s: &JString) -> Option<String> {
    if s.is_null() {
        return None;
    }
    env.get_string(s).ok().map(|js| js.into())
}

/// Build a Java `String` from a Rust `String`, or a Java `null` on failure.
fn new_jstring(env: &mut JNIEnv, s: String) -> jstring {
    match env.new_string(s) {
        Ok(js) => js.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Shared body for the JSON-in / JSON-out methods: borrow the handle, read the
/// input string, run `op` (which returns `Result<String, _>` of serialized
/// JSON), and hand a Java string back — Java `null` on any error or panic.
fn json_op<F>(env: &mut JNIEnv, handle: jlong, input: &JString, op: F) -> jstring
where
    F: FnOnce(&Arc<Storage>, &str) -> Option<String>,
{
    let json = match jstring_to_string(env, input) {
        Some(j) => j,
        None => return std::ptr::null_mut(),
    };
    let produced = catch_unwind(AssertUnwindSafe(|| {
        let storage = unsafe { handle_storage(handle) }?;
        op(storage, &json)
    }));
    match produced {
        Ok(Some(s)) => new_jstring(env, s),
        _ => std::ptr::null_mut(),
    }
}

// --- exported JNI ABI -------------------------------------------------------
// Symbol mangling targets `dev.genesisblock.GenesisDB` (see SPEC §B-2 Kotlin
// wrapper). Renaming the Kotlin package/class requires renaming these symbols.

/// `external fun nativeOpen(path: String): Long` — open or create a GenesisDB at
/// `path`. Returns a nonzero handle, or 0 on error (null path or engine open
/// failure).
#[no_mangle]
pub extern "system" fn Java_dev_genesisblock_GenesisDB_nativeOpen(
    mut env: JNIEnv,
    _class: JClass,
    path: JString,
) -> jlong {
    let path = match jstring_to_string(&mut env, &path) {
        Some(p) => p,
        None => return 0,
    };
    let result = catch_unwind(|| {
        let opts = OpenOptions {
            path,
            page_cache_mb: None,
            read_only: None,
            vector_dim: None,
            retention: None,
        };
        match Storage::open(opts) {
            Ok(storage) => Box::into_raw(Box::new(Arc::new(storage))) as jlong,
            Err(_) => 0,
        }
    });
    result.unwrap_or(0)
}

/// `external fun nativeClose(handle: Long)` — close a handle and free its
/// resources. No-op on a 0 handle. The handle is dangling afterwards.
#[no_mangle]
pub extern "system" fn Java_dev_genesisblock_GenesisDB_nativeClose(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) {
    if handle == 0 {
        return;
    }
    // Reclaim the Box; dropping the inner Arc<Storage> (when last owner) joins
    // the WAL/index threads via Storage::Drop.
    let _ = catch_unwind(AssertUnwindSafe(|| unsafe {
        drop(Box::from_raw(handle as *mut Arc<Storage>));
    }));
}

/// `external fun nativeAddNode(handle: Long, jsonInput: String): String?` —
/// add a node from a [`NodeInput`] JSON string; returns a [`crate::NodeOutput`]
/// JSON string or `null` on error.
#[no_mangle]
pub extern "system" fn Java_dev_genesisblock_GenesisDB_nativeAddNode(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    json_input: JString,
) -> jstring {
    json_op(&mut env, handle, &json_input, |storage, json| {
        let input: NodeInput = serde_json::from_str(json).ok()?;
        let output = storage.add_node(input).ok()?;
        serde_json::to_string(&output).ok()
    })
}

/// `external fun nativeSearch(handle: Long, jsonInput: String): String?` —
/// hybrid search from a [`HybridSearchInput`] JSON string; returns a JSON array
/// of `NeighborOutput` or `null` on error.
#[no_mangle]
pub extern "system" fn Java_dev_genesisblock_GenesisDB_nativeSearch(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    json_input: JString,
) -> jstring {
    json_op(&mut env, handle, &json_input, |storage, json| {
        let input: HybridSearchInput = serde_json::from_str(json).ok()?;
        let results = storage.hybrid_search(input).ok()?;
        serde_json::to_string(&results).ok()
    })
}

/// `external fun nativeExecuteHql(handle: Long, query: String): String?` —
/// execute an HQL query; returns the result as a JSON string or `null` on error.
#[no_mangle]
pub extern "system" fn Java_dev_genesisblock_GenesisDB_nativeExecuteHql(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    query: JString,
) -> jstring {
    json_op(&mut env, handle, &query, |storage, q| {
        let value = storage.execute_hql(q).ok()?;
        serde_json::to_string(&value).ok()
    })
}

/// `external fun nativeExecuteQueryIr(handle: Long, jsonInput: String):
/// String?` — WP-2.2: execute a versioned Typed Query IR request
/// (`QueryIrRequest` JSON, contract `query-ir.v1`; supports
/// `temporal.valid_at` and the replica-local `temporal.tx_as_of`); returns
/// the IR response envelope as JSON or `null` on error.
#[no_mangle]
pub extern "system" fn Java_dev_genesisblock_GenesisDB_nativeExecuteQueryIr(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    json_input: JString,
) -> jstring {
    json_op(&mut env, handle, &json_input, |storage, json| {
        let request: serde_json::Value = serde_json::from_str(json).ok()?;
        let value = storage.execute_query_ir_json(request).ok()?;
        serde_json::to_string(&value).ok()
    })
}

/// `external fun nativeQueryIrCapabilities(handle: Long): String?` — WP-2.2:
/// the Query IR capability manifest (incl. `temporal.history_horizon` and
/// the retention profile — ADR I6) as JSON, or `null` on error.
#[no_mangle]
pub extern "system" fn Java_dev_genesisblock_GenesisDB_nativeQueryIrCapabilities(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
) -> jstring {
    use std::panic::{catch_unwind, AssertUnwindSafe};
    let result = catch_unwind(AssertUnwindSafe(|| {
        let storage = unsafe { handle_storage(handle) }?;
        serde_json::to_string(&storage.query_ir_capabilities()).ok()
    }));
    match result {
        Ok(Some(s)) => env
            .new_string(s)
            .map(|j| j.into_raw())
            .unwrap_or(std::ptr::null_mut()),
        _ => std::ptr::null_mut(),
    }
}

/// `external fun nativeRetrieveContext(handle: Long, jsonInput: String):
/// String?` — retrieve a tiered context package from a `RetrieveContextInput`
/// JSON string (`{ "target_id", "tier", "budget", "fuzzy" }`); returns a
/// [`crate::ContextPackage`] JSON string or `null` on error.
#[no_mangle]
pub extern "system" fn Java_dev_genesisblock_GenesisDB_nativeRetrieveContext(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    json_input: JString,
) -> jstring {
    json_op(&mut env, handle, &json_input, |storage, json| {
        let input: RetrieveContextInput = serde_json::from_str(json).ok()?;
        let pkg = storage
            .retrieve_context(&input.target_id, &input.tier, input.budget, input.fuzzy)
            .ok()?;
        serde_json::to_string(&pkg).ok()
    })
}

#[no_mangle]
pub extern "system" fn Java_dev_genesisblock_GenesisDB_nativeRegisterRelationalSchema(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    json_input: JString,
) -> jstring {
    json_op(&mut env, handle, &json_input, |storage, json| {
        let package = serde_json::from_str::<RelationalSchemaPackage>(json).ok()?;
        storage
            .register_relational_schema(package)
            .ok()
            .map(|version| version.to_string())
    })
}

#[no_mangle]
pub extern "system" fn Java_dev_genesisblock_GenesisDB_nativeGetRelationalSchema(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    namespace: JString,
) -> jstring {
    json_op(&mut env, handle, &namespace, |storage, namespace| {
        serde_json::to_string(&storage.get_relational_schema(namespace).ok()?).ok()
    })
}

#[no_mangle]
pub extern "system" fn Java_dev_genesisblock_GenesisDB_nativeApplyRelationalBatch(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    json_input: JString,
) -> jstring {
    json_op(&mut env, handle, &json_input, |storage, json| {
        let batch = serde_json::from_str::<RelationalMutationBatch>(json).ok()?;
        serde_json::to_string(&storage.apply_relational_batch(batch).ok()?).ok()
    })
}

#[no_mangle]
pub extern "system" fn Java_dev_genesisblock_GenesisDB_nativeApplyRelationalRows(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    json_input: JString,
) -> jstring {
    json_op(&mut env, handle, &json_input, |storage, json| {
        let input = serde_json::from_str::<RelationalRowsInput>(json).ok()?;
        storage
            .apply_relational_rows(&input.namespace, input.mutations)
            .ok()?;
        Some("null".to_string())
    })
}

#[no_mangle]
pub extern "system" fn Java_dev_genesisblock_GenesisDB_nativeQueryRelational(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    json_input: JString,
) -> jstring {
    json_op(&mut env, handle, &json_input, |storage, json| {
        let query = serde_json::from_str::<RelationalQuery>(json).ok()?;
        serde_json::to_string(&storage.query_relational(query).ok()?).ok()
    })
}

#[no_mangle]
pub extern "system" fn Java_dev_genesisblock_GenesisDB_nativeExecuteNamedQuery(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    json_input: JString,
) -> jstring {
    json_op(&mut env, handle, &json_input, |storage, json| {
        let request = serde_json::from_str::<NamedQueryRequest>(json).ok()?;
        serde_json::to_string(&storage.execute_named_query(request).ok()?).ok()
    })
}

#[no_mangle]
pub extern "system" fn Java_dev_genesisblock_GenesisDB_nativeCommitTransaction(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    json_input: JString,
) -> jstring {
    json_op(&mut env, handle, &json_input, |storage, json| {
        let transaction = serde_json::from_str::<GenesisTransaction>(json).ok()?;
        serde_json::to_string(&storage.commit_transaction(transaction).ok()?).ok()
    })
}

/// `external fun nativeFlushIndex(handle: Long): Int` — flush the async HNSW
/// index so staged vectors become searchable (read-your-write). Returns 0 on
/// success, nonzero on a 0/invalid handle or a caught panic.
#[no_mangle]
pub extern "system" fn Java_dev_genesisblock_GenesisDB_nativeFlushIndex(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) -> jint {
    let result = catch_unwind(AssertUnwindSafe(|| {
        let storage = match unsafe { handle_storage(handle) } {
            Some(s) => s,
            None => return 1,
        };
        storage.flush_index();
        0
    }));
    result.unwrap_or(2)
}
