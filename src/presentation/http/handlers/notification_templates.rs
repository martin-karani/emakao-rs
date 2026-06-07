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

use crate::{
    application::{
        errors::AppError,
        use_cases::notification_template::{
            delete_template::{DeleteTemplateInput, DeleteTemplateUseCase},
            list_templates::{ListTemplatesInput, ListTemplatesUseCase},
            upsert_template::{UpsertTemplateInput, UpsertTemplateUseCase},
        },
    },
    domain::auth::AuthenticatedUser,
    infrastructure::{
        db::notification_template_repository_sqlx::PgNotificationTemplateRepo,
        notifications::template_validator::MiniJinjaValidator,
    },
    presentation::{
        app_state::AppState,
        http::{
            dto::notification_template::{
                DeleteTemplateQuery, ListTemplatesQuery, UpsertTemplateRequest,
            },
            responses::notification_template::NotificationTemplateResponse,
        },
    },
};

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
            property_id: q.property_id,
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
    let validator = Arc::new(MiniJinjaValidator::new((*state.jinja).clone()));

    UpsertTemplateUseCase::new(repo, validator)
        .execute(UpsertTemplateInput {
            agency_id: user.agency_id,
            property_id: body.property_id,
            channel: channel.clone(),
            event_key: event_key.clone(),
            locale: body.locale,
            subject: body.subject,
            body: body.body,
        })
        .await?;

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
            property_id: q.property_id,
            channel,
            event_key,
            locale: q.locale,
        })
        .await?;

    Ok(StatusCode::NO_CONTENT)
}
