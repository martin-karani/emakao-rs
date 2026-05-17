use async_trait::async_trait;
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        ports::inspection_repository::{
            CreateInspectionCommand, InspectionFilter, InspectionRepository,
            UpdateInspectionCommand,
        },
    },
    domain::{
        enums::{InspectionStatus, InspectionType},
        inspection::{Inspection, InspectionItem},
    },
};

pub struct PgInspectionRepo {
    pool: PgPool,
}

impl PgInspectionRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

// ── Internal row type ─────────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct InspectionRow {
    id: Uuid,
    property_id: Uuid,
    unit_id: Uuid,
    agreement_id: Option<Uuid>,
    inspection_type: String,
    status: String,
    scheduled_at: time::OffsetDateTime,
    completed_at: Option<time::OffsetDateTime>,
    conducted_by: Option<Uuid>,
    items: Value,
    summary_notes: Option<String>,
    created_by: Uuid,
    created_at: time::OffsetDateTime,
    updated_at: time::OffsetDateTime,
}

impl TryFrom<InspectionRow> for Inspection {
    type Error = AppError;

    fn try_from(row: InspectionRow) -> Result<Self, Self::Error> {
        let inspection_type: InspectionType =
            serde_json::from_value(Value::String(row.inspection_type))
                .map_err(|e| AppError::ExternalService(e.to_string()))?;

        let status: InspectionStatus = serde_json::from_value(Value::String(row.status))
            .map_err(|e| AppError::ExternalService(e.to_string()))?;

        let items: Vec<InspectionItem> = serde_json::from_value(row.items)
            .map_err(|e| AppError::ExternalService(e.to_string()))?;

        Ok(Inspection {
            id: row.id,
            property_id: row.property_id,
            unit_id: row.unit_id,
            agreement_id: row.agreement_id,
            inspection_type,
            status,
            scheduled_at: row.scheduled_at,
            completed_at: row.completed_at,
            conducted_by: row.conducted_by,
            items,
            summary_notes: row.summary_notes,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

// ── Repository impl ───────────────────────────────────────────────────────────

#[async_trait]
impl InspectionRepository for PgInspectionRepo {
    async fn find_all(&self, filter: InspectionFilter) -> Result<Vec<Inspection>, AppError> {
        let rows = sqlx::query_as::<_, InspectionRow>(
            r#"
            SELECT
                id, property_id, unit_id, agreement_id,
                inspection_type, status,
                scheduled_at, completed_at, conducted_by,
                items, summary_notes,
                created_by, created_at, updated_at
            FROM inspections
            WHERE ($1::uuid IS NULL OR property_id   = $1)
              AND ($2::uuid IS NULL OR unit_id        = $2)
              AND ($3::uuid IS NULL OR agreement_id   = $3)
              AND ($4::text IS NULL OR status         = $4::text::inspection_status)
              AND ($5::text IS NULL OR inspection_type = $5::text::inspection_type)
            ORDER BY scheduled_at DESC
            LIMIT $6 OFFSET $7
            "#,
        )
        .bind(filter.property_id)
        .bind(filter.unit_id)
        .bind(filter.agreement_id)
        .bind(filter.status.as_ref().map(inspection_status_str))
        .bind(filter.inspection_type.as_ref().map(inspection_type_str))
        .bind(filter.limit)
        .bind(filter.offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        rows.into_iter().map(Inspection::try_from).collect()
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Inspection>, AppError> {
        let row = sqlx::query_as::<_, InspectionRow>(
            r#"
            SELECT
                id, property_id, unit_id, agreement_id,
                inspection_type, status,
                scheduled_at, completed_at, conducted_by,
                items, summary_notes,
                created_by, created_at, updated_at
            FROM inspections
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        row.map(Inspection::try_from).transpose()
    }

    async fn create(&self, cmd: CreateInspectionCommand) -> Result<Inspection, AppError> {
        let row = sqlx::query_as::<_, InspectionRow>(
            r#"
            INSERT INTO inspections (
                id, property_id, unit_id, agreement_id,
                inspection_type, status, scheduled_at, created_by
            )
            VALUES (
                uuidv7(), $1, $2, $3,
                $4::text::inspection_type, 'scheduled', $5, $6
            )
            RETURNING
                id, property_id, unit_id, agreement_id,
                inspection_type, status,
                scheduled_at, completed_at, conducted_by,
                items, summary_notes,
                created_by, created_at, updated_at
            "#,
        )
        .bind(cmd.property_id)
        .bind(cmd.unit_id)
        .bind(cmd.agreement_id)
        .bind(inspection_type_str(&cmd.inspection_type))
        .bind(cmd.scheduled_at)
        .bind(cmd.created_by)
        .fetch_one(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        Inspection::try_from(row)
    }

    async fn update(&self, cmd: UpdateInspectionCommand) -> Result<Inspection, AppError> {
        let status_s = cmd.status.as_ref().map(inspection_status_str);
        let items_json = cmd
            .items
            .as_ref()
            .map(|i| serde_json::to_value(i).unwrap_or(Value::Array(vec![])));

        let row = sqlx::query_as::<_, InspectionRow>(
            r#"
            UPDATE inspections SET
                status        = COALESCE($2::text::inspection_status, status),
                scheduled_at  = COALESCE($3::timestamptz, scheduled_at),
                completed_at  = CASE WHEN $4::boolean THEN $5::timestamptz ELSE completed_at END,
                conducted_by  = CASE WHEN $6::boolean THEN $7::uuid ELSE conducted_by END,
                items         = COALESCE($8::jsonb, items),
                summary_notes = COALESCE($9::text, summary_notes),
                updated_at    = now()
            WHERE id = $1
            RETURNING
                id, property_id, unit_id, agreement_id,
                inspection_type, status,
                scheduled_at, completed_at, conducted_by,
                items, summary_notes,
                created_by, created_at, updated_at
            "#,
        )
        .bind(cmd.id) // $1
        .bind(status_s) // $2
        .bind(cmd.scheduled_at) // $3
        .bind(cmd.completed_at.is_some()) // $4
        .bind(cmd.completed_at) // $5
        .bind(cmd.conducted_by.is_some()) // $6
        .bind(cmd.conducted_by.flatten()) // $7
        .bind(items_json) // $8
        .bind(cmd.summary_notes) // $9
        .fetch_optional(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?
        .ok_or_else(|| AppError::NotFound(format!("inspection {}", cmd.id)))?;

        Inspection::try_from(row)
    }

    async fn delete(&self, id: Uuid) -> Result<(), AppError> {
        let result = sqlx::query("DELETE FROM inspections WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("inspection {id}")));
        }
        Ok(())
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn inspection_type_str(t: &InspectionType) -> &'static str {
    match t {
        InspectionType::MoveIn => "move_in",
        InspectionType::MoveOut => "move_out",
        InspectionType::Routine => "routine",
        InspectionType::Emergency => "emergency",
    }
}

fn inspection_status_str(s: &InspectionStatus) -> &'static str {
    match s {
        InspectionStatus::Scheduled => "scheduled",
        InspectionStatus::InProgress => "in_progress",
        InspectionStatus::Completed => "completed",
        InspectionStatus::Cancelled => "cancelled",
    }
}
