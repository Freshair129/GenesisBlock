use std::net::SocketAddr;
use std::sync::Arc;
use parking_lot::RwLock;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use genesis_block_native::{Storage, OpenOptions};
use genesis_block_native::router::{build_router, AppState};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "genesis_db_server=info,tower_http=debug".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let data_dir = std::env::var("GENESIS_DATA_DIR").unwrap_or_else(|_| ".brain/gks/storage".into());
    let port: u16 = std::env::var("GENESIS_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);

    let storage = Storage::open(OpenOptions {
        path: data_dir,
        page_cache_mb: Some(64),
        read_only: Some(false),
        vector_dim: None,
    })?;

    let state = AppState {
        storage: Arc::new(RwLock::new(storage)),
    };

    let app = build_router(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("GenesisBlockDB Standalone Server listening on {}", addr);
    axum::serve(listener, app).await?;

    Ok(())
}
