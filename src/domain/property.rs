use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

// ── Property type / config discriminated union ──────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PropertyType {
    SingleFamily,
    Multifamily,
    Commercial,
    CommunityAssociation,
    StudentHousing,
    AffordableHousing,
}

/// The `type` field discriminates the variant (e.g. `"single_family"`).
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PropertyConfig {
    SingleFamily {
        bedrooms: u8,
        bathrooms: u8,
    },
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
    pub created_by: Uuid,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

// ── Unit ────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnitStatus {
    Vacant,
    Occupied,
    Maintenance,
    Reserved,
    Inactive,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Unit {
    pub id: Uuid,
    pub property_id: Uuid,
    pub parent_unit_id: Option<Uuid>,
    pub unit_number: String,
    pub floor: Option<i32>,
    pub size_sqm: Option<f64>,
    pub bedrooms: Option<i16>,
    pub bathrooms: Option<i16>,
    pub status: UnitStatus,
    pub created_at: OffsetDateTime,
}

// ── Commands (in domain layer — pure data) ──────────────────────────────

pub struct CreatePropertyCommand {
    pub agency_id: Uuid,
    pub created_by: Uuid,
    pub name: String,
    pub address: String,
    pub city: String,
    pub property_type: PropertyType,
    pub config: PropertyConfig,
}

pub struct UpdatePropertyCommand {
    pub id: Uuid,
    pub agency_id: Uuid,
    pub name: Option<String>,
    pub address: Option<String>,
    pub city: Option<String>,
}
