// src/presentation/http/handlers/notification_templates.rs
//
//   GET    /api/agency/notification-templates
//   PUT    /api/agency/notification-templates/:channel/:event_key
//   DELETE /api/agency/notification-templates/:channel/:event_key
//
// All require "agency_settings:write".

use std::sync::Arc;

use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    application::{
        errors::AppError,
        use_cases::notification_template::{
            delete_template::{DeleteTemplateInput, DeleteTemplateUseCase},
            list_templates::{ListTemplatesInput, ListTemplatesUseCase},
            upsert_template::{UpsertTemplateInput, UpsertTemplateUseCase},
        },
    },
    domain::{auth::AuthenticatedUser, notification_template::NotificationTemplate},
    infrastructure::{
        db::notification_template_repository_sqlx::PgNotificationTemplateRepo,
        notifications::template_validator::MiniJinjaValidator,
    },
    presentation::app_state::AppState,
};

// ── Response DTO ──────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, ToSchema)]
pub struct NotificationTemplateResponse {
    pub id: uuid::Uuid,
    pub channel: String,
    pub event_key: String,
    pub locale: String,
    pub subject: Option<String>,
    pub body: String,
}

impl From<NotificationTemplate> for NotificationTemplateResponse {
    fn from(t: NotificationTemplate) -> Self {
        Self {
            id: t.id,
            channel: t.channel,
            event_key: t.event_key,
            locale: t.locale,
            subject: t.subject,
            body: t.body,
        }
    }
}

// ── Request DTOs ──────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ListTemplatesQuery {
    pub channel: Option<String>,
    pub event_key: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpsertTemplateRequest {
    #[serde(default = "default_locale")]
    pub locale: String,
    pub subject: Option<String>,
    pub body: String,
}

#[derive(Debug, Deserialize)]
pub struct DeleteTemplateQuery {
    #[serde(default = "default_locale")]
    pub locale: String,
}

fn default_locale() -> String {
    "en".into()
}

// ── Handlers ──────────────────────────────────────────────────────────────────

pub async fn list_templates(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Query(q): Query<ListTemplatesQuery>,
) -> Result<impl IntoResponse, AppError> {
    state
        .customisation()
        .permissions
        .require(user.user_id, user.agency_id, "agency_settings:write")
        .await?;

    let repo = Arc::new(PgNotificationTemplateRepo::new(
        state.infra.tenant_pools.platform().clone(),
    ));
    let templates = ListTemplatesUseCase::new(repo)
        .execute(ListTemplatesInput {
            agency_id: user.agency_id,
            channel: q.channel,
            event_key: q.event_key,
        })
        .await?;

    Ok(Json(
        templates
            .into_iter()
            .map(NotificationTemplateResponse::from)
            .collect::<Vec<_>>(),
    ))
}

pub async fn upsert_template(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path((channel, event_key)): Path<(String, String)>,
    Json(body): Json<UpsertTemplateRequest>,
) -> Result<impl IntoResponse, AppError> {
    state
        .customisation()
        .permissions
        .require(user.user_id, user.agency_id, "agency_settings:write")
        .await?;

    let repo = Arc::new(PgNotificationTemplateRepo::new(
        state.infra.tenant_pools.platform().clone(),
    ));
    // The MiniJinja environment lives in AppState; we wrap it in the
    // infrastructure adapter that implements the `TemplateValidator` port.
    let validator = Arc::new(MiniJinjaValidator::new((*state.jinja).clone()));

    UpsertTemplateUseCase::new(repo, validator)
        .execute(UpsertTemplateInput {
            agency_id: user.agency_id,
            channel: channel.clone(),
            event_key: event_key.clone(),
            locale: body.locale,
            subject: body.subject,
            body: body.body,
        })
        .await?;

    // Audit — template mutations are always logged.
    state
        .customisation()
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

pub async fn delete_template(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path((channel, event_key)): Path<(String, String)>,
    Query(q): Query<DeleteTemplateQuery>,
) -> Result<impl IntoResponse, AppError> {
    state
        .customisation()
        .permissions
        .require(user.user_id, user.agency_id, "agency_settings:write")
        .await?;

    let repo = Arc::new(PgNotificationTemplateRepo::new(
        state.infra.tenant_pools.platform().clone(),
    ));
    DeleteTemplateUseCase::new(repo)
        .execute(DeleteTemplateInput {
            agency_id: user.agency_id,
            channel,
            event_key,
            locale: q.locale,
        })
        .await?;

    Ok(StatusCode::NO_CONTENT)
}
