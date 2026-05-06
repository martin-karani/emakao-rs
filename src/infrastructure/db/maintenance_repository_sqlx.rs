use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::maintenance_repository::MaintenanceRepository},
    domain::{
        enums::{WorkOrderPriority, WorkOrderStatus},
        maintenance::{CreateWorkOrderCommand, UpdateWorkOrderCommand, WorkOrder},
    },
};

pub struct PgMaintenanceRepo {
    pool: PgPool,
}

impl PgMaintenanceRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl From<PgPool> for PgMaintenanceRepo {
    fn from(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct WorkOrderRow {
    id: Uuid,
    property_id: Uuid,
    unit_id: Option<Uuid>,
    title: String,
    description: Option<String>,
    status: String,
    priority: String,
    vendor_id: Option<Uuid>,
    reported_by: Uuid,
    created_at: time::OffsetDateTime,
    updated_at: time::OffsetDateTime,
}

fn parse_status(s: &str) -> WorkOrderStatus {
    match s {
        "in_progress" => WorkOrderStatus::InProgress,
        "completed" => WorkOrderStatus::Completed,
        "cancelled" => WorkOrderStatus::Cancelled,
        _ => WorkOrderStatus::Open,
    }
}

fn status_str(s: &WorkOrderStatus) -> &'static str {
    match s {
        WorkOrderStatus::Open => "open",
        WorkOrderStatus::InProgress => "in_progress",
        WorkOrderStatus::Completed => "completed",
        WorkOrderStatus::Cancelled => "cancelled",
    }
}

fn parse_priority(s: &str) -> WorkOrderPriority {
    match s {
        "high" => WorkOrderPriority::High,
        "emergency" => WorkOrderPriority::Emergency,
        "low" => WorkOrderPriority::Low,
        _ => WorkOrderPriority::Medium,
    }
}

fn priority_str(p: &WorkOrderPriority) -> &'static str {
    match p {
        WorkOrderPriority::Low => "low",
        WorkOrderPriority::Medium => "medium",
        WorkOrderPriority::High => "high",
        WorkOrderPriority::Emergency => "emergency",
    }
}

impl From<WorkOrderRow> for WorkOrder {
    fn from(r: WorkOrderRow) -> Self {
        Self {
            id: r.id,
            property_id: r.property_id,
            unit_id: r.unit_id,
            title: r.title,
            description: r.description,
            status: parse_status(&r.status),
            priority: parse_priority(&r.priority),
            vendor_id: r.vendor_id,
            reported_by: r.reported_by,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[async_trait]
impl MaintenanceRepository for PgMaintenanceRepo {
    async fn find_all(
        &self,
        agency_id: Uuid,
        property_id: Option<Uuid>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<WorkOrder>, AppError> {
        let rows = sqlx::query_as!(
            WorkOrderRow,
            r#"
            SELECT wo.id, wo.property_id, wo.unit_id, wo.title, wo.description,
                   wo.status, wo.priority, wo.vendor_id, wo.reported_by,
                   wo.created_at, wo.updated_at
            FROM work_orders wo
            JOIN properties p ON p.id = wo.property_id
            WHERE p.agency_id = $1::uuid
              AND ($2::uuid IS NULL OR wo.property_id = $2::uuid)
            ORDER BY
                CASE wo.priority
                    WHEN 'emergency' THEN 1
                    WHEN 'high'      THEN 2
                    WHEN 'medium'    THEN 3
                    ELSE 4
                END,
                wo.created_at DESC
            LIMIT $3 OFFSET $4
            "#,
            agency_id,
            property_id,
            limit,
            offset
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(WorkOrder::from).collect())
    }

    async fn find_by_id(&self, agency_id: Uuid, id: Uuid) -> Result<Option<WorkOrder>, AppError> {
        let row = sqlx::query_as!(
            WorkOrderRow,
            r#"
            SELECT wo.id, wo.property_id, wo.unit_id, wo.title, wo.description,
                   wo.status, wo.priority, wo.vendor_id, wo.reported_by,
                   wo.created_at, wo.updated_at
            FROM work_orders wo
            JOIN properties p ON p.id = wo.property_id
            WHERE wo.id = $1::uuid AND p.agency_id = $2::uuid
            "#,
            id,
            agency_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(WorkOrder::from))
    }

    async fn create(&self, cmd: CreateWorkOrderCommand) -> Result<WorkOrder, AppError> {
        let row = sqlx::query_as!(
            WorkOrderRow,
            r#"
            INSERT INTO work_orders (
                id, property_id, unit_id, title, description,
                status, priority, vendor_id, reported_by
            )
            VALUES ($1::uuid, $2::uuid, $3::uuid, $4::text, $5::text,
                    'open', $6::text, $7::uuid, $8::uuid)
            RETURNING
                id, property_id, unit_id, title, description,
                status, priority, vendor_id, reported_by,
                created_at, updated_at
            "#,
            Uuid::new_v4(),
            cmd.property_id,
            cmd.unit_id,
            cmd.title,
            cmd.description,
            priority_str(&cmd.priority),
            cmd.vendor_id,
            cmd.reported_by
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(WorkOrder::from(row))
    }

    async fn update(&self, cmd: UpdateWorkOrderCommand) -> Result<WorkOrder, AppError> {
        // $1 = id, $2 = status (Option<&str>), $3 = vendor_id_is_some (bool),
        // $4 = vendor_id (Option<Uuid>), $5 = description (Option<String>)
        // All $N are referenced in SQL so Postgres can infer every type.
        let row = sqlx::query_as!(
            WorkOrderRow,
            r#"
            UPDATE work_orders
            SET
                status      = COALESCE($2::text, status),
                vendor_id   = CASE
                                WHEN $3::boolean THEN $4::uuid
                                ELSE vendor_id
                              END,
                description = COALESCE($5::text, description),
                updated_at  = now()
            WHERE id = $1::uuid
            RETURNING
                id, property_id, unit_id, title, description,
                status, priority, vendor_id, reported_by,
                created_at, updated_at
            "#,
            cmd.id,
            cmd.status.as_ref().map(status_str), // $2
            cmd.vendor_id.is_some(),             // $3
            cmd.vendor_id.flatten(),             // $4
            cmd.description,                     // $5
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("work order {}", cmd.id)))?;

        Ok(WorkOrder::from(row))
    }
}
