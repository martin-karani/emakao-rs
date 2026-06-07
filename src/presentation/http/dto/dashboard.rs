use serde::Deserialize;
use utoipa::IntoParams;

#[derive(Debug, Deserialize, IntoParams)]
pub struct DashboardParams {
    pub expiring_days: Option<i64>,
}
