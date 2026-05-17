use async_trait::async_trait;
use rust_decimal::Decimal;
use sqlx::PgPool;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        ports::analytics_repository::{AnalyticsQuery, AnalyticsRepository},
    },
    domain::analytics::*,
};

pub struct PgAnalyticsRepo {
    pool: PgPool,
}

impl PgAnalyticsRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

// ── Internal row types ────────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct MonthlyRevenueRow {
    month: Date,
    charged_kes: Decimal,
    collected_kes: Decimal,
    late_fees_kes: Decimal,
}

#[derive(sqlx::FromRow)]
struct MonthlyOccupancyRow {
    month: Date,
    occupied_units: i64,
    total_units: i64,
}

#[derive(sqlx::FromRow)]
struct PropertyPerfRow {
    property_id: Uuid,
    property_name: String,
    city: String,
    units: i64,
    occupied_units: i64,
    revenue_kes: Decimal,
    outstanding_kes: Decimal,
    maintenance_cost_kes: Decimal,
}

#[derive(sqlx::FromRow)]
struct CategoryBreakdownRow {
    category: String,
    count: i64,
    total_cost_kes: Decimal,
    avg_resolution_days: Decimal,
}

#[derive(sqlx::FromRow)]
struct MaintStatsRow {
    total: i64,
    open: i64,
    completed: i64,
    avg_days: Decimal,
    total_cost: Decimal,
    est_cost: Decimal,
}

// ── Repository ────────────────────────────────────────────────────────────────

#[async_trait]
impl AnalyticsRepository for PgAnalyticsRepo {
    async fn portfolio_analytics(
        &self,
        query: AnalyticsQuery,
    ) -> Result<PortfolioAnalytics, AppError> {
        let generated_at = OffsetDateTime::now_utc();

        // ── Monthly revenue rows ──────────────────────────────────────────────
        let rev_rows = self.fetch_monthly_revenue(&query).await?;
        let total_charged: Decimal = rev_rows.iter().map(|r| r.charged_kes).sum();
        let total_collected: Decimal = rev_rows.iter().map(|r| r.collected_kes).sum();
        let total_late_fees: Decimal = rev_rows.iter().map(|r| r.late_fees_kes).sum();
        let total_outstanding = (total_charged - total_collected).max(Decimal::ZERO);
        let collection_rate = if total_charged > Decimal::ZERO {
            total_collected * Decimal::from(100) / total_charged
        } else {
            Decimal::from(100)
        };

        // ── Maintenance stats ─────────────────────────────────────────────────
        let maint_stats = sqlx::query_as::<_, MaintStatsRow>(
            r#"
            SELECT
                COUNT(*)::BIGINT                                            AS total,
                COUNT(*) FILTER (WHERE status NOT IN ('completed','cancelled'))::BIGINT AS open,
                COUNT(*) FILTER (WHERE status = 'completed')::BIGINT        AS completed,
                COALESCE(AVG(
                    CASE WHEN completed_at IS NOT NULL
                    THEN EXTRACT(EPOCH FROM (completed_at - created_at))/86400
                    END
                ), 0)::NUMERIC(10,1)                                        AS avg_days,
                COALESCE(SUM(actual_cost_kes),0)                            AS total_cost,
                COALESCE(SUM(estimated_cost_kes),0)                         AS est_cost
            FROM work_orders
            WHERE created_at BETWEEN $1 AND $2
              AND ($3::uuid IS NULL OR property_id = $3)
            "#,
        )
        .bind(query.period_start)
        .bind(query.period_end)
        .bind(query.property_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        // ── Category breakdown ────────────────────────────────────────────────
        let cat_rows: Vec<CategoryBreakdownRow> = sqlx::query_as::<_, CategoryBreakdownRow>(
            r#"
            SELECT
                category::text          AS category,
                COUNT(*)::BIGINT        AS count,
                COALESCE(SUM(actual_cost_kes),0) AS total_cost_kes,
                COALESCE(AVG(
                    CASE WHEN completed_at IS NOT NULL
                    THEN EXTRACT(EPOCH FROM (completed_at - created_at))/86400
                    END
                ), 0)::NUMERIC(10,1)   AS avg_resolution_days
            FROM work_orders
            WHERE created_at BETWEEN $1 AND $2
              AND ($3::uuid IS NULL OR property_id = $3)
            GROUP BY category
            ORDER BY count DESC
            "#,
        )
        .bind(query.period_start)
        .bind(query.period_end)
        .bind(query.property_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        // ── Occupancy trend ───────────────────────────────────────────────────
        let occ_rows = self.fetch_monthly_occupancy(&query).await?;
        let (total_units, occupied_units) = occ_rows
            .last()
            .map(|r| (r.total_units, r.occupied_units))
            .unwrap_or((0, 0));
        let avg_vacancy = Decimal::ZERO; // simplification; needs vacancy-day tracking

        // ── Property performance ──────────────────────────────────────────────
        let prop_rows: Vec<PropertyPerfRow> = sqlx::query_as::<_, PropertyPerfRow>(
            r#"
            SELECT
                p.id                    AS property_id,
                p.name                  AS property_name,
                p.city,
                COUNT(u.id)::BIGINT     AS units,
                COUNT(u.id) FILTER (WHERE u.status = 'occupied')::BIGINT AS occupied_units,
                COALESCE(
                    (SELECT SUM(le.amount_kes) FROM ledger_entries le
                     JOIN agreements a ON a.id = le.agreement_id
                     WHERE a.property_id = p.id
                       AND le.entry_type = 'payment'
                       AND le.posted_at BETWEEN $1 AND $2), 0
                ) AS revenue_kes,
                COALESCE(
                    (SELECT SUM(le.amount_kes)
                     FROM ledger_entries le
                     JOIN agreements a ON a.id = le.agreement_id
                     WHERE a.property_id = p.id
                       AND le.entry_type IN ('rent_charge','late_fee')
                    ) -
                    (SELECT COALESCE(SUM(le.amount_kes), 0)
                     FROM ledger_entries le
                     JOIN agreements a ON a.id = le.agreement_id
                     WHERE a.property_id = p.id
                       AND le.entry_type IN ('payment','credit','waiver')
                    ), 0
                ) AS outstanding_kes,
                COALESCE(
                    (SELECT SUM(wo.actual_cost_kes) FROM work_orders wo
                     WHERE wo.property_id = p.id
                       AND wo.status = 'completed'
                       AND wo.completed_at BETWEEN $1 AND $2), 0
                ) AS maintenance_cost_kes
            FROM properties p
            LEFT JOIN units u ON u.property_id = p.id
            WHERE ($3::uuid IS NULL OR p.id = $3)
            GROUP BY p.id, p.name, p.city
            ORDER BY revenue_kes DESC
            LIMIT 20
            "#,
        )
        .bind(query.period_start)
        .bind(query.period_end)
        .bind(query.property_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        let top_properties: Vec<PropertyPerformance> = prop_rows
            .into_iter()
            .map(|r| {
                let occ_rate = if r.units > 0 {
                    Decimal::from(r.occupied_units * 100) / Decimal::from(r.units)
                } else {
                    Decimal::ZERO
                };
                let roi = if r.revenue_kes > Decimal::ZERO {
                    (r.revenue_kes - r.maintenance_cost_kes) * Decimal::from(100) / r.revenue_kes
                } else {
                    Decimal::ZERO
                };
                PropertyPerformance {
                    property_id: r.property_id,
                    property_name: r.property_name,
                    city: r.city,
                    units: r.units,
                    occupancy_rate_pct: occ_rate,
                    revenue_kes: r.revenue_kes,
                    outstanding_kes: r.outstanding_kes,
                    maintenance_cost_kes: r.maintenance_cost_kes,
                    roi_pct: roi,
                }
            })
            .collect();

        let net_operating_income = total_collected - maint_stats.total_cost;

        Ok(PortfolioAnalytics {
            generated_at,
            period_start: query.period_start,
            period_end: query.period_end,
            occupancy: OccupancyAnalytics {
                total_units,
                occupied_units,
                vacant_units: total_units - occupied_units,
                occupancy_rate_pct: if total_units > 0 {
                    Decimal::from(occupied_units * 100) / Decimal::from(total_units)
                } else {
                    Decimal::ZERO
                },
                avg_vacancy_days: avg_vacancy,
                monthly_trend: occ_rows
                    .into_iter()
                    .map(|r| {
                        let rate = if r.total_units > 0 {
                            Decimal::from(r.occupied_units * 100) / Decimal::from(r.total_units)
                        } else {
                            Decimal::ZERO
                        };
                        MonthlyOccupancyPoint {
                            month: r.month,
                            occupied_units: r.occupied_units,
                            total_units: r.total_units,
                            rate_pct: rate,
                        }
                    })
                    .collect(),
            },
            revenue: RevenueAnalytics {
                total_charged_kes: total_charged,
                total_collected_kes: total_collected,
                total_outstanding_kes: total_outstanding,
                collection_rate_pct: collection_rate,
                total_late_fees_kes: total_late_fees,
                total_maintenance_cost_kes: maint_stats.total_cost,
                net_operating_income_kes: net_operating_income,
                monthly_trend: rev_rows
                    .into_iter()
                    .map(|r| {
                        let outstanding = (r.charged_kes - r.collected_kes).max(Decimal::ZERO);
                        MonthlyRevenuePoint {
                            month: r.month,
                            charged_kes: r.charged_kes,
                            collected_kes: r.collected_kes,
                            outstanding_kes: outstanding,
                            late_fees_kes: r.late_fees_kes,
                        }
                    })
                    .collect(),
            },
            maintenance: MaintenanceAnalytics {
                total_work_orders: maint_stats.total,
                open_work_orders: maint_stats.open,
                completed_work_orders: maint_stats.completed,
                avg_resolution_days: maint_stats.avg_days,
                total_actual_cost_kes: maint_stats.total_cost,
                total_estimated_cost_kes: maint_stats.est_cost,
                by_category: cat_rows
                    .into_iter()
                    .map(|r| MaintenanceCategoryBreakdown {
                        category: r.category,
                        count: r.count,
                        total_cost_kes: r.total_cost_kes,
                        avg_resolution_days: r.avg_resolution_days,
                    })
                    .collect(),
            },
            top_properties,
        })
    }

    async fn revenue_report(&self, query: AnalyticsQuery) -> Result<RevenueReport, AppError> {
        let generated_at = OffsetDateTime::now_utc();
        let months = self.fetch_monthly_revenue(&query).await?;

        let total_charged: Decimal = months.iter().map(|r| r.charged_kes).sum();
        let total_collected: Decimal = months.iter().map(|r| r.collected_kes).sum();
        let total_late: Decimal = months.iter().map(|r| r.late_fees_kes).sum();
        let outstanding = (total_charged - total_collected).max(Decimal::ZERO);
        let collection_rate = if total_charged > Decimal::ZERO {
            total_collected * Decimal::from(100) / total_charged
        } else {
            Decimal::from(100)
        };

        let monthly_trend = months
            .iter()
            .map(|r| MonthlyRevenuePoint {
                month: r.month,
                charged_kes: r.charged_kes,
                collected_kes: r.collected_kes,
                outstanding_kes: (r.charged_kes - r.collected_kes).max(Decimal::ZERO),
                late_fees_kes: r.late_fees_kes,
            })
            .collect();

        Ok(RevenueReport {
            generated_at,
            period_start: query.period_start,
            period_end: query.period_end,
            property_id: query.property_id,
            months: monthly_trend,
            summary: RevenueAnalytics {
                total_charged_kes: total_charged,
                total_collected_kes: total_collected,
                total_outstanding_kes: outstanding,
                collection_rate_pct: collection_rate,
                total_late_fees_kes: total_late,
                total_maintenance_cost_kes: Decimal::ZERO,
                net_operating_income_kes: total_collected,
                monthly_trend: Vec::new(),
            },
        })
    }

    async fn occupancy_trends(
        &self,
        query: AnalyticsQuery,
    ) -> Result<OccupancyTrendReport, AppError> {
        let generated_at = OffsetDateTime::now_utc();
        let rows = self.fetch_monthly_occupancy(&query).await?;
        let months = rows
            .into_iter()
            .map(|r| {
                let rate = if r.total_units > 0 {
                    Decimal::from(r.occupied_units * 100) / Decimal::from(r.total_units)
                } else {
                    Decimal::ZERO
                };
                MonthlyOccupancyPoint {
                    month: r.month,
                    occupied_units: r.occupied_units,
                    total_units: r.total_units,
                    rate_pct: rate,
                }
            })
            .collect();

        Ok(OccupancyTrendReport {
            generated_at,
            property_id: query.property_id,
            months,
        })
    }
}

// ── Shared helpers ────────────────────────────────────────────────────────────

impl PgAnalyticsRepo {
    async fn fetch_monthly_revenue(
        &self,
        query: &AnalyticsQuery,
    ) -> Result<Vec<MonthlyRevenueRow>, AppError> {
        sqlx::query_as::<_, MonthlyRevenueRow>(
            r#"
            SELECT
                DATE_TRUNC('month', le.posted_at)::DATE  AS month,
                COALESCE(SUM(le.amount_kes) FILTER (WHERE le.entry_type = 'rent_charge'), 0) AS charged_kes,
                COALESCE(SUM(le.amount_kes) FILTER (WHERE le.entry_type = 'payment'), 0)     AS collected_kes,
                COALESCE(SUM(le.amount_kes) FILTER (WHERE le.entry_type = 'late_fee'), 0)    AS late_fees_kes
            FROM   ledger_entries le
            LEFT JOIN agreements a ON a.id = le.agreement_id
            WHERE  le.posted_at BETWEEN $1 AND $2
              AND ($3::uuid IS NULL OR a.property_id = $3)
            GROUP  BY 1
            ORDER  BY 1
            "#,
        )
        .bind(query.period_start)
        .bind(query.period_end)
        .bind(query.property_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))
    }

    async fn fetch_monthly_occupancy(
        &self,
        query: &AnalyticsQuery,
    ) -> Result<Vec<MonthlyOccupancyRow>, AppError> {
        // We approximate monthly occupancy by snapshotting per-unit status.
        // For a production system you'd use a vacancy-event table.
        sqlx::query_as::<_, MonthlyOccupancyRow>(
            r#"
            SELECT
                gs.month                         AS month,
                (SELECT COUNT(*)::BIGINT FROM units u
                 JOIN properties p ON p.id = u.property_id
                 WHERE ($3::uuid IS NULL OR p.id = $3)) AS total_units,
                (SELECT COUNT(*)::BIGINT FROM units u
                 JOIN properties p ON p.id = u.property_id
                 WHERE u.status = 'occupied'
                   AND ($3::uuid IS NULL OR p.id = $3)) AS occupied_units
            FROM (
                SELECT generate_series(
                    DATE_TRUNC('month', $1::date),
                    DATE_TRUNC('month', $2::date),
                    '1 month'::interval
                )::DATE AS month
            ) gs
            ORDER BY gs.month
            "#,
        )
        .bind(query.period_start)
        .bind(query.period_end)
        .bind(query.property_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))
    }
}
