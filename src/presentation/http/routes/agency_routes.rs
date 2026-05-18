use axum::{routing::post, Router};

use crate::presentation::{
    app_state::AppState,
    http::handlers::agency::{
        create_agency, create_staff_user, delete_permission_tuple, update_auth_model,
        write_permission_tuple,
    },
};

pub fn admin_routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/admin/agencies", post(create_agency))
        .route(
            "/api/v1/admin/agencies/{agency_id}/staff",
            post(create_staff_user),
        )
        // ── OpenFGA permission management ────────────────────────────────────
        .route(
            "/api/v1/admin/agencies/{fga_store_id}/permissions/tuples",
            post(write_permission_tuple).delete(delete_permission_tuple),
        )
        .route(
            "/api/v1/admin/agencies/{fga_store_id}/permissions/model",
            post(update_auth_model),
        )
}
