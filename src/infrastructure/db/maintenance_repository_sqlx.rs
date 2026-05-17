use async_trait::async_trait;
use rust_decimal::Decimal;
use serde_json::Value;
use sqlx::PgPool;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::maintenance_repository::MaintenanceRepository},
    domain::{
        enums::{
            WorkOrderCategory, WorkOrderCommentAuthorType, WorkOrderPriority,
            WorkOrderReporterType, WorkOrderStatus,
        },
        maintenance::{
            Caretaker, CreateCaretakerCommand, CreateWorkOrderCommand,
            CreateWorkOrderCommentCommand, UpdateCaretakerCommand, UpdateWorkOrderCommand,
            WorkOrder, WorkOrderActivity, WorkOrderAttachment, WorkOrderComment,
        },
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

// ── Low-level row structs ─────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct CaretakerRow {
    id: Uuid,
    property_id: Uuid,
    user_id: Option<Uuid>,
    first_name: String,
    last_name: String,
    phone: Option<String>,
    email: Option<String>,
    is_active: bool,
    created_by: Uuid,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

impl From<CaretakerRow> for Caretaker {
    fn from(r: CaretakerRow) -> Self {
        Self {
            id: r.id,
            property_id: r.property_id,
            user_id: r.user_id,
            first_name: r.first_name,
            last_name: r.last_name,
            phone: r.phone,
            email: r.email,
            is_active: r.is_active,
            created_by: r.created_by,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct WorkOrderRow {
    id: Uuid,
    property_id: Uuid,
    unit_id: Option<Uuid>,
    vendor_id: Option<Uuid>,
    code: String,
    work_order_number: i32,
    title: String,
    description: Option<String>,
    category: String,
    status: String,
    priority: String,
    reported_by: Uuid,
    reporter_type: String,
    reporter_resident_id: Option<Uuid>,
    reporter_caretaker_id: Option<Uuid>,
    assigned_to: Option<Uuid>,
    assigned_caretaker_id: Option<Uuid>,
    due_date: Option<Date>,
    scheduled_at: Option<OffsetDateTime>,
    started_at: Option<OffsetDateTime>,
    completed_at: Option<OffsetDateTime>,
    estimated_cost_kes: Option<Decimal>,
    actual_cost_kes: Option<Decimal>,
    is_tenant_visible: bool,
    internal_notes: Option<String>,
    attachments: Value,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

fn parse_attachments(v: Value) -> Vec<WorkOrderAttachment> {
    serde_json::from_value(v).unwrap_or_default()
}

fn parse_category(s: &str) -> WorkOrderCategory {
    match s {
        "plumbing" => WorkOrderCategory::Plumbing,
        "electrical" => WorkOrderCategory::Electrical,
        "structural" => WorkOrderCategory::Structural,
        "hvac" => WorkOrderCategory::Hvac,
        "appliance" => WorkOrderCategory::Appliance,
        "painting" => WorkOrderCategory::Painting,
        "cleaning" => WorkOrderCategory::Cleaning,
        "security" => WorkOrderCategory::Security,
        "landscaping" => WorkOrderCategory::Landscaping,
        "pest_control" => WorkOrderCategory::PestControl,
        _ => WorkOrderCategory::General,
    }
}

fn category_str(c: &WorkOrderCategory) -> &'static str {
    match c {
        WorkOrderCategory::Plumbing => "plumbing",
        WorkOrderCategory::Electrical => "electrical",
        WorkOrderCategory::Structural => "structural",
        WorkOrderCategory::Hvac => "hvac",
        WorkOrderCategory::Appliance => "appliance",
        WorkOrderCategory::Painting => "painting",
        WorkOrderCategory::Cleaning => "cleaning",
        WorkOrderCategory::Security => "security",
        WorkOrderCategory::Landscaping => "landscaping",
        WorkOrderCategory::PestControl => "pest_control",
        WorkOrderCategory::General => "general",
    }
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

fn parse_reporter_type(s: &str) -> WorkOrderReporterType {
    match s {
        "resident" => WorkOrderReporterType::Resident,
        "caretaker" => WorkOrderReporterType::Caretaker,
        "owner" => WorkOrderReporterType::Owner,
        "vendor" => WorkOrderReporterType::Vendor,
        _ => WorkOrderReporterType::Staff,
    }
}

fn reporter_type_str(r: &WorkOrderReporterType) -> &'static str {
    match r {
        WorkOrderReporterType::Staff => "staff",
        WorkOrderReporterType::Resident => "resident",
        WorkOrderReporterType::Caretaker => "caretaker",
        WorkOrderReporterType::Owner => "owner",
        WorkOrderReporterType::Vendor => "vendor",
    }
}

fn parse_author_type(s: &str) -> WorkOrderCommentAuthorType {
    match s {
        "resident" => WorkOrderCommentAuthorType::Resident,
        "caretaker" => WorkOrderCommentAuthorType::Caretaker,
        "owner" => WorkOrderCommentAuthorType::Owner,
        "vendor" => WorkOrderCommentAuthorType::Vendor,
        _ => WorkOrderCommentAuthorType::Staff,
    }
}

fn author_type_str(a: &WorkOrderCommentAuthorType) -> &'static str {
    match a {
        WorkOrderCommentAuthorType::Staff => "staff",
        WorkOrderCommentAuthorType::Resident => "resident",
        WorkOrderCommentAuthorType::Caretaker => "caretaker",
        WorkOrderCommentAuthorType::Owner => "owner",
        WorkOrderCommentAuthorType::Vendor => "vendor",
    }
}

impl From<WorkOrderRow> for WorkOrder {
    fn from(r: WorkOrderRow) -> Self {
        Self {
            id: r.id,
            property_id: r.property_id,
            unit_id: r.unit_id,
            vendor_id: r.vendor_id,
            code: r.code,
            work_order_number: r.work_order_number,
            title: r.title,
            description: r.description,
            category: parse_category(&r.category),
            status: parse_status(&r.status),
            priority: parse_priority(&r.priority),
            reported_by: r.reported_by,
            reporter_type: parse_reporter_type(&r.reporter_type),
            reporter_resident_id: r.reporter_resident_id,
            reporter_caretaker_id: r.reporter_caretaker_id,
            assigned_to: r.assigned_to,
            assigned_caretaker_id: r.assigned_caretaker_id,
            due_date: r.due_date,
            scheduled_at: r.scheduled_at,
            started_at: r.started_at,
            completed_at: r.completed_at,
            estimated_cost_kes: r.estimated_cost_kes,
            actual_cost_kes: r.actual_cost_kes,
            is_tenant_visible: r.is_tenant_visible,
            internal_notes: r.internal_notes,
            attachments: parse_attachments(r.attachments),
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct CommentRow {
    id: Uuid,
    work_order_id: Uuid,
    parent_comment_id: Option<Uuid>,
    author_id: Uuid,
    author_type: String,
    author_resident_id: Option<Uuid>,
    author_caretaker_id: Option<Uuid>,
    body: String,
    is_internal: bool,
    attachments: Value,
    is_edited: bool,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

impl From<CommentRow> for WorkOrderComment {
    fn from(r: CommentRow) -> Self {
        Self {
            id: r.id,
            work_order_id: r.work_order_id,
            parent_comment_id: r.parent_comment_id,
            author_id: r.author_id,
            author_type: parse_author_type(&r.author_type),
            author_resident_id: r.author_resident_id,
            author_caretaker_id: r.author_caretaker_id,
            body: r.body,
            is_internal: r.is_internal,
            attachments: parse_attachments(r.attachments),
            is_edited: r.is_edited,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct ActivityRow {
    id: Uuid,
    work_order_id: Uuid,
    actor_id: Uuid,
    actor_type: String,
    event_type: String,
    payload: Value,
    created_at: OffsetDateTime,
}

impl From<ActivityRow> for WorkOrderActivity {
    fn from(r: ActivityRow) -> Self {
        Self {
            id: r.id,
            work_order_id: r.work_order_id,
            actor_id: r.actor_id,
            actor_type: parse_author_type(&r.actor_type),
            event_type: r.event_type,
            payload: r.payload,
            created_at: r.created_at,
        }
    }
}

// ── Trait implementation ──────────────────────────────────────────────────────

#[async_trait]
impl MaintenanceRepository for PgMaintenanceRepo {
    // ── Caretakers ────────────────────────────────────────────────────────────

    async fn find_caretakers(
        &self,
        property_id: Option<Uuid>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Caretaker>, AppError> {
        let rows = sqlx::query_as::<_, CaretakerRow>(
            r#"
            SELECT id, property_id, user_id, first_name, last_name, phone, email,
                   is_active, created_by, created_at, updated_at
            FROM caretakers
            WHERE ($1::uuid IS NULL OR property_id = $1::uuid)
            ORDER BY first_name, last_name
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(property_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(Caretaker::from).collect())
    }

    async fn find_caretaker_by_id(&self, id: Uuid) -> Result<Option<Caretaker>, AppError> {
        let row = sqlx::query_as::<_, CaretakerRow>(
            r#"
            SELECT id, property_id, user_id, first_name, last_name, phone, email,
                   is_active, created_by, created_at, updated_at
            FROM caretakers WHERE id = $1::uuid
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(Caretaker::from))
    }

    async fn create_caretaker(&self, cmd: CreateCaretakerCommand) -> Result<Caretaker, AppError> {
        let row = sqlx::query_as::<_, CaretakerRow>(
            r#"
            INSERT INTO caretakers (
                id, property_id, first_name, last_name, phone, email, created_by
            )
            VALUES (uuidv7(), $1::uuid, $2::text, $3::text, $4::text, $5::text, $6::uuid)
            RETURNING id, property_id, user_id, first_name, last_name, phone, email,
                      is_active, created_by, created_at, updated_at
            "#,
        )
        .bind(cmd.property_id)
        .bind(cmd.first_name)
        .bind(cmd.last_name)
        .bind(cmd.phone)
        .bind(cmd.email)
        .bind(cmd.created_by)
        .fetch_one(&self.pool)
        .await?;
        Ok(Caretaker::from(row))
    }

    async fn update_caretaker(&self, cmd: UpdateCaretakerCommand) -> Result<Caretaker, AppError> {
        let row = sqlx::query_as::<_, CaretakerRow>(
            r#"
            UPDATE caretakers SET
                first_name = COALESCE($2::text, first_name),
                last_name  = COALESCE($3::text, last_name),
                phone      = COALESCE($4::text, phone),
                email      = COALESCE($5::text, email),
                is_active  = COALESCE($6::boolean, is_active),
                updated_at = now()
            WHERE id = $1::uuid
            RETURNING id, property_id, user_id, first_name, last_name, phone, email,
                      is_active, created_by, created_at, updated_at
            "#,
        )
        .bind(cmd.id)
        .bind(cmd.first_name)
        .bind(cmd.last_name)
        .bind(cmd.phone)
        .bind(cmd.email)
        .bind(cmd.is_active)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("caretaker {}", cmd.id)))?;
        Ok(Caretaker::from(row))
    }

    // ── Work Orders ───────────────────────────────────────────────────────────

    async fn find_all(
        &self,
        agency_id: Uuid,
        property_id: Option<Uuid>,
        unit_id: Option<Uuid>,
        status: Option<WorkOrderStatus>,
        priority: Option<WorkOrderPriority>,
        category: Option<WorkOrderCategory>,
        reporter_type: Option<WorkOrderReporterType>,
        assigned_caretaker_id: Option<Uuid>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<WorkOrder>, AppError> {
        let status_s = status.as_ref().map(status_str);
        let priority_s = priority.as_ref().map(priority_str);
        let category_s = category.as_ref().map(category_str);
        let reporter_s = reporter_type.as_ref().map(reporter_type_str);

        let rows = sqlx::query_as::<_, WorkOrderRow>(
            r#"
            SELECT wo.id, wo.property_id, wo.unit_id, wo.vendor_id,
                   wo.code, wo.work_order_number,
                   wo.title, wo.description, wo.category::text AS category,
                   wo.status::text AS status, wo.priority::text AS priority,
                   wo.reported_by, wo.reporter_type::text AS reporter_type,
                   wo.reporter_resident_id, wo.reporter_caretaker_id,
                   wo.assigned_to, wo.assigned_caretaker_id,
                   wo.due_date, wo.scheduled_at, wo.started_at, wo.completed_at,
                   wo.estimated_cost_kes, wo.actual_cost_kes,
                   wo.is_tenant_visible, wo.internal_notes, wo.attachments,
                   wo.created_at, wo.updated_at
            FROM work_orders wo
            JOIN properties p ON p.id = wo.property_id
            WHERE p.agency_id = $1
              AND ($2::uuid IS NULL OR wo.property_id = $2::uuid)
              AND ($3::uuid IS NULL OR wo.unit_id = $3::uuid)
              AND ($4::text IS NULL OR wo.status = $4::text::work_order_status)
              AND ($5::text IS NULL OR wo.priority = $5::text::work_order_priority)
              AND ($6::text IS NULL OR wo.category = $6::text::work_order_category)
              AND ($7::text IS NULL OR wo.reporter_type = $7::text::work_order_reporter_type)
              AND ($8::uuid IS NULL OR wo.assigned_caretaker_id = $8::uuid)
            ORDER BY
                CASE wo.priority
                    WHEN 'emergency' THEN 1
                    WHEN 'high'      THEN 2
                    WHEN 'medium'    THEN 3
                    ELSE 4
                END,
                wo.created_at DESC
            LIMIT $9 OFFSET $10
            "#,
        )
        .bind(agency_id)
        .bind(property_id)
        .bind(unit_id)
        .bind(status_s)
        .bind(priority_s)
        .bind(category_s)
        .bind(reporter_s)
        .bind(assigned_caretaker_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(WorkOrder::from).collect())
    }

    async fn find_by_id(&self, agency_id: Uuid, id: Uuid) -> Result<Option<WorkOrder>, AppError> {
        let row = sqlx::query_as::<_, WorkOrderRow>(
            r#"
            SELECT wo.id, wo.property_id, wo.unit_id, wo.vendor_id,
                   wo.code, wo.work_order_number,
                   wo.title, wo.description, wo.category::text AS category,
                   wo.status::text AS status, wo.priority::text AS priority,
                   wo.reported_by, wo.reporter_type::text AS reporter_type,
                   wo.reporter_resident_id, wo.reporter_caretaker_id,
                   wo.assigned_to, wo.assigned_caretaker_id,
                   wo.due_date, wo.scheduled_at, wo.started_at, wo.completed_at,
                   wo.estimated_cost_kes, wo.actual_cost_kes,
                   wo.is_tenant_visible, wo.internal_notes, wo.attachments,
                   wo.created_at, wo.updated_at
            FROM work_orders wo
            JOIN properties p ON p.id = wo.property_id
            WHERE wo.id = $1::uuid AND p.agency_id = $2::uuid
            "#,
        )
        .bind(id)
        .bind(agency_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(WorkOrder::from))
    }

    async fn find_for_resident(
        &self,
        resident_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<WorkOrder>, AppError> {
        let rows = sqlx::query_as::<_, WorkOrderRow>(
            r#"
            SELECT wo.id, wo.property_id, wo.unit_id, wo.vendor_id,
                   wo.code, wo.work_order_number,
                   wo.title, wo.description, wo.category::text AS category,
                   wo.status::text AS status, wo.priority::text AS priority,
                   wo.reported_by, wo.reporter_type::text AS reporter_type,
                   wo.reporter_resident_id, wo.reporter_caretaker_id,
                   wo.assigned_to, wo.assigned_caretaker_id,
                   wo.due_date, wo.scheduled_at, wo.started_at, wo.completed_at,
                   wo.estimated_cost_kes, wo.actual_cost_kes,
                   wo.is_tenant_visible, wo.internal_notes, wo.attachments,
                   wo.created_at, wo.updated_at
            FROM work_orders wo
            JOIN agreements ag ON ag.unit_id = wo.unit_id
            WHERE ag.resident_id = $1::uuid
              AND ag.status = 'active'
              AND wo.is_tenant_visible = true
            ORDER BY wo.created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(resident_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(WorkOrder::from).collect())
    }

    async fn find_for_caretaker(
        &self,
        caretaker_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<WorkOrder>, AppError> {
        let rows = sqlx::query_as::<_, WorkOrderRow>(
            r#"
            SELECT wo.id, wo.property_id, wo.unit_id, wo.vendor_id,
                   wo.code, wo.work_order_number,
                   wo.title, wo.description, wo.category::text AS category,
                   wo.status::text AS status, wo.priority::text AS priority,
                   wo.reported_by, wo.reporter_type::text AS reporter_type,
                   wo.reporter_resident_id, wo.reporter_caretaker_id,
                   wo.assigned_to, wo.assigned_caretaker_id,
                   wo.due_date, wo.scheduled_at, wo.started_at, wo.completed_at,
                   wo.estimated_cost_kes, wo.actual_cost_kes,
                   wo.is_tenant_visible, wo.internal_notes, wo.attachments,
                   wo.created_at, wo.updated_at
            FROM work_orders wo
            JOIN caretakers c ON c.property_id = wo.property_id
            WHERE c.id = $1::uuid
            ORDER BY wo.created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(caretaker_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(WorkOrder::from).collect())
    }

    async fn find_work_orders_by_vendor_id(
        &self,
        vendor_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<WorkOrder>, AppError> {
        let rows = sqlx::query_as::<_, WorkOrderRow>(
            r#"
            SELECT id, property_id, unit_id, vendor_id,
                   code, work_order_number,
                   title, description, category::text AS category,
                   status::text AS status, priority::text AS priority,
                   reported_by, reporter_type::text AS reporter_type,
                   reporter_resident_id, reporter_caretaker_id,
                   assigned_to, assigned_caretaker_id,
                   due_date, scheduled_at, started_at, completed_at,
                   estimated_cost_kes, actual_cost_kes,
                   is_tenant_visible, internal_notes, attachments,
                   created_at, updated_at
            FROM work_orders
            WHERE vendor_id = $1::uuid
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(vendor_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(WorkOrder::from).collect())
    }

    async fn find_by_code(
        &self,
        agency_id: Uuid,
        code: &str,
    ) -> Result<Option<WorkOrder>, AppError> {
        let code_upper = code.to_ascii_uppercase();

        let row = sqlx::query_as::<_, WorkOrderRow>(
            r#"
            SELECT wo.id, wo.property_id, wo.unit_id, wo.vendor_id,
                   wo.code, wo.work_order_number,
                   wo.title, wo.description, wo.category::text AS category,
                   wo.status::text AS status, wo.priority::text AS priority,
                   wo.reported_by, wo.reporter_type::text AS reporter_type,
                   wo.reporter_resident_id, wo.reporter_caretaker_id,
                   wo.assigned_to, wo.assigned_caretaker_id,
                   wo.due_date, wo.scheduled_at, wo.started_at, wo.completed_at,
                   wo.estimated_cost_kes, wo.actual_cost_kes,
                   wo.is_tenant_visible, wo.internal_notes, wo.attachments,
                   wo.created_at, wo.updated_at
            FROM   work_orders wo
            JOIN   properties p ON p.id = wo.property_id
            WHERE  wo.code = $1
              AND  p.agency_id = $2::uuid
            "#,
        )
        .bind(code_upper)
        .bind(agency_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(WorkOrder::from))
    }

    async fn create(&self, cmd: CreateWorkOrderCommand) -> Result<WorkOrder, AppError> {
        let attachments =
            serde_json::to_value(&cmd.attachments).unwrap_or(serde_json::Value::Array(vec![]));

        let mut tx = self.pool.begin().await?;

        let (seq, prefix): (i32, String) = sqlx::query_as(
            r#"
            UPDATE properties
            SET    work_order_seq = work_order_seq + 1
            WHERE  id = $1
            RETURNING work_order_seq, work_order_prefix
            "#,
        )
        .bind(cmd.property_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| AppError::NotFound(format!("property {}", cmd.property_id)))?;

        let code = crate::domain::work_order_code::format_code(&prefix, seq);

        let row = sqlx::query_as::<_, WorkOrderRow>(
            r#"
            INSERT INTO work_orders (
                id, property_id, unit_id, vendor_id,
                code, work_order_number,
                title, description, category, status, priority,
                reported_by, reporter_type, reporter_resident_id, reporter_caretaker_id,
                assigned_to, assigned_caretaker_id,
                due_date, scheduled_at, estimated_cost_kes,
                is_tenant_visible, internal_notes, attachments
            )
            VALUES (
                uuidv7(), $1::uuid, $2::uuid, $3::uuid,
                $4::text, $5::int,
                $6::text, $7::text, $8::text, 'open', $9::text,
                $10::uuid, $11::text, $12::uuid, $13::uuid,
                $14::uuid, $15::uuid,
                $16::date, $17::timestamptz, $18,
                $19::boolean, $20::text, $21::jsonb
            )
            RETURNING
                id, property_id, unit_id, vendor_id,
                code, work_order_number,
                title, description, category::text AS category,
                status::text AS status, priority::text AS priority,
                reported_by, reporter_type::text AS reporter_type,
                reporter_resident_id, reporter_caretaker_id,
                assigned_to, assigned_caretaker_id,
                due_date, scheduled_at, started_at, completed_at,
                estimated_cost_kes, actual_cost_kes,
                is_tenant_visible, internal_notes, attachments,
                created_at, updated_at
            "#,
        )
        .bind(cmd.property_id)
        .bind(cmd.unit_id)
        .bind(cmd.vendor_id)
        .bind(code)
        .bind(seq)
        .bind(cmd.title)
        .bind(cmd.description)
        .bind(category_str(&cmd.category))
        .bind(priority_str(&cmd.priority))
        .bind(cmd.reported_by)
        .bind(reporter_type_str(&cmd.reporter_type))
        .bind(cmd.reporter_resident_id)
        .bind(cmd.reporter_caretaker_id)
        .bind(cmd.assigned_to)
        .bind(cmd.assigned_caretaker_id)
        .bind(cmd.due_date)
        .bind(cmd.scheduled_at)
        .bind(cmd.estimated_cost_kes)
        .bind(cmd.is_tenant_visible)
        .bind(cmd.internal_notes)
        .bind(attachments)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(WorkOrder::from(row))
    }

    async fn update(&self, cmd: UpdateWorkOrderCommand) -> Result<WorkOrder, AppError> {
        let status_s = cmd.status.as_ref().map(status_str);
        let priority_s = cmd.priority.as_ref().map(priority_str);
        let category_s = cmd.category.as_ref().map(category_str);
        let attachments = cmd
            .attachments
            .as_ref()
            .map(|a| serde_json::to_value(a).unwrap_or(Value::Array(vec![])));

        let row = sqlx::query_as::<_, WorkOrderRow>(
            r#"
            UPDATE work_orders SET
                status               = COALESCE($2::text,      status::text)::work_order_status,
                priority             = COALESCE($3::text,      priority::text)::work_order_priority,
                category             = COALESCE($4::text,      category::text)::work_order_category,
                vendor_id            = CASE WHEN $5::boolean THEN $6::uuid       ELSE vendor_id END,
                assigned_to          = CASE WHEN $7::boolean THEN $8::uuid       ELSE assigned_to END,
                assigned_caretaker_id= CASE WHEN $9::boolean THEN $10::uuid      ELSE assigned_caretaker_id END,
                description          = COALESCE($11::text,     description),
                internal_notes       = COALESCE($12::text,     internal_notes),
                due_date             = CASE WHEN $13::boolean THEN $14::date     ELSE due_date END,
                scheduled_at         = CASE WHEN $15::boolean THEN $16::timestamptz ELSE scheduled_at END,
                started_at           = CASE WHEN $17::boolean THEN $18::timestamptz ELSE started_at END,
                completed_at         = CASE WHEN $19::boolean THEN $20::timestamptz ELSE completed_at END,
                estimated_cost_kes   = CASE WHEN $21::boolean THEN $22           ELSE estimated_cost_kes END,
                actual_cost_kes      = CASE WHEN $23::boolean THEN $24           ELSE actual_cost_kes END,
                is_tenant_visible    = COALESCE($25::boolean,  is_tenant_visible),
                attachments          = COALESCE($26::jsonb,    attachments),
                updated_at           = now()
            WHERE id = $1::uuid
            RETURNING
                id, property_id, unit_id, vendor_id,
                code, work_order_number,
                title, description, category::text AS category,
                status::text AS status, priority::text AS priority,
                reported_by, reporter_type::text AS reporter_type,
                reporter_resident_id, reporter_caretaker_id,
                assigned_to, assigned_caretaker_id,
                due_date, scheduled_at, started_at, completed_at,
                estimated_cost_kes, actual_cost_kes,
                is_tenant_visible, internal_notes, attachments,
                created_at, updated_at
            "#,
        )
        .bind(cmd.id)
        .bind(status_s)
        .bind(priority_s)
        .bind(category_s)
        .bind(cmd.vendor_id.is_some())
        .bind(cmd.vendor_id.flatten())
        .bind(cmd.assigned_to.is_some())
        .bind(cmd.assigned_to.flatten())
        .bind(cmd.assigned_caretaker_id.is_some())
        .bind(cmd.assigned_caretaker_id.flatten())
        .bind(cmd.description)
        .bind(cmd.internal_notes)
        .bind(cmd.due_date.is_some())
        .bind(cmd.due_date.flatten())
        .bind(cmd.scheduled_at.is_some())
        .bind(cmd.scheduled_at.flatten())
        .bind(cmd.started_at.is_some())
        .bind(cmd.started_at.flatten())
        .bind(cmd.completed_at.is_some())
        .bind(cmd.completed_at.flatten())
        .bind(cmd.estimated_cost_kes.is_some())
        .bind(cmd.estimated_cost_kes.flatten())
        .bind(cmd.actual_cost_kes.is_some())
        .bind(cmd.actual_cost_kes.flatten())
        .bind(cmd.is_tenant_visible)
        .bind(attachments)
        .fetch_optional(&self.pool).await?
        .ok_or_else(|| AppError::NotFound(format!("work order {}", cmd.id)))?;
        Ok(WorkOrder::from(row))
    }

    // ── Comments ──────────────────────────────────────────────────────────────

    async fn find_comments(
        &self,
        work_order_id: Uuid,
        include_internal: bool,
        top_level_only: bool,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<WorkOrderComment>, AppError> {
        let rows = sqlx::query_as::<_, CommentRow>(
            r#"
            SELECT id, work_order_id, parent_comment_id,
                   author_id, author_type, author_resident_id, author_caretaker_id,
                   body, is_internal, attachments, is_edited, created_at, updated_at
            FROM work_order_comments
            WHERE work_order_id = $1::uuid
              AND ($2::boolean = true OR is_internal = false)
              AND ($3::boolean = false OR parent_comment_id IS NULL)
            ORDER BY created_at ASC
            LIMIT $4 OFFSET $5
            "#,
        )
        .bind(work_order_id)
        .bind(include_internal)
        .bind(top_level_only)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(WorkOrderComment::from).collect())
    }

    async fn find_comment_replies(
        &self,
        parent_comment_id: Uuid,
        include_internal: bool,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<WorkOrderComment>, AppError> {
        let rows = sqlx::query_as::<_, CommentRow>(
            r#"
            SELECT id, work_order_id, parent_comment_id,
                   author_id, author_type, author_resident_id, author_caretaker_id,
                   body, is_internal, attachments, is_edited, created_at, updated_at
            FROM work_order_comments
            WHERE parent_comment_id = $1::uuid
              AND ($2::boolean = true OR is_internal = false)
            ORDER BY created_at ASC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(parent_comment_id)
        .bind(include_internal)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(WorkOrderComment::from).collect())
    }

    async fn create_comment(
        &self,
        cmd: CreateWorkOrderCommentCommand,
    ) -> Result<WorkOrderComment, AppError> {
        let attachments = serde_json::to_value(&cmd.attachments).unwrap_or(Value::Array(vec![]));
        let row = sqlx::query_as::<_, CommentRow>(
            r#"
            INSERT INTO work_order_comments (
                id, work_order_id, parent_comment_id,
                author_id, author_type, author_resident_id, author_caretaker_id,
                body, is_internal, attachments
            )
            VALUES (
                uuidv7(), $1::uuid, $2::uuid,
                $3::uuid, $4::text, $5::uuid, $6::uuid,
                $7::text, $8::boolean, $9::jsonb
            )
            RETURNING
                id, work_order_id, parent_comment_id,
                author_id, author_type, author_resident_id, author_caretaker_id,
                body, is_internal, attachments, is_edited, created_at, updated_at
            "#,
        )
        .bind(cmd.work_order_id)
        .bind(cmd.parent_comment_id)
        .bind(cmd.author_id)
        .bind(author_type_str(&cmd.author_type))
        .bind(cmd.author_resident_id)
        .bind(cmd.author_caretaker_id)
        .bind(cmd.body)
        .bind(cmd.is_internal)
        .bind(attachments)
        .fetch_one(&self.pool)
        .await?;
        Ok(WorkOrderComment::from(row))
    }

    // ── Activity Log ──────────────────────────────────────────────────────────

    async fn find_activity(
        &self,
        work_order_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<WorkOrderActivity>, AppError> {
        let rows = sqlx::query_as::<_, ActivityRow>(
            r#"
            SELECT id, work_order_id, actor_id, actor_type, event_type, payload, created_at
            FROM work_order_activity
            WHERE work_order_id = $1::uuid
            ORDER BY created_at ASC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(work_order_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(WorkOrderActivity::from).collect())
    }

    async fn log_activity(
        &self,
        work_order_id: Uuid,
        actor_id: Uuid,
        actor_type: &str,
        event_type: &str,
        payload: Value,
    ) -> Result<(), AppError> {
        sqlx::query(
            r#"
            INSERT INTO work_order_activity
                (id, work_order_id, actor_id, actor_type, event_type, payload)
            VALUES (uuidv7(), $1::uuid, $2::uuid, $3::text, $4::text, $5::jsonb)
            "#,
        )
        .bind(work_order_id)
        .bind(actor_id)
        .bind(actor_type)
        .bind(event_type)
        .bind(payload)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
