// src/presentation/http/dto/unit.rs
//
// Request body DTOs for unit creation and update.
// Validated with `garde` — matches rules stated in the spec.

use garde::Validate;
use rust_decimal::Decimal;
use serde::Deserialize;
use utoipa::ToSchema;

use uuid::Uuid;

use crate::domain::enums::UnitStatus;

// ── Create ────────────────────────────────────────────────────────────────────

/// Fields for a single unit to be created under a property.
#[derive(Debug, Deserialize, Validate, ToSchema, Clone)]
pub struct CreateUnitDto {
    /// E.g. "1A", "GF-01", "101". Must be unique within the property.
    #[garde(length(min = 1, max = 50))]
    pub unit_number: String,

    /// Optional link to a UnitType defined on the property.
    #[garde(skip)]
    pub unit_type_id: Option<Uuid>,

    /// Floor number (negative for basement, e.g. -1).
    #[garde(range(min = -10, max = 200))]
    pub floor: Option<i32>,

    /// Net area in square metres.
    #[garde(range(min = 0.0))]
    pub size_sqm: Option<f64>,

    /// Number of bedrooms (0 for studio / commercial units).
    #[garde(range(min = 0))]
    pub bedrooms: Option<i16>,

    /// Number of bathrooms.
    #[garde(range(min = 0))]
    pub bathrooms: Option<i16>,

    /// Monthly rent in KES. Must be ≥ 0.
    #[garde(skip)]
    pub rent_amount_kes: Decimal,

    /// Security deposit in KES (defaults to 0 on the server).
    #[garde(skip)]
    pub deposit_kes: Option<Decimal>,

    /// Free-text description shown to prospective tenants.
    #[garde(length(max = 2000))]
    pub description: Option<String>,
}

/// Batch payload — POST /api/v1/properties/{propertyId}/units
///
/// Accepts one **or more** units so the wizard can submit all units in a
/// single round-trip after property creation.
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateUnitsDto {
    #[garde(length(min = 1))]
    pub units: Vec<CreateUnitDto>,
}

// ── Update ────────────────────────────────────────────────────────────────────

/// Partial update payload — PUT /api/v1/units/{unitId}
///
/// Every field is optional so callers can update only what changed.
/// `None` in the JSON means "leave unchanged"; an explicit `null` (using
/// `Option<Option<T>>`) clears a nullable column.
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdateUnitDto {
    #[garde(length(min = 1, max = 50))]
    pub unit_number: Option<String>,

    /// `null` clears the unit type; omitting leaves it unchanged.
    #[garde(skip)]
    pub unit_type_id: Option<Option<Uuid>>,

    /// `null` clears the floor; omitting leaves it unchanged.
    #[garde(skip)]
    pub floor: Option<i32>,

    #[garde(skip)]
    pub size_sqm: Option<f64>,

    #[garde(range(min = 0))]
    pub bedrooms: Option<i16>,

    #[garde(range(min = 0))]
    pub bathrooms: Option<i16>,

    #[garde(skip)]
    pub rent_amount_kes: Option<Decimal>,

    #[garde(skip)]
    pub deposit_kes: Option<Decimal>,

    #[garde(skip)]
    pub status: Option<UnitStatus>,

    #[garde(length(max = 2000))]
    pub description: Option<String>,
}
