mod config;
mod db;
mod models;
mod routes;
mod sync;

use std::net::SocketAddr;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info};

use crate::config::AppConfig;
use crate::db::Database;
use crate::routes::{build_router, AppState};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "channelzero=info,tower_http=warn".into()),
        )
        .init();

    let config = AppConfig::load_and_validate()?;
    info!(
        "Starting ChannelZero on port {} (region={}, tz={}, lookback={}d, refresh={}h)",
        config.port,
        config.region,
        config.timezone,
        config.new_release_days,
        config.refresh_interval_hours
    );

    let db = Database::connect_and_init(&config.data_dir).await?;
    let is_syncing = Arc::new(AtomicBool::new(false));

    // Spawn background periodic sync (runs immediately on startup, then every refresh_interval_hours)
    {
        let db_bg = db.clone();
        let cfg_bg = config.clone();
        let flag_bg = is_syncing.clone();
        tokio::spawn(async move {
            let interval_secs = (cfg_bg.refresh_interval_hours as u64) * 3600;
            loop {
                info!("Running scheduled metadata sync...");
                if let Err(e) = sync::run_sync(&db_bg, &cfg_bg, flag_bg.clone()).await {
                    error!("Scheduled sync failed: {e}");
                }
                tokio::time::sleep(Duration::from_secs(interval_secs)).await;
            }
        });
    }

    let state = AppState {
        db,
        config: config.clone(),
        is_syncing,
    };

    let app = build_router(state);
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("ChannelZero listening on http://{addr}");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let terminate = async {
        if let Ok(mut sig) =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            sig.recv().await;
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    info!("Shutdown signal received, exiting.");
}
