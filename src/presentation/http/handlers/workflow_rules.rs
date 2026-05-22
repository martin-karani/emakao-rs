// src/presentation/http/handlers/workflow_rules.rs
//
//   GET    /api/agency/workflow-rules
//   POST   /api/agency/workflow-rules
//   PATCH  /api/agency/workflow-rules/:id
//   DELETE /api/agency/workflow-rules/:id
//
// All require "agency_settings:write".

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
    application::errors::AppError, domain::auth::AuthenticatedUser,
    presentation::app_state::AppState,
};

// ── DTOs ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, ToSchema)]
pub struct WorkflowRuleDto {
    pub id: Uuid,
    pub name: String,
    pub event_type: String,
    pub is_active: bool,
    pub offset_hours: i32,
    /// JSONLogic condition object.
    pub conditions: serde_json::Value,
    /// Array of action objects: `[{"type":"send_sms","params":{...}}]`
    pub actions: serde_json::Value,
}

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
        .custom()
        .permissions
        .require(user.user_id, "agency_settings:write")
        .await?;

    let rows = sqlx::query!(
        r#"
        SELECT id, name, event_type, is_active, offset_hours, conditions, actions
        FROM   workflow_rules
        WHERE  agency_id = $1
        ORDER  BY event_type, name
        "#,
        user.agency_id,
    )
    .fetch_all(state.infra.tenant_pools.platform())
    .await
    .map_err(AppError::Database)?;

    let dtos: Vec<WorkflowRuleDto> = rows
        .into_iter()
        .map(|r| WorkflowRuleDto {
            id: r.id,
            name: r.name,
            event_type: r.event_type,
            is_active: r.is_active,
            offset_hours: r.offset_hours.unwrap_or(0),
            conditions: r.conditions,
            actions: r.actions,
        })
        .collect();

    Ok(Json(dtos))
}

pub async fn create_rule(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(body): Json<CreateWorkflowRuleRequest>,
) -> Result<impl IntoResponse, AppError> {
    state
        .custom()
        .permissions
        .require(user.user_id, "agency_settings:write")
        .await?;

    let id = sqlx::query_scalar!(
        r#"
        INSERT INTO workflow_rules
            (agency_id, name, event_type, is_active, offset_hours, conditions, actions)
        VALUES
            ($1, $2, $3, $4, $5, $6, $7)
        RETURNING id
        "#,
        user.agency_id,
        body.name,
        body.event_type,
        body.is_active,
        body.offset_hours,
        body.conditions,
        body.actions,
    )
    .fetch_one(state.infra.tenant_pools.platform())
    .await
    .map_err(|e| {
        if e.to_string().contains("unique") {
            AppError::Conflict(format!("a rule named '{}' already exists", body.name))
        } else {
            AppError::Database(e)
        }
    })?;

    Ok((StatusCode::CREATED, Json(serde_json::json!({ "id": id }))))
}

pub async fn patch_rule(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(rule_id): Path<Uuid>,
    Json(body): Json<PatchWorkflowRuleRequest>,
) -> Result<impl IntoResponse, AppError> {
    state
        .custom()
        .permissions
        .require(user.user_id, "agency_settings:write")
        .await?;

    // Only update fields that are present in the patch.
    sqlx::query!(
        r#"
        UPDATE workflow_rules
        SET
            name         = COALESCE($3, name),
            is_active    = COALESCE($4, is_active),
            offset_hours = COALESCE($5, offset_hours),
            conditions   = COALESCE($6, conditions),
            actions      = COALESCE($7, actions)
        WHERE id        = $1
          AND agency_id = $2
        "#,
        rule_id,
        user.agency_id,
        body.name,
        body.is_active,
        body.offset_hours,
        body.conditions,
        body.actions,
    )
    .execute(state.infra.tenant_pools.platform())
    .await
    .map_err(AppError::Database)?;

    Ok(StatusCode::OK)
}

pub async fn delete_rule(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(rule_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    state
        .custom()
        .permissions
        .require(user.user_id, "agency_settings:write")
        .await?;

    sqlx::query!(
        "DELETE FROM workflow_rules WHERE id = $1 AND agency_id = $2",
        rule_id,
        user.agency_id,
    )
    .execute(state.infra.tenant_pools.platform())
    .await
    .map_err(AppError::Database)?;

    Ok(StatusCode::NO_CONTENT)
}

use sqlx;
