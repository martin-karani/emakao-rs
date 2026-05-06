use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        ports::agreement_repository::{AgreementRepository, CreateAgreementCommand},
    },
    domain::{
        agreement::Agreement,
        enums::{AgreementStatus, BillingFrequency},
    },
};

pub struct PgAgreementRepo {
    pool: PgPool,
}

impl PgAgreementRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl From<PgPool> for PgAgreementRepo {
    fn from(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct AgreementRow {
    id: Uuid,
    property_id: Uuid,
    unit_id: Uuid,
    resident_id: Uuid,
    start_date: time::Date,
    end_date: Option<time::Date>,
    rent_amount_kes: rust_decimal::Decimal,
    deposit_kes: rust_decimal::Decimal,
    billing_frequency: String,
    status: String,
    created_at: time::OffsetDateTime,
    updated_at: time::OffsetDateTime,
}

fn parse_status(s: &str) -> AgreementStatus {
    match s {
        "expired" => AgreementStatus::Expired,
        "terminated" => AgreementStatus::Terminated,
        "pending_renewal" => AgreementStatus::PendingRenewal,
        "renewed" => AgreementStatus::Renewed,
        _ => AgreementStatus::Active,
    }
}

fn parse_billing_frequency(s: &str) -> BillingFrequency {
    match s {
        "quarterly" => BillingFrequency::Quarterly,
        "semi_annual" => BillingFrequency::SemiAnnual,
        "annual" => BillingFrequency::Annual,
        _ => BillingFrequency::Monthly,
    }
}

fn status_str(s: &AgreementStatus) -> &'static str {
    match s {
        AgreementStatus::Active => "active",
        AgreementStatus::Expired => "expired",
        AgreementStatus::Renewed => "renewed",
        AgreementStatus::Draft => "draft",
        AgreementStatus::PendingSignature => "pending_signature",
        AgreementStatus::Terminated => "terminated",
        AgreementStatus::PendingRenewal => "pending_renewal",
    }
}

impl From<AgreementRow> for Agreement {
    fn from(r: AgreementRow) -> Self {
        Self {
            id: r.id,
            property_id: r.property_id,
            unit_id: r.unit_id,
            resident_id: r.resident_id,
            start_date: r.start_date,
            end_date: r.end_date,
            rent_amount_kes: r.rent_amount_kes,
            deposit_kes: r.deposit_kes,
            billing_frequency: parse_billing_frequency(&r.billing_frequency),
            status: parse_status(&r.status),
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[async_trait]
impl AgreementRepository for PgAgreementRepo {
    async fn find_all(
        &self,
        agency_id: Uuid,
        property_id: Option<Uuid>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Agreement>, AppError> {
        let rows = sqlx::query_as!(
            AgreementRow,
            r#"
            SELECT a.id, a.property_id, a.unit_id, a.resident_id,
                   a.start_date, a.end_date, a.rent_amount_kes, a.deposit_kes,
                   a.billing_frequency, a.status, a.created_at, a.updated_at
            FROM agreements a
            JOIN properties p ON p.id = a.property_id
            WHERE p.agency_id = $1
              AND ($2::uuid IS NULL OR a.property_id = $2)
            ORDER BY a.created_at DESC
            LIMIT $3 OFFSET $4
            "#,
            agency_id,
            property_id,
            limit,
            offset
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Agreement::from).collect())
    }

    async fn find_by_id(&self, agency_id: Uuid, id: Uuid) -> Result<Option<Agreement>, AppError> {
        let row = sqlx::query_as!(
            AgreementRow,
            r#"
            SELECT a.id, a.property_id, a.unit_id, a.resident_id,
                   a.start_date, a.end_date, a.rent_amount_kes, a.deposit_kes,
                   a.billing_frequency, a.status, a.created_at, a.updated_at
            FROM agreements a
            JOIN properties p ON p.id = a.property_id
            WHERE a.id = $1 AND p.agency_id = $2
            "#,
            id,
            agency_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Agreement::from))
    }

    async fn create(&self, cmd: CreateAgreementCommand) -> Result<Agreement, AppError> {
        let billing_str = match cmd.billing_frequency {
            BillingFrequency::Monthly => "monthly",
            BillingFrequency::Quarterly => "quarterly",
            BillingFrequency::SemiAnnual => "semi_annual",
            BillingFrequency::Annual => "annual",
            BillingFrequency::Daily => "daily",
            BillingFrequency::Weekly => "weekly",
            BillingFrequency::OneTime => "one_time",
        };

        let row = sqlx::query_as!(
            AgreementRow,
            r#"
            INSERT INTO agreements (
                id, property_id, unit_id, resident_id,
                start_date, end_date, rent_amount_kes, deposit_kes,
                billing_frequency, status
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'active')
            RETURNING
                id, property_id, unit_id, resident_id,
                start_date, end_date, rent_amount_kes, deposit_kes,
                billing_frequency, status, created_at, updated_at
            "#,
            Uuid::new_v4(),
            cmd.property_id,
            cmd.unit_id,
            cmd.resident_id,
            cmd.start_date,
            cmd.end_date,
            cmd.rent_amount_kes,
            cmd.deposit_kes,
            billing_str
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(Agreement::from(row))
    }

    async fn update_status(
        &self,
        id: Uuid,
        status: AgreementStatus,
        _updated_by: Uuid,
    ) -> Result<(), AppError> {
        let result = sqlx::query!(
            r#"
            UPDATE agreements
            SET status = $2, updated_at = now()
            WHERE id = $1
            "#,
            id,
            status_str(&status)
        )
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("agreement {id}")));
        }
        Ok(())
    }

    async fn has_active_agreement(&self, unit_id: Uuid) -> Result<bool, AppError> {
        let row = sqlx::query!(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM agreements
                WHERE unit_id = $1 AND status = 'active'
            ) AS exists
            "#,
            unit_id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(row.exists.unwrap_or(false))
    }
}
