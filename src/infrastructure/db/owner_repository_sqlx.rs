use async_trait::async_trait;
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::owner_repository::OwnerRepository},
    domain::{
        disbursement::Disbursement,
        owner::{CreateOwnerCommand, Owner, OwnerPortalStatus, UpdateOwnerCommand},
        property::{Property, PropertyConfig, PropertyType},
    },
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
    email: Option<String>,
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
        _agency_id: Uuid,
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
            ORDER BY o.created_at DESC
            LIMIT $1 OFFSET $2
            "#,
            limit,
            offset
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Owner::from).collect())
    }

    async fn find_by_id(&self, _agency_id: Uuid, id: Uuid) -> Result<Option<Owner>, AppError> {
        let row = sqlx::query_as!(
            OwnerRow,
            r#"
            SELECT o.id, o.user_id, o.first_name, o.last_name, o.email,
                   o.phone, o.company_name, o.kra_pin, o.bank_name,
                   o.bank_account, o.mpesa_number, o.portal_status,
                   o.created_at, o.updated_at
            FROM owners o
            WHERE o.id = $1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Owner::from))
    }

    async fn find_by_user_id(&self, user_id: Uuid) -> Result<Option<Owner>, AppError> {
        let row = sqlx::query_as!(
            OwnerRow,
            r#"
            SELECT o.id, o.user_id, o.first_name, o.last_name, o.email,
                   o.phone, o.company_name, o.kra_pin, o.bank_name,
                   o.bank_account, o.mpesa_number, o.portal_status,
                   o.created_at, o.updated_at
            FROM owners o
            WHERE o.user_id = $1
            "#,
            user_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Owner::from))
    }

    async fn find_by_email(
        &self,
        _agency_id: Uuid,
        email: &str,
    ) -> Result<Option<Owner>, AppError> {
        let row = sqlx::query_as!(
            OwnerRow,
            r#"
            SELECT o.id, o.user_id, o.first_name, o.last_name, o.email,
                   o.phone, o.company_name, o.kra_pin, o.bank_name,
                   o.bank_account, o.mpesa_number, o.portal_status,
                   o.created_at, o.updated_at
            FROM owners o
            WHERE o.email = $1
            LIMIT 1
            "#,
            email
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Owner::from))
    }

    async fn create(&self, cmd: CreateOwnerCommand) -> Result<Owner, AppError> {
        let row = sqlx::query_as!(
            OwnerRow,
            r#"
        INSERT INTO owners (
            id, user_id, first_name, last_name, email, phone,
            company_name, kra_pin, bank_name, bank_account, mpesa_number,
            portal_status
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        RETURNING
            id, user_id, first_name, last_name, email, phone,
            company_name, kra_pin, bank_name, bank_account, mpesa_number,
            portal_status, created_at, updated_at
        "#,
            Uuid::new_v4(),
            cmd.user_id,
            cmd.first_name,
            cmd.last_name,
            cmd.email,
            cmd.phone,
            cmd.company_name,
            cmd.kra_pin,
            cmd.bank_name,
            cmd.bank_account,
            cmd.mpesa_number,
            "invited"
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(Owner::from(row))
    }

    async fn update(&self, cmd: UpdateOwnerCommand) -> Result<Owner, AppError> {
        let row = sqlx::query_as!(
            OwnerRow,
            r#"
            UPDATE owners
            SET
                first_name   = COALESCE($2, first_name),
                last_name    = COALESCE($3, last_name),
                phone        = COALESCE($4, phone),
                company_name = COALESCE($5, company_name),
                kra_pin      = COALESCE($6, kra_pin),
                bank_name    = COALESCE($7, bank_name),
                bank_account = COALESCE($8, bank_account),
                mpesa_number = COALESCE($9, mpesa_number),
                updated_at   = now()
            WHERE id = $1
            RETURNING
                id, user_id, first_name, last_name, email, phone,
                company_name, kra_pin, bank_name, bank_account, mpesa_number,
                portal_status, created_at, updated_at
            "#,
            cmd.id,
            cmd.first_name,
            cmd.last_name,
            cmd.phone,
            cmd.company_name,
            cmd.kra_pin,
            cmd.bank_name,
            cmd.bank_account,
            cmd.mpesa_number
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
            VALUES ($1, $2, $3)
            ON CONFLICT (property_id, owner_id)
            DO UPDATE SET ownership_percent = EXCLUDED.ownership_percent
            "#,
            property_id,
            owner_id,
            ownership_percent
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn find_properties_by_owner_id(
        &self,
        owner_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<(Property, Decimal)>, AppError> {
        struct PropertyRow {
            id: Uuid,
            agency_id: Uuid,
            name: String,
            address: String,
            city: String,
            country_code: String,
            property_type: String,
            config: sqlx::types::Json<serde_json::Value>,
            created_by: Uuid,
            created_at: time::OffsetDateTime,
            updated_at: time::OffsetDateTime,
            ownership_percent: Decimal,
        }

        let rows = sqlx::query_as!(
            PropertyRow,
            r#"
        SELECT
            p.id, p.agency_id, p.name, p.address, p.city, p.country_code,
            p.property_type, p.config as "config: sqlx::types::Json<serde_json::Value>",
            p.created_by, p.created_at, p.updated_at,
            po.ownership_percent as "ownership_percent!"
        FROM properties p
        JOIN property_owners po ON po.property_id = p.id
        WHERE po.owner_id = $1
        ORDER BY p.created_at DESC
        LIMIT $2 OFFSET $3
        "#,
            owner_id,
            limit,
            offset
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                let property_type: PropertyType =
                    serde_json::from_value(serde_json::Value::String(row.property_type))
                        .map_err(|e| AppError::ExternalService(e.to_string()))?;
                let config: PropertyConfig = serde_json::from_value(row.config.0)
                    .map_err(|e| AppError::ExternalService(e.to_string()))?;
                let property = Property {
                    id: row.id,
                    agency_id: row.agency_id,
                    name: row.name,
                    address: row.address,
                    city: row.city,
                    country_code: row.country_code,
                    property_type,
                    config,
                    created_by: row.created_by,
                    created_at: row.created_at,
                    updated_at: row.updated_at,
                };
                Ok((property, row.ownership_percent))
            })
            .collect()
    }

    async fn find_disbursements_by_owner_id(
        &self,
        owner_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Disbursement>, AppError> {
        struct DisbursementRow {
            id: Uuid,
            agency_id: Uuid,
            owner_id: Uuid,
            property_id: Uuid,
            amount_kes: Decimal,
            method: String,
            reference: Option<String>,
            status: String,
            period_start: time::Date,
            period_end: time::Date,
            notes: Option<String>,
            created_by: Uuid,
            created_at: time::OffsetDateTime,
            updated_at: time::OffsetDateTime,
        }

        let rows = sqlx::query_as!(
            DisbursementRow,
            r#"
        SELECT id, agency_id, owner_id, property_id, amount_kes,
               method, reference, status, period_start, period_end,
               notes, created_by, created_at, updated_at
        FROM disbursements
        WHERE owner_id = $1
        ORDER BY created_at DESC
        LIMIT $2 OFFSET $3
        "#,
            owner_id,
            limit,
            offset
        )
        .fetch_all(&self.pool)
        .await?;

        fn parse_method(s: &str) -> crate::domain::disbursement::DisbursementMethod {
            match s {
                "bank_transfer" => crate::domain::disbursement::DisbursementMethod::BankTransfer,
                "mpesa_b2c" => crate::domain::disbursement::DisbursementMethod::MpesaB2C,
                _ => crate::domain::disbursement::DisbursementMethod::Cheque,
            }
        }

        fn parse_status(s: &str) -> crate::domain::disbursement::DisbursementStatus {
            match s {
                "processing" => crate::domain::disbursement::DisbursementStatus::Processing,
                "completed" => crate::domain::disbursement::DisbursementStatus::Completed,
                "failed" => crate::domain::disbursement::DisbursementStatus::Failed,
                _ => crate::domain::disbursement::DisbursementStatus::Pending,
            }
        }

        rows.into_iter()
            .map(|r| {
                Ok(Disbursement {
                    id: r.id,
                    agency_id: r.agency_id,
                    owner_id: r.owner_id,
                    property_id: r.property_id,
                    amount_kes: r.amount_kes,
                    method: parse_method(&r.method),
                    reference: r.reference,
                    status: parse_status(&r.status),
                    period_start: r.period_start,
                    period_end: r.period_end,
                    notes: r.notes,
                    created_by: r.created_by,
                    created_at: r.created_at,
                    updated_at: r.updated_at,
                })
            })
            .collect()
    }
}
