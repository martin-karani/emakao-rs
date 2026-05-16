// src/application/use_cases/ai_insights/maintenance_prediction.rs
//
// Predictive Maintenance Alerts + Expense Forecasting + Smart Vendor Allocation

use std::sync::Arc;

use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use sqlx::PgPool;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::{
    application::errors::AppError,
    domain::ai_insights::{
        ExpenseForecast, MonthlyExpenseForecast, PredictiveMaintenanceAlert, RiskLevel,
        VendorAllocationRecommendation, VendorScore,
    },
};

// ─────────────────────────────────────────────────────────────────────────────
// 1. Predictive Maintenance
// ─────────────────────────────────────────────────────────────────────────────

pub struct PredictiveMaintenanceUseCase {
    pub pool: Arc<PgPool>,
}

#[derive(sqlx::FromRow)]
struct MaintenancePatternRow {
    unit_id: Uuid,
    unit_number: String,
    property_id: Uuid,
    property_name: String,
    category: String,
    count_12m: i64,
    avg_interval_days: Option<Decimal>,
    last_occurred: Option<Date>,
}

impl PredictiveMaintenanceUseCase {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    pub async fn execute(
        &self,
        agency_id: Uuid,
        min_historical_count: Option<i64>,
    ) -> Result<Vec<PredictiveMaintenanceAlert>, AppError> {
        let min_count = min_historical_count.unwrap_or(2);
        let now = OffsetDateTime::now_utc();

        let rows = sqlx::query_as!(
            MaintenancePatternRow,
            r#"
            SELECT
                u.id                    AS unit_id,
                u.unit_number,
                p.id                    AS property_id,
                p.name                  AS property_name,
                wo.category::text       AS "category!",
                COUNT(*)                AS "count_12m!",
                -- avg days between consecutive work orders for this category+unit
                AVG(
                    EXTRACT(EPOCH FROM (wo.created_at -
                        LAG(wo.created_at) OVER (
                            PARTITION BY wo.unit_id, wo.category
                            ORDER BY wo.created_at
                        )
                    )) / 86400
                )::NUMERIC(10,1)       AS avg_interval_days,
                MAX(wo.created_at)::DATE AS last_occurred
            FROM   work_orders wo
            JOIN   units       u  ON u.id = wo.unit_id
            JOIN   properties  p  ON p.id = wo.property_id
            WHERE  wo.created_at >= now() - INTERVAL '12 months'
              AND  wo.unit_id IS NOT NULL
            GROUP  BY u.id, u.unit_number, p.id, p.name, wo.category
            HAVING COUNT(*) >= $1
            ORDER  BY COUNT(*) DESC
            "#,
            min_count
        )
        .fetch_all(self.pool.as_ref())
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
    pub pool: Arc<PgPool>,
}

#[derive(sqlx::FromRow)]
struct MonthlyActualRow {
    month: Date,
    maintenance_kes: Decimal,
    utility_kes: Decimal,
}

impl ExpenseForecastUseCase {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    pub async fn execute(
        &self,
        agency_id: Uuid,
        property_id: Option<Uuid>,
        horizon_months: Option<u8>,
    ) -> Result<ExpenseForecast, AppError> {
        let horizon = horizon_months.unwrap_or(6).min(24);
        let now = OffsetDateTime::now_utc();

        // Fetch 12 months of actuals as baseline.
        let actuals = sqlx::query_as!(
            MonthlyActualRow,
            r#"
            SELECT
                DATE_TRUNC('month', wo.completed_at)::DATE AS "month!",
                COALESCE(SUM(wo.actual_cost_kes), 0)        AS "maintenance_kes!",
                0::NUMERIC(14,2)                            AS "utility_kes!"
            FROM work_orders wo
            JOIN properties p ON p.id = wo.property_id
            WHERE wo.completed_at >= now() - INTERVAL '12 months'
              AND ($1::uuid IS NULL OR p.id = $1)
            GROUP BY 1
            ORDER BY 1
            "#,
            property_id
        )
        .fetch_all(self.pool.as_ref())
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

fn advance_month(d: Date, months: i32) -> Date {
    let total_months = d.month() as i32 + months - 1;
    let year = d.year() + total_months / 12;
    let month_num = ((total_months % 12) + 1) as u8;
    let month = time::Month::try_from(month_num).unwrap_or(time::Month::January);
    Date::from_calendar_date(year, month, 1).unwrap_or(d)
}

// ─────────────────────────────────────────────────────────────────────────────
// 3. Smart Vendor Allocation
// ─────────────────────────────────────────────────────────────────────────────

pub struct VendorAllocationUseCase {
    pub pool: Arc<PgPool>,
}

#[derive(sqlx::FromRow)]
struct VendorPerfRow {
    vendor_id: Uuid,
    vendor_name: String,
    phone: Option<String>,
    email: Option<String>,
    completed_jobs: i64,
    avg_resolution_days: Option<Decimal>,
    avg_cost_kes: Option<Decimal>,
    last_job_days_ago: Option<i64>,
}

impl VendorAllocationUseCase {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    pub async fn execute(
        &self,
        agency_id: Uuid,
        work_order_id: Uuid,
        category: &str,
    ) -> Result<VendorAllocationRecommendation, AppError> {
        let now = OffsetDateTime::now_utc();

        let rows = sqlx::query_as!(
            VendorPerfRow,
            r#"
            SELECT
                v.id                AS vendor_id,
                v.name              AS vendor_name,
                v.phone,
                v.email,
                COUNT(wo.id)        AS "completed_jobs!",
                AVG(
                    EXTRACT(EPOCH FROM (wo.completed_at - wo.created_at)) / 86400
                )::NUMERIC(10,1)   AS avg_resolution_days,
                AVG(wo.actual_cost_kes)::NUMERIC(14,2) AS avg_cost_kes,
                EXTRACT(EPOCH FROM (now() - MAX(wo.completed_at))) / 86400 AS last_job_days_ago
            FROM   vendors   v
            LEFT JOIN work_orders wo
                   ON wo.vendor_id = v.id
                  AND wo.status    = 'completed'
                  AND wo.category  = $2::text::work_order_category
            WHERE  v.is_active = true
            GROUP  BY v.id, v.name, v.phone, v.email
            ORDER  BY completed_jobs DESC
            "#,
            agency_id, // unused in WHERE but kept for schema context
            category
        )
        .fetch_all(self.pool.as_ref())
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
            .map(|row| Self::score_vendor(row, &avg_cost_peer))
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

    fn score_vendor(row: VendorPerfRow, avg_cost_peer: &Decimal) -> VendorScore {
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
            Some(cost) if cost <= *avg_cost_peer * Decimal::new(80, 2) => 100,
            Some(cost) if cost <= *avg_cost_peer => 75,
            Some(cost) if cost <= *avg_cost_peer * Decimal::new(120, 2) => 50,
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
            score: composite.min(100) as u8,
            completed_jobs: row.completed_jobs,
            avg_resolution_days: resolution_days,
            cost_efficiency_score: cost_score,
            recency_score,
            category_specialisation_score: spec_score,
        }
    }
}
