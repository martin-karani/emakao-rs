use axum::{
    middleware,
    routing::{delete, get, patch, post},
    Router,
};

use crate::presentation::{
    app_state::AppState,
    http::handlers::subscription::{
        // admin
        admin_cancel_subscription,
        admin_change_plan,
        admin_get_entitlements,
        admin_get_status,
        admin_remove_override,
        admin_set_override,
        // agency
        cancel_subscription,
        change_plan,
        get_entitlements,
        get_plan,
        get_status,
        get_usage,
        initiate_payment,
        list_invoices,
        list_plans,
        subscribe,
    },
    middleware::subscription::subscription_middleware,
};

/// No auth, no tenant — anyone can browse plans (e.g. marketing pages).
pub fn public_routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/subscription/plans", get(list_plans))
        .route("/api/v1/subscription/plans/{slug}", get(get_plan))
}

/// Requires auth + tenant but NOT an active subscription.
/// Used for payment initiation (which is how you get an active subscription).
pub fn billing_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/subscription/initiate-payment",
            post(initiate_payment),
        )
        .route("/api/v1/subscription/subscribe", post(subscribe))
}

/// Requires auth + tenant + active subscription.
pub fn agency_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/api/v1/subscription", get(get_status))
        .route("/api/v1/subscription/entitlements", get(get_entitlements))
        .route("/api/v1/subscription/usage", get(get_usage))
        .route("/api/v1/subscription/invoices", get(list_invoices))
        .route("/api/v1/subscription/plan", patch(change_plan))
        .route("/api/v1/subscription", delete(cancel_subscription))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            subscription_middleware,
        ))
}

/// Platform admin only — no subscription check (admins always have access).
pub fn admin_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/admin/subscriptions/{agency_id}",
            get(admin_get_status).delete(admin_cancel_subscription),
        )
        .route(
            "/api/v1/admin/subscriptions/{agency_id}/entitlements",
            get(admin_get_entitlements),
        )
        .route(
            "/api/v1/admin/subscriptions/{agency_id}/plan",
            patch(admin_change_plan),
        )
        .route(
            "/api/v1/admin/subscriptions/{agency_id}/overrides",
            post(admin_set_override),
        )
        .route(
            "/api/v1/admin/subscriptions/{agency_id}/overrides/{feature_key}",
            delete(admin_remove_override),
        )
}
