use garde::Validate;
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};

use crate::domain::{enums::PropertyType, property::PropertyConfig};

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreatePropertyDto {
    #[garde(length(min = 1, max = 200))]
    pub name: String,

    #[garde(length(min = 1, max = 500))]
    pub address: String,

    #[garde(length(max = 100))]
    pub city: String,

    #[garde(skip)]
    pub property_type: PropertyType,

    #[garde(skip)]
    pub config: PropertyConfig,

    /// Optional work-order code prefix, e.g. `"PARK"` or `"MG1"`.
    /// Must be 2–8 uppercase letters/digits and start with a letter.
    /// When omitted, the prefix is auto-generated from the property name.
    #[garde(length(min = 2, max = 8), pattern(r"^[A-Z][A-Z0-9]{1,7}$"))]
    pub work_order_prefix: Option<String>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdatePropertyDto {
    #[garde(length(min = 1, max = 200))]
    pub name: Option<String>,

    #[garde(length(min = 1, max = 500))]
    pub address: Option<String>,

    #[garde(length(max = 100))]
    pub city: Option<String>,
}

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct ListPropertiesParams {
    pub property_type: Option<PropertyType>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}
