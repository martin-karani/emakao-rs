// src/application/use_cases/dashboard/get_dashboard.rs

use std::sync::Arc;

use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        ports::dashboard_repository::{DashboardQuery, DashboardRepository},
    },
    domain::dashboard::DashboardSummary,
};

pub struct GetDashboardInput {
    pub agency_id: Uuid,
    /// Days ahead to surface expiring leases (default 60).
    pub expiring_lease_days: Option<i64>,
}

pub struct GetDashboardUseCase {
    pub repo: Arc<dyn DashboardRepository>,
}

impl GetDashboardUseCase {
    pub fn new(repo: Arc<dyn DashboardRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, input: GetDashboardInput) -> Result<DashboardSummary, AppError> {
        self.repo
            .get_summary(DashboardQuery {
                agency_id: input.agency_id,
                expiring_lease_days: input.expiring_lease_days.unwrap_or(60),
                expiring_lease_limit: 20,
                pending_maintenance_limit: 20,
            })
            .await
    }
}
