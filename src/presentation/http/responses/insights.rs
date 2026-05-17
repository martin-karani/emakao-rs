//
// Insight use-cases return domain value objects that already derive
// Serialize + ToSchema. These response wrappers follow the same pattern as
// analytics.rs — thin newtypes for HTTP-layer decoupling.

use serde::Serialize;
use utoipa::ToSchema;

use crate::domain::insights::{
    ExpenseForecast, PredictiveMaintenanceAlert, RentDefaultRiskScore, TenantChurnPrediction,
    VendorAllocationRecommendation,
};

/// Response body for GET /api/v1/insights/rent-risk
#[derive(Debug, Serialize, ToSchema)]
pub struct RentRiskResponse {
    pub scores: Vec<RentDefaultRiskScore>,
}

/// Response body for GET /api/v1/insights/churn
#[derive(Debug, Serialize, ToSchema)]
pub struct ChurnResponse {
    pub predictions: Vec<TenantChurnPrediction>,
}

/// Response body for GET /api/v1/insights/maintenance-alerts
#[derive(Debug, Serialize, ToSchema)]
pub struct MaintenanceAlertsResponse {
    pub alerts: Vec<PredictiveMaintenanceAlert>,
}

/// Response body for GET /api/v1/insights/expense-forecast
#[derive(Debug, Serialize, ToSchema)]
pub struct ExpenseForecastResponse(pub ExpenseForecast);

impl From<ExpenseForecast> for ExpenseForecastResponse {
    fn from(f: ExpenseForecast) -> Self {
        Self(f)
    }
}

/// Response body for GET /api/v1/insights/vendor-allocation/:work_order_id
#[derive(Debug, Serialize, ToSchema)]
pub struct VendorAllocationResponse(pub VendorAllocationRecommendation);

impl From<VendorAllocationRecommendation> for VendorAllocationResponse {
    fn from(v: VendorAllocationRecommendation) -> Self {
        Self(v)
    }
}
