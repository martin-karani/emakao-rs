// src/presentation/http/responses/analytics.rs
//
// The analytics use-cases return domain value objects that already derive
// Serialize + ToSchema, so these responses are thin re-export wrappers.
// They exist to keep the HTTP presentation layer decoupled from domain types
// and to allow future envelope additions without changing use-case signatures.

use serde::Serialize;
use utoipa::ToSchema;

use crate::domain::analytics::{OccupancyTrendReport, PortfolioAnalytics, RevenueReport};

/// Response body for GET /api/v1/analytics/portfolio
#[derive(Debug, Serialize, ToSchema)]
pub struct PortfolioAnalyticsResponse(pub PortfolioAnalytics);

impl From<PortfolioAnalytics> for PortfolioAnalyticsResponse {
    fn from(p: PortfolioAnalytics) -> Self {
        Self(p)
    }
}

/// Response body for GET /api/v1/analytics/revenue
#[derive(Debug, Serialize, ToSchema)]
pub struct RevenueReportResponse(pub RevenueReport);

impl From<RevenueReport> for RevenueReportResponse {
    fn from(r: RevenueReport) -> Self {
        Self(r)
    }
}

/// Response body for GET /api/v1/analytics/occupancy
#[derive(Debug, Serialize, ToSchema)]
pub struct OccupancyTrendResponse(pub OccupancyTrendReport);

impl From<OccupancyTrendReport> for OccupancyTrendResponse {
    fn from(o: OccupancyTrendReport) -> Self {
        Self(o)
    }
}
