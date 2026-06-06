use axum::{
    routing::{get, patch, post, put},
    Router,
};

use crate::presentation::{
    app_state::AppState,
    http::{
        handlers::{
            agency_integrations::{delete_integration, list_integrations, upsert_integration},
            agency_settings::{get_settings, patch_settings},
            communication::{broadcast_notice, broadcast_statements},
            notification_templates::{delete_template, list_templates, upsert_template},
            property_billing::get_agency_billing_summary,
            workflow_rules::{create_rule, delete_rule, list_rules, patch_rule},
        },
        routes::role_routes,
    },
};

pub fn routes() -> Router<AppState> {
    Router::new()
        // ── Communication & Broadcasts ────────────────────────────────────────
        .route("/api/agency/broadcast", post(broadcast_notice))
        .route("/api/agency/statements", post(broadcast_statements))
        // ── Roles & Permissions ───────────────────────────────────────────────
        .nest("/api/agency/roles", role_routes::routes())
        // ── Agency settings ───────────────────────────────────────────────────
        .route(
            "/api/agency/settings",
            get(get_settings).patch(patch_settings),
        )
        .route(
            "/api/agency/billing-summary",
            get(get_agency_billing_summary),
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
