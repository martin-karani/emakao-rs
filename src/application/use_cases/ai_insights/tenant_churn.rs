// src/application/use_cases/ai_insights/tenant_churn.rs
//
// Tenant churn prediction — surfaces leases expiring soon and scores
// how likely the tenant is NOT to renew.
//
// Churn score model (100 pts):
//   - Days until expiry           (<30d=30, 30-60d=20, 60-90d=10, >90d=0)
//   - Late payment count 12m      (0=0, 1=10, 2+=20)
//   - Outstanding balance > 0     (+15 pts)
//   - Prior agreements for tenant (only 1 = first-timer risk = +10)
//   - No payment in >45 days      (+15 pts)
//   - Lease is open-ended         (no end date = low churn = 0 bonus)

use std::sync::Arc;

use rust_decimal::Decimal;
use sqlx::PgPool;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::{
    application::errors::AppError,
    domain::ai_insights::{RiskLevel, TenantChurnPrediction},
};

pub struct TenantChurnUseCase {
    pub pool: Arc<PgPool>,
}

#[derive(sqlx::FromRow)]
struct ChurnRow {
    agreement_id: Uuid,
    resident_id: Uuid,
    resident_name: String,
    unit_id: Uuid,
    unit_number: String,
    property_id: Uuid,
    property_name: String,
    end_date: Option<Date>,
    days_until_expiry: Option<i64>,
    outstanding_kes: Decimal,
    late_payments_12m: i64,
    prior_agreements: i64,
    days_since_last_payment: Option<i64>,
}

impl TenantChurnUseCase {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Return churn predictions for leases expiring within `horizon_days`.
    pub async fn execute(
        &self,
        agency_id: Uuid,
        horizon_days: Option<i64>,
    ) -> Result<Vec<TenantChurnPrediction>, AppError> {
        let horizon = horizon_days.unwrap_or(90);
        let now = OffsetDateTime::now_utc();

        let rows = sqlx::query_as!(
            ChurnRow,
            r#"
            SELECT
                a.id   AS agreement_id,
                r.id   AS resident_id,
                (r.first_name || ' ' || r.last_name) AS resident_name,
                u.id   AS unit_id,
                u.unit_number,
                p.id   AS property_id,
                p.name AS property_name,
                a.end_date,
                (a.end_date - CURRENT_DATE)::BIGINT AS days_until_expiry,

                COALESCE(
                    (SELECT SUM(amount_kes) FROM ledger_entries
                     WHERE agreement_id = a.id AND entry_type IN ('rent_charge','late_fee','deposit_charge'))
                    -
                    (SELECT COALESCE(SUM(amount_kes),0) FROM ledger_entries
                     WHERE agreement_id = a.id AND entry_type IN ('payment','credit','waiver')),
                    0
                ) AS outstanding_kes,

                (SELECT COUNT(*) FROM late_fee_charges
                 WHERE agreement_id = a.id AND charged_at >= now() - INTERVAL '12 months'
                ) AS late_payments_12m,

                (SELECT COUNT(*) FROM agreements
                 WHERE resident_id = r.id
                ) AS prior_agreements,

                (SELECT EXTRACT(EPOCH FROM (now() - MAX(posted_at))) / 86400
                 FROM ledger_entries
                 WHERE agreement_id = a.id AND entry_type = 'payment'
                )::BIGINT AS days_since_last_payment

            FROM agreements a
            JOIN residents  r ON r.id = a.resident_id
            JOIN units      u ON u.id = a.unit_id
            JOIN properties p ON p.id = a.property_id
            WHERE a.status = 'active'
              AND a.end_date IS NOT NULL
              AND (a.end_date - CURRENT_DATE) <= $1
            ORDER BY a.end_date ASC
            "#,
            horizon
        )
        .fetch_all(self.pool.as_ref())
        .await?;

        let predictions = rows
            .into_iter()
            .map(|row| {
                let (score, factors) = Self::compute_score(&row);
                let action = Self::recommended_action(score, &row);
                TenantChurnPrediction {
                    agreement_id: row.agreement_id,
                    resident_id: row.resident_id,
                    resident_name: row.resident_name,
                    unit_id: row.unit_id,
                    unit_number: row.unit_number,
                    property_id: row.property_id,
                    property_name: row.property_name,
                    days_until_expiry: row.days_until_expiry.unwrap_or(0),
                    lease_end_date: row.end_date.unwrap(),
                    churn_score: score,
                    risk_level: RiskLevel::from_score(score),
                    recommended_action: action,
                    churn_factors: factors,
                    generated_at: now,
                }
            })
            .collect();

        Ok(predictions)
    }

    fn compute_score(row: &ChurnRow) -> (u8, Vec<String>) {
        let mut score = 0u8;
        let mut factors = Vec::new();

        // Days until expiry
        let expiry_pts = match row.days_until_expiry {
            Some(d) if d <= 30 => 30u8,
            Some(d) if d <= 60 => 20,
            Some(d) if d <= 90 => 10,
            _ => 0,
        };
        score = score.saturating_add(expiry_pts);
        if expiry_pts > 0 {
            factors.push(format!(
                "Lease expires in {} days",
                row.days_until_expiry.unwrap_or(0)
            ));
        }

        // Late payments
        let late_pts = match row.late_payments_12m {
            0 => 0,
            1 => 10,
            _ => 20,
        } as u8;
        score = score.saturating_add(late_pts);
        if late_pts > 0 {
            factors.push(format!(
                "{} late payments in 12 months",
                row.late_payments_12m
            ));
        }

        // Outstanding balance
        if row.outstanding_kes > Decimal::ZERO {
            score = score.saturating_add(15);
            factors.push(format!(
                "Outstanding balance KES {:.0}",
                row.outstanding_kes
            ));
        }

        // First-time tenant
        if row.prior_agreements <= 1 {
            score = score.saturating_add(10);
            factors.push("First tenancy — limited history".to_string());
        }

        // Days since last payment
        if matches!(row.days_since_last_payment, Some(d) if d > 45) {
            score = score.saturating_add(15);
            factors.push(format!(
                "No payment for {} days",
                row.days_since_last_payment.unwrap_or(0)
            ));
        }

        (score.min(100), factors)
    }

    fn recommended_action(score: u8, row: &ChurnRow) -> String {
        match score {
            0..=30 => "Monitor — send standard renewal notice 30 days before expiry".to_string(),
            31..=59 => format!(
                "Proactive outreach — contact {} about renewal terms",
                row.resident_name
            ),
            60..=79 => format!(
                "Priority retention — offer incentive or meeting to {} before lease ends",
                row.resident_name
            ),
            _ => format!(
                "Urgent — begin vacancy planning for unit {}; high churn risk for {}",
                row.unit_number, row.resident_name
            ),
        }
    }
}
