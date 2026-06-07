use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Deserialize, ToSchema)]
pub struct BroadcastNoticeRequest {
    pub subject: String,
    pub body: String,
    pub channels: String,
    pub property_id: Uuid,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct BroadcastStatementRequest {
    pub property_id: Uuid,
}
