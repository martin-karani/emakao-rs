use axum::Router;

use crate::presentation::app_state::AppState;

pub mod agency_routes;
pub mod agreement_routes;
pub mod auth_routes;
pub mod health_routes;
pub mod ledger_routes;
pub mod maintenance_routes;
pub mod owner_routes;
pub mod payment_routes;
pub mod property_routes;
pub mod resident_routes;
pub mod subscription_routes;
pub mod utility_routes;
pub mod vendor_routes;
pub mod webhook_routes;
pub mod websocket_routes;

pub fn all_routes() -> Router<AppState> {
    Router::new()
        .merge(property_routes::routes())
        .merge(resident_routes::routes())
        .merge(agreement_routes::routes())
        .merge(payment_routes::routes())
        .merge(ledger_routes::routes())
        .merge(utility_routes::routes())
        .merge(maintenance_routes::routes())
        .merge(owner_routes::routes())
        .merge(vendor_routes::routes())
}
