use std::sync::Arc;
use time::Date;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        ports::analytics_repository::{AnalyticsQuery, AnalyticsRepository},
    },
    domain::analytics::PortfolioAnalytics,
};

pub struct PortfolioAnalyticsInput {
    pub agency_id: Uuid,
    pub period_start: Date,
    pub period_end: Date,
    pub property_id: Option<Uuid>,
}

pub struct PortfolioAnalyticsUseCase {
    pub repo: Arc<dyn AnalyticsRepository>,
}

impl PortfolioAnalyticsUseCase {
    pub async fn execute(
        &self,
        input: PortfolioAnalyticsInput,
    ) -> Result<PortfolioAnalytics, AppError> {
        self.repo
            .portfolio_analytics(AnalyticsQuery {
                agency_id: input.agency_id,
                period_start: input.period_start,
                period_end: input.period_end,
                property_id: input.property_id,
            })
            .await
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Revenue Report use case
// ─────────────────────────────────────────────────────────────────────────────

use crate::domain::analytics::RevenueReport;

pub struct RevenueReportInput {
    pub agency_id: Uuid,
    pub period_start: Date,
    pub period_end: Date,
    pub property_id: Option<Uuid>,
}

pub struct RevenueReportUseCase {
    pub repo: Arc<dyn AnalyticsRepository>,
}

impl RevenueReportUseCase {
    pub async fn execute(&self, input: RevenueReportInput) -> Result<RevenueReport, AppError> {
        self.repo
            .revenue_report(AnalyticsQuery {
                agency_id: input.agency_id,
                period_start: input.period_start,
                period_end: input.period_end,
                property_id: input.property_id,
            })
            .await
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Occupancy Trend use case
// ─────────────────────────────────────────────────────────────────────────────

use crate::domain::analytics::OccupancyTrendReport;

pub struct OccupancyTrendInput {
    pub agency_id: Uuid,
    pub period_start: Date,
    pub period_end: Date,
    pub property_id: Option<Uuid>,
}

pub struct OccupancyTrendUseCase {
    pub repo: Arc<dyn AnalyticsRepository>,
}

impl OccupancyTrendUseCase {
    pub async fn execute(
        &self,
        input: OccupancyTrendInput,
    ) -> Result<OccupancyTrendReport, AppError> {
        self.repo
            .occupancy_trends(AnalyticsQuery {
                agency_id: input.agency_id,
                period_start: input.period_start,
                period_end: input.period_end,
                property_id: input.property_id,
            })
            .await
    }
}
