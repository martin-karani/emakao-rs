use axum::{
    routing::{get, patch, post, put},
    Router,
};

use crate::presentation::{
    app_state::AppState,
    http::{
        handlers::{
            agency::{
                deactivate_agency_integration, get_agency_settings, list_agency_integrations,
                list_property_billing_summaries, patch_agency_settings, upsert_agency_integration,
            },
            communication::{broadcast_notice, broadcast_statements},
            notification_templates::{delete_template, list_templates, upsert_template},
            workflow_rules::{create_rule, delete_rule, list_rules, patch_rule},
        },
        routes::role_routes,
    },
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/agency/broadcast", post(broadcast_notice))
        .route("/api/agency/statements", post(broadcast_statements))
        .nest("/api/agency/roles", role_routes::routes())
        .route(
            "/api/agency/settings",
            get(get_agency_settings).patch(patch_agency_settings),
        )
        .route(
            "/api/agency/billing-summary",
            get(list_property_billing_summaries),
        )
        .route("/api/agency/integrations", get(list_agency_integrations))
        .route(
            "/api/agency/integrations/{provider_type}/{provider_key}",
            put(upsert_agency_integration).delete(deactivate_agency_integration),
        )
        .route("/api/agency/notification-templates", get(list_templates))
        .route(
            "/api/agency/notification-templates/{channel}/{event_key}",
            put(upsert_template).delete(delete_template),
        )
        .route(
            "/api/agency/workflow-rules",
            get(list_rules).post(create_rule),
        )
        .route(
            "/api/agency/workflow-rules/{id}",
            patch(patch_rule).delete(delete_rule),
        )
}
