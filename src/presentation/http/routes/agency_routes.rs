use axum::Router;

use crate::presentation::{
    app_state::AppState,
    http::handlers::agency::{
        create_agency, create_staff_user, grant_permission_tuple, publish_auth_model,
        revoke_permission_tuple,
    },
};

pub fn management_routes() -> Router<AppState> {
    Router::new()
}

pub fn admin_routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/admin/agencies", axum::routing::post(create_agency))
        .route(
            "/api/v1/admin/agencies/{agency_id}/staff",
            axum::routing::post(create_staff_user),
        )
        .route(
            "/api/v1/admin/agencies/{fga_store_id}/permissions/tuples",
            axum::routing::post(grant_permission_tuple).delete(revoke_permission_tuple),
        )
        .route(
            "/api/v1/admin/agencies/{fga_store_id}/permissions/model",
            axum::routing::post(publish_auth_model),
        )
}
