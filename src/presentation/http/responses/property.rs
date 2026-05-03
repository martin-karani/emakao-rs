use rust_decimal::Decimal;
use serde::Serialize;
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::property::{Property, PropertyConfig, PropertyType};

#[derive(Debug, Serialize, ToSchema)]
pub struct PropertyResponse {
    pub id: Uuid,
    pub agency_id: Uuid,
    pub name: String,
    pub address: String,
    pub city: String,
    pub country_code: String,
    pub property_type: PropertyType,
    pub config: PropertyConfig,
    pub created_by: Uuid,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PropertyWithPercentResponse {
    #[serde(flatten)]
    pub property: PropertyResponse,
    pub ownership_percent: Decimal,
}

impl From<Property> for PropertyResponse {
    fn from(p: Property) -> Self {
        Self {
            id: p.id,
            agency_id: p.agency_id,
            name: p.name,
            address: p.address,
            city: p.city,
            country_code: p.country_code,
            property_type: p.property_type,
            config: p.config,
            created_by: p.created_by,
            created_at: p.created_at,
            updated_at: p.updated_at,
        }
    }
}

impl PropertyWithPercentResponse {
    pub fn from(p: Property, percent: Decimal) -> Self {
        Self {
            property: PropertyResponse::from(p),
            ownership_percent: percent,
        }
    }
}
