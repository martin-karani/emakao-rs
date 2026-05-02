use axum::{routing::post, Router};

use crate::presentation::{
    app_state::AppState,
    http::handlers::agency::{
        create_agency, delete_permission_tuple, update_auth_model, write_permission_tuple,
    },
};

///
/// ```rust
/// router.merge(agency_routes::admin_routes())
///       .layer(middleware::from_fn_with_state(state, admin_auth_middleware))
/// ```
pub fn admin_routes() -> Router<AppState> {
    Router::new()
        // ── Agency provisioning ───────────────────────────────────────────
        .route("/api/v1/admin/agencies", post(create_agency))
        // ── Per-agency FGA tuple management ──────────────────────────────
        // :fga_store_id is the OpenFGA store ID stored in agencies.fga_store_id
        .route(
            "/api/v1/admin/agencies/:fga_store_id/permissions/tuples",
            post(write_permission_tuple).delete(delete_permission_tuple),
        )
        // ── Per-agency authorization model updates ────────────────────────
        .route(
            "/api/v1/admin/agencies/:fga_store_id/permissions/model",
            post(update_auth_model),
        )
}
