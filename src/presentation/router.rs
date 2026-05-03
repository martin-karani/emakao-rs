use axum::{middleware, Router};
use tower_http::{compression::CompressionLayer, cors::CorsLayer, trace::TraceLayer};
use utoipa_swagger_ui::SwaggerUi;

use crate::presentation::{
    app_state::AppState,
    http::routes::{
        agency_routes, agreement_routes, auth_routes, health_routes, ledger_routes,
        maintenance_routes, owner_routes, payment_routes, property_routes, resident_routes,
        subscription_routes, utility_routes, vendor_routes, webhook_routes, websocket_routes,
    },
    middleware::{
        admin_auth::require_admin, auth::require_auth, subscription::subscription_middleware,
        tenant_resolver::tenant_resolver,
    },
    openapi::ApiDoc,
};

pub fn build_router(state: AppState) -> Router {
    // ── Health + readiness (no middleware) ────────────────────────────────────
    let infra = health_routes::routes();

    // ── Webhooks (no auth — Safaricom calls these directly) ───────────────────
    let webhooks = webhook_routes::routes();

    // ── WebSocket (self-authenticating via ?token= query param) ──────────────
    let websocket = websocket_routes::routes();

    // ── Public subscription routes (no auth — browse plans) ──────────────────
    let public_sub = subscription_routes::public_routes();

    let auth = auth_routes::routes().layer(middleware::from_fn_with_state(
        state.clone(),
        tenant_resolver,
    ));

    // Agencies use these when they don't yet have an active subscription.
    let billing_api = subscription_routes::billing_routes()
        .layer(middleware::from_fn_with_state(state.clone(), require_auth))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            tenant_resolver,
        ));

    // ── Protected API (auth + tenant + active subscription) ──────────────────
    let api = Router::new()
        .merge(property_routes::routes())
        .merge(resident_routes::routes())
        .merge(agreement_routes::routes())
        .merge(payment_routes::routes())
        .merge(ledger_routes::routes())
        .merge(utility_routes::routes())
        .merge(maintenance_routes::routes())
        .merge(owner_routes::routes())
        .merge(vendor_routes::routes())
        .merge(subscription_routes::agency_routes(state.clone()))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            subscription_middleware,
        ))
        .layer(middleware::from_fn_with_state(state.clone(), require_auth))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            tenant_resolver,
        ));

    let platform_admin = Router::new()
        .merge(agency_routes::admin_routes())
        .merge(subscription_routes::admin_routes(state.clone()))
        .layer(middleware::from_fn_with_state(state.clone(), require_admin));

    let mut router = Router::new()
        .merge(infra)
        .merge(webhooks)
        .merge(websocket)
        .merge(public_sub)
        .merge(auth)
        .merge(billing_api)
        .merge(api)
        .merge(platform_admin)
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state);

    // ── Swagger UI — only compiled into debug builds ──────────────────────────
    // Access at: http://localhost:3000/swagger-ui
    // Raw JSON:  http://localhost:3000/api-docs/openapi.json
    #[cfg(debug_assertions)]
    {
        use utoipa::OpenApi;
        router = router
            .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()));
    }

    router
}
