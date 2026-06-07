use serde::Deserialize;
use utoipa::IntoParams;
use uuid::Uuid;

#[derive(Debug, Deserialize, IntoParams)]
pub struct RiskScoreParams {
    pub min_score: Option<u8>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct ChurnParams {
    pub horizon_days: Option<i64>,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct MaintenanceAlertParams {
    pub min_historical_count: Option<i64>,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct ForecastParams {
    pub property_id: Option<Uuid>,
    pub horizon_months: Option<u8>,
}
