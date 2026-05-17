//
// Portfolio-level dashboard aggregates. These types are assembled by
// `DashboardRepository::get_summary` via a set of SQL queries and returned
// to the staff dashboard in a single response.

use rust_decimal::Decimal;
use serde::Serialize;
use time::{Date, OffsetDateTime};
use utoipa::ToSchema;
use uuid::Uuid;

// ── Top-level summary ─────────────────────────────────────────────────────────

/// Everything the multi-property dashboard needs in one struct.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct DashboardSummary {
    pub generated_at: OffsetDateTime,

    /// High-level portfolio numbers.
    pub portfolio: PortfolioStats,

    /// Per-property occupancy snapshot.
    pub properties: Vec<PropertyOccupancySummary>,

    /// Leases expiring within the next `days_ahead` days.
    pub expiring_leases: Vec<ExpiringLease>,

    /// Open/in-progress work orders that need attention.
    pub pending_maintenance: Vec<MaintenanceSummary>,

    /// Rent collection totals for the current calendar month.
    pub rent_collection: RentCollectionSummary,
}

// ── Portfolio-level numbers ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PortfolioStats {
    pub total_properties: i64,
    pub total_units: i64,
    pub occupied_units: i64,
    pub vacant_units: i64,
    /// 0.0 – 100.0
    pub occupancy_rate_pct: Decimal,
    pub total_active_leases: i64,
    pub total_open_work_orders: i64,
}

// ── Per-property snapshot ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PropertyOccupancySummary {
    pub property_id: Uuid,
    pub property_name: String,
    pub city: String,
    pub total_units: i64,
    pub occupied_units: i64,
    pub vacant_units: i64,
    /// 0.0 – 100.0
    pub occupancy_rate_pct: Decimal,
    pub open_work_orders: i64,
    /// KES collected in the current calendar month.
    pub rent_collected_this_month_kes: Decimal,
}

// ── Expiring leases ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ExpiringLease {
    pub agreement_id: Uuid,
    pub unit_id: Uuid,
    pub unit_number: String,
    pub property_id: Uuid,
    pub property_name: String,
    pub resident_id: Uuid,
    pub resident_name: String,
    pub end_date: Date,
    /// Number of calendar days until `end_date` (can be negative if past).
    pub days_until_expiry: i64,
    pub rent_amount_kes: Decimal,
}

// ── Maintenance summary ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MaintenanceSummary {
    pub work_order_id: Uuid,
    pub code: String,
    pub title: String,
    pub priority: String,
    pub status: String,
    pub property_id: Uuid,
    pub property_name: String,
    pub unit_id: Option<Uuid>,
    pub unit_number: Option<String>,
    pub created_at: OffsetDateTime,
    pub days_open: i64,
}

// ── Rent collection ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RentCollectionSummary {
    /// Calendar month being summarised (first day of month).
    pub period_start: Date,
    /// Total rent charged (from `rent_charges`) this month.
    pub total_charged_kes: Decimal,
    /// Total payments received this month.
    pub total_collected_kes: Decimal,
    /// `total_charged - total_collected`.
    pub outstanding_kes: Decimal,
    /// Number of agreements with at least one overdue charge.
    pub overdue_count: i64,
    /// Collection rate 0.0 – 100.0 (collected / charged × 100).
    pub collection_rate_pct: Decimal,
}
