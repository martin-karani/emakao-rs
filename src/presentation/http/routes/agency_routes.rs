use axum::{
    routing::{get, patch},
    Router,
};

use crate::presentation::{
    app_state::AppState,
    http::handlers::agency::{
        create_agency, create_staff_user, delete_permission_tuple, update_auth_model,
        write_permission_tuple,
    },
};

// ── Staff: agency profile management ─────────────────────────────────────────
//
// Mounted inside `build_staff_api` with only `require_auth` applied —
// no `resolve_agency_context` or `subscription_middleware` because the
// agency_id is read directly from the authenticated user's JWT claim.
//
// Add `get_agency` / `update_agency` handlers to
// `src/presentation/http/handlers/agency.rs` when ready and wire them below.

pub fn management_routes() -> Router<AppState> {
    Router::new()
    // GET  /api/v1/agency  — fetch the authenticated agency's own profile
    // PATCH /api/v1/agency — update name, contact info, logo URL, etc.
    //
    // Uncomment once the handlers exist:
    //
    // .route(
    //     "/api/v1/agency",
    //     get(get_agency).patch(update_agency),
    // )
    //
    // For now the router compiles as a no-op; add routes incrementally.
}

// ── Platform admin ────────────────────────────────────────────────────────────
//
// Mounted inside `build_admin_api` behind the `require_admin` middleware.

pub fn admin_routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/admin/agencies", axum::routing::post(create_agency))
        .route(
            "/api/v1/admin/agencies/{agency_id}/staff",
            axum::routing::post(create_staff_user),
        )
        // OpenFGA permission management
        .route(
            "/api/v1/admin/agencies/{fga_store_id}/permissions/tuples",
            axum::routing::post(write_permission_tuple).delete(delete_permission_tuple),
        )
        .route(
            "/api/v1/admin/agencies/{fga_store_id}/permissions/model",
            axum::routing::post(update_auth_model),
        )
}
