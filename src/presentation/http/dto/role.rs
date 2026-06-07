use std::collections::HashSet;

use garde::Validate;
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateRoleRequest {
    #[garde(length(min = 1, max = 100))]
    pub name: String,
    #[garde(skip)]
    pub permissions: HashSet<String>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdateRoleRequest {
    #[garde(skip)]
    pub permissions: HashSet<String>,
}
