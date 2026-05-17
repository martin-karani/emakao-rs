use axum::{
    routing::{get, post},
    Router,
};

use crate::presentation::{
    app_state::AppState,
    http::handlers::bank_reconciliation::{
        get_reconciliation_report, get_statement, import_statement, list_statements, match_line,
        unmatch_line,
    },
};

pub fn routes() -> Router<AppState> {
    Router::new()
        // Statement CRUD
        .route(
            "/api/v1/bank-statements",
            get(list_statements).post(import_statement),
        )
        .route("/api/v1/bank-statements/:id", get(get_statement))
        // Line matching
        .route(
            "/api/v1/bank-statements/:id/lines/:line_id/match",
            post(match_line),
        )
        .route(
            "/api/v1/bank-statements/:id/lines/:line_id/unmatch",
            post(unmatch_line),
        )
        // Reconciliation report
        .route(
            "/api/v1/bank-statements/:id/reconciliation",
            get(get_reconciliation_report),
        )
}
