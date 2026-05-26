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
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
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
    domain::{auth::AuthenticatedUser, workflow_rule::WorkflowRule},
    infrastructure::db::workflow_rule_repository_sqlx::PgWorkflowRuleRepo,
    presentation::app_state::AppState,
};

// ── Response DTO ──────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, ToSchema)]
pub struct WorkflowRuleResponse {
    pub id: Uuid,
    pub name: String,
    pub event_type: String,
    pub is_active: bool,
    pub offset_hours: i32,
    pub conditions: serde_json::Value,
    pub actions: serde_json::Value,
}

impl From<WorkflowRule> for WorkflowRuleResponse {
    fn from(r: WorkflowRule) -> Self {
        Self {
            id: r.id,
            name: r.name,
            event_type: r.event_type,
            is_active: r.is_active,
            offset_hours: r.offset_hours,
            conditions: r.conditions,
            actions: r.actions,
        }
    }
}

// ── Request DTOs ──────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateWorkflowRuleRequest {
    pub name: String,
    pub event_type: String,
    #[serde(default)]
    pub is_active: bool,
    #[serde(default)]
    pub offset_hours: i32,
    #[serde(default = "empty_object")]
    pub conditions: serde_json::Value,
    #[serde(default = "empty_array")]
    pub actions: serde_json::Value,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PatchWorkflowRuleRequest {
    pub name: Option<String>,
    pub is_active: Option<bool>,
    pub offset_hours: Option<i32>,
    pub conditions: Option<serde_json::Value>,
    pub actions: Option<serde_json::Value>,
}

fn empty_object() -> serde_json::Value {
    serde_json::json!({})
}
fn empty_array() -> serde_json::Value {
    serde_json::json!([])
}

// ── Handlers ──────────────────────────────────────────────────────────────────

pub async fn list_rules(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
) -> Result<impl IntoResponse, AppError> {
    state
        .customisation()
        .permissions
        .require(user.user_id, "agency_settings:write")
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
        .require(user.user_id, "agency_settings:write")
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
        .require(user.user_id, "agency_settings:write")
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
        .require(user.user_id, "agency_settings:write")
        .await?;

    let repo = Arc::new(PgWorkflowRuleRepo::new(state.infra.tenant_pools.platform().clone()));
    DeleteRuleUseCase::new(repo)
        .execute(user.agency_id, rule_id)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}
