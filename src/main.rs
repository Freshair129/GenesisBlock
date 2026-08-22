use parking_lot::RwLock;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use genesis_block_native::router::{build_router, AppState};
use genesis_block_native::{OpenOptions, Storage};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "genesis_db_server=info,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let data_dir =
        std::env::var("GENESIS_DATA_DIR").unwrap_or_else(|_| ".brain/gks/storage".into());
    let port: u16 = std::env::var("GENESIS_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);
    let host: std::net::IpAddr = std::env::var("GENESIS_HOST")
        .ok()
        .and_then(|h| h.parse().ok())
        .unwrap_or(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST));

    let storage = Storage::open(OpenOptions {
        path: data_dir,
        page_cache_mb: Some(64),
        read_only: Some(false),
        vector_dim: None,
        retention: std::env::var("GENESIS_RETENTION").ok(),
    })?;

    let api_key = std::env::var("GENESIS_API_KEY").ok();
    if api_key.is_some() {
        tracing::info!("API key authentication enabled (GENESIS_API_KEY is set)");
    } else {
        tracing::warn!("No GENESIS_API_KEY set — server is unauthenticated; set it for any non-local deployment");
    }
    let storage = Arc::new(RwLock::new(storage));
    let state = AppState {
        storage: Arc::clone(&storage),
        api_key,
    };

    let app = build_router(state);

    let addr = SocketAddr::new(host, port);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("GenesisBlockDB Standalone Server listening on {} (set GENESIS_HOST=0.0.0.0 to bind all interfaces)", addr);

    // Graceful shutdown (storage-readiness audit: the server never
    // checkpointed and `Drop` never ran). `with_graceful_shutdown` stops
    // accepting, lets in-flight requests finish, and only then returns — so
    // the checkpoint below runs on a quiescent engine rather than racing a
    // write. Durability does NOT depend on this: every acked write is already
    // in the journal and replays on the next open. What the checkpoint buys is
    // an instant next start (snapshot instead of full replay) and, under
    // frontier_only, the fold that bounds journal growth — both of which a
    // long-lived server otherwise never gets.
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    tracing::info!("shutdown signal received; checkpointing before exit");
    match storage.read().save_state() {
        Ok(()) => tracing::info!("checkpoint complete"),
        // A failed checkpoint is reported, never hidden, and never fatal:
        // the journal still holds every acked write, so exiting is safe.
        Err(e) => tracing::error!("final checkpoint failed (journal remains authoritative): {e}"),
    }
    Ok(())
}

/// Resolves on SIGINT (Ctrl-C) everywhere, and additionally on SIGTERM on
/// Unix — the signal every container runtime and process supervisor sends
/// first, which a Ctrl-C-only handler would miss.
async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(e) = tokio::signal::ctrl_c().await {
            tracing::error!("failed to install Ctrl-C handler: {e}");
        }
    };
    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut sig) => {
                sig.recv().await;
            }
            Err(e) => tracing::error!("failed to install SIGTERM handler: {e}"),
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
