use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::application::{
    errors::AppError,
    ports::insight_repository::{
        InsightRepository, MaintenancePatternData, MonthlyActualData, RentDefaultRiskData,
        TenantChurnData, VendorPerformanceData,
    },
};

pub struct PgInsightRepo {
    pool: PgPool,
}

impl PgInsightRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl InsightRepository for PgInsightRepo {
    async fn get_rent_default_stats(
        &self,
        agency_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<RentDefaultRiskData>, AppError> {
        let rows = sqlx::query(
            r#"
            SELECT
                a.id                AS agreement_id,
                r.id                AS resident_id,
                (r.first_name || ' ' || r.last_name) AS resident_name,
                u.id                AS unit_id,
                u.unit_number,
                p.id                AS property_id,
                p.name              AS property_name,
                a.rent_amount_kes,

                COALESCE(
                    (SELECT SUM(amount_kes) FROM ledger_entries
                     WHERE agreement_id = a.id
                       AND entry_type IN ('rent'::ledger_entry_type, 'late_fee'::ledger_entry_type, 'maintenance_charge'::ledger_entry_type)
                    ), 0
                ) - COALESCE(
                    (SELECT SUM(amount_kes) FROM ledger_entries
                     WHERE agreement_id = a.id
                       AND entry_type IN ('payment_mpesa'::ledger_entry_type, 'payment_bank'::ledger_entry_type, 'payment_cash'::ledger_entry_type)
                    ), 0
                ) AS outstanding_kes,

                (
                    SELECT COUNT(*) FROM late_fee_charges
                    WHERE agreement_id = a.id
                      AND charged_at > now() - INTERVAL '6 months'
                ) AS late_fee_count_6m,

                COALESCE(
                    (SELECT AVG(EXTRACT(EPOCH FROM (le.posted_at - rc.period_end)) / 86400)
                     FROM ledger_entries le
                     JOIN rent_charges rc ON rc.agreement_id = le.agreement_id
                     WHERE le.agreement_id = a.id
                       AND le.entry_type IN ('payment_mpesa'::ledger_entry_type, 'payment_bank'::ledger_entry_type, 'payment_cash'::ledger_entry_type)
                       AND le.posted_at > rc.period_end
                    ), 0
                ) AS avg_payment_delay_days,

                (
                    SELECT MAX(posted_at) FROM ledger_entries
                    WHERE agreement_id = a.id
                      AND entry_type IN ('payment_mpesa'::ledger_entry_type, 'payment_bank'::ledger_entry_type, 'payment_cash'::ledger_entry_type)
                ) AS last_payment_at
            FROM agreements a
            JOIN residents  r ON r.id = a.resident_id
            JOIN units      u ON u.id = a.unit_id
            JOIN properties p ON p.id = a.property_id
            WHERE a.agency_id = $1
              AND a.status::text = 'active'
            ORDER BY a.created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(agency_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        use rust_decimal::Decimal;
        use sqlx::Row;

        Ok(rows
            .into_iter()
            .map(|r| RentDefaultRiskData {
                agreement_id: r.get("agreement_id"),
                resident_id: r.get("resident_id"),
                resident_name: r
                    .get::<Option<String>, _>("resident_name")
                    .unwrap_or_else(|| "Unknown".to_string()),
                unit_id: r.get("unit_id"),
                unit_number: r.get("unit_number"),
                property_id: r.get("property_id"),
                property_name: r.get("property_name"),
                rent_amount_kes: r.get("rent_amount_kes"),
                outstanding_kes: r
                    .get::<Option<Decimal>, _>("outstanding_kes")
                    .unwrap_or_default(),
                late_fee_count_6m: r
                    .get::<Option<i64>, _>("late_fee_count_6m")
                    .unwrap_or_default(),
                avg_payment_delay_days: r
                    .get::<Option<f64>, _>("avg_payment_delay_days")
                    .unwrap_or_default(),
                last_payment_at: r.get("last_payment_at"),
            })
            .collect())
    }

    async fn get_tenant_churn_stats(
        &self,
        agency_id: Uuid,
    ) -> Result<Vec<TenantChurnData>, AppError> {
        let rows = sqlx::query(
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
                     WHERE agreement_id = a.id AND entry_type IN ('rent'::ledger_entry_type, 'late_fee'::ledger_entry_type, 'deposit_charge'::ledger_entry_type))
                    -
                    (SELECT COALESCE(SUM(amount_kes),0) FROM ledger_entries
                     WHERE agreement_id = a.id AND entry_type IN ('payment_mpesa'::ledger_entry_type, 'payment_bank'::ledger_entry_type, 'payment_cash'::ledger_entry_type)),
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
                 WHERE agreement_id = a.id AND entry_type IN ('payment_mpesa'::ledger_entry_type, 'payment_bank'::ledger_entry_type, 'payment_cash'::ledger_entry_type)
                )::BIGINT AS days_since_last_payment

            FROM agreements a
            JOIN residents  r ON r.id = a.resident_id
            JOIN units      u ON u.id = a.unit_id
            JOIN properties p ON p.id = a.property_id
            WHERE a.agency_id = $1
              AND a.status::text = 'active'
              AND a.end_date IS NOT NULL
            "#,
        )
        .bind(agency_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        use rust_decimal::Decimal;
        use sqlx::Row;

        Ok(rows
            .into_iter()
            .map(|r| TenantChurnData {
                agreement_id: r.get("agreement_id"),
                resident_id: r.get("resident_id"),
                resident_name: r
                    .get::<Option<String>, _>("resident_name")
                    .unwrap_or_else(|| "Unknown".to_string()),
                unit_id: r.get("unit_id"),
                unit_number: r.get("unit_number"),
                property_id: r.get("property_id"),
                property_name: r.get("property_name"),
                end_date: r.get("end_date"),
                days_until_expiry: r.get::<Option<i64>, _>("days_until_expiry"),
                outstanding_kes: r
                    .get::<Option<Decimal>, _>("outstanding_kes")
                    .unwrap_or_default(),
                late_payments_12m: r
                    .get::<Option<i64>, _>("late_payments_12m")
                    .unwrap_or_default(),
                prior_agreements: r
                    .get::<Option<i64>, _>("prior_agreements")
                    .unwrap_or_default(),
                days_since_last_payment: r.get("days_since_last_payment"),
            })
            .collect())
    }

    async fn get_maintenance_patterns(
        &self,
        agency_id: Uuid,
        min_count: i64,
    ) -> Result<Vec<MaintenancePatternData>, AppError> {
        let rows = sqlx::query(
            r#"
            SELECT
                u.id                    AS unit_id,
                u.unit_number,
                p.id                    AS property_id,
                p.name                  AS property_name,
                wo.category::text       AS category,
                COUNT(*)                AS count_12m,
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
            WHERE  p.agency_id = $1
              AND  wo.created_at >= now() - INTERVAL '12 months'
              AND  wo.unit_id IS NOT NULL
            GROUP  BY u.id, u.unit_number, p.id, p.name, wo.category
            HAVING COUNT(*) >= $2
            ORDER  BY COUNT(*) DESC
            "#,
        )
        .bind(agency_id)
        .bind(min_count)
        .fetch_all(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        use sqlx::Row;

        Ok(rows
            .into_iter()
            .map(|r| MaintenancePatternData {
                unit_id: r.get("unit_id"),
                unit_number: r.get("unit_number"),
                property_id: r.get("property_id"),
                property_name: r.get("property_name"),
                category: r.get("category"),
                count_12m: r.get("count_12m"),
                avg_interval_days: r.get("avg_interval_days"),
                last_occurred: r.get("last_occurred"),
            })
            .collect())
    }

    async fn get_expense_baseline(
        &self,
        agency_id: Uuid,
        property_id: Option<Uuid>,
    ) -> Result<Vec<MonthlyActualData>, AppError> {
        let rows = sqlx::query(
            r#"
            SELECT
                DATE_TRUNC('month', wo.completed_at)::DATE AS month,
                COALESCE(SUM(wo.actual_cost_kes), 0)        AS maintenance_kes,
                0::NUMERIC(14,2)                            AS utility_kes
            FROM work_orders wo
            JOIN properties p ON p.id = wo.property_id
            WHERE p.agency_id = $1
              AND wo.completed_at >= now() - INTERVAL '12 months'
              AND ($2::uuid IS NULL OR p.id = $2)
            GROUP BY 1
            ORDER BY 1
            "#,
        )
        .bind(agency_id)
        .bind(property_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        use sqlx::Row;

        Ok(rows
            .into_iter()
            .map(|r| MonthlyActualData {
                month: r.get("month"),
                maintenance_kes: r.get("maintenance_kes"),
                utility_kes: r.get("utility_kes"),
            })
            .collect())
    }

    async fn get_vendor_performance(
        &self,
        agency_id: Uuid,
        category: &str,
    ) -> Result<Vec<VendorPerformanceData>, AppError> {
        let rows = sqlx::query(
            r#"
            SELECT
                v.id                AS vendor_id,
                v.name              AS vendor_name,
                v.phone,
                v.email,
                COUNT(wo.id)        AS completed_jobs,
                AVG(
                    EXTRACT(EPOCH FROM (wo.completed_at - wo.created_at)) / 86400
                )::NUMERIC(10,1)   AS avg_resolution_days,
                AVG(wo.actual_cost_kes)::NUMERIC(14,2) AS avg_cost_kes,
                EXTRACT(EPOCH FROM (now() - MAX(wo.completed_at))) / 86400 AS last_job_days_ago
            FROM   vendors   v
            LEFT JOIN work_orders wo
                   ON wo.vendor_id = v.id
                  AND wo.status::text = 'completed'
                  AND wo.category  = $2::text::work_order_category
            WHERE  v.status::text = 'active'
              AND  v.agency_id = $1
            GROUP  BY v.id, v.name, v.phone, v.email
            ORDER  BY completed_jobs DESC
            "#,
        )
        .bind(agency_id)
        .bind(category)
        .fetch_all(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        use sqlx::Row;

        Ok(rows
            .into_iter()
            .map(|r| VendorPerformanceData {
                vendor_id: r.get("vendor_id"),
                vendor_name: r.get("vendor_name"),
                phone: r.get("phone"),
                email: r.get("email"),
                completed_jobs: r.get("completed_jobs"),
                avg_resolution_days: r.get("avg_resolution_days"),
                avg_cost_kes: r.get("avg_cost_kes"),
                last_job_days_ago: r
                    .get::<Option<f64>, _>("last_job_days_ago")
                    .map(|f| f as i64),
            })
            .collect())
    }
}
