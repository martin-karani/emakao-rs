// src/application/ports/analytics_repository.rs

use async_trait::async_trait;
use time::Date;
use uuid::Uuid;

use crate::{
    application::errors::AppError,
    domain::analytics::{OccupancyTrendReport, PortfolioAnalytics, RevenueReport},
};

pub struct AnalyticsQuery {
    pub agency_id: Uuid,
    pub period_start: Date,
    pub period_end: Date,
    /// Filter to a single property; None = all properties.
    pub property_id: Option<Uuid>,
}

#[async_trait]
pub trait AnalyticsRepository: Send + Sync + 'static {
    /// Full portfolio analytics over a date range.
    async fn portfolio_analytics(
        &self,
        query: AnalyticsQuery,
    ) -> Result<PortfolioAnalytics, AppError>;

    /// Month-by-month revenue detail over a date range.
    async fn revenue_report(&self, query: AnalyticsQuery) -> Result<RevenueReport, AppError>;

    /// Month-by-month occupancy trend.
    async fn occupancy_trends(
        &self,
        query: AnalyticsQuery,
    ) -> Result<OccupancyTrendReport, AppError>;
}
