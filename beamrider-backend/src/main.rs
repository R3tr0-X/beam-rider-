use std::net::SocketAddr;

use beamrider_backend::{
    config::AppConfig, middleware::trace::init_tracing, routes, state::AppState, workers,
};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    let config = AppConfig::from_env()?;
    let addr: SocketAddr = format!("{}:{}", config.host, config.port).parse()?;
    let state = AppState::from_config(config).await?;
    let _workers = workers::spawn_enabled(&state);
    let app = routes::router(state);

    let listener = TcpListener::bind(addr).await?;
    tracing::info!(%addr, "BeamRider backend listening");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install terminate handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
}
