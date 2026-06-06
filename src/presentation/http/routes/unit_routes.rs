use axum::{
    routing::get,
    Router,
};

use crate::presentation::{
    app_state::AppState,
    http::handlers::unit::{create_units, delete_unit, get_unit, list_units, update_unit},
};

pub fn routes() -> Router<AppState> {
    Router::new()
        // ── Property-scoped unit endpoints ────────────────────────────────────
        .route(
            "/api/v1/properties/{propertyId}/units",
            get(list_units).post(create_units),
        )
        // ── Unit-scoped endpoints ─────────────────────────────────────────────
        .route(
            "/api/v1/units/{unitId}",
            get(get_unit).put(update_unit).delete(delete_unit),
        )
}
