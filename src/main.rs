use std::sync::Arc;
use anyhow::Result;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod domain;
mod application;
mod infrastructure;
mod presentation;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "emakao_backend=debug,tower_http=debug,sqlx=warn".into()),
        )
        .with(tracing_subscriber::fmt::layer().with_target(true))
        .init();

    let cfg = Arc::new(config::Config::from_env()?);
    tracing::info!(port = cfg.port, "building application state…");

    let state = presentation::app_state::AppState::build(Arc::clone(&cfg)).await?;
    let router = presentation::router::build_router(state);

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], cfg.port));
    tracing::info!(%addr, "emakao-backend listening");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, router).await?;
    Ok(())
}