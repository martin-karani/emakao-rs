use async_trait::async_trait;
use rust_decimal::Decimal;
use sqlx::{PgPool, Row};
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::billing_repository::BillingRepository},
    domain::{
        billing::{ActiveAgreementBillingView, LateFeePolicy},
        enums::BillingFrequency,
    },
};

pub struct PgBillingRepo {
    pool: PgPool,
    agency_id: Uuid,
    schema_name: String,
}

impl PgBillingRepo {
    pub fn new(pool: PgPool, agency_id: Uuid, schema_name: String) -> Self {
        Self {
            pool,
            agency_id,
            schema_name,
        }
    }

    /// Convenience constructor that derives `schema_name` from `agency_id`.
    /// Schema name format: `agency_<uuid_simple>` — must match the convention
    /// used by `AgencyPoolManager` when provisioning per-tenant schemas.
    pub fn for_agency(pool: PgPool, agency_id: Uuid) -> Self {
        let schema_name = format!("agency_{}", agency_id.simple());
        Self::new(pool, agency_id, schema_name)
    }
}

#[async_trait]
impl BillingRepository for PgBillingRepo {
    async fn find_active_agreements_for_agency(
        &self,
        _agency_id: Uuid,
    ) -> Result<Vec<ActiveAgreementBillingView>, AppError> {
        // Per-agency pool already scopes to the right schema.
        // query_as! is skipped: multi-table JOIN over per-tenant schemas
        // cannot be verified at compile time.
        let rows = sqlx::query(
            r#"
            SELECT
                a.id                                   AS agreement_id,
                a.unit_id,
                a.resident_id,
                a.rent_amount_kes,
                a.billing_frequency::text,
                a.start_date,
                a.end_date,
                r.email                                AS resident_email,
                r.phone                                AS resident_phone,
                (r.first_name || ' ' || r.last_name)   AS resident_name,
                u.unit_number
            FROM agreements a
            JOIN residents r ON r.id = a.resident_id
            JOIN units     u ON u.id = a.unit_id
            WHERE a.status::text = 'active'
              AND (a.end_date IS NULL OR a.end_date >= CURRENT_DATE)
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::InternalServer(e.to_string()))?;

        let agency_id = self.agency_id;
        let schema_name = self.schema_name.clone();

        let views = rows
            .into_iter()
            .map(|r| {
                let freq_str: String = r.get("billing_frequency");
                let billing_frequency = match freq_str.as_str() {
                    "daily" => BillingFrequency::Daily,
                    "weekly" => BillingFrequency::Weekly,
                    "quarterly" => BillingFrequency::Quarterly,
                    "semi_annual" => BillingFrequency::SemiAnnual,
                    "annual" => BillingFrequency::Annual,
                    "one_time" => BillingFrequency::OneTime,
                    _ => BillingFrequency::Monthly,
                };

                ActiveAgreementBillingView {
                    agreement_id: r.get("agreement_id"),
                    agency_id,
                    schema_name: schema_name.clone(),
                    unit_id: r.get("unit_id"),
                    resident_id: r.get("resident_id"),
                    rent_amount_kes: r.get("rent_amount_kes"),
                    billing_frequency,
                    start_date: r.get("start_date"),
                    end_date: r.get("end_date"),
                    resident_email: r.get("resident_email"),
                    resident_phone: r.get("resident_phone"),
                    resident_name: r.get("resident_name"),
                    unit_number: r.get("unit_number"),
                }
            })
            .collect();

        Ok(views)
    }

    async fn rent_charge_exists(
        &self,
        agreement_id: Uuid,
        period_start: Date,
    ) -> Result<bool, AppError> {
        let row =
            sqlx::query("SELECT 1 FROM rent_charges WHERE agreement_id = $1 AND period_start = $2")
                .bind(agreement_id)
                .bind(period_start)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| AppError::InternalServer(e.to_string()))?;

        Ok(row.is_some())
    }

    async fn record_rent_charge(
        &self,
        agreement_id: Uuid,
        period_start: Date,
        period_end: Date,
        amount_kes: Decimal,
    ) -> Result<(), AppError> {
        sqlx::query(
            r#"
            INSERT INTO rent_charges (id, agreement_id, period_start, period_end, amount_kes)
            VALUES (uuidv7(), $1, $2, $3, $4)
            ON CONFLICT (agreement_id, period_start) DO NOTHING
            "#,
        )
        .bind(agreement_id)
        .bind(period_start)
        .bind(period_end)
        .bind(amount_kes)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::InternalServer(e.to_string()))?;

        Ok(())
    }

    async fn load_late_fee_policy(&self) -> Result<LateFeePolicy, AppError> {
        let row = sqlx::query(
            r#"
            SELECT grace_period_days, flat_amount_kes, rate_percent
            FROM late_fee_policy
            ORDER BY created_at ASC
            LIMIT 1
            "#,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::InternalServer(e.to_string()))?;

        Ok(match row {
            Some(r) => LateFeePolicy {
                grace_period_days: r.get("grace_period_days"),
                flat_amount_kes: r.get("flat_amount_kes"),
                rate_percent: r.get("rate_percent"),
            },
            None => LateFeePolicy {
                grace_period_days: 5,
                flat_amount_kes: None,
                rate_percent: Some(Decimal::from(5)),
            },
        })
    }

    async fn late_fee_exists(
        &self,
        agreement_id: Uuid,
        period_start: Date,
    ) -> Result<bool, AppError> {
        let row = sqlx::query(
            "SELECT 1 FROM late_fee_charges WHERE agreement_id = $1 AND period_start = $2",
        )
        .bind(agreement_id)
        .bind(period_start)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::InternalServer(e.to_string()))?;

        Ok(row.is_some())
    }

    async fn record_late_fee(
        &self,
        agreement_id: Uuid,
        period_start: Date,
        amount_kes: Decimal,
    ) -> Result<(), AppError> {
        sqlx::query(
            r#"
            INSERT INTO late_fee_charges (id, agreement_id, period_start, amount_kes)
            VALUES (uuidv7(), $1, $2, $3)
            ON CONFLICT (agreement_id, period_start) DO NOTHING
            "#,
        )
        .bind(agreement_id)
        .bind(period_start)
        .bind(amount_kes)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::InternalServer(e.to_string()))?;

        Ok(())
    }

    async fn paid_amount_since(
        &self,
        agreement_id: Uuid,
        since: OffsetDateTime,
    ) -> Result<Decimal, AppError> {
        let row = sqlx::query(
            r#"
            SELECT COALESCE(SUM(amount_kes), 0) AS total
            FROM ledger_entries
            WHERE agreement_id = $1
              AND entry_type::text   = 'payment'
              AND created_at  >= $2
            "#,
        )
        .bind(agreement_id)
        .bind(since)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::InternalServer(e.to_string()))?;

        Ok(row
            .get::<Option<Decimal>, _>("total")
            .unwrap_or(Decimal::ZERO))
    }

    async fn reminder_sent(
        &self,
        agreement_id: Uuid,
        period_start: Date,
        channel: &str,
    ) -> Result<bool, AppError> {
        let row = sqlx::query(
            r#"
            SELECT 1
            FROM rent_reminders_sent
            WHERE agreement_id = $1
              AND period_start  = $2
              AND channel       = $3
            "#,
        )
        .bind(agreement_id)
        .bind(period_start)
        .bind(channel)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::InternalServer(e.to_string()))?;

        Ok(row.is_some())
    }

    async fn record_reminder_sent(
        &self,
        agreement_id: Uuid,
        period_start: Date,
        channel: &str,
    ) -> Result<(), AppError> {
        sqlx::query(
            r#"
            INSERT INTO rent_reminders_sent (id, agreement_id, period_start, channel)
            VALUES (uuidv7(), $1, $2, $3)
            ON CONFLICT (agreement_id, period_start, channel) DO NOTHING
            "#,
        )
        .bind(agreement_id)
        .bind(period_start)
        .bind(channel)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::InternalServer(e.to_string()))?;

        Ok(())
    }
}
