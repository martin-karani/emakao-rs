use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct UploadResponse {
    pub key: String,
    pub url: String,
    pub content_type: String,
    pub size_bytes: usize,
}
