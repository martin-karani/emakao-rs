use axum::{routing::get, Router};

use crate::presentation::{
    app_state::AppState,
    http::handlers::vendor::{
        create_vendor, get_my_profile, get_vendor, list_my_work_orders, list_vendors, update_vendor,
    },
};

/// Staff‑facing vendor directory management
pub fn staff_routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/vendors", get(list_vendors).post(create_vendor))
        .route("/api/v1/vendors/{id}", get(get_vendor).put(update_vendor))
}

/// Vendor portal – own profile, work orders, bids
pub fn vendor_portal_routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/vendors/me", get(get_my_profile))
        .route("/api/v1/vendors/me/work-orders", get(list_my_work_orders))
}
