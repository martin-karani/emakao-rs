//
// All routes require:
//   1. Bearer token auth (applied by the surrounding staff API middleware stack)
//   2. `acct_double_entry` feature entitlement (applied by `subscription_gate`
//      middleware — add it to the route group in router.rs if not already there)

use axum::{
    routing::{delete, get, post},
    Router,
};

use crate::presentation::{
    app_state::AppState,
    http::handlers::accounting::{
        create_account, delete_account, get_journal_entry, get_trial_balance, list_accounts,
        list_journal_entries, post_journal_entry, void_journal_entry,
    },
};

pub fn routes() -> Router<AppState> {
    Router::new()
        // Chart of accounts
        .route("/api/v1/accounts", get(list_accounts).post(create_account))
        .route("/api/v1/accounts/:id", delete(delete_account))
        // Journal entries
        .route(
            "/api/v1/journal-entries",
            get(list_journal_entries).post(post_journal_entry),
        )
        .route("/api/v1/journal-entries/:id", get(get_journal_entry))
        .route("/api/v1/journal-entries/:id/void", post(void_journal_entry))
        // Reports
        .route("/api/v1/accounting/trial-balance", get(get_trial_balance))
}
