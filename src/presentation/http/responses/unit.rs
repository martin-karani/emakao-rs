// ── REFACTORED ────────────────────────────────────────────────────────────────
// Changes from original:
//   - Applied #[serde(skip_serializing_if)] to optional fields
// ─────────────────────────────────────────────────────────────────────────────

// src/presentation/http/responses/unit.rs
//
// Serialisable response type returned from unit endpoints.

use rust_decimal::Decimal;
use serde::Serialize;
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::{enums::UnitStatus, property::Unit};

#[derive(Debug, Serialize, ToSchema)]
pub struct UnitResponse {
    pub id: Uuid,
    pub property_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit_type_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_unit_id: Option<Uuid>,
    pub unit_number: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub floor: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_sqm: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bedrooms: Option<i16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bathrooms: Option<i16>,
    /// Monthly rent in KES (serialised as a decimal string to avoid float drift).
    pub rent_amount_kes: Decimal,
    /// Security deposit in KES.
    pub deposit_kes: Decimal,
    pub status: UnitStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

impl From<Unit> for UnitResponse {
    fn from(u: Unit) -> Self {
        Self {
            id: u.id,
            property_id: u.property_id,
            unit_type_id: u.unit_type_id,
            parent_unit_id: u.parent_unit_id,
            unit_number: u.unit_number,
            floor: u.floor,
            size_sqm: u.size_sqm,
            bedrooms: u.bedrooms,
            bathrooms: u.bathrooms,
            rent_amount_kes: u.rent_amount_kes,
            deposit_kes: u.deposit_kes,
            status: u.status,
            description: u.description,
            created_at: u.created_at,
            updated_at: u.updated_at,
        }
    }
}
