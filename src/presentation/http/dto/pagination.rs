use serde::Deserialize;
use utoipa::IntoParams;

#[derive(Debug, Deserialize, IntoParams)]
pub struct PaginationParams {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}
