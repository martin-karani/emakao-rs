use garde::Validate;
use serde::Deserialize;

use crate::domain::property::{PropertyConfig, PropertyType};

#[derive(Debug, Deserialize, Validate)]
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

#[derive(Debug, Deserialize, Validate)]
pub struct UpdatePropertyDto {
    #[garde(length(min = 1, max = 200))]
    pub name: Option<String>,
    #[garde(length(min = 1, max = 500))]
    pub address: Option<String>,
    #[garde(length(min = 1, max = 100))]
    pub city: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListPropertiesParams {
    pub property_type: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}
