use std::sync::Arc;

use emakao::{
    config::Config,
    infrastructure::scheduler::billing_worker::{build_billing_monitor, BillingContext},
    presentation::{app_state::AppState, router::build_router},
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // ── Environment & tracing ─────────────────────────────────────────────────
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "emakao=debug,tower_http=debug,sqlx=warn".into()),
        )
        .json()
        .init();

    // ── Config ────────────────────────────────────────────────────────────────
    let cfg = Arc::new(Config::from_env()?);

    // ── AppState (builds DB pools, Redis, all adapters internally) ────────────
    let state = AppState::build(Arc::clone(&cfg)).await?;

    // ── Billing monitor ───────────────────────────────────────────────────────
    //
    // BillingContext is a narrow view of AppState that the billing workers need.
    // It avoids passing the full AppState into Apalis jobs.
    let billing_ctx = BillingContext {
        pool_manager: Arc::clone(&state.infra.tenant_pools),
        notifications: state.notifications.clone(), // ✓
    };

    // Redis is exposed from InfraState so the billing monitor can subscribe to
    // queues without going through AppState.
    let billing_monitor = build_billing_monitor(billing_ctx, cfg.redis_url.clone()).await?;

    tokio::spawn(async move {
        if let Err(e) = billing_monitor.run().await {
            tracing::error!(err = %e, "billing monitor crashed");
        }
    });

    // ── HTTP server ───────────────────────────────────────────────────────────
    let router = build_router(state);
    let addr = format!("0.0.0.0:{}", cfg.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    tracing::info!(addr = %addr, "emakao listening");
    axum::serve(listener, router).await?;

    Ok(())
}
