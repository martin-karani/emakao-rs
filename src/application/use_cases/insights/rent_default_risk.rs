//! Rent default risk scoring – predicts the probability that a tenant will
//! fall behind on rent within the next 3 months.

use rust_decimal::Decimal;
use std::sync::Arc;
use time::OffsetDateTime;

use crate::{
    application::{
        errors::AppError,
        ports::insight_repository::{InsightRepository, RentDefaultRiskData},
    },
    domain::insights::{RentDefaultRiskScore, RiskLevel},
    presentation::extractors::AgencyContext,
};

// ------------------------------------------------------------
// Use case
// ------------------------------------------------------------

pub struct RentDefaultRiskUseCase {
    repo: Arc<dyn InsightRepository>,
}

impl RentDefaultRiskUseCase {
    pub fn new(repo: Arc<dyn InsightRepository>) -> Self {
        Self { repo }
    }

    /// Compute risk scores for all active agreements in the given agency.
    pub async fn execute(
        &self,
        ctx: &AgencyContext,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<RentDefaultRiskScore>, AppError> {
        let stats = self
            .repo
            .get_rent_default_stats(ctx.agency.id, limit, offset)
            .await?;

        if stats.is_empty() {
            return Ok(vec![]);
        }

        let now = OffsetDateTime::now_utc();
        let mut scores = Vec::with_capacity(stats.len());
        for data in stats {
            let risk = self.compute_risk_for_agreement(data, now).await?;
            scores.push(risk);
        }

        Ok(scores)
    }

    async fn compute_risk_for_agreement(
        &self,
        data: RentDefaultRiskData,
        now: OffsetDateTime,
    ) -> Result<RentDefaultRiskScore, AppError> {
        let mut risk_score: f64 = 0.0;
        let mut factors = Vec::new();

        // Factor 1: late fees in last 6 months
        if data.late_fee_count_6m > 2 {
            risk_score += 0.4;
            factors.push(format!(
                "{} late fees in last 6 months",
                data.late_fee_count_6m
            ));
        } else if data.late_fee_count_6m > 0 {
            risk_score += 0.2;
            factors.push(format!(
                "{} late fee(s) in last 6 months",
                data.late_fee_count_6m
            ));
        }

        // Factor 2: average payment delay
        if data.avg_payment_delay_days > 7.0 {
            risk_score += 0.3;
            factors.push(format!(
                "average payment delay of {:.1} days",
                data.avg_payment_delay_days
            ));
        } else if data.avg_payment_delay_days > 3.0 {
            risk_score += 0.1;
            factors.push("slightly delayed payments".to_string());
        }

        // Factor 3: outstanding balance vs rent
        let rent = data.rent_amount_kes;
        if data.outstanding_kes > rent * Decimal::new(15, 1) {
            risk_score += 0.3;
            factors.push(format!(
                "outstanding balance of KES {}",
                data.outstanding_kes
            ));
        } else if data.outstanding_kes > rent {
            risk_score += 0.15;
            factors.push("moderate outstanding balance".to_string());
        }

        // Factor 4: days since last payment
        let days_since_last_payment = data
            .last_payment_at
            .map(|dt| (now - dt).whole_days())
            .unwrap_or(999);
        if days_since_last_payment > 45 {
            risk_score += 0.2;
            factors.push(format!("no payment in {} days", days_since_last_payment));
        } else if days_since_last_payment > 30 {
            risk_score += 0.1;
            factors.push("payment overdue by more than 30 days".to_string());
        }

        risk_score = risk_score.clamp(0.0, 1.0);
        let score_u8 = (risk_score * 100.0) as u8;

        Ok(RentDefaultRiskScore {
            agreement_id: data.agreement_id,
            resident_id: data.resident_id,
            resident_name: data.resident_name,
            unit_id: data.unit_id,
            unit_number: data.unit_number,
            property_id: data.property_id,
            property_name: data.property_name,
            score: score_u8,
            risk_level: RiskLevel::from_score(score_u8),
            risk_factors: factors,
            outstanding_kes: data.outstanding_kes,
            late_payments_6m: data.late_fee_count_6m,
            days_since_last_payment: data.last_payment_at.map(|dt| (now - dt).whole_days()),
            generated_at: now,
        })
    }
}
