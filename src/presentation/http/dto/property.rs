use garde::Validate;
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};

use crate::domain::{
    enums::PropertyType,
    property::{PropertyConfig, PropertyDocument},
};
use rust_decimal::Decimal;
use uuid::Uuid;

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateUnitTypeDto {
    pub name: String,
    pub bedrooms: i16,
    pub bathrooms: i16,
    pub base_rent: Option<Decimal>,
    pub base_deposit: Option<Decimal>,
    pub quantity: i32,
}

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

    #[garde(skip)]
    pub unit_types: Option<Vec<CreateUnitTypeDto>>,

    #[garde(skip)]
    pub photos: Option<Vec<String>>,

    #[garde(skip)]
    pub documents: Option<Vec<PropertyDocument>>,

    /// Optional work-order code prefix, e.g. `"PARK"` or `"MG1"`.
    /// Must be 2–8 uppercase letters/digits and start with a letter.
    /// When omitted, the prefix is auto-generated from the property name.
    #[garde(length(min = 2, max = 8), pattern(r"^[A-Z][A-Z0-9]{1,7}$"))]
    pub work_order_prefix: Option<String>,

    #[garde(skip)]
    pub owner_ids: Option<Vec<Uuid>>,

    #[garde(skip)]
    pub agent_ids: Option<Vec<Uuid>>,

    #[garde(skip)]
    pub new_caretakers: Option<Vec<NewCaretakerDto>>,

    #[garde(skip)]
    pub portal_base_url: Option<String>,

    #[garde(skip)]
    pub agency_name: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct NewCaretakerDto {
    pub first_name: String,
    pub last_name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdatePropertyDto {
    #[garde(length(min = 1, max = 200))]
    pub name: Option<String>,

    #[garde(length(min = 1, max = 500))]
    pub address: Option<String>,

    #[garde(length(max = 100))]
    pub city: Option<String>,

    #[garde(length(min = 2, max = 8), pattern(r"^[A-Z][A-Z0-9]{1,7}$"))]
    pub work_order_prefix: Option<String>,
}

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct ListPropertiesParams {
    pub property_type: Option<PropertyType>,
    pub q: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}
