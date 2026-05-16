//! Rent default risk scoring – predicts the probability that a tenant will
//! fall behind on rent within the next 3 months.

use rust_decimal::Decimal;
use std::sync::Arc;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::ledger_repository::LedgerRepository},
    domain::{
        agreement::Agreement,
        enums::{AgreementStatus, LedgerEntryType},
    },
    infrastructure::db::{
        agreement_repository_sqlx::PgAgreementRepo,
        ledger_repository_sqlx::{HasPool, PgLedgerRepo},
    },
    presentation::extractors::AgencyContext,
};

// ------------------------------------------------------------
// Risk score for a single agreement
// ------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct RentDefaultRiskScore {
    pub agreement_id: Uuid,
    pub resident_id: Uuid,
    pub unit_ref: String,
    pub risk_score: f64,
    pub risk_level: &'static str,
    pub contributing_factors: Vec<String>,
}

// ------------------------------------------------------------
// Use case
// ------------------------------------------------------------

pub struct RentDefaultRiskUseCase {
    agreement_repo: Arc<PgAgreementRepo>,
    ledger_repo: Arc<PgLedgerRepo>,
}

impl RentDefaultRiskUseCase {
    pub fn new(agreement_repo: Arc<PgAgreementRepo>, ledger_repo: Arc<PgLedgerRepo>) -> Self {
        Self {
            agreement_repo,
            ledger_repo,
        }
    }

    /// Compute risk scores for all active agreements in the given agency.
    pub async fn execute(
        &self,
        ctx: &AgencyContext,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<RentDefaultRiskScore>, AppError> {
        let agreements = self
            .agreement_repo
            .find_all(ctx.agency.id, None, limit, offset)
            .await?;
        let active_agreements: Vec<&Agreement> = agreements
            .iter()
            .filter(|a| a.status == AgreementStatus::Active)
            .collect();

        if active_agreements.is_empty() {
            return Ok(vec![]);
        }

        let mut scores = Vec::with_capacity(active_agreements.len());
        for agreement in active_agreements {
            let risk = self.compute_risk_for_agreement(agreement).await?;
            scores.push(risk);
        }

        Ok(scores)
    }

    async fn compute_risk_for_agreement(
        &self,
        agreement: &Agreement,
    ) -> Result<RentDefaultRiskScore, AppError> {
        let agreement_id = agreement.id;
        let stats = self.fetch_payment_stats(agreement_id).await?;
        let unit_ref = self.fetch_unit_reference(agreement.unit_id).await?;

        let mut risk_score = 0.0;
        let mut factors = Vec::new();

        // Factor 1: late fees in last 6 months
        if stats.late_fee_count_6m > 2 {
            risk_score += 0.4;
            factors.push(format!(
                "{} late fees in last 6 months",
                stats.late_fee_count_6m
            ));
        } else if stats.late_fee_count_6m > 0 {
            risk_score += 0.2;
            factors.push(format!(
                "{} late fee(s) in last 6 months",
                stats.late_fee_count_6m
            ));
        }

        // Factor 2: average payment delay
        if stats.avg_payment_delay_days > 7.0 {
            risk_score += 0.3;
            factors.push(format!(
                "average payment delay of {:.1} days",
                stats.avg_payment_delay_days
            ));
        } else if stats.avg_payment_delay_days > 3.0 {
            risk_score += 0.1;
            factors.push("slightly delayed payments".to_string());
        }

        // Factor 3: outstanding balance vs rent
        let rent = agreement.rent_amount_kes;
        if stats.outstanding_kes > rent * Decimal::from_f64(1.5).unwrap() {
            risk_score += 0.3;
            factors.push(format!(
                "outstanding balance of KES {}",
                stats.outstanding_kes
            ));
        } else if stats.outstanding_kes > rent {
            risk_score += 0.15;
            factors.push("moderate outstanding balance".to_string());
        }

        // Factor 4: days since last payment
        let days_since_last_payment = stats
            .last_payment_at
            .map(|dt| (OffsetDateTime::now_utc() - dt).whole_days())
            .unwrap_or(999);
        if days_since_last_payment > 45 {
            risk_score += 0.2;
            factors.push(format!("no payment in {} days", days_since_last_payment));
        } else if days_since_last_payment > 30 {
            risk_score += 0.1;
            factors.push("payment overdue by more than 30 days".to_string());
        }

        risk_score = risk_score.clamp(0.0, 1.0);
        let risk_level = if risk_score >= 0.7 {
            "High"
        } else if risk_score >= 0.4 {
            "Medium"
        } else {
            "Low"
        };

        Ok(RentDefaultRiskScore {
            agreement_id,
            resident_id: agreement.resident_id,
            unit_ref,
            risk_score,
            risk_level,
            contributing_factors: factors,
        })
    }

    async fn fetch_payment_stats(&self, agreement_id: Uuid) -> Result<PaymentStats, AppError> {
        use sqlx::types::Decimal;

        let row = sqlx::query!(
            r#"
            SELECT
                COALESCE(
                    (SELECT SUM(amount_kes) FROM ledger_entries
                     WHERE agreement_id = $1
                       AND entry_type IN ('rent'::ledger_entry_type, 'late_fee'::ledger_entry_type, 'maintenance_charge'::ledger_entry_type)
                    ), 0
                ) - COALESCE(
                    (SELECT SUM(amount_kes) FROM ledger_entries
                     WHERE agreement_id = $1
                       AND entry_type IN ('payment_mpesa'::ledger_entry_type, 'payment_bank'::ledger_entry_type, 'payment_cash'::ledger_entry_type)
                    ), 0
                ) AS "outstanding_kes!: Decimal",

                (
                    SELECT COUNT(*) FROM late_fee_charges
                    WHERE agreement_id = $1
                      AND charged_at > now() - INTERVAL '6 months'
                ) AS "late_fee_count_6m!: i64",

                COALESCE(
                    (SELECT AVG(EXTRACT(EPOCH FROM (le.posted_at - rc.period_end)) / 86400)
                     FROM ledger_entries le
                     JOIN rent_charges rc ON rc.agreement_id = le.agreement_id
                     WHERE le.agreement_id = $1
                       AND le.entry_type IN ('payment_mpesa'::ledger_entry_type, 'payment_bank'::ledger_entry_type, 'payment_cash'::ledger_entry_type)
                       AND le.posted_at > rc.period_end
                    ), 0
                ) AS "avg_payment_delay_days!: f64",

                (
                    SELECT MAX(posted_at) FROM ledger_entries
                    WHERE agreement_id = $1
                      AND entry_type IN ('payment_mpesa'::ledger_entry_type, 'payment_bank'::ledger_entry_type, 'payment_cash'::ledger_entry_type)
                ) AS last_payment_at
            "#,
            agreement_id
        )
        .fetch_one(self.ledger_repo.pool())
        .await?;

        Ok(PaymentStats {
            outstanding_kes: row.outstanding_kes,
            late_fee_count_6m: row.late_fee_count_6m as i32,
            avg_payment_delay_days: row.avg_payment_delay_days,
            last_payment_at: row.last_payment_at,
        })
    }

    async fn fetch_unit_reference(&self, unit_id: Uuid) -> Result<String, AppError> {
        let unit_number: Option<String> =
            sqlx::query_scalar!("SELECT unit_number FROM units WHERE id = $1", unit_id)
                .fetch_one(self.ledger_repo.pool())
                .await?;
        Ok(unit_number.unwrap_or_else(|| "Unknown".to_string()))
    }
}

// ------------------------------------------------------------
// Internal helper struct
// ------------------------------------------------------------

struct PaymentStats {
    outstanding_kes: Decimal,
    late_fee_count_6m: i32,
    avg_payment_delay_days: f64,
    last_payment_at: Option<OffsetDateTime>,
}
