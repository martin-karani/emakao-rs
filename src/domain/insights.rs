//
// Rule-based predictive insight models.
// These are read-model value objects — no side effects.
// The scoring algorithms live in use-case layer; this module defines the
// shapes returned to callers.

use rust_decimal::Decimal;
use serde::Serialize;
use time::{Date, OffsetDateTime};
use utoipa::ToSchema;
use uuid::Uuid;

// ── Risk levels ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl RiskLevel {
    pub fn from_score(score: u8) -> Self {
        match score {
            0..=30 => Self::Low,
            31..=59 => Self::Medium,
            60..=79 => Self::High,
            _ => Self::Critical,
        }
    }
}

// ── 1. Rent Default Risk ──────────────────────────────────────────────────────

/// Risk score per active agreement.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RentDefaultRiskScore {
    pub agreement_id: Uuid,
    pub resident_id: Uuid,
    pub resident_name: String,
    pub unit_id: Uuid,
    pub unit_number: String,
    pub property_id: Uuid,
    pub property_name: String,

    /// 0 – 100 composite risk score (higher = more risky).
    pub score: u8,
    pub risk_level: RiskLevel,

    /// Human-readable factors that drove the score.
    pub risk_factors: Vec<String>,

    /// Current outstanding balance in KES.
    pub outstanding_kes: Decimal,
    /// Number of times tenant paid late in the last 6 months.
    pub late_payments_6m: i64,
    /// Days since last payment (None if never paid).
    pub days_since_last_payment: Option<i64>,

    pub generated_at: OffsetDateTime,
}

// ── 2. Tenant Churn Prediction ────────────────────────────────────────────────

/// Churn risk per active agreement approaching renewal.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct TenantChurnPrediction {
    pub agreement_id: Uuid,
    pub resident_id: Uuid,
    pub resident_name: String,
    pub unit_id: Uuid,
    pub unit_number: String,
    pub property_id: Uuid,
    pub property_name: String,

    /// Days until lease end (negative if already past).
    pub days_until_expiry: i64,
    pub lease_end_date: Date,

    /// 0 – 100 churn probability score (higher = more likely to leave).
    pub churn_score: u8,
    pub risk_level: RiskLevel,

    /// Recommended action for the property manager.
    pub recommended_action: String,
    pub churn_factors: Vec<String>,

    pub generated_at: OffsetDateTime,
}

// ── 3. Predictive Maintenance Alerts ─────────────────────────────────────────

/// Maintenance risk per unit based on historical work-order patterns.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PredictiveMaintenanceAlert {
    pub unit_id: Uuid,
    pub unit_number: String,
    pub property_id: Uuid,
    pub property_name: String,

    /// Category predicted to need attention (e.g. "plumbing", "electrical").
    pub predicted_category: String,
    /// 0 – 100 likelihood of a work order in next 30 days.
    pub likelihood_score: u8,
    pub risk_level: RiskLevel,

    /// Work orders in this category in the last 12 months.
    pub historical_count: i64,
    /// Average days between work orders of this category.
    pub avg_interval_days: Decimal,
    /// Date of most recent work order in this category.
    pub last_occurred: Option<Date>,
    /// Estimated days until next likely issue.
    pub estimated_days_until_next: Option<i64>,

    pub generated_at: OffsetDateTime,
}

// ── 4. Expense Forecast ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ExpenseForecast {
    pub property_id: Option<Uuid>, // None = portfolio-wide
    pub generated_at: OffsetDateTime,

    /// Forecast horizon in months.
    pub horizon_months: u8,
    /// Monthly forecast entries.
    pub months: Vec<MonthlyExpenseForecast>,

    /// 12-month rolling actual cost (baseline).
    pub trailing_12m_actual_kes: Decimal,
    /// Forecast total over the horizon period.
    pub forecast_total_kes: Decimal,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MonthlyExpenseForecast {
    pub month: Date,
    pub forecasted_maintenance_kes: Decimal,
    pub forecasted_utility_kes: Decimal,
    pub forecasted_total_kes: Decimal,
    /// Confidence interval lower bound.
    pub lower_bound_kes: Decimal,
    /// Confidence interval upper bound.
    pub upper_bound_kes: Decimal,
}

// ── 5. Smart Vendor Allocation ────────────────────────────────────────────────

/// Ranked vendor recommendations for a given work-order category.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct VendorAllocationRecommendation {
    pub work_order_id: Uuid,
    pub category: String,
    pub generated_at: OffsetDateTime,
    pub recommendations: Vec<VendorScore>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct VendorScore {
    pub vendor_id: Uuid,
    pub vendor_name: String,
    pub phone: Option<String>,
    pub email: Option<String>,

    /// Composite score 0–100 (higher = better fit).
    pub score: u8,

    /// Component scores driving the composite.
    pub completed_jobs: i64,
    pub avg_resolution_days: Decimal,
    pub cost_efficiency_score: u8, // 0-100, lower cost relative to peers = higher score
    pub recency_score: u8,         // 0-100, more recently active = higher
    pub category_specialisation_score: u8, // 0-100
}
