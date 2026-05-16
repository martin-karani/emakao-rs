// src/infrastructure/db/billing_repository_sqlx.rs

use async_trait::async_trait;
use rust_decimal::Decimal;
use sqlx::PgPool;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        ports::billing_repository::{BillingRepository, LateFeePolicy},
    },
    domain::agreement::ActiveAgreementBillingView,
};

pub struct PgBillingRepo {
    pool: PgPool,
}

impl PgBillingRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl BillingRepository for PgBillingRepo {
    async fn find_active_agreements_for_agency(
        &self,
        _agency_id: Uuid,
    ) -> Result<Vec<ActiveAgreementBillingView>, AppError> {
        // agency_id is already scoped by the per-agency connection pool —
        // we just query the local schema.
        let rows = sqlx::query_as!(
            ActiveAgreementBillingView,
            r#"
            SELECT
                a.id              AS agreement_id,
                a.unit_id,
                a.resident_id,
                a.rent_amount_kes,
                a.billing_day,
                a.billing_frequency AS "billing_frequency: String",
                a.start_date,
                a.end_date,
                r.email           AS resident_email,
                r.phone           AS resident_phone,
                r.full_name       AS resident_name
            FROM agreements a
            JOIN residents r ON r.id = a.resident_id
            WHERE a.status = 'active'
              AND (a.end_date IS NULL OR a.end_date >= CURRENT_DATE)
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    async fn rent_charge_exists(
        &self,
        agreement_id: Uuid,
        period_start: Date,
    ) -> Result<bool, AppError> {
        let row = sqlx::query!(
            "SELECT 1 AS exists FROM rent_charges WHERE agreement_id = $1 AND period_start = $2",
            agreement_id,
            period_start
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.is_some())
    }

    async fn record_rent_charge(
        &self,
        agreement_id: Uuid,
        period_start: Date,
        period_end: Date,
        amount_kes: Decimal,
    ) -> Result<(), AppError> {
        // ON CONFLICT DO NOTHING handles scheduler retries without error
        sqlx::query!(
            r#"
            INSERT INTO rent_charges (id, agreement_id, period_start, period_end, amount_kes)
            VALUES (uuidv7(), $1, $2, $3, $4)
            ON CONFLICT (agreement_id, period_start) DO NOTHING
            "#,
            agreement_id,
            period_start,
            period_end,
            amount_kes
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn late_fee_exists(
        &self,
        agreement_id: Uuid,
        period_start: Date,
    ) -> Result<bool, AppError> {
        let row = sqlx::query!(
            "SELECT 1 AS e FROM late_fee_charges WHERE agreement_id = $1 AND period_start = $2",
            agreement_id,
            period_start
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.is_some())
    }

    async fn record_late_fee(
        &self,
        agreement_id: Uuid,
        period_start: Date,
        amount_kes: Decimal,
    ) -> Result<(), AppError> {
        sqlx::query!(
            r#"
            INSERT INTO late_fee_charges (id, agreement_id, period_start, amount_kes)
            VALUES (uuidv7(), $1, $2, $3)
            ON CONFLICT (agreement_id, period_start) DO NOTHING
            "#,
            agreement_id,
            period_start,
            amount_kes
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn paid_amount_since(
        &self,
        agreement_id: Uuid,
        since: OffsetDateTime,
    ) -> Result<Decimal, AppError> {
        let row = sqlx::query!(
            r#"
            SELECT COALESCE(SUM(amount_kes), 0) AS total
            FROM ledger_entries
            WHERE agreement_id = $1
              AND entry_type   = 'payment'
              AND created_at  >= $2
            "#,
            agreement_id,
            since
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(row.total.unwrap_or(Decimal::ZERO))
    }

    async fn reminder_sent(
        &self,
        agreement_id: Uuid,
        period_start: Date,
        channel: &str,
    ) -> Result<bool, AppError> {
        let row = sqlx::query!(
            r#"
            SELECT 1 AS e
            FROM rent_reminders_sent
            WHERE agreement_id = $1 AND period_start = $2 AND channel = $3
            "#,
            agreement_id,
            period_start,
            channel
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.is_some())
    }

    async fn record_reminder_sent(
        &self,
        agreement_id: Uuid,
        period_start: Date,
        channel: &str,
    ) -> Result<(), AppError> {
        sqlx::query!(
            r#"
            INSERT INTO rent_reminders_sent (id, agreement_id, period_start, channel)
            VALUES (uuidv7(), $1, $2, $3)
            ON CONFLICT (agreement_id, period_start, channel) DO NOTHING
            "#,
            agreement_id,
            period_start,
            channel
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn load_late_fee_policy(&self) -> Result<LateFeePolicy, AppError> {
        let row = sqlx::query!(
            r#"
            SELECT grace_period_days, flat_amount_kes, rate_percent
            FROM late_fee_policy
            ORDER BY created_at ASC
            LIMIT 1
            "#
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(match row {
            Some(r) => LateFeePolicy {
                grace_period_days: r.grace_period_days,
                flat_amount_kes: r.flat_amount_kes,
                rate_percent: r.rate_percent,
            },
            None => LateFeePolicy {
                grace_period_days: 5,
                flat_amount_kes: None,
                rate_percent: Some(Decimal::from(5)),
            },
        })
    }
}
