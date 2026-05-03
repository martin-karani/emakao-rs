use serde::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct ModelVersionResponse {
    pub authorization_model_id: String,
}
