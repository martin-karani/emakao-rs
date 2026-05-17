use async_trait::async_trait;
use uuid::Uuid;

use crate::{application::errors::AppError, domain::dashboard::DashboardSummary};

pub struct DashboardQuery {
    pub agency_id: Uuid,
    /// How many days ahead to look for expiring leases.
    pub expiring_lease_days: i64,
    /// Maximum number of expiring leases to return.
    pub expiring_lease_limit: i64,
    /// Maximum number of pending maintenance items to return.
    pub pending_maintenance_limit: i64,
}

#[async_trait]
pub trait DashboardRepository: Send + Sync + 'static {
    /// Assemble the full dashboard summary in a single database round-trip
    /// (implemented as multiple queries in the same pool connection).
    async fn get_summary(&self, query: DashboardQuery) -> Result<DashboardSummary, AppError>;
}
