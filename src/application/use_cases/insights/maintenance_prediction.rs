//
// Predictive Maintenance Alerts + Expense Forecasting + Smart Vendor Allocation

use std::sync::Arc;

use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::insight_repository::InsightRepository},
    domain::insights::{
        ExpenseForecast, MonthlyExpenseForecast, PredictiveMaintenanceAlert, RiskLevel,
        VendorAllocationRecommendation, VendorScore,
    },
};

// ─────────────────────────────────────────────────────────────────────────────
// 1. Predictive Maintenance
// ─────────────────────────────────────────────────────────────────────────────

pub struct PredictiveMaintenanceUseCase {
    pub repo: Arc<dyn InsightRepository>,
}

impl PredictiveMaintenanceUseCase {
    pub fn new(repo: Arc<dyn InsightRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(
        &self,
        agency_id: Uuid,
        min_historical_count: Option<i64>,
    ) -> Result<Vec<PredictiveMaintenanceAlert>, AppError> {
        let min_count = min_historical_count.unwrap_or(2);
        let now = OffsetDateTime::now_utc();

        let rows = self
            .repo
            .get_maintenance_patterns(agency_id, min_count)
            .await?;

        let today = now.date();
        let alerts = rows
            .into_iter()
            .map(|row| {
                let avg_days = row.avg_interval_days.unwrap_or(Decimal::from(30));
                let days_since_last = row.last_occurred.map(|d| (today - d).whole_days());

                let estimated_days_until_next = days_since_last.and_then(|since| {
                    let remaining = avg_days.to_i64().unwrap_or(30) - since;
                    Some(remaining.max(0))
                });

                // Likelihood: higher if we're past the average interval.
                let likelihood_score: u8 = match (days_since_last, avg_days.to_i64()) {
                    (Some(since), Some(avg)) if since >= avg => 80,
                    (Some(since), Some(avg)) if since >= avg / 2 => 50,
                    _ => 25,
                };

                PredictiveMaintenanceAlert {
                    unit_id: row.unit_id,
                    unit_number: row.unit_number,
                    property_id: row.property_id,
                    property_name: row.property_name,
                    predicted_category: row.category,
                    likelihood_score,
                    risk_level: RiskLevel::from_score(likelihood_score),
                    historical_count: row.count_12m,
                    avg_interval_days: avg_days,
                    last_occurred: row.last_occurred,
                    estimated_days_until_next,
                    generated_at: now,
                }
            })
            .collect();

        Ok(alerts)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 2. Expense Forecasting
// ─────────────────────────────────────────────────────────────────────────────

pub struct ExpenseForecastUseCase {
    pub repo: Arc<dyn InsightRepository>,
}

impl ExpenseForecastUseCase {
    pub fn new(repo: Arc<dyn InsightRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(
        &self,
        agency_id: Uuid,
        property_id: Option<Uuid>,
        horizon_months: Option<u8>,
    ) -> Result<ExpenseForecast, AppError> {
        let horizon = horizon_months.unwrap_or(6).min(24);
        let now = OffsetDateTime::now_utc();

        let actuals = self
            .repo
            .get_expense_baseline(agency_id, property_id)
            .await?;

        // Compute averages.
        let count = actuals.len().max(1) as i64;
        let avg_maint =
            actuals.iter().map(|r| r.maintenance_kes).sum::<Decimal>() / Decimal::from(count);
        let avg_util =
            actuals.iter().map(|r| r.utility_kes).sum::<Decimal>() / Decimal::from(count);
        let trailing_total: Decimal = actuals
            .iter()
            .map(|r| r.maintenance_kes + r.utility_kes)
            .sum();

        // Build forecast months with a simple 5% YoY growth rate.
        let growth = Decimal::new(105, 2); // 1.05
        let mut months = Vec::with_capacity(horizon as usize);
        let mut forecast_total = Decimal::ZERO;

        for i in 0..horizon {
            // Rough month offset from today.
            let month = advance_month(now.date(), i as i32 + 1);
            let maint = avg_maint * growth;
            let util = avg_util * growth;
            let total = maint + util;
            let lower = total * Decimal::new(80, 2);
            let upper = total * Decimal::new(120, 2);
            forecast_total += total;
            months.push(MonthlyExpenseForecast {
                month,
                forecasted_maintenance_kes: maint,
                forecasted_utility_kes: util,
                forecasted_total_kes: total,
                lower_bound_kes: lower,
                upper_bound_kes: upper,
            });
        }

        Ok(ExpenseForecast {
            property_id,
            generated_at: now,
            horizon_months: horizon,
            months,
            trailing_12m_actual_kes: trailing_total,
            forecast_total_kes: forecast_total,
        })
    }
}

fn advance_month(d: time::Date, months: i32) -> time::Date {
    let total_months = d.month() as i32 + months - 1;
    let year = d.year() + total_months / 12;
    let month_num = ((total_months % 12) + 1) as u8;
    let month = time::Month::try_from(month_num).unwrap_or(time::Month::January);
    time::Date::from_calendar_date(year, month, 1).unwrap_or(d)
}

// ─────────────────────────────────────────────────────────────────────────────
// 3. Smart Vendor Allocation
// ─────────────────────────────────────────────────────────────────────────────

pub struct VendorAllocationUseCase {
    pub repo: Arc<dyn InsightRepository>,
}

impl VendorAllocationUseCase {
    pub fn new(repo: Arc<dyn InsightRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(
        &self,
        agency_id: Uuid,
        work_order_id: Uuid,
        category: &str,
    ) -> Result<VendorAllocationRecommendation, AppError> {
        let now = OffsetDateTime::now_utc();

        let rows = self
            .repo
            .get_vendor_performance(agency_id, category)
            .await?;

        // Compute peer benchmarks for relative scoring.
        let avg_cost_peer: Decimal = {
            let costs: Vec<Decimal> = rows.iter().filter_map(|r| r.avg_cost_kes).collect();
            if costs.is_empty() {
                Decimal::from(5000)
            } else {
                costs.iter().sum::<Decimal>() / Decimal::from(costs.len())
            }
        };

        let recommendations = rows
            .into_iter()
            .map(|row| {
                // Specialisation: more completed jobs in this category = better.
                let spec_score = (row.completed_jobs.min(20) * 5) as u8;

                // Resolution speed: faster = better (vs 7-day benchmark).
                let resolution_days = row.avg_resolution_days.unwrap_or(Decimal::from(14));
                let resolution_score: u8 = if resolution_days <= Decimal::from(3) {
                    100
                } else if resolution_days <= Decimal::from(7) {
                    75
                } else if resolution_days <= Decimal::from(14) {
                    50
                } else {
                    25
                };

                // Cost efficiency: lower than peers = better.
                let cost_score: u8 = match row.avg_cost_kes {
                    None => 50,
                    Some(cost) if cost <= avg_cost_peer * Decimal::new(80, 2) => 100,
                    Some(cost) if cost <= avg_cost_peer => 75,
                    Some(cost) if cost <= avg_cost_peer * Decimal::new(120, 2) => 50,
                    _ => 25,
                };

                // Recency: active in last 30 days = better.
                let recency_score: u8 = match row.last_job_days_ago {
                    None => 0,
                    Some(d) if d <= 30 => 100,
                    Some(d) if d <= 60 => 70,
                    Some(d) if d <= 90 => 50,
                    _ => 25,
                };

                // Composite weighted score.
                let composite = (spec_score as u16 * 30
                    + resolution_score as u16 * 25
                    + cost_score as u16 * 25
                    + recency_score as u16 * 20)
                    / 100;

                VendorScore {
                    vendor_id: row.vendor_id,
                    vendor_name: row.vendor_name,
                    phone: row.phone,
                    email: row.email,
                    score: (composite.min(100) as u8),
                    completed_jobs: row.completed_jobs,
                    avg_resolution_days: resolution_days,
                    cost_efficiency_score: cost_score,
                    recency_score,
                    category_specialisation_score: spec_score,
                }
            })
            .collect::<Vec<_>>();

        let mut recs = recommendations;
        recs.sort_by(|a, b| b.score.cmp(&a.score));

        Ok(VendorAllocationRecommendation {
            work_order_id,
            category: category.to_string(),
            generated_at: now,
            recommendations: recs,
        })
    }
}
