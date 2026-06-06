use crate::domain::enums::{PropertyType, UnitStatus};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

// ── PropertyConfig discriminated union ───────────────────────────────────────

/// The `type` field discriminates the variant (e.g. `"single_family"`).
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PropertyConfig {
    SingleFamily,
    Multifamily {
        floors: u8,
        parking_spaces: u16,
    },
    Commercial {
        cam_rate: f64,
        building_class: BuildingClass,
    },
    CommunityAssociation {
        due_billing_cycle: BillingCycle,
        board_seats: u8,
    },
    StudentHousing {
        campus_proximity_km: f32,
        semester_start_month: u8,
    },
    AffordableHousing {
        program_id: String,
        ami_percentage: u8,
    },
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, ToSchema)]
pub enum BuildingClass {
    A,
    B,
    C,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, ToSchema)]
pub enum BillingCycle {
    Monthly,
    Quarterly,
    SemiAnnual,
    Annual,
}

// ── Property ──────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct UnitType {
    pub id: Uuid,
    pub name: String,
    pub bedrooms: i16,
    pub bathrooms: i16,
    pub base_rent: Option<Decimal>,
    pub base_deposit: Option<Decimal>,
    pub quantity: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Property {
    pub id: Uuid,
    pub agency_id: Uuid,
    pub name: String,
    pub address: String,
    pub city: String,
    pub country_code: String,
    pub property_type: PropertyType,
    pub config: PropertyConfig,
    pub unit_types: Vec<UnitType>,
    pub photos: Vec<String>,
    pub documents: Vec<PropertyDocument>,
    pub maintenance: PropertyMaintenanceConfig,
    pub created_by: Uuid,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct PropertyMaintenanceConfig {
    pub work_order_prefix: String,
    pub work_order_seq: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct PropertyDocument {
    pub name: String,
    pub url: String,
    pub document_type: String,
}

// ── Unit ──────────────────────────────────────────────────────────────────────

/// A lettable unit belonging to a property.
///
/// Mirrors the `units` table exactly, including financial columns and the
/// optional description field.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Unit {
    pub id: Uuid,
    pub property_id: Uuid,
    pub unit_type_id: Option<Uuid>,
    /// Optional parent (for sub-units / rooms within a larger unit).
    pub parent_unit_id: Option<Uuid>,
    pub unit_number: String,
    pub floor: Option<i32>,
    pub size_sqm: Option<f64>,
    pub bedrooms: Option<i16>,
    pub bathrooms: Option<i16>,
    /// Monthly rent quoted for this unit (KES).
    pub rent_amount_kes: Decimal,
    /// Security deposit (KES).
    pub deposit_kes: Decimal,
    pub status: UnitStatus,
    pub description: Option<String>,
    pub photos: Vec<String>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

// ── Property commands ─────────────────────────────────────────────────────────

pub struct CreatePropertyCommand {
    pub agency_id: Uuid,
    pub created_by: Uuid,
    pub name: String,
    pub address: String,
    pub city: String,
    pub property_type: PropertyType,
    pub config: PropertyConfig,
    pub unit_types: Vec<UnitType>,
    pub photos: Vec<String>,
    pub documents: Vec<PropertyDocument>,
    pub work_order_prefix: Option<String>,
    pub country_code: String,
    pub owner_ids: Vec<Uuid>,
    pub agent_ids: Vec<Uuid>,
}

pub struct UpdatePropertyCommand {
    pub id: Uuid,
    pub agency_id: Uuid,
    pub name: Option<String>,
    pub address: Option<String>,
    pub city: Option<String>,
    pub work_order_prefix: Option<String>,
}

// ── Unit commands ─────────────────────────────────────────────────────────────

pub struct CreateUnitCommand {
    pub property_id: Uuid,
    pub unit_type_id: Option<Uuid>,
    pub unit_number: String,
    pub floor: Option<i32>,
    pub size_sqm: Option<f64>,
    pub bedrooms: Option<i16>,
    pub bathrooms: Option<i16>,
    pub rent_amount_kes: Decimal,
    pub deposit_kes: Decimal,
    pub description: Option<String>,
}

pub struct UpdateUnitCommand {
    pub id: Uuid,
    /// Used only for a permission sanity-check — the DB key is `id` alone.
    pub property_id: Uuid,
    pub unit_type_id: Option<Option<Uuid>>,
    pub unit_number: Option<String>,
    /// `Some(None)` clears the field; `None` leaves it unchanged.
    pub floor: Option<Option<i32>>,
    pub size_sqm: Option<Option<f64>>,
    pub bedrooms: Option<Option<i16>>,
    pub bathrooms: Option<Option<i16>>,
    pub rent_amount_kes: Option<Decimal>,
    pub deposit_kes: Option<Decimal>,
    pub status: Option<UnitStatus>,
    pub description: Option<Option<String>>,
}
