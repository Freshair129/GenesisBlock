//! Tauri command surface over the embedded GenesisBlockDB engine.
//!
//! Each command holds `Arc<Storage>` as Tauri-managed state, clones the `Arc`,
//! and runs the **synchronous** engine call on a blocking thread via
//! `tauri::async_runtime::spawn_blocking`. The engine error type implements
//! `Display`, so errors are mapped to `String` with `.to_string()`.
//!
//! Command names here are the IPC contract the frontend invokes — do not rename
//! without updating the frontend.

use std::sync::Arc;

use genesis_block_native::{
    ContextPackage, DatabaseStatus, HybridSearchInput, NeighborInput, NeighborOutput, NodeInput,
    NodeOutput, Storage,
};
use tauri::State;

/// Shared handle managed by Tauri (`app.manage(Arc<Storage>)`).
type Db<'a> = State<'a, Arc<Storage>>;

/// Add a node (stages its embedding; searchable after `flush_index`).
#[tauri::command]
pub async fn add_node(db: Db<'_>, input: NodeInput) -> Result<NodeOutput, String> {
    let storage = db.inner().clone();
    tauri::async_runtime::spawn_blocking(move || storage.add_node(input))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

/// Hybrid (vector + lexical) search.
#[tauri::command]
pub async fn search(db: Db<'_>, input: HybridSearchInput) -> Result<Vec<NeighborOutput>, String> {
    let storage = db.inner().clone();
    tauri::async_runtime::spawn_blocking(move || storage.hybrid_search(input))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

/// Execute a raw HQL query; returns the engine's JSON result.
#[tauri::command]
pub async fn execute_hql(db: Db<'_>, query: String) -> Result<serde_json::Value, String> {
    let storage = db.inner().clone();
    tauri::async_runtime::spawn_blocking(move || storage.execute_hql(&query))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

/// Retrieve a tiered context package — the primary graph + context source.
#[tauri::command]
pub async fn retrieve_context(
    db: Db<'_>,
    target_id: String,
    tier: String,
    budget: Option<u32>,
    fuzzy: bool,
) -> Result<ContextPackage, String> {
    let storage = db.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        storage.retrieve_context(&target_id, &tier, budget, fuzzy)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}

/// Graph neighbors of `seed` up to `depth` (current view, not inferred).
#[tauri::command]
pub async fn neighbors(db: Db<'_>, seed: String, depth: u32) -> Result<Vec<NeighborOutput>, String> {
    let storage = db.inner().clone();
    let args = NeighborInput {
        depth: Some(depth),
        rel: None,
        rels: None,
        direction: None,
        as_of: None,
        include_invalid: None,
        limit: None,
    };
    tauri::async_runtime::spawn_blocking(move || storage.neighbors(seed, args, false))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

/// Flush the async HNSW index so staged vectors become searchable.
#[tauri::command]
pub async fn flush_index(db: Db<'_>) -> Result<(), String> {
    let storage = db.inner().clone();
    tauri::async_runtime::spawn_blocking(move || storage.flush_index())
        .await
        .map_err(|e| e.to_string())
}

/// Current database status. `status_sync` is infallible.
#[tauri::command]
pub async fn get_status(db: Db<'_>) -> Result<DatabaseStatus, String> {
    let storage = db.inner().clone();
    tauri::async_runtime::spawn_blocking(move || storage.status_sync())
        .await
        .map_err(|e| e.to_string())
}
