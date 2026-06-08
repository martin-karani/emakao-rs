// ── REFACTORED ────────────────────────────────────────────────────────────────
// Changes from original:
//   - Added UnitTypeResponse, PropertyDocumentResponse (own nested types)
//   - Added PropertySummaryResponse for list endpoints
//   - Flattened maintenance.work_order_prefix into work_order_prefix: String
//   - Removed created_by (internal audit detail)
//   - Added rfc3339 to created_at / updated_at
//   - Removed Clone from PropertyWithPercentResponse
// ─────────────────────────────────────────────────────────────────────────────

use rust_decimal::Decimal;
use serde::Serialize;
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::enums::PropertyType;
use crate::domain::property::{Property, PropertyConfig, PropertyPolicies, UnitType, PropertyDocument};

#[derive(Debug, Serialize, ToSchema)]
pub struct UnitTypeResponse {
    pub id: Uuid,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit_type: Option<String>,
    pub bedrooms: i16,
    pub bathrooms: i16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_rent: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_deposit: Option<Decimal>,
    pub quantity: i32,
}

impl From<UnitType> for UnitTypeResponse {
    fn from(ut: UnitType) -> Self {
        Self {
            id: ut.id,
            name: ut.name,
            unit_type: ut.unit_type,
            bedrooms: ut.bedrooms,
            bathrooms: ut.bathrooms,
            base_rent: ut.base_rent,
            base_deposit: ut.base_deposit,
            quantity: ut.quantity,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PropertyDocumentResponse {
    pub name: String,
    pub url: String,
    pub document_type: String,
}

impl From<PropertyDocument> for PropertyDocumentResponse {
    fn from(d: PropertyDocument) -> Self {
        Self {
            name: d.name,
            url: d.url,
            document_type: d.document_type,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PropertySummaryResponse {
    pub id: Uuid,
    pub agency_id: Uuid,
    pub slug: String,
    pub name: String,
    pub address: String,
    pub city: String,
    pub country_code: String,
    pub property_type: PropertyType,
    /// Number of configured unit type templates (not individual units).
    pub unit_type_count: usize,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

impl From<Property> for PropertySummaryResponse {
    fn from(p: Property) -> Self {
        Self {
            id: p.id,
            agency_id: p.agency_id,
            slug: p.slug.clone(),
            name: p.name,
            address: p.address,
            city: p.city,
            country_code: p.country_code,
            property_type: p.property_type,
            unit_type_count: p.unit_types.len(),
            created_at: p.created_at,
            updated_at: p.updated_at,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PropertyResponse {
    pub id: Uuid,
    pub agency_id: Uuid,
    pub slug: String,
    pub name: String,
    pub address: String,
    pub city: String,
    pub country_code: String,
    pub property_type: PropertyType,
    pub config: PropertyConfig,
    pub unit_types: Vec<UnitTypeResponse>,
    pub photos: Vec<String>,
    pub documents: Vec<PropertyDocumentResponse>,
    pub work_order_prefix: String,
    /// Per-property policy settings: payment methods, agent commission,
    /// late-fee overrides, deposit rules, and service charges.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policies: Option<PropertyPolicies>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

// created_by intentionally excluded — internal audit detail.
// Use GET /api/v1/audit-log?entity=property&id={id} for actor history.
impl From<Property> for PropertyResponse {
    fn from(p: Property) -> Self {
        Self {
            id: p.id,
            agency_id: p.agency_id,
            slug: p.slug,
            name: p.name,
            address: p.address,
            city: p.city,
            country_code: p.country_code,
            property_type: p.property_type,
            config: p.config,
            unit_types: p.unit_types.into_iter().map(UnitTypeResponse::from).collect(),
            photos: p.photos,
            documents: p.documents.into_iter().map(PropertyDocumentResponse::from).collect(),
            work_order_prefix: p.maintenance.work_order_prefix,
            policies: p.policies,
            created_at: p.created_at,
            updated_at: p.updated_at,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PropertyWithPercentResponse {
    #[serde(flatten)]
    pub property: PropertyResponse,
    pub ownership_percent: Decimal,
}

impl PropertyWithPercentResponse {
    pub fn new(p: Property, percent: Decimal) -> Self {
        Self {
            property: PropertyResponse::from(p),
            ownership_percent: percent,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BillingSummaryResponse {
    pub property_id: Uuid,
    pub property_name: String,
    pub currency_code: String,
    pub rent_due_day: i32,
    pub late_fee_summary: String,
    pub utility_summary: String,
}
