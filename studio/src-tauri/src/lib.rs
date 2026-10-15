use std::sync::{Arc, Mutex};

use genesis_block_native::{NamedQueryRequest, OpenOptions, Storage, StudioGraphSceneRequest};
use serde::Serialize;
use serde_json::{json, Value};
use tauri::State;
#[cfg(desktop)]
use tauri_plugin_dialog::DialogExt;

mod remote;
use remote::{RemoteConnection, RemoteState, StudioRemoteRequest};

#[derive(Default)]
struct StudioState {
    storage: Arc<Mutex<Option<Storage>>>,
    remote: RemoteState,
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

#[tauri::command]
async fn studio_pick_local_root(app: tauri::AppHandle) -> Result<Option<String>, String> {
    #[cfg(desktop)]
    {
        app.dialog()
            .file()
            .set_title("Select a GenesisBlockDB data root")
            .blocking_pick_folder()
            .map(|selected| {
                selected
                    .into_path()
                    .map(|path| path.to_string_lossy().into_owned())
                    .map_err(|error| error.to_string())
            })
            .transpose()
    }
    #[cfg(not(desktop))]
    {
        let _ = app;
        Err("Native local-root selection is only available in desktop builds".to_string())
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

fn validate_local_root_path(path: &str) -> Result<(), String> {
    if path.trim().is_empty() {
        return Err("A local GenesisBlockDB data-root path is required".to_string());
    }
    let metadata = std::fs::metadata(path).map_err(|error| {
        format!(
            "The GenesisBlockDB data root '{}' could not be accessed: {error}. Choose an existing folder.",
            path
        )
    })?;
    if !metadata.is_dir() {
        return Err(format!(
            "The GenesisBlockDB data root '{}' is not a folder. Choose an existing folder.",
            path
        ));
    }
    Ok(())
}

#[tauri::command]
async fn studio_open_local(state: State<'_, StudioState>, path: String) -> Result<Value, String> {
    let storage_state = Arc::clone(&state.storage);
    tauri::async_runtime::spawn_blocking(move || {
        validate_local_root_path(&path)?;
        let storage = Storage::open(OpenOptions {
            path,
            page_cache_mb: Some(64),
            read_only: Some(true),
            vector_dim: None,
            retention: None,
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

#[tauri::command]
async fn studio_remote_configure(
    state: State<'_, StudioState>,
    base_url: String,
    token: String,
) -> Result<(), String> {
    let configuration = state.remote.begin_reconfigure()?;
    let connection = RemoteConnection::new(&base_url, &token)?;
    state.remote.install(configuration, connection)
}

#[tauri::command]
async fn studio_remote_request(
    state: State<'_, StudioState>,
    request: StudioRemoteRequest,
) -> Result<Value, String> {
    let connection = state.remote.connection()?;
    connection.request(request).await
}

#[tauri::command]
async fn studio_remote_close(state: State<'_, StudioState>) -> Result<(), String> {
    state.remote.close()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(StudioState::default())
        .invoke_handler(tauri::generate_handler![
            studio_shell_info,
            studio_pick_local_root,
            studio_open_local,
            studio_close_local,
            studio_local_capabilities,
            studio_local_status,
            studio_local_collections,
            studio_local_relational_schemas,
            studio_local_graph,
            studio_local_inspect,
            studio_local_hql,
            studio_local_named_query,
            studio_remote_configure,
            studio_remote_request,
            studio_remote_close
        ])
        .run(tauri::generate_context!())
        .expect("error while running Genesis Studio");
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{studio_shell_info, validate_local_root_path};

    #[test]
    fn shell_starts_detached_and_negotiates_its_transport() {
        let info = studio_shell_info();
        assert_eq!(info.transport, "negotiated");
        assert!(!info.engine_attached);
    }

    #[test]
    fn local_root_validation_rejects_blank_missing_and_non_directory_paths() {
        assert!(validate_local_root_path("  ")
            .unwrap_err()
            .contains("path is required"));

        let missing = std::env::temp_dir().join(format!(
            "Genesis Studio missing root {}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&missing);
        assert!(validate_local_root_path(&missing.to_string_lossy())
            .unwrap_err()
            .contains("could not be accessed"));

        let root = std::env::temp_dir().join(format!(
            "Genesis Studio root validation - 雪 {}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let file = root.join("not a folder.txt");
        fs::write(&file, "test").unwrap();
        assert!(validate_local_root_path(&file.to_string_lossy())
            .unwrap_err()
            .contains("is not a folder"));
        assert!(validate_local_root_path(&root.to_string_lossy()).is_ok());
        fs::remove_dir_all(root).unwrap();
    }
}
