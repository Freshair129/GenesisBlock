//! C ABI over the storage core (mobile SDK task 0-C, SPEC--MOBILE-SDK §0-C).
//!
//! This is a thin, panic-safe C FFI shim around [`Storage`]. It is built with
//! the napi bindings OFF (`--no-default-features --features "mobile ffi"`), so it
//! talks to the plain-native storage core via the `Error`/`Result` shim — never
//! to the napi-only `GenesisDatabase` wrapper.
//!
//! ## Contract
//! - Every entry point is wrapped in [`std::panic::catch_unwind`] so a panic
//!   inside the engine can never unwind across the C boundary (UB). On panic or
//!   error, the JSON-returning functions return a null pointer and
//!   [`genesisdb_flush_index`] returns a nonzero code.
//! - JSON in / JSON out mirrors the existing REST/NAPI serde types
//!   ([`NodeInput`], [`HybridSearchInput`], [`NodeOutput`], …) so the REST and
//!   NAPI tests act as implicit contract tests for this surface.
//! - Returned strings are heap-allocated via [`CString::into_raw`]; the caller
//!   MUST hand each one back to [`genesisdb_free_string`] exactly once. Freeing
//!   with any other allocator is undefined behaviour.
//! - All `Storage` methods are synchronous (the async offload lives in the napi
//!   wrapper, not the core), so no tokio runtime is needed here — we call the
//!   sync methods directly, mirroring how `src/router.rs` drives `Storage`.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::Arc;

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

/// Opaque handle handed back to C. Wraps an `Arc<Storage>` so the engine's
/// internal background threads (WAL writer, async HNSW indexer) keep their
/// reference-counted owner alive for the handle's whole lifetime.
pub struct GenesisHandle {
    storage: Arc<Storage>,
}

/// Input shape for [`genesisdb_retrieve_context`]. `Storage::retrieve_context`
/// takes scalar args rather than a struct, so we define the JSON contract here.
/// Mirrors the HQL `CONTEXT` parameters.
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

/// Borrow a C string as `&str`. Returns `None` on a null pointer or non-UTF-8.
///
/// # Safety
/// `ptr` must be null or point to a valid NUL-terminated C string that stays
/// alive for the duration of the borrow.
unsafe fn cstr_to_str<'a>(ptr: *const c_char) -> Option<&'a str> {
    if ptr.is_null() {
        return None;
    }
    CStr::from_ptr(ptr).to_str().ok()
}

/// Move a Rust `String` into a freshly allocated C string. Returns null if the
/// string contains an interior NUL byte (which JSON never does).
fn string_to_cstr(s: String) -> *const c_char {
    match CString::new(s) {
        Ok(c) => c.into_raw() as *const c_char,
        Err(_) => std::ptr::null(),
    }
}

/// Borrow the `Arc<Storage>` from a raw handle without taking ownership.
///
/// # Safety
/// `handle` must be null or a pointer previously returned by
/// [`genesisdb_open`] and not yet passed to [`genesisdb_close`].
unsafe fn handle_storage<'a>(handle: *mut GenesisHandle) -> Option<&'a Arc<Storage>> {
    handle.as_ref().map(|h| &h.storage)
}

/// Run a JSON-returning body, swallowing any panic and returning null on a
/// caught panic. Errors inside the closure are the closure's own concern (it
/// returns null for them too).
fn guard_json<F: FnOnce() -> *const c_char>(f: F) -> *const c_char {
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(ptr) => ptr,
        Err(_) => std::ptr::null(),
    }
}

// --- exported C ABI ---------------------------------------------------------

/// Open or create a GenesisDB at `path`. Returns an opaque handle, or null on
/// error (null/invalid path, or the engine failed to open).
///
/// # Safety
/// `path` must be a valid NUL-terminated C string.
#[no_mangle]
pub extern "C" fn genesisdb_open(path: *const c_char) -> *mut GenesisHandle {
    let result = catch_unwind(|| {
        let path = match unsafe { cstr_to_str(path) } {
            Some(p) => p.to_string(),
            None => return std::ptr::null_mut(),
        };
        let opts = OpenOptions {
            path,
            page_cache_mb: None,
            read_only: None,
            vector_dim: None,
            retention: None,
        };
        match Storage::open(opts) {
            Ok(storage) => {
                let handle = GenesisHandle {
                    storage: Arc::new(storage),
                };
                Box::into_raw(Box::new(handle))
            }
            Err(_) => std::ptr::null_mut(),
        }
    });
    result.unwrap_or(std::ptr::null_mut())
}

/// Close a handle and free its resources. No-op on null. After this call the
/// pointer is dangling and must not be reused.
///
/// # Safety
/// `handle` must be null or a pointer from [`genesisdb_open`] that has not
/// already been closed.
#[no_mangle]
pub extern "C" fn genesisdb_close(handle: *mut GenesisHandle) {
    if handle.is_null() {
        return;
    }
    // Reclaim the Box; dropping it drops the Arc<Storage>, which (when it is the
    // last owner) joins the WAL/index threads via Storage::Drop.
    let _ = catch_unwind(AssertUnwindSafe(|| unsafe {
        drop(Box::from_raw(handle));
    }));
}

/// Add a node. `json_input` is a [`NodeInput`] JSON string; returns a
/// [`NodeOutput`] JSON string (free with [`genesisdb_free_string`]) or null on
/// error.
///
/// # Safety
/// `handle` must be a live handle; `json_input` a valid C string.
#[no_mangle]
pub extern "C" fn genesisdb_add_node(
    handle: *mut GenesisHandle,
    json_input: *const c_char,
) -> *const c_char {
    guard_json(|| {
        let storage = match unsafe { handle_storage(handle) } {
            Some(s) => s,
            None => return std::ptr::null(),
        };
        let json = match unsafe { cstr_to_str(json_input) } {
            Some(j) => j,
            None => return std::ptr::null(),
        };
        let input: NodeInput = match serde_json::from_str(json) {
            Ok(i) => i,
            Err(_) => return std::ptr::null(),
        };
        match storage.add_node(input) {
            Ok(output) => match serde_json::to_string(&output) {
                Ok(s) => string_to_cstr(s),
                Err(_) => std::ptr::null(),
            },
            Err(_) => std::ptr::null(),
        }
    })
}

/// Hybrid (vector + lexical) search. `json_input` is a [`HybridSearchInput`]
/// JSON string; returns a JSON array of `NeighborOutput` (free with
/// [`genesisdb_free_string`]) or null on error.
///
/// # Safety
/// `handle` must be a live handle; `json_input` a valid C string.
#[no_mangle]
pub extern "C" fn genesisdb_search(
    handle: *mut GenesisHandle,
    json_input: *const c_char,
) -> *const c_char {
    guard_json(|| {
        let storage = match unsafe { handle_storage(handle) } {
            Some(s) => s,
            None => return std::ptr::null(),
        };
        let json = match unsafe { cstr_to_str(json_input) } {
            Some(j) => j,
            None => return std::ptr::null(),
        };
        let input: HybridSearchInput = match serde_json::from_str(json) {
            Ok(i) => i,
            Err(_) => return std::ptr::null(),
        };
        match storage.hybrid_search(input) {
            Ok(results) => match serde_json::to_string(&results) {
                Ok(s) => string_to_cstr(s),
                Err(_) => std::ptr::null(),
            },
            Err(_) => std::ptr::null(),
        }
    })
}

/// Execute an HQL query string. Returns the query result as a JSON string (free
/// with [`genesisdb_free_string`]) or null on error.
///
/// # Safety
/// `handle` must be a live handle; `hql` a valid C string.
#[no_mangle]
pub extern "C" fn genesisdb_execute_hql(
    handle: *mut GenesisHandle,
    hql: *const c_char,
) -> *const c_char {
    guard_json(|| {
        let storage = match unsafe { handle_storage(handle) } {
            Some(s) => s,
            None => return std::ptr::null(),
        };
        let query = match unsafe { cstr_to_str(hql) } {
            Some(q) => q,
            None => return std::ptr::null(),
        };
        match storage.execute_hql(query) {
            Ok(value) => match serde_json::to_string(&value) {
                Ok(s) => string_to_cstr(s),
                Err(_) => std::ptr::null(),
            },
            Err(_) => std::ptr::null(),
        }
    })
}

/// Retrieve a tiered context package. `json_input` is a `RetrieveContextInput`
/// JSON string: `{ "target_id": "...", "tier": "H1", "budget": null, "fuzzy":
/// false }`. Returns a [`crate::ContextPackage`] JSON string (free with
/// [`genesisdb_free_string`]) or null on error.
///
/// # Safety
/// `handle` must be a live handle; `json_input` a valid C string.
#[no_mangle]
pub extern "C" fn genesisdb_retrieve_context(
    handle: *mut GenesisHandle,
    json_input: *const c_char,
) -> *const c_char {
    guard_json(|| {
        let storage = match unsafe { handle_storage(handle) } {
            Some(s) => s,
            None => return std::ptr::null(),
        };
        let json = match unsafe { cstr_to_str(json_input) } {
            Some(j) => j,
            None => return std::ptr::null(),
        };
        let input: RetrieveContextInput = match serde_json::from_str(json) {
            Ok(i) => i,
            Err(_) => return std::ptr::null(),
        };
        match storage.retrieve_context(&input.target_id, &input.tier, input.budget, input.fuzzy) {
            Ok(pkg) => match serde_json::to_string(&pkg) {
                Ok(s) => string_to_cstr(s),
                Err(_) => std::ptr::null(),
            },
            Err(_) => std::ptr::null(),
        }
    })
}

/// Register a versioned relational schema package encoded as JSON.
#[no_mangle]
pub extern "C" fn genesisdb_register_relational_schema(
    handle: *mut GenesisHandle,
    json_input: *const c_char,
) -> *const c_char {
    guard_json(|| {
        let storage = match unsafe { handle_storage(handle) } {
            Some(storage) => storage,
            None => return std::ptr::null(),
        };
        let json = match unsafe { cstr_to_str(json_input) } {
            Some(json) => json,
            None => return std::ptr::null(),
        };
        let package = match serde_json::from_str::<RelationalSchemaPackage>(json) {
            Ok(package) => package,
            Err(_) => return std::ptr::null(),
        };
        match storage.register_relational_schema(package) {
            Ok(version) => string_to_cstr(version.to_string()),
            Err(_) => std::ptr::null(),
        }
    })
}

/// Return the current relational schema package for a namespace.
#[no_mangle]
pub extern "C" fn genesisdb_get_relational_schema(
    handle: *mut GenesisHandle,
    namespace: *const c_char,
) -> *const c_char {
    guard_json(|| {
        let output = (|| -> Option<String> {
            let storage = unsafe { handle_storage(handle) }?;
            let namespace = unsafe { cstr_to_str(namespace) }?;
            let package = storage.get_relational_schema(namespace).ok()?;
            serde_json::to_string(&package).ok()
        })();
        output.map(string_to_cstr).unwrap_or(std::ptr::null())
    })
}

/// Apply an idempotent U2 mutation batch encoded as JSON.
#[no_mangle]
pub extern "C" fn genesisdb_apply_relational_batch(
    handle: *mut GenesisHandle,
    json_input: *const c_char,
) -> *const c_char {
    guard_json(|| {
        let output = (|| -> Option<String> {
            let storage = unsafe { handle_storage(handle) }?;
            let json = unsafe { cstr_to_str(json_input) }?;
            let batch = serde_json::from_str::<RelationalMutationBatch>(json).ok()?;
            let result = storage.apply_relational_batch(batch).ok()?;
            serde_json::to_string(&result).ok()
        })();
        output.map(string_to_cstr).unwrap_or(std::ptr::null())
    })
}

/// Apply a typed relational mutation batch encoded as JSON.
#[no_mangle]
pub extern "C" fn genesisdb_apply_relational_rows(
    handle: *mut GenesisHandle,
    json_input: *const c_char,
) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        let storage = match unsafe { handle_storage(handle) } {
            Some(storage) => storage,
            None => return 1,
        };
        let json = match unsafe { cstr_to_str(json_input) } {
            Some(json) => json,
            None => return 1,
        };
        let input = match serde_json::from_str::<RelationalRowsInput>(json) {
            Ok(input) => input,
            Err(_) => return 1,
        };
        match storage.apply_relational_rows(&input.namespace, input.mutations) {
            Ok(()) => 0,
            Err(_) => 1,
        }
    }))
    .unwrap_or(2)
}

/// Execute a bounded relational query encoded as JSON.
#[no_mangle]
pub extern "C" fn genesisdb_query_relational(
    handle: *mut GenesisHandle,
    json_input: *const c_char,
) -> *const c_char {
    guard_json(|| {
        let storage = match unsafe { handle_storage(handle) } {
            Some(storage) => storage,
            None => return std::ptr::null(),
        };
        let json = match unsafe { cstr_to_str(json_input) } {
            Some(json) => json,
            None => return std::ptr::null(),
        };
        let query = match serde_json::from_str::<RelationalQuery>(json) {
            Ok(query) => query,
            Err(_) => return std::ptr::null(),
        };
        match storage.query_relational(query) {
            Ok(rows) => match serde_json::to_string(&rows) {
                Ok(json) => string_to_cstr(json),
                Err(_) => std::ptr::null(),
            },
            Err(_) => std::ptr::null(),
        }
    })
}

/// Execute a registered named query encoded as JSON.
#[no_mangle]
pub extern "C" fn genesisdb_execute_named_query(
    handle: *mut GenesisHandle,
    json_input: *const c_char,
) -> *const c_char {
    guard_json(|| {
        let output = (|| -> Option<String> {
            let storage = unsafe { handle_storage(handle) }?;
            let json = unsafe { cstr_to_str(json_input) }?;
            let request = serde_json::from_str::<NamedQueryRequest>(json).ok()?;
            let rows = storage.execute_named_query(request).ok()?;
            serde_json::to_string(&rows).ok()
        })();
        output.map(string_to_cstr).unwrap_or(std::ptr::null())
    })
}

/// Commit one canonical cross-domain transaction encoded as JSON.
#[no_mangle]
pub extern "C" fn genesisdb_commit_transaction(
    handle: *mut GenesisHandle,
    json_input: *const c_char,
) -> *const c_char {
    guard_json(|| {
        let storage = match unsafe { handle_storage(handle) } {
            Some(storage) => storage,
            None => return std::ptr::null(),
        };
        let json = match unsafe { cstr_to_str(json_input) } {
            Some(json) => json,
            None => return std::ptr::null(),
        };
        let transaction = match serde_json::from_str::<GenesisTransaction>(json) {
            Ok(transaction) => transaction,
            Err(_) => return std::ptr::null(),
        };
        match storage.commit_transaction(transaction) {
            Ok(result) => match serde_json::to_string(&result) {
                Ok(json) => string_to_cstr(json),
                Err(_) => std::ptr::null(),
            },
            Err(_) => std::ptr::null(),
        }
    })
}

/// Flush the async HNSW index so staged vectors become searchable
/// (read-your-write). Returns 0 on success, nonzero on a null/invalid handle or
/// a caught panic.
///
/// # Safety
/// `handle` must be null or a live handle.
#[no_mangle]
pub extern "C" fn genesisdb_flush_index(handle: *mut GenesisHandle) -> i32 {
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

/// Free a string previously returned by any `genesisdb_*` function. No-op on
/// null. Each returned string must be freed exactly once.
///
/// # Safety
/// `s` must be null or a pointer returned by this library and not yet freed.
#[no_mangle]
pub extern "C" fn genesisdb_free_string(s: *const c_char) {
    if s.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| unsafe {
        // Reclaim the CString allocated by `into_raw` so it is dropped/freed.
        drop(CString::from_raw(s as *mut c_char));
    }));
}
