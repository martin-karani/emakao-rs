use async_trait::async_trait;
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::vendor_repository::VendorRepository},
    domain::{
        enums::{
            PortalStatus, VendorStatus, WorkOrderCategory, WorkOrderPriority,
            WorkOrderReporterType, WorkOrderStatus,
        },
        maintenance::{WorkOrder, WorkOrderAttachment},
        vendor::{CreateVendorCommand, UpdateVendorCommand, Vendor},
    },
};

pub struct PgVendorRepo {
    pool: PgPool,
}

impl PgVendorRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl From<PgPool> for PgVendorRepo {
    fn from(pool: PgPool) -> Self {
        Self { pool }
    }
}

// ── Vendor row ────────────────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct VendorRow {
    id: Uuid,
    agency_id: Uuid,
    user_id: Option<Uuid>,
    name: String,
    contact_name: Option<String>,
    email: Option<String>,
    phone: Option<String>,
    speciality: Option<String>,
    status: String,
    portal_status: String,
    notes: Option<String>,
    created_at: time::OffsetDateTime,
    updated_at: time::OffsetDateTime,
}

fn parse_vendor_status(s: &str) -> VendorStatus {
    match s {
        "inactive" => VendorStatus::Inactive,
        "blacklisted" => VendorStatus::Blacklisted,
        _ => VendorStatus::Active,
    }
}

fn vendor_status_str(s: &VendorStatus) -> &'static str {
    match s {
        VendorStatus::Active => "active",
        VendorStatus::Inactive => "inactive",
        VendorStatus::Blacklisted => "blacklisted",
    }
}

fn parse_portal_status(s: &str) -> PortalStatus {
    match s {
        "active" => PortalStatus::Active,
        "suspended" => PortalStatus::Suspended,
        _ => PortalStatus::Invited,
    }
}

impl From<VendorRow> for Vendor {
    fn from(r: VendorRow) -> Self {
        Self {
            id: r.id,
            agency_id: r.agency_id,
            user_id: r.user_id,
            name: r.name,
            contact_name: r.contact_name,
            email: r.email,
            phone: r.phone,
            speciality: r.speciality,
            status: parse_vendor_status(&r.status),
            portal_status: parse_portal_status(&r.portal_status),
            notes: r.notes,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

// ── Repository ────────────────────────────────────────────────────────────────

#[async_trait]
impl VendorRepository for PgVendorRepo {
    async fn find_all(
        &self,
        agency_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Vendor>, AppError> {
        let rows = sqlx::query_as::<_, VendorRow>(
            r#"
            SELECT id, agency_id, user_id, name, contact_name, email, phone,
                   speciality, status::text AS status, portal_status::text as portal_status,
                   notes, created_at, updated_at
            FROM vendors
            WHERE agency_id = $1
            ORDER BY name ASC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(agency_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Vendor::from).collect())
    }

    async fn find_by_id(&self, agency_id: Uuid, id: Uuid) -> Result<Option<Vendor>, AppError> {
        let row = sqlx::query_as::<_, VendorRow>(
            r#"
            SELECT id, agency_id, user_id, name, contact_name, email, phone,
                   speciality, status::text AS status, portal_status::text as portal_status,
                   notes, created_at, updated_at
            FROM vendors
            WHERE id = $1 AND agency_id = $2
            "#,
        )
        .bind(id)
        .bind(agency_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Vendor::from))
    }

    async fn find_by_user_id(&self, user_id: Uuid) -> Result<Option<Vendor>, AppError> {
        let row = sqlx::query_as::<_, VendorRow>(
            r#"
            SELECT id, agency_id, user_id, name, contact_name, email, phone,
                   speciality, status::text AS status, portal_status::text as portal_status,
                   notes, created_at, updated_at
            FROM vendors
            WHERE user_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Vendor::from))
    }

    async fn create(&self, cmd: CreateVendorCommand) -> Result<Vendor, AppError> {
        let row = sqlx::query_as::<_, VendorRow>(
            r#"
            INSERT INTO vendors (
                id, agency_id, user_id, name, contact_name, email,
                phone, speciality, notes
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING
                id, agency_id, user_id, name, contact_name, email,
                phone, speciality, status::text AS status, portal_status::text as portal_status,
                notes, created_at, updated_at
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(cmd.agency_id)
        .bind(cmd.user_id)
        .bind(cmd.name)
        .bind(cmd.contact_name)
        .bind(cmd.email)
        .bind(cmd.phone)
        .bind(cmd.speciality)
        .bind(cmd.notes)
        .fetch_one(&self.pool)
        .await?;

        Ok(Vendor::from(row))
    }

    async fn update(&self, cmd: UpdateVendorCommand) -> Result<Vendor, AppError> {
        let row: Option<VendorRow> = sqlx::query_as::<_, VendorRow>(
            r#"
            UPDATE vendors
            SET
                name         = COALESCE($3, name),
                contact_name = COALESCE($4, contact_name),
                email        = COALESCE($5, email),
                phone        = COALESCE($6, phone),
                speciality   = COALESCE($7, speciality),
                status       = COALESCE($8::text::vendor_status, status),
                notes        = COALESCE($9, notes),
                updated_at   = now()
            WHERE id = $1 AND agency_id = $2
            RETURNING
                id, agency_id, user_id, name, contact_name, email,
                phone, speciality, status::text AS status, portal_status::text as portal_status,
                notes, created_at, updated_at
            "#,
        )
        .bind(cmd.id)
        .bind(cmd.agency_id)
        .bind(cmd.name)
        .bind(cmd.contact_name)
        .bind(cmd.email)
        .bind(cmd.phone)
        .bind(cmd.speciality)
        .bind(cmd.status.as_ref().map(vendor_status_str))
        .bind(cmd.notes)
        .fetch_optional(&self.pool)
        .await?;

        let row = row.ok_or_else(|| AppError::NotFound(format!("vendor {}", cmd.id)))?;

        Ok(Vendor::from(row))
    }

    async fn find_work_orders_by_vendor_id(
        &self,
        vendor_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<WorkOrder>, AppError> {
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
            due_date: Option<time::Date>,
            scheduled_at: Option<time::OffsetDateTime>,
            started_at: Option<time::OffsetDateTime>,
            completed_at: Option<time::OffsetDateTime>,
            estimated_cost_kes: Option<Decimal>,
            actual_cost_kes: Option<Decimal>,
            is_tenant_visible: bool,
            internal_notes: Option<String>,
            attachments: serde_json::Value,
            created_at: time::OffsetDateTime,
            updated_at: time::OffsetDateTime,
        }

        let rows = sqlx::query_as::<_, WorkOrderRow>(
            r#"
            SELECT
                id, property_id, unit_id, vendor_id,
                code, work_order_number,
                title, description, category::text AS category,
                status::text AS status, priority::text AS priority,
                reported_by, reporter_type::text AS reporter_type,
                reporter_resident_id, reporter_caretaker_id,
                assigned_to, assigned_caretaker_id,
                due_date, scheduled_at, started_at, completed_at,
                estimated_cost_kes, actual_cost_kes,
                is_tenant_visible, internal_notes,
                COALESCE(attachments, '[]'::jsonb) AS attachments,
                created_at, updated_at
            FROM work_orders
            WHERE vendor_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(vendor_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let results: Vec<Result<WorkOrder, AppError>> = rows
            .into_iter()
            .map(|r| {
                let status = parse_work_order_status(&r.status);
                let priority = parse_work_order_priority(&r.priority);
                let category = parse_work_order_category(&r.category);
                let reporter_type = parse_reporter_type(&r.reporter_type);

                let attachments: Vec<WorkOrderAttachment> =
                    serde_json::from_value(r.attachments).unwrap_or_default();

                Ok(WorkOrder {
                    id: r.id,
                    property_id: r.property_id,
                    unit_id: r.unit_id,
                    vendor_id: r.vendor_id,
                    code: r.code,
                    work_order_number: r.work_order_number,
                    title: r.title,
                    description: r.description,
                    category,
                    status,
                    priority,
                    reported_by: r.reported_by,
                    reporter_type,
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
                    attachments,
                    created_at: r.created_at,
                    updated_at: r.updated_at,
                })
            })
            .collect();

        results.into_iter().collect()
    }
}

// ── Work order parse helpers ──────────────────────────────────────────────────

fn parse_work_order_status(s: &str) -> WorkOrderStatus {
    match s {
        "in_progress" => WorkOrderStatus::InProgress,
        "completed" => WorkOrderStatus::Completed,
        "cancelled" => WorkOrderStatus::Cancelled,
        _ => WorkOrderStatus::Open,
    }
}

fn parse_work_order_priority(s: &str) -> WorkOrderPriority {
    match s {
        "high" => WorkOrderPriority::High,
        "emergency" => WorkOrderPriority::Emergency,
        "low" => WorkOrderPriority::Low,
        _ => WorkOrderPriority::Medium,
    }
}

fn parse_work_order_category(s: &str) -> WorkOrderCategory {
    match s {
        "plumbing" => WorkOrderCategory::Plumbing,
        "electrical" => WorkOrderCategory::Electrical,
        "hvac" => WorkOrderCategory::Hvac,
        "structural" => WorkOrderCategory::Structural,
        "cleaning" => WorkOrderCategory::Cleaning,
        "landscaping" => WorkOrderCategory::Landscaping,
        "security" => WorkOrderCategory::Security,
        "appliance" => WorkOrderCategory::Appliance,
        "pest_control" => WorkOrderCategory::PestControl,
        _ => WorkOrderCategory::General,
    }
}

fn parse_reporter_type(s: &str) -> WorkOrderReporterType {
    match s {
        "resident" => WorkOrderReporterType::Resident,
        "caretaker" => WorkOrderReporterType::Caretaker,
        "owner" => WorkOrderReporterType::Owner,
        _ => WorkOrderReporterType::Staff,
    }
}
