use std::sync::Arc;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::insight_repository::InsightRepository},
    domain::insights::{RiskLevel, TenantChurnPrediction},
};

pub struct TenantChurnUseCase {
    pub repo: Arc<dyn InsightRepository>,
}

impl TenantChurnUseCase {
    pub fn new(repo: Arc<dyn InsightRepository>) -> Self {
        Self { repo }
    }

    /// Return churn predictions for leases expiring within `horizon_days`.
    pub async fn execute(
        &self,
        agency_id: Uuid,
        horizon_days: Option<i64>,
    ) -> Result<Vec<TenantChurnPrediction>, AppError> {
        let horizon = horizon_days.unwrap_or(90);
        let now = OffsetDateTime::now_utc();

        let stats = self.repo.get_tenant_churn_stats(agency_id).await?;

        let predictions = stats
            .into_iter()
            .filter(|r| r.days_until_expiry.unwrap_or(999) <= horizon)
            .map(|row| {
                let mut score: u8 = 0;
                let mut factors = Vec::new();

                // 1. Days until expiry
                let days = row.days_until_expiry.unwrap_or(999);
                if days < 30 {
                    score += 30;
                    factors.push("Lease expires in < 30 days".to_string());
                } else if days < 60 {
                    score += 20;
                    factors.push("Lease expires in < 60 days".to_string());
                } else if days < 90 {
                    score += 10;
                    factors.push("Lease expires in < 90 days".to_string());
                }

                // 2. Late payments
                if row.late_payments_12m >= 2 {
                    score += 20;
                    factors.push(format!(
                        "{} late payments in last 12m",
                        row.late_payments_12m
                    ));
                } else if row.late_payments_12m == 1 {
                    score += 10;
                    factors.push("1 late payment in last 12m".to_string());
                }

                // 3. Outstanding balance
                if row.outstanding_kes > rust_decimal::Decimal::ZERO {
                    score += 15;
                    factors.push(format!("Outstanding balance: KES {}", row.outstanding_kes));
                }

                // 4. Tenure (Loyalty)
                if row.prior_agreements <= 1 {
                    score += 10;
                    factors.push("First-time tenant (no prior history)".to_string());
                }

                // 5. Payment recency
                let silence = row.days_since_last_payment.unwrap_or(999);
                if silence > 45 {
                    score += 15;
                    factors.push(format!("No payment recorded in {} days", silence));
                }

                // 6. Open-ended lease
                let is_open_ended = row.end_date.is_none();
                if is_open_ended {
                    // Open-ended leases have lower churn risk usually
                    score = score.saturating_sub(10);
                }

                let final_score = score.min(100);
                let risk_level = RiskLevel::from_score(final_score);

                let recommended_action = match risk_level {
                    RiskLevel::Critical | RiskLevel::High => {
                        "Contact tenant immediately for renewal discussion"
                    }
                    RiskLevel::Medium => "Send renewal invitation and check satisfaction",
                    RiskLevel::Low => "Automated renewal reminder",
                };

                TenantChurnPrediction {
                    agreement_id: row.agreement_id,
                    resident_id: row.resident_id,
                    resident_name: row.resident_name,
                    unit_id: row.unit_id,
                    unit_number: row.unit_number,
                    property_id: row.property_id,
                    property_name: row.property_name,
                    days_until_expiry: days,
                    lease_end_date: row.end_date.unwrap_or_else(|| now.date()),
                    churn_score: final_score,
                    risk_level,
                    recommended_action: recommended_action.to_string(),
                    churn_factors: factors,
                    generated_at: now,
                }
            })
            .collect();

        Ok(predictions)
    }
}
