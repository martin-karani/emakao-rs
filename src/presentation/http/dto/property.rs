use garde::Validate;
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};

use crate::domain::property::{PropertyConfig, PropertyType};

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreatePropertyDto {
    #[garde(length(min = 1, max = 200))]
    pub name: String,
    #[garde(length(min = 1, max = 500))]
    pub address: String,
    #[garde(length(min = 1, max = 100))]
    pub city: String,
    #[garde(skip)]
    pub property_type: PropertyType,
    #[garde(skip)]
    pub config: PropertyConfig,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdatePropertyDto {
    #[garde(length(min = 1, max = 200))]
    pub name: Option<String>,
    #[garde(length(min = 1, max = 500))]
    pub address: Option<String>,
    #[garde(length(min = 1, max = 100))]
    pub city: Option<String>,
}

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct ListPropertiesParams {
    /// Filter by property type slug (e.g. `single_family`)
    pub property_type: Option<String>,
    /// Maximum records to return (default 20, max 100)
    pub limit: Option<i64>,
    /// Pagination offset (default 0)
    pub offset: Option<i64>,
}
