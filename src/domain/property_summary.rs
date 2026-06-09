use rust_decimal::Decimal;
use serde::Serialize;
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::dashboard::{ExpiringLease, MaintenanceSummary};

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PropertySummary {
    pub generated_at: OffsetDateTime,
    pub property_id: Uuid,
    pub name: String,
    pub slug: String,
    
    /// Statistics for this specific property.
    pub stats: PropertyStats,

    /// Leases expiring soon in this property.
    pub expiring_leases: Vec<ExpiringLease>,

    /// Open work orders for this property.
    pub pending_maintenance: Vec<MaintenanceSummary>,

    /// Rent collection for the current month for this property.
    pub rent_collection: PropertyRentSummary,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PropertyStats {
    pub total_units: i64,
    pub occupied_units: i64,
    pub vacant_units: i64,
    pub occupancy_rate_pct: Decimal,
    pub active_leases: i64,
    pub open_work_orders: i64,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PropertyRentSummary {
    pub total_expected_kes: Decimal,
    pub total_collected_kes: Decimal,
    pub outstanding_kes: Decimal,
    pub collection_rate_pct: Decimal,
}
