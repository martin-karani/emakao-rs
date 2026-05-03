use garde::Validate;
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};

use crate::domain::vendor::VendorStatus;

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateVendorDto {
    #[garde(length(min = 1, max = 200))]
    pub name: String,
    #[garde(length(min = 1, max = 100))]
    pub contact_name: Option<String>,
    #[garde(email)]
    pub email: Option<String>,
    #[garde(length(min = 7, max = 20))]
    pub phone: Option<String>,
    #[garde(length(min = 1, max = 100))]
    pub speciality: Option<String>,
    #[garde(length(max = 1000))]
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdateVendorDto {
    #[garde(length(min = 1, max = 200))]
    pub name: Option<String>,
    #[garde(length(min = 1, max = 100))]
    pub contact_name: Option<String>,
    #[garde(email)]
    pub email: Option<String>,
    #[garde(length(min = 7, max = 20))]
    pub phone: Option<String>,
    #[garde(length(min = 1, max = 100))]
    pub speciality: Option<String>,
    #[garde(skip)]
    pub status: Option<VendorStatus>,
    #[garde(length(max = 1000))]
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct ListVendorsParams {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}
