//! GenesisBlock mobile app — Tauri v2 backend.
//!
//! All app logic lives here (`main.rs` is a thin passthrough). On setup we open
//! the embedded GenesisBlockDB engine under the platform app-data directory,
//! wrap it in an `Arc`, and manage it as Tauri state so every command can offload
//! the synchronous engine calls onto a blocking thread.

use std::sync::Arc;

use genesis_block_native::{OpenOptions, Storage};
use tauri::Manager;

mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            // Resolve a per-app, per-platform data directory and place the
            // database under a stable `genesisdb` subfolder.
            let mut db_path = app.path().app_data_dir()?;
            db_path.push("genesisdb");
            std::fs::create_dir_all(&db_path)?;

            let storage = Storage::open(OpenOptions {
                path: db_path.to_string_lossy().into_owned(),
                page_cache_mb: None,
                read_only: None,
                vector_dim: None,
            })
            .map_err(|e| e.to_string())?;

            app.manage(Arc::new(storage));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::add_node,
            commands::search,
            commands::execute_hql,
            commands::retrieve_context,
            commands::neighbors,
            commands::flush_index,
            commands::get_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
