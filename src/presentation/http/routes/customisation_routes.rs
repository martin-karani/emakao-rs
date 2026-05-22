use axum::{
    routing::{delete, get, patch, put},
    Router,
};

use crate::presentation::{
    app_state::AppState,
    http::handlers::{
        agency_integrations::{delete_integration, list_integrations, upsert_integration},
        agency_settings::{get_settings, patch_settings},
        notification_templates::{delete_template, list_templates, upsert_template},
        workflow_rules::{create_rule, delete_rule, list_rules, patch_rule},
    },
};

pub fn routes() -> Router<AppState> {
    Router::new()
        // ── Agency settings ───────────────────────────────────────────────────
        .route(
            "/api/agency/settings",
            get(get_settings).patch(patch_settings),
        )
        // ── Integration credentials ───────────────────────────────────────────
        .route("/api/agency/integrations", get(list_integrations))
        .route(
            // FIX: Axum 0.8 uses {param} not :param
            "/api/agency/integrations/{provider_type}/{provider_key}",
            put(upsert_integration).delete(delete_integration),
        )
        // ── Notification templates ────────────────────────────────────────────
        .route("/api/agency/notification-templates", get(list_templates))
        .route(
            // FIX: Axum 0.8 uses {param} not :param
            "/api/agency/notification-templates/{channel}/{event_key}",
            put(upsert_template).delete(delete_template),
        )
        // ── Workflow rules ────────────────────────────────────────────────────
        .route(
            "/api/agency/workflow-rules",
            get(list_rules).post(create_rule),
        )
        .route(
            // FIX: Axum 0.8 uses {param} not :param
            "/api/agency/workflow-rules/{id}",
            patch(patch_rule).delete(delete_rule),
        )
}
