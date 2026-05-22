// src/presentation/http/handlers/notification_templates.rs
//
// Handlers for per-agency notification templates stored in
// `notification_templates`.
//
//   GET    /api/agency/notification-templates
//   PUT    /api/agency/notification-templates/:channel/:event_key
//   DELETE /api/agency/notification-templates/:channel/:event_key
//
// All require "agency_settings:write".

use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    application::errors::AppError, domain::auth::AuthenticatedUser,
    presentation::app_state::AppState,
};

// ── List ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ListTemplatesQuery {
    pub channel: Option<String>,
    pub event_key: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct NotificationTemplateDto {
    pub id: uuid::Uuid,
    pub channel: String,
    pub event_key: String,
    pub locale: String,
    pub subject: Option<String>,
    pub body: String,
}

pub async fn list_templates(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Query(q): Query<ListTemplatesQuery>,
) -> Result<impl IntoResponse, AppError> {
    state
        .custom()
        .permissions
        .require(user.user_id, "agency_settings:write")
        .await?;

    let rows = sqlx::query!(
        r#"
        SELECT id, channel, event_key, locale, subject, body
        FROM   notification_templates
        WHERE  agency_id   = $1
          AND  ($2::text IS NULL OR channel   = $2)
          AND  ($3::text IS NULL OR event_key = $3)
        ORDER  BY channel, event_key, locale
        "#,
        user.agency_id,
        q.channel,
        q.event_key,
    )
    .fetch_all(state.infra.tenant_pools.platform())
    .await
    .map_err(AppError::Database)?;

    let dtos: Vec<NotificationTemplateDto> = rows
        .into_iter()
        .map(|r| NotificationTemplateDto {
            id: r.id,
            channel: r.channel,
            event_key: r.event_key,
            locale: r.locale,
            subject: r.subject,
            body: r.body,
        })
        .collect();

    Ok(Json(dtos))
}

// ── Upsert ────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpsertTemplateRequest {
    #[serde(default = "default_locale")]
    pub locale: String,
    pub subject: Option<String>,
    /// MiniJinja template string. Variables available depend on `event_key`.
    pub body: String,
}
fn default_locale() -> String {
    "en".into()
}

pub async fn upsert_template(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path((channel, event_key)): Path<(String, String)>,
    Json(body): Json<UpsertTemplateRequest>,
) -> Result<impl IntoResponse, AppError> {
    state
        .custom()
        .permissions
        .require(user.user_id, "agency_settings:write")
        .await?;

    // Validate that the body parses as a MiniJinja template before saving.
    state
        .jinja
        .template_from_str(&body.body)
        .map_err(|e| AppError::Validation(format!("invalid MiniJinja template: {e}")))?;

    sqlx::query!(
        r#"
        INSERT INTO notification_templates
            (agency_id, channel, event_key, locale, subject, body)
        VALUES
            ($1, $2, $3, $4, $5, $6)
        ON CONFLICT (agency_id, channel, event_key, locale)
        DO UPDATE SET
            subject    = EXCLUDED.subject,
            body       = EXCLUDED.body
        "#,
        user.agency_id,
        channel,
        event_key,
        body.locale,
        body.subject,
        body.body,
    )
    .execute(state.infra.tenant_pools.platform())
    .await
    .map_err(AppError::Database)?;

    state
        .custom()
        .audit
        .log(crate::infrastructure::audit::AuditEvent {
            agency_id: user.agency_id,
            actor_id: Some(user.user_id),
            actor_role: Some(user.role.clone()),
            action: format!("notification_template.{channel}.{event_key}.upserted"),
            entity_type: "notification_template".into(),
            entity_id: user.agency_id,
            old_data: None,
            new_data: Some(serde_json::json!({ "channel": channel, "event_key": event_key })),
            ip_address: None,
        });

    Ok(StatusCode::OK)
}

// ── Delete ────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct DeleteTemplateQuery {
    #[serde(default = "default_locale")]
    pub locale: String,
}

pub async fn delete_template(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path((channel, event_key)): Path<(String, String)>,
    Query(q): Query<DeleteTemplateQuery>,
) -> Result<impl IntoResponse, AppError> {
    state
        .custom()
        .permissions
        .require(user.user_id, "agency_settings:write")
        .await?;

    sqlx::query!(
        r#"
        DELETE FROM notification_templates
        WHERE agency_id = $1
          AND channel   = $2
          AND event_key = $3
          AND locale    = $4
        "#,
        user.agency_id,
        channel,
        event_key,
        q.locale,
    )
    .execute(state.infra.tenant_pools.platform())
    .await
    .map_err(AppError::Database)?;

    Ok(StatusCode::NO_CONTENT)
}

use sqlx;
