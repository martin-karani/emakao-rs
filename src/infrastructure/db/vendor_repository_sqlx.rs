use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::vendor_repository::VendorRepository},
    domain::vendor::{CreateVendorCommand, UpdateVendorCommand, Vendor, VendorStatus},
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
    name: String,
    contact_name: Option<String>,
    email: Option<String>,
    phone: Option<String>,
    speciality: Option<String>,
    status: String,
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

impl From<VendorRow> for Vendor {
    fn from(r: VendorRow) -> Self {
        Self {
            id: r.id,
            agency_id: r.agency_id,
            name: r.name,
            contact_name: r.contact_name,
            email: r.email,
            phone: r.phone,
            speciality: r.speciality,
            status: parse_status(&r.status),
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
            SELECT id, agency_id, name, contact_name, email, phone,
                   speciality, status, notes, created_at, updated_at
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
            SELECT id, agency_id, name, contact_name, email, phone,
                   speciality, status, notes, created_at, updated_at
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

    async fn create(&self, cmd: CreateVendorCommand) -> Result<Vendor, AppError> {
        let row = sqlx::query_as!(
            VendorRow,
            r#"
            INSERT INTO vendors (
                id, agency_id, name, contact_name, email,
                phone, speciality, notes
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING
                id, agency_id, name, contact_name, email,
                phone, speciality, status, notes, created_at, updated_at
            "#,
            Uuid::new_v4(),
            cmd.agency_id,
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
                id, agency_id, name, contact_name, email,
                phone, speciality, status, notes, created_at, updated_at
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
}
