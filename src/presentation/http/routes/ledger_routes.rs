use axum::{
    routing::{get, post},
    Router,
};

use crate::presentation::{
    app_state::AppState,
    http::handlers::ledger::{get_balance, list_entries, post_charge},
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/agreements/{id}/ledger", get(list_entries))
        .route("/api/v1/agreements/{id}/balance", get(get_balance))
        .route("/api/v1/agreements/{id}/charges", post(post_charge))
}
