// src/domain/analytics.rs
//
// Analytics & Reporting domain value objects.
// These are read-model types returned by analytical queries — no mutations.

use rust_decimal::Decimal;
use serde::Serialize;
use time::{Date, OffsetDateTime};
use utoipa::ToSchema;
use uuid::Uuid;

// ── Portfolio-level analytics ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PortfolioAnalytics {
    pub generated_at: OffsetDateTime,
    pub period_start: Date,
    pub period_end: Date,

    pub occupancy: OccupancyAnalytics,
    pub revenue: RevenueAnalytics,
    pub maintenance: MaintenanceAnalytics,
    pub top_properties: Vec<PropertyPerformance>,
}

// ── Occupancy ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct OccupancyAnalytics {
    pub total_units: i64,
    pub occupied_units: i64,
    pub vacant_units: i64,
    pub occupancy_rate_pct: Decimal,
    /// Average number of days a unit stays vacant before re-letting.
    pub avg_vacancy_days: Decimal,
    /// Month-by-month occupancy rate for the period.
    pub monthly_trend: Vec<MonthlyOccupancyPoint>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MonthlyOccupancyPoint {
    /// First day of the month.
    pub month: Date,
    pub occupied_units: i64,
    pub total_units: i64,
    pub rate_pct: Decimal,
}

// ── Revenue ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RevenueAnalytics {
    pub total_charged_kes: Decimal,
    pub total_collected_kes: Decimal,
    pub total_outstanding_kes: Decimal,
    pub collection_rate_pct: Decimal,
    pub total_late_fees_kes: Decimal,
    pub total_maintenance_cost_kes: Decimal,
    /// Net operating income = collected - maintenance costs.
    pub net_operating_income_kes: Decimal,
    /// Month-by-month revenue trend for the period.
    pub monthly_trend: Vec<MonthlyRevenuePoint>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MonthlyRevenuePoint {
    pub month: Date,
    pub charged_kes: Decimal,
    pub collected_kes: Decimal,
    pub outstanding_kes: Decimal,
    pub late_fees_kes: Decimal,
}

// ── Maintenance ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MaintenanceAnalytics {
    pub total_work_orders: i64,
    pub open_work_orders: i64,
    pub completed_work_orders: i64,
    pub avg_resolution_days: Decimal,
    pub total_actual_cost_kes: Decimal,
    pub total_estimated_cost_kes: Decimal,
    /// Breakdown by category (plumbing, electrical, etc.).
    pub by_category: Vec<MaintenanceCategoryBreakdown>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MaintenanceCategoryBreakdown {
    pub category: String,
    pub count: i64,
    pub total_cost_kes: Decimal,
    pub avg_resolution_days: Decimal,
}

// ── Property performance ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PropertyPerformance {
    pub property_id: Uuid,
    pub property_name: String,
    pub city: String,
    pub units: i64,
    pub occupancy_rate_pct: Decimal,
    pub revenue_kes: Decimal,
    pub outstanding_kes: Decimal,
    pub maintenance_cost_kes: Decimal,
    /// (revenue - maintenance_cost) / revenue × 100
    pub roi_pct: Decimal,
}

// ── Occupancy trend (time-series) ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct OccupancyTrendReport {
    pub generated_at: OffsetDateTime,
    pub property_id: Option<Uuid>, // None = all properties
    pub months: Vec<MonthlyOccupancyPoint>,
}

// ── Revenue report ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RevenueReport {
    pub generated_at: OffsetDateTime,
    pub period_start: Date,
    pub period_end: Date,
    pub property_id: Option<Uuid>,
    pub months: Vec<MonthlyRevenuePoint>,
    pub summary: RevenueAnalytics,
}
