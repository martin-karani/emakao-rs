use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        ports::agreement_repository::{
            AgreementRepository, CreateAgreementCommand, TaxAgreementView,
        },
    },
    domain::{
        agreement::Agreement,
        enums::{AgreementStatus, BillingFrequency},
    },
};

// ── Struct ────────────────────────────────────────────────────────────────────

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

// ── Row types ─────────────────────────────────────────────────────────────────

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

/// Row returned by `find_active_for_agency` — joins through to properties.
#[derive(sqlx::FromRow)]
struct TaxAgreementRow {
    id: Uuid,
    property_id: Uuid,
    /// Resolved from `properties.owner_id`
    owner_id: Uuid,
    /// Resolved from `owners.kra_pin` (nullable — owner may not have filed it yet)
    owner_kra_pin: Option<String>,
    rent_amount_kes: rust_decimal::Decimal,
    billing_frequency: String,
    start_date: time::Date,
    end_date: Option<time::Date>,
}

// ── Helpers ───────────────────────────────────────────────────────────────────

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
        "daily" => BillingFrequency::Daily,
        "weekly" => BillingFrequency::Weekly,
        "one_time" => BillingFrequency::OneTime,
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

impl From<TaxAgreementRow> for TaxAgreementView {
    fn from(r: TaxAgreementRow) -> Self {
        Self {
            id: r.id,
            property_id: r.property_id,
            owner_id: r.owner_id,
            owner_kra_pin: r.owner_kra_pin,
            rent_amount_kes: r.rent_amount_kes,
            billing_frequency: parse_billing_frequency(&r.billing_frequency),
            start_date: r.start_date,
            end_date: r.end_date,
        }
    }
}

// ── Impl ──────────────────────────────────────────────────────────────────────

#[async_trait]
impl AgreementRepository for PgAgreementRepo {
    async fn find_all(
        &self,
        agency_id: Uuid,
        property_id: Option<Uuid>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Agreement>, AppError> {
        let rows = sqlx::query_as::<_, AgreementRow>(
            r#"
            SELECT a.id, a.property_id, a.unit_id, a.resident_id,
                   a.start_date, a.end_date, a.rent_amount_kes, a.deposit_kes,
                   a.billing_frequency::text, a.status::text, a.created_at, a.updated_at
            FROM agreements a
            JOIN properties p ON p.id = a.property_id
            WHERE p.agency_id = $1
              AND ($2::uuid IS NULL OR a.property_id = $2)
            ORDER BY a.created_at DESC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(agency_id)
        .bind(property_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::InternalServer(e.to_string()))?;

        Ok(rows.into_iter().map(Agreement::from).collect())
    }

    async fn find_by_id(&self, agency_id: Uuid, id: Uuid) -> Result<Option<Agreement>, AppError> {
        let row = sqlx::query_as::<_, AgreementRow>(
            r#"
            SELECT a.id, a.property_id, a.unit_id, a.resident_id,
                   a.start_date, a.end_date, a.rent_amount_kes, a.deposit_kes,
                   a.billing_frequency::text, a.status::text, a.created_at, a.updated_at
            FROM agreements a
            JOIN properties p ON p.id = a.property_id
            WHERE a.id = $1 AND p.agency_id = $2
            "#,
        )
        .bind(id)
        .bind(agency_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::InternalServer(e.to_string()))?;

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

        let row = sqlx::query_as::<_, AgreementRow>(
            r#"
            INSERT INTO agreements (
                id, property_id, unit_id, resident_id,
                start_date, end_date, rent_amount_kes, deposit_kes,
                billing_frequency, status
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9::billing_frequency, 'active'::agreement_status)
            RETURNING
                id, property_id, unit_id, resident_id,
                start_date, end_date, rent_amount_kes, deposit_kes,
                billing_frequency::text, status::text, created_at, updated_at
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(cmd.property_id)
        .bind(cmd.unit_id)
        .bind(cmd.resident_id)
        .bind(cmd.start_date)
        .bind(cmd.end_date)
        .bind(cmd.rent_amount_kes)
        .bind(cmd.deposit_kes)
        .bind(billing_str)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::InternalServer(e.to_string()))?;

        Ok(Agreement::from(row))
    }

    async fn update_status(
        &self,
        id: Uuid,
        status: AgreementStatus,
        _updated_by: Uuid,
    ) -> Result<(), AppError> {
        let result = sqlx::query(
            r#"
            UPDATE agreements
            SET status = $2::agreement_status, updated_at = now()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(status_str(&status))
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::InternalServer(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("agreement {id}")));
        }
        Ok(())
    }

    async fn has_active_agreement(&self, unit_id: Uuid) -> Result<bool, AppError> {
        let row = sqlx::query(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM agreements
                WHERE unit_id = $1 AND status = 'active'
            ) AS exists
            "#,
        )
        .bind(unit_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::InternalServer(e.to_string()))?;

        use sqlx::Row;
        Ok(row.get::<Option<bool>, _>("exists").unwrap_or(false))
    }

    // ── NEW ───────────────────────────────────────────────────────────────────

    async fn find_active_for_agency(
        &self,
        agency_id: Uuid,
    ) -> Result<Vec<TaxAgreementView>, AppError> {
        // Joins through units → properties to resolve owner_id.
        // Left-joins owners to get kra_pin (owner may not have one on file yet).
        let rows = sqlx::query_as::<_, TaxAgreementRow>(
            r#"
            SELECT
                a.id,
                a.property_id,
                p.owner_id,
                o.kra_pin          AS owner_kra_pin,
                a.rent_amount_kes,
                a.billing_frequency::text,
                a.start_date,
                a.end_date
            FROM agreements a
            JOIN units      u ON u.id         = a.unit_id
            JOIN properties p ON p.id         = u.property_id
            LEFT JOIN owners o ON o.id         = p.owner_id
            WHERE p.agency_id = $1
              AND a.status    = 'active'
            ORDER BY a.id
            "#,
        )
        .bind(agency_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::InternalServer(format!("find_active_for_agency: {e}")))?;

        Ok(rows.into_iter().map(TaxAgreementView::from).collect())
    }

    async fn get_deposit_kes(&self, agreement_id: Uuid) -> Result<rust_decimal::Decimal, AppError> {
        let row = sqlx::query_as::<_, (rust_decimal::Decimal,)>(
            "SELECT deposit_kes FROM agreements WHERE id = $1",
        )
        .bind(agreement_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::InternalServer(format!("get_deposit_kes: {e}")))?
        .ok_or_else(|| AppError::NotFound(format!("agreement {agreement_id}")))?;

        Ok(row.0)
    }
}
