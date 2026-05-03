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
