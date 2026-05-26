// src/infrastructure/db/workflow_rule_repository_sqlx.rs

use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        ports::workflow_rule_repository::{
            CreateWorkflowRuleCommand, PatchWorkflowRuleCommand, WorkflowRuleRepository,
        },
    },
    domain::workflow_rule::WorkflowRule,
};

pub struct PgWorkflowRuleRepo {
    pool: PgPool,
}

impl PgWorkflowRuleRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

// ── Internal row ──────────────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct WorkflowRuleRow {
    id: Uuid,
    agency_id: Uuid,
    name: String,
    event_type: String,
    is_active: bool,
    offset_hours: Option<i32>,
    conditions: serde_json::Value,
    actions: serde_json::Value,
    created_at: time::OffsetDateTime,
}

impl From<WorkflowRuleRow> for WorkflowRule {
    fn from(r: WorkflowRuleRow) -> Self {
        Self {
            id: r.id,
            agency_id: r.agency_id,
            name: r.name,
            event_type: r.event_type,
            is_active: r.is_active,
            offset_hours: r.offset_hours.unwrap_or(0),
            conditions: r.conditions,
            actions: r.actions,
            created_at: r.created_at,
        }
    }
}

// ── Implementation ────────────────────────────────────────────────────────────

#[async_trait]
impl WorkflowRuleRepository for PgWorkflowRuleRepo {
    async fn list(&self, agency_id: Uuid) -> Result<Vec<WorkflowRule>, AppError> {
        let rows = sqlx::query_as::<_, WorkflowRuleRow>(
            r#"
            SELECT id, agency_id, name, event_type, is_active,
                   offset_hours, conditions, actions, created_at
            FROM   workflow_rules
            WHERE  agency_id = $1
            ORDER  BY event_type, name
            "#,
        )
        .bind(agency_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(rows.into_iter().map(WorkflowRule::from).collect())
    }

    async fn create(&self, cmd: CreateWorkflowRuleCommand) -> Result<WorkflowRule, AppError> {
        let row = sqlx::query_as::<_, WorkflowRuleRow>(
            r#"
            INSERT INTO workflow_rules
                (agency_id, name, event_type, is_active, offset_hours, conditions, actions)
            VALUES
                ($1, $2, $3, $4, $5, $6, $7)
            RETURNING
                id, agency_id, name, event_type, is_active,
                offset_hours, conditions, actions, created_at
            "#,
        )
        .bind(cmd.agency_id)
        .bind(&cmd.name)
        .bind(&cmd.event_type)
        .bind(cmd.is_active)
        .bind(cmd.offset_hours)
        .bind(&cmd.conditions)
        .bind(&cmd.actions)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("unique") {
                AppError::Conflict(format!("a rule named '{}' already exists", cmd.name))
            } else {
                AppError::Database(e)
            }
        })?;

        Ok(WorkflowRule::from(row))
    }

    async fn patch(&self, cmd: PatchWorkflowRuleCommand) -> Result<(), AppError> {
        sqlx::query(
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
        )
        .bind(cmd.rule_id)
        .bind(cmd.agency_id)
        .bind(&cmd.name)
        .bind(cmd.is_active)
        .bind(cmd.offset_hours)
        .bind(&cmd.conditions)
        .bind(&cmd.actions)
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(())
    }

    async fn delete(&self, agency_id: Uuid, rule_id: Uuid) -> Result<(), AppError> {
        sqlx::query(
            "DELETE FROM workflow_rules WHERE id = $1 AND agency_id = $2",
        )
        .bind(rule_id)
        .bind(agency_id)
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(())
    }
}
