use async_trait::async_trait;
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::owner_repository::OwnerRepository},
    domain::owner::{CreateOwnerCommand, Owner, OwnerPortalStatus, UpdateOwnerCommand},
};

pub struct PgOwnerRepo {
    pool: PgPool,
}

impl PgOwnerRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl From<PgPool> for PgOwnerRepo {
    fn from(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct OwnerRow {
    id: Uuid,
    user_id: Option<Uuid>,
    first_name: String,
    last_name: String,
    email: String,
    phone: Option<String>,
    company_name: Option<String>,
    kra_pin: Option<String>,
    bank_name: Option<String>,
    bank_account: Option<String>,
    mpesa_number: Option<String>,
    portal_status: String,
    created_at: time::OffsetDateTime,
    updated_at: time::OffsetDateTime,
}

fn parse_portal_status(s: &str) -> OwnerPortalStatus {
    match s {
        "active" => OwnerPortalStatus::Active,
        "suspended" => OwnerPortalStatus::Suspended,
        _ => OwnerPortalStatus::Invited,
    }
}

impl From<OwnerRow> for Owner {
    fn from(r: OwnerRow) -> Self {
        Self {
            id: r.id,
            user_id: r.user_id,
            first_name: r.first_name,
            last_name: r.last_name,
            email: r.email,
            phone: r.phone,
            company_name: r.company_name,
            kra_pin: r.kra_pin,
            bank_name: r.bank_name,
            bank_account: r.bank_account,
            mpesa_number: r.mpesa_number,
            portal_status: parse_portal_status(&r.portal_status),
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[async_trait]
impl OwnerRepository for PgOwnerRepo {
    async fn find_all(
        &self,
        agency_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Owner>, AppError> {
        let rows = sqlx::query_as!(
            OwnerRow,
            r#"
            SELECT o.id, o.user_id, o.first_name, o.last_name, o.email,
                   o.phone, o.company_name, o.kra_pin, o.bank_name,
                   o.bank_account, o.mpesa_number, o.portal_status,
                   o.created_at, o.updated_at
            FROM owners o
            JOIN users u ON u.id = o.user_id
            WHERE u.agency_id = $1::uuid
            ORDER BY o.created_at DESC
            LIMIT $2 OFFSET $3
            "#,
            agency_id,
            limit,
            offset
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Owner::from).collect())
    }

    async fn find_by_id(&self, agency_id: Uuid, id: Uuid) -> Result<Option<Owner>, AppError> {
        let row = sqlx::query_as!(
            OwnerRow,
            r#"
            SELECT o.id, o.user_id, o.first_name, o.last_name, o.email,
                   o.phone, o.company_name, o.kra_pin, o.bank_name,
                   o.bank_account, o.mpesa_number, o.portal_status,
                   o.created_at, o.updated_at
            FROM owners o
            JOIN users u ON u.id = o.user_id
            WHERE o.id = $1::uuid AND u.agency_id = $2::uuid
            "#,
            id,
            agency_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Owner::from))
    }

    async fn find_by_email(&self, agency_id: Uuid, email: &str) -> Result<Option<Owner>, AppError> {
        let row = sqlx::query_as!(
            OwnerRow,
            r#"
            SELECT o.id, o.user_id, o.first_name, o.last_name, o.email,
                   o.phone, o.company_name, o.kra_pin, o.bank_name,
                   o.bank_account, o.mpesa_number, o.portal_status,
                   o.created_at, o.updated_at
            FROM owners o
            JOIN users u ON u.id = o.user_id
            WHERE o.email = $1::text AND u.agency_id = $2::uuid
            LIMIT 1
            "#,
            email,
            agency_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Owner::from))
    }

    async fn create(&self, cmd: CreateOwnerCommand) -> Result<Owner, AppError> {
        let mut tx = self.pool.begin().await?;

        let user_id = Uuid::new_v4();
        sqlx::query!(
            r#"
            INSERT INTO users (id, agency_id, email, password_hash, role, is_active)
            VALUES ($1::uuid, $2::uuid, $3::text, '', 'owner', false)
            ON CONFLICT (agency_id, email) DO NOTHING
            "#,
            user_id,
            cmd.agency_id,
            cmd.email
        )
        .execute(&mut *tx)
        .await?;

        let real_user_id: Uuid = sqlx::query_scalar!(
            "SELECT id FROM users WHERE agency_id = $1::uuid AND email = $2::text",
            cmd.agency_id,
            cmd.email
        )
        .fetch_one(&mut *tx)
        .await?;

        let row = sqlx::query_as!(
            OwnerRow,
            r#"
            INSERT INTO owners (
                id, user_id, first_name, last_name, email, phone,
                company_name, kra_pin, bank_name, bank_account, mpesa_number
            )
            VALUES (
                $1::uuid, $2::uuid, $3::text, $4::text, $5::text, $6::text,
                $7::text, $8::text, $9::text, $10::text, $11::text
            )
            RETURNING
                id, user_id, first_name, last_name, email, phone,
                company_name, kra_pin, bank_name, bank_account, mpesa_number,
                portal_status, created_at, updated_at
            "#,
            Uuid::new_v4(),
            real_user_id,
            cmd.first_name,
            cmd.last_name,
            cmd.email,
            cmd.phone,
            cmd.company_name,
            cmd.kra_pin,
            cmd.bank_name,
            cmd.bank_account,
            cmd.mpesa_number
        )
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(Owner::from(row))
    }

    async fn update(&self, cmd: UpdateOwnerCommand) -> Result<Owner, AppError> {
        // $1 = id, $2 = agency_id (used in WHERE subquery so Postgres sees it),
        // $3..$10 = optional fields. Every $N is referenced — no gaps.
        let row = sqlx::query_as!(
            OwnerRow,
            r#"
            UPDATE owners
            SET
                first_name   = COALESCE($3::text,  first_name),
                last_name    = COALESCE($4::text,  last_name),
                phone        = COALESCE($5::text,  phone),
                company_name = COALESCE($6::text,  company_name),
                kra_pin      = COALESCE($7::text,  kra_pin),
                bank_name    = COALESCE($8::text,  bank_name),
                bank_account = COALESCE($9::text,  bank_account),
                mpesa_number = COALESCE($10::text, mpesa_number),
                updated_at   = now()
            WHERE id = $1::uuid
              AND user_id IN (
                  SELECT id FROM users WHERE agency_id = $2::uuid
              )
            RETURNING
                id, user_id, first_name, last_name, email, phone,
                company_name, kra_pin, bank_name, bank_account, mpesa_number,
                portal_status, created_at, updated_at
            "#,
            cmd.id,           // $1
            cmd.agency_id,    // $2 — now referenced in WHERE, type is known
            cmd.first_name,   // $3
            cmd.last_name,    // $4
            cmd.phone,        // $5
            cmd.company_name, // $6
            cmd.kra_pin,      // $7
            cmd.bank_name,    // $8
            cmd.bank_account, // $9
            cmd.mpesa_number, // $10
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("owner {}", cmd.id)))?;

        Ok(Owner::from(row))
    }

    async fn assign_to_property(
        &self,
        owner_id: Uuid,
        property_id: Uuid,
        ownership_percent: Decimal,
    ) -> Result<(), AppError> {
        sqlx::query!(
            r#"
            INSERT INTO property_owners (property_id, owner_id, ownership_percent)
            VALUES ($1::uuid, $2::uuid, $3)
            ON CONFLICT (property_id, owner_id)
            DO UPDATE SET ownership_percent = EXCLUDED.ownership_percent
            "#,
            property_id,
            owner_id,
            ownership_percent // Decimal binds directly to NUMERIC — no cast needed
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
