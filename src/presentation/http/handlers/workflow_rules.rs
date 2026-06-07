// src/presentation/http/handlers/workflow_rules.rs
//
//   GET    /api/agency/workflow-rules
//   POST   /api/agency/workflow-rules
//   PATCH  /api/agency/workflow-rules/:id
//   DELETE /api/agency/workflow-rules/:id
//
// All require "agency_settings:write".

use std::sync::Arc;

use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        use_cases::workflow_rule::{
            create_rule::{CreateRuleInput, CreateRuleUseCase},
            delete_rule::DeleteRuleUseCase,
            list_rules::ListRulesUseCase,
            patch_rule::{PatchRuleInput, PatchRuleUseCase},
        },
    },
    domain::auth::AuthenticatedUser,
    infrastructure::db::workflow_rule_repository_sqlx::PgWorkflowRuleRepo,
    presentation::{
        app_state::AppState,
        http::{
            dto::workflow_rule::{CreateWorkflowRuleRequest, PatchWorkflowRuleRequest},
            responses::workflow_rule::WorkflowRuleResponse,
        },
    },
};

pub async fn list_rules(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
) -> Result<impl IntoResponse, AppError> {
    state
        .customisation()
        .permissions
        .require(user.user_id, user.agency_id, "agency_settings:write")
        .await?;

    let repo = Arc::new(PgWorkflowRuleRepo::new(state.infra.tenant_pools.platform().clone()));
    let rules = ListRulesUseCase::new(repo).execute(user.agency_id).await?;

    Ok(Json(
        rules
            .into_iter()
            .map(WorkflowRuleResponse::from)
            .collect::<Vec<_>>(),
    ))
}

pub async fn create_rule(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(body): Json<CreateWorkflowRuleRequest>,
) -> Result<impl IntoResponse, AppError> {
    state
        .customisation()
        .permissions
        .require(user.user_id, user.agency_id, "agency_settings:write")
        .await?;

    let repo = Arc::new(PgWorkflowRuleRepo::new(state.infra.tenant_pools.platform().clone()));
    let rule = CreateRuleUseCase::new(repo)
        .execute(CreateRuleInput {
            agency_id: user.agency_id,
            name: body.name,
            event_type: body.event_type,
            is_active: body.is_active,
            offset_hours: body.offset_hours,
            conditions: body.conditions,
            actions: body.actions,
        })
        .await?;

    Ok((StatusCode::CREATED, Json(WorkflowRuleResponse::from(rule))))
}

pub async fn patch_rule(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(rule_id): Path<Uuid>,
    Json(body): Json<PatchWorkflowRuleRequest>,
) -> Result<impl IntoResponse, AppError> {
    state
        .customisation()
        .permissions
        .require(user.user_id, user.agency_id, "agency_settings:write")
        .await?;

    let repo = Arc::new(PgWorkflowRuleRepo::new(state.infra.tenant_pools.platform().clone()));
    PatchRuleUseCase::new(repo)
        .execute(PatchRuleInput {
            agency_id: user.agency_id,
            rule_id,
            name: body.name,
            is_active: body.is_active,
            offset_hours: body.offset_hours,
            conditions: body.conditions,
            actions: body.actions,
        })
        .await?;

    Ok(StatusCode::OK)
}

pub async fn delete_rule(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(rule_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    state
        .customisation()
        .permissions
        .require(user.user_id, user.agency_id, "agency_settings:write")
        .await?;

    let repo = Arc::new(PgWorkflowRuleRepo::new(state.infra.tenant_pools.platform().clone()));
    DeleteRuleUseCase::new(repo)
        .execute(user.agency_id, rule_id)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}
