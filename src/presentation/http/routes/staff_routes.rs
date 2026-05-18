use axum::{
    routing::{get, post},
    Router,
};

use crate::presentation::{
    app_state::AppState,
    http::handlers::staff::{deactivate_staff, get_staff_member, invite_staff, list_staff},
};

/// Staff management routes — all require a valid staff JWT.
///
/// These are merged into `build_staff_api` which already wraps the whole set
/// with `require_auth` + `resolve_agency_context` + `subscription` middleware.
///
/// Registered in `build_staff_api`:
/// ```
/// .merge(staff_routes::staff_routes())
/// ```
pub fn staff_routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/staff/invite", post(invite_staff))
        .route("/api/v1/staff", get(list_staff))
        .route(
            "/api/v1/staff/{membership_id}",
            get(get_staff_member).delete(deactivate_staff),
        )
}
