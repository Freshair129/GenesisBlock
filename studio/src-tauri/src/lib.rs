use std::sync::{Arc, Mutex};

use genesis_block_native::{NamedQueryRequest, OpenOptions, Storage, StudioGraphSceneRequest};
use serde::Serialize;
use serde_json::{json, Value};
use tauri::State;

#[derive(Default)]
struct StudioState {
    storage: Arc<Mutex<Option<Storage>>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StudioShellInfo {
    shell_version: &'static str,
    transport: &'static str,
    engine_attached: bool,
}

#[tauri::command]
fn studio_shell_info() -> StudioShellInfo {
    StudioShellInfo {
        shell_version: env!("CARGO_PKG_VERSION"),
        transport: "negotiated",
        engine_attached: false,
    }
}

fn with_storage<T>(
    state: &Arc<Mutex<Option<Storage>>>,
    operation: impl FnOnce(&Storage) -> Result<T, String>,
) -> Result<T, String> {
    let guard = state
        .lock()
        .map_err(|_| "Studio storage state is poisoned".to_string())?;
    let storage = guard
        .as_ref()
        .ok_or_else(|| "No local GenesisBlockDB is open".to_string())?;
    operation(storage)
}

#[tauri::command]
async fn studio_open_local(state: State<'_, StudioState>, path: String) -> Result<Value, String> {
    if path.trim().is_empty() {
        return Err("A local GenesisBlockDB data-root path is required".to_string());
    }
    let storage_state = Arc::clone(&state.storage);
    tauri::async_runtime::spawn_blocking(move || {
        let storage = Storage::open(OpenOptions {
            path,
            page_cache_mb: Some(64),
            read_only: Some(true),
            vector_dim: None,
        })
        .map_err(|error| error.to_string())?;
        let capabilities = storage.studio_capabilities("local", vec!["os-user".to_string()]);
        let value = serde_json::to_value(capabilities).map_err(|error| error.to_string())?;
        let mut guard = storage_state
            .lock()
            .map_err(|_| "Studio storage state is poisoned".to_string())?;
        *guard = Some(storage);
        Ok(value)
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
async fn studio_close_local(state: State<'_, StudioState>) -> Result<(), String> {
    let storage_state = Arc::clone(&state.storage);
    tauri::async_runtime::spawn_blocking(move || {
        let storage = storage_state
            .lock()
            .map_err(|_| "Studio storage state is poisoned".to_string())?
            .take();
        drop(storage);
        Ok(())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
async fn studio_local_capabilities(state: State<'_, StudioState>) -> Result<Value, String> {
    let storage_state = Arc::clone(&state.storage);
    tauri::async_runtime::spawn_blocking(move || {
        with_storage(&storage_state, |storage| {
            serde_json::to_value(storage.studio_capabilities("local", vec!["os-user".to_string()]))
                .map_err(|error| error.to_string())
        })
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
async fn studio_local_status(state: State<'_, StudioState>) -> Result<Value, String> {
    let storage_state = Arc::clone(&state.storage);
    tauri::async_runtime::spawn_blocking(move || {
        with_storage(&storage_state, |storage| {
            Ok(json!({
                "open": true,
                "read_only": storage.read_only,
                "node_count": storage.nodes.len(),
                "edge_count": storage.edges.len(),
                "collection_count": storage.collections.len(),
                "index_lag": storage.index_lag(),
                "logical_clock": storage.get_logical_clock(),
                "memory_usage_mb": Value::Null,
                "frontier": storage.stable_frontier()
            }))
        })
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
async fn studio_local_collections(state: State<'_, StudioState>) -> Result<Value, String> {
    let storage_state = Arc::clone(&state.storage);
    tauri::async_runtime::spawn_blocking(move || {
        with_storage(&storage_state, |storage| {
            serde_json::to_value(storage.list_collections()).map_err(|error| error.to_string())
        })
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
async fn studio_local_relational_schemas(state: State<'_, StudioState>) -> Result<Value, String> {
    let storage_state = Arc::clone(&state.storage);
    tauri::async_runtime::spawn_blocking(move || {
        with_storage(&storage_state, |storage| {
            let schemas = storage
                .list_relational_schemas()
                .map_err(|error| error.to_string())?;
            serde_json::to_value(schemas).map_err(|error| error.to_string())
        })
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
async fn studio_local_graph(
    state: State<'_, StudioState>,
    request: StudioGraphSceneRequest,
) -> Result<Value, String> {
    let storage_state = Arc::clone(&state.storage);
    tauri::async_runtime::spawn_blocking(move || {
        with_storage(&storage_state, |storage| {
            let scene = storage
                .studio_graph_scene(request)
                .map_err(|error| error.to_string())?;
            serde_json::to_value(scene).map_err(|error| error.to_string())
        })
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
async fn studio_local_inspect(
    state: State<'_, StudioState>,
    entity_id: String,
) -> Result<Value, String> {
    let storage_state = Arc::clone(&state.storage);
    tauri::async_runtime::spawn_blocking(move || {
        with_storage(&storage_state, |storage| {
            let inspection = storage
                .studio_inspect_entity(&entity_id)
                .map_err(|error| error.to_string())?;
            serde_json::to_value(inspection).map_err(|error| error.to_string())
        })
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
async fn studio_local_hql(state: State<'_, StudioState>, query: String) -> Result<Value, String> {
    let storage_state = Arc::clone(&state.storage);
    tauri::async_runtime::spawn_blocking(move || {
        with_storage(&storage_state, |storage| {
            storage
                .execute_hql_read_only(&query)
                .map_err(|error| error.to_string())
        })
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
async fn studio_local_named_query(
    state: State<'_, StudioState>,
    request: NamedQueryRequest,
) -> Result<Value, String> {
    let storage_state = Arc::clone(&state.storage);
    tauri::async_runtime::spawn_blocking(move || {
        with_storage(&storage_state, |storage| {
            let rows = storage
                .execute_named_query(request)
                .map_err(|error| error.to_string())?;
            Ok(Value::Array(rows))
        })
    })
    .await
    .map_err(|error| error.to_string())?
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(StudioState::default())
        .invoke_handler(tauri::generate_handler![
            studio_shell_info,
            studio_open_local,
            studio_close_local,
            studio_local_capabilities,
            studio_local_status,
            studio_local_collections,
            studio_local_relational_schemas,
            studio_local_graph,
            studio_local_inspect,
            studio_local_hql,
            studio_local_named_query
        ])
        .run(tauri::generate_context!())
        .expect("error while running Genesis Studio");
}

#[cfg(test)]
mod tests {
    use super::studio_shell_info;

    #[test]
    fn shell_starts_detached_and_negotiates_its_transport() {
        let info = studio_shell_info();
        assert_eq!(info.transport, "negotiated");
        assert!(!info.engine_attached);
    }
}
