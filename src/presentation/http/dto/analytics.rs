use serde::Deserialize;
use utoipa::IntoParams;
use uuid::Uuid;

#[derive(Debug, Deserialize, IntoParams)]
pub struct AnalyticsParams {
    pub from: time::Date,
    pub to: time::Date,
    pub property_id: Option<Uuid>,
}
