// src/infrastructure/db/dashboard_repository_sqlx.rs

use async_trait::async_trait;
use rust_decimal::Decimal;
use sqlx::PgPool;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        ports::dashboard_repository::{DashboardQuery, DashboardRepository},
    },
    domain::dashboard::{
        DashboardSummary, ExpiringLease, MaintenanceSummary, PortfolioStats,
        PropertyOccupancySummary, RentCollectionSummary,
    },
};

pub struct PgDashboardRepo {
    pool: PgPool,
}

impl PgDashboardRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

// ── Internal row types ────────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct PortfolioRow {
    total_properties: i64,
    total_units: i64,
    occupied_units: i64,
    total_active_leases: i64,
    total_open_work_orders: i64,
}

#[derive(sqlx::FromRow)]
struct PropertyOccupancyRow {
    property_id: Uuid,
    property_name: String,
    city: String,
    total_units: i64,
    occupied_units: i64,
    open_work_orders: i64,
    rent_collected_this_month_kes: Decimal,
}

#[derive(sqlx::FromRow)]
struct ExpiringLeaseRow {
    agreement_id: Uuid,
    unit_id: Uuid,
    unit_number: String,
    property_id: Uuid,
    property_name: String,
    resident_id: Uuid,
    resident_name: String,
    end_date: Date,
    days_until_expiry: i64,
    rent_amount_kes: Decimal,
}

#[derive(sqlx::FromRow)]
struct MaintenanceRow {
    work_order_id: Uuid,
    code: String,
    title: String,
    priority: String,
    status: String,
    property_id: Uuid,
    property_name: String,
    unit_id: Option<Uuid>,
    unit_number: Option<String>,
    created_at: OffsetDateTime,
    days_open: i64,
}

#[derive(sqlx::FromRow)]
struct RentCollectionRow {
    total_charged_kes: Decimal,
    total_collected_kes: Decimal,
    overdue_count: i64,
}

// ── Repository impl ───────────────────────────────────────────────────────────

#[async_trait]
impl DashboardRepository for PgDashboardRepo {
    async fn get_summary(&self, query: DashboardQuery) -> Result<DashboardSummary, AppError> {
        let generated_at = OffsetDateTime::now_utc();

        // ── Portfolio stats ───────────────────────────────────────────────────
        let portfolio_row = sqlx::query_as!(
            PortfolioRow,
            r#"
            SELECT
                (SELECT COUNT(*) FROM properties)::BIGINT                                AS "total_properties!",
                (SELECT COUNT(*) FROM units)::BIGINT                                     AS "total_units!",
                (SELECT COUNT(*) FROM units WHERE status = 'occupied')::BIGINT           AS "occupied_units!",
                (SELECT COUNT(*) FROM agreements WHERE status = 'active')::BIGINT        AS "total_active_leases!",
                (SELECT COUNT(*) FROM work_orders WHERE status NOT IN ('completed','cancelled'))::BIGINT AS "total_open_work_orders!"
            "#
        )
        .fetch_one(&self.pool)
        .await?;

        let total_units = portfolio_row.total_units.max(1);
        let occupancy_rate =
            Decimal::from(portfolio_row.occupied_units * 100) / Decimal::from(total_units);

        let portfolio = PortfolioStats {
            total_properties: portfolio_row.total_properties,
            total_units: portfolio_row.total_units,
            occupied_units: portfolio_row.occupied_units,
            vacant_units: portfolio_row.total_units - portfolio_row.occupied_units,
            occupancy_rate_pct: occupancy_rate,
            total_active_leases: portfolio_row.total_active_leases,
            total_open_work_orders: portfolio_row.total_open_work_orders,
        };

        // ── Per-property occupancy ────────────────────────────────────────────
        let prop_rows = sqlx::query_as!(
            PropertyOccupancyRow,
            r#"
            SELECT
                p.id                    AS property_id,
                p.name                  AS property_name,
                p.city,
                COUNT(u.id)             AS "total_units!",
                COUNT(u.id) FILTER (WHERE u.status = 'occupied') AS "occupied_units!",
                (SELECT COUNT(*) FROM work_orders wo
                 WHERE wo.property_id = p.id
                   AND wo.status NOT IN ('completed','cancelled')) AS "open_work_orders!",
                COALESCE(
                    (SELECT SUM(le.amount_kes)
                     FROM   ledger_entries le
                     JOIN   agreements a ON a.id = le.agreement_id
                     WHERE  a.property_id = p.id
                       AND  le.entry_type = 'payment'
                       AND  le.posted_at >= DATE_TRUNC('month', now())
                    ), 0
                )                       AS "rent_collected_this_month_kes!"
            FROM  properties p
            LEFT JOIN units u ON u.property_id = p.id
            GROUP BY p.id, p.name, p.city
            ORDER BY p.name
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        let properties: Vec<PropertyOccupancySummary> = prop_rows
            .into_iter()
            .map(|r| {
                let rate = if r.total_units > 0 {
                    Decimal::from(r.occupied_units * 100) / Decimal::from(r.total_units)
                } else {
                    Decimal::ZERO
                };
                PropertyOccupancySummary {
                    property_id: r.property_id,
                    property_name: r.property_name,
                    city: r.city,
                    total_units: r.total_units,
                    occupied_units: r.occupied_units,
                    vacant_units: r.total_units - r.occupied_units,
                    occupancy_rate_pct: rate,
                    open_work_orders: r.open_work_orders,
                    rent_collected_this_month_kes: r.rent_collected_this_month_kes,
                }
            })
            .collect();

        // ── Expiring leases ───────────────────────────────────────────────────
        let lease_rows = sqlx::query_as!(
            ExpiringLeaseRow,
            r#"
            SELECT
                a.id                                          AS agreement_id,
                u.id                                          AS unit_id,
                u.unit_number,
                p.id                                          AS property_id,
                p.name                                        AS property_name,
                r.id                                          AS resident_id,
                (r.first_name || ' ' || r.last_name)          AS "resident_name!",
                a.end_date                                    AS "end_date!",
                (a.end_date - CURRENT_DATE)::BIGINT           AS "days_until_expiry!",
                a.rent_amount_kes
            FROM  agreements a
            JOIN  units       u ON u.id = a.unit_id
            JOIN  properties  p ON p.id = a.property_id
            JOIN  residents   r ON r.id = a.resident_id
            WHERE a.status = 'active'
              AND a.end_date IS NOT NULL
              AND a.end_date <= CURRENT_DATE + ($1 || ' days')::INTERVAL
            ORDER BY a.end_date ASC
            LIMIT $2
            "#,
            query.expiring_lease_days.to_string(),
            query.expiring_lease_limit
        )
        .fetch_all(&self.pool)
        .await?;

        let expiring_leases: Vec<ExpiringLease> = lease_rows
            .into_iter()
            .map(|r| ExpiringLease {
                agreement_id: r.agreement_id,
                unit_id: r.unit_id,
                unit_number: r.unit_number,
                property_id: r.property_id,
                property_name: r.property_name,
                resident_id: r.resident_id,
                resident_name: r.resident_name,
                end_date: r.end_date,
                days_until_expiry: r.days_until_expiry,
                rent_amount_kes: r.rent_amount_kes,
            })
            .collect();

        // ── Pending maintenance ───────────────────────────────────────────────
        let maint_rows = sqlx::query_as!(
            MaintenanceRow,
            r#"
            SELECT
                wo.id               AS work_order_id,
                wo.code,
                wo.title,
                wo.priority::text   AS "priority!",
                wo.status::text     AS "status!",
                p.id                AS property_id,
                p.name              AS property_name,
                u.id                AS unit_id,
                u.unit_number,
                wo.created_at,
                EXTRACT(EPOCH FROM (now() - wo.created_at)) / 86400 AS "days_open!"
            FROM  work_orders wo
            JOIN  properties  p ON p.id = wo.property_id
            LEFT JOIN units   u ON u.id = wo.unit_id
            WHERE wo.status NOT IN ('completed','cancelled')
            ORDER BY
                CASE wo.priority
                    WHEN 'emergency' THEN 0
                    WHEN 'high'      THEN 1
                    WHEN 'medium'    THEN 2
                    ELSE 3
                END,
                wo.created_at ASC
            LIMIT $1
            "#,
            query.pending_maintenance_limit
        )
        .fetch_all(&self.pool)
        .await?;

        let pending_maintenance: Vec<MaintenanceSummary> = maint_rows
            .into_iter()
            .map(|r| MaintenanceSummary {
                work_order_id: r.work_order_id,
                code: r.code,
                title: r.title,
                priority: r.priority,
                status: r.status,
                property_id: r.property_id,
                property_name: r.property_name,
                unit_id: r.unit_id,
                unit_number: r.unit_number,
                created_at: r.created_at,
                days_open: r.days_open as i64,
            })
            .collect();

        // ── Rent collection (current month) ───────────────────────────────────
        let rc = sqlx::query_as!(
            RentCollectionRow,
            r#"
            SELECT
                COALESCE(
                    (SELECT SUM(rc.amount_kes) FROM rent_charges rc
                     WHERE rc.charged_at >= DATE_TRUNC('month', now())),
                    0
                ) AS "total_charged_kes!",
                COALESCE(
                    (SELECT SUM(le.amount_kes)
                     FROM   ledger_entries le
                     WHERE  le.entry_type = 'payment'
                       AND  le.posted_at >= DATE_TRUNC('month', now())),
                    0
                ) AS "total_collected_kes!",
                (SELECT COUNT(DISTINCT rc2.agreement_id)
                 FROM   rent_charges rc2
                 LEFT JOIN ledger_entries le2
                        ON le2.agreement_id = rc2.agreement_id
                       AND le2.entry_type = 'payment'
                       AND le2.posted_at >= rc2.charged_at
                 WHERE  rc2.charged_at >= DATE_TRUNC('month', now()) - INTERVAL '7 days'
                   AND  le2.id IS NULL
                )::BIGINT AS "overdue_count!"
            "#
        )
        .fetch_one(&self.pool)
        .await?;

        let outstanding = (rc.total_charged_kes - rc.total_collected_kes).max(Decimal::ZERO);
        let collection_rate = if rc.total_charged_kes > Decimal::ZERO {
            rc.total_collected_kes * Decimal::from(100) / rc.total_charged_kes
        } else {
            Decimal::from(100)
        };

        let period_start = {
            let now_date = generated_at.date();
            Date::from_calendar_date(now_date.year(), now_date.month(), 1).unwrap_or(now_date)
        };

        let rent_collection = RentCollectionSummary {
            period_start,
            total_charged_kes: rc.total_charged_kes,
            total_collected_kes: rc.total_collected_kes,
            outstanding_kes: outstanding,
            overdue_count: rc.overdue_count,
            collection_rate_pct: collection_rate,
        };

        Ok(DashboardSummary {
            generated_at,
            portfolio,
            properties,
            expiring_leases,
            pending_maintenance,
            rent_collection,
        })
    }
}
