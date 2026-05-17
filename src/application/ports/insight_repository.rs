use async_trait::async_trait;
use rust_decimal::Decimal;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::application::errors::AppError;

pub struct RentDefaultRiskData {
    pub agreement_id: Uuid,
    pub resident_id: Uuid,
    pub resident_name: String,
    pub unit_id: Uuid,
    pub unit_number: String,
    pub property_id: Uuid,
    pub property_name: String,
    pub rent_amount_kes: Decimal,
    pub outstanding_kes: Decimal,
    pub late_fee_count_6m: i64,
    pub avg_payment_delay_days: f64,
    pub last_payment_at: Option<OffsetDateTime>,
}

pub struct TenantChurnData {
    pub agreement_id: Uuid,
    pub resident_id: Uuid,
    pub resident_name: String,
    pub unit_id: Uuid,
    pub unit_number: String,
    pub property_id: Uuid,
    pub property_name: String,
    pub end_date: Option<Date>,
    pub days_until_expiry: Option<i64>,
    pub outstanding_kes: Decimal,
    pub late_payments_12m: i64,
    pub prior_agreements: i64,
    pub days_since_last_payment: Option<i64>,
}

pub struct MaintenancePatternData {
    pub unit_id: Uuid,
    pub unit_number: String,
    pub property_id: Uuid,
    pub property_name: String,
    pub category: String,
    pub count_12m: i64,
    pub avg_interval_days: Option<Decimal>,
    pub last_occurred: Option<Date>,
}

pub struct MonthlyActualData {
    pub month: Date,
    pub maintenance_kes: Decimal,
    pub utility_kes: Decimal,
}

pub struct VendorPerformanceData {
    pub vendor_id: Uuid,
    pub vendor_name: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub completed_jobs: i64,
    pub avg_resolution_days: Option<Decimal>,
    pub avg_cost_kes: Option<Decimal>,
    pub last_job_days_ago: Option<i64>,
}

#[async_trait]
pub trait InsightRepository: Send + Sync + 'static {
    /// Fetch raw stats for rent default risk scoring.
    async fn get_rent_default_stats(
        &self,
        agency_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<RentDefaultRiskData>, AppError>;

    /// Fetch raw stats for tenant churn prediction.
    async fn get_tenant_churn_stats(
        &self,
        agency_id: Uuid,
    ) -> Result<Vec<TenantChurnData>, AppError>;

    /// Fetch maintenance patterns for predictive alerts.
    async fn get_maintenance_patterns(
        &self,
        agency_id: Uuid,
        min_count: i64,
    ) -> Result<Vec<MaintenancePatternData>, AppError>;

    /// Fetch 12 months of actual costs for expense forecasting.
    async fn get_expense_baseline(
        &self,
        agency_id: Uuid,
        property_id: Option<Uuid>,
    ) -> Result<Vec<MonthlyActualData>, AppError>;

    /// Fetch vendor performance stats for smart allocation.
    async fn get_vendor_performance(
        &self,
        agency_id: Uuid,
        category: &str,
    ) -> Result<Vec<VendorPerformanceData>, AppError>;
}
