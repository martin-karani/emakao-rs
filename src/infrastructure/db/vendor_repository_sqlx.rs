use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::vendor_repository::VendorRepository},
    domain::{
        maintenance::{WorkOrder, WorkOrderPriority, WorkOrderStatus},
        vendor::{
            CreateVendorCommand, UpdateVendorCommand, Vendor, VendorPortalStatus, VendorStatus,
        },
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

fn parse_status(s: &str) -> VendorStatus {
    match s {
        "inactive" => VendorStatus::Inactive,
        "blacklisted" => VendorStatus::Blacklisted,
        _ => VendorStatus::Active,
    }
}

fn status_str(s: &VendorStatus) -> &'static str {
    match s {
        VendorStatus::Active => "active",
        VendorStatus::Inactive => "inactive",
        VendorStatus::Blacklisted => "blacklisted",
    }
}

fn parse_portal_status(s: &str) -> VendorPortalStatus {
    match s {
        "active" => VendorPortalStatus::Active,
        "suspended" => VendorPortalStatus::Suspended,
        _ => VendorPortalStatus::Invited,
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
            status: parse_status(&r.status),
            portal_status: parse_portal_status(&r.portal_status),
            notes: r.notes,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[async_trait]
impl VendorRepository for PgVendorRepo {
    async fn find_all(
        &self,
        agency_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Vendor>, AppError> {
        let rows = sqlx::query_as!(
            VendorRow,
            r#"
            SELECT id, agency_id, user_id, name, contact_name, email, phone,
                   speciality, status, 'invited'::text as "portal_status!",
                   notes, created_at, updated_at
            FROM vendors
            WHERE agency_id = $1
            ORDER BY name ASC
            LIMIT $2 OFFSET $3
            "#,
            agency_id,
            limit,
            offset
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Vendor::from).collect())
    }

    async fn find_by_id(&self, agency_id: Uuid, id: Uuid) -> Result<Option<Vendor>, AppError> {
        let row = sqlx::query_as!(
            VendorRow,
            r#"
            SELECT id, agency_id, user_id, name, contact_name, email, phone,
                   speciality, status, 'invited'::text as "portal_status!",
                   notes, created_at, updated_at
            FROM vendors
            WHERE id = $1 AND agency_id = $2
            "#,
            id,
            agency_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Vendor::from))
    }

    async fn find_by_user_id(&self, user_id: Uuid) -> Result<Option<Vendor>, AppError> {
        let row = sqlx::query_as!(
            VendorRow,
            r#"
            SELECT id, agency_id, user_id, name, contact_name, email, phone,
                   speciality, status, 'invited'::text as "portal_status!",
                   notes, created_at, updated_at
            FROM vendors
            WHERE user_id = $1
            "#,
            user_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Vendor::from))
    }

    async fn create(&self, cmd: CreateVendorCommand) -> Result<Vendor, AppError> {
        let row = sqlx::query_as!(
            VendorRow,
            r#"
            INSERT INTO vendors (
                id, agency_id, user_id, name, contact_name, email,
                phone, speciality, notes
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING
                id, agency_id, user_id, name, contact_name, email,
                phone, speciality, status, 'invited'::text as "portal_status!",
                notes, created_at, updated_at
            "#,
            Uuid::new_v4(),
            cmd.agency_id,
            cmd.user_id,
            cmd.name,
            cmd.contact_name,
            cmd.email,
            cmd.phone,
            cmd.speciality,
            cmd.notes
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(Vendor::from(row))
    }

    async fn update(&self, cmd: UpdateVendorCommand) -> Result<Vendor, AppError> {
        let row = sqlx::query_as!(
            VendorRow,
            r#"
            UPDATE vendors
            SET
                name         = COALESCE($3, name),
                contact_name = COALESCE($4, contact_name),
                email        = COALESCE($5, email),
                phone        = COALESCE($6, phone),
                speciality   = COALESCE($7, speciality),
                status       = COALESCE($8, status),
                notes        = COALESCE($9, notes),
                updated_at   = now()
            WHERE id = $1 AND agency_id = $2
            RETURNING
                id, agency_id, user_id, name, contact_name, email,
                phone, speciality, status, 'invited'::text as "portal_status!",
                notes, created_at, updated_at
            "#,
            cmd.id,
            cmd.agency_id,
            cmd.name,
            cmd.contact_name,
            cmd.email,
            cmd.phone,
            cmd.speciality,
            cmd.status.as_ref().map(status_str),
            cmd.notes
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("vendor {}", cmd.id)))?;

        Ok(Vendor::from(row))
    }

    async fn find_work_orders_by_vendor_id(
        &self,
        vendor_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<WorkOrder>, AppError> {
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

        let rows = sqlx::query_as!(
            WorkOrderRow,
            r#"
            SELECT id, property_id, unit_id, title, description,
                   status, priority, vendor_id, reported_by,
                   created_at, updated_at
            FROM work_orders
            WHERE vendor_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
            vendor_id,
            limit,
            offset
        )
        .fetch_all(&self.pool)
        .await?;

        fn parse_status(s: &str) -> WorkOrderStatus {
            match s {
                "in_progress" => WorkOrderStatus::InProgress,
                "completed" => WorkOrderStatus::Completed,
                "cancelled" => WorkOrderStatus::Cancelled,
                _ => WorkOrderStatus::Open,
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

        rows.into_iter()
            .map(|r| {
                Ok(WorkOrder {
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
                })
            })
            .collect()
    }
}
