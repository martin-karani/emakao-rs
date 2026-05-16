// src/presentation/http/responses/dashboard.rs
//
// The dashboard handler returns `DashboardSummary` directly (it already
// derives Serialize + ToSchema), so this module provides a re-export alias
// and any thin wrapper needed for HTTP presentation.

use serde::Serialize;
use utoipa::ToSchema;

use crate::domain::dashboard::DashboardSummary;

/// HTTP response body for GET /api/v1/dashboard.
/// Wraps the domain type in a newtype to keep the presentation layer clean
/// and allow adding envelope fields (e.g. pagination, request_id) later.
#[derive(Debug, Serialize, ToSchema)]
pub struct DashboardResponse(pub DashboardSummary);

impl From<DashboardSummary> for DashboardResponse {
    fn from(s: DashboardSummary) -> Self {
        Self(s)
    }
}
