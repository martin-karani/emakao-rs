use crate::domain::enums::{PropertyType, UnitStatus};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

// ── Property policies / configuration ────────────────────────────────────────

/// Accepted method of rent / service-charge payment for this property.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum PaymentMethodKind {
    #[default]
    Mpesa,
    Bank,
    Cash,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct PaymentMethod {
    pub method: PaymentMethodKind,
    /// e.g. M-Pesa paybill number or bank account number.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_number: Option<String>,
    /// e.g. bank name or M-Pesa paybill name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_name: Option<String>,
    /// Free-text instructions printed on rent statements (e.g. "Use unit number as reference").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
}

/// A configured service charge for a property
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ServiceChargeConfig {
    /// Identifier or name (e.g. "Security", "Water", "Garbage")
    pub name: String,
    /// Whether this is a per-unit charge or a flat rate for all units
    pub is_per_unit: bool,
    /// The amount (KES). If per-unit, this is the base amount per unit.
    pub amount: Decimal,
    /// Optional label for per-unit charges (e.g. "per liter", "per kWh")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub per_unit_label: Option<String>,
}

/// All per-property policy configuration that agents/owners can customise.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct PropertyPolicies {
    /// Accepted rent payment methods for this property.
    #[serde(default)]
    pub payment_methods: Vec<PaymentMethod>,
    /// Agent management fee as a percentage of collected rent (2.5–10 %).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_commission_percent: Option<Decimal>,
    /// "flat" or "percent".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub late_fee_type: Option<String>,
    /// Late-fee amount (KES if flat, percentage if percent).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub late_fee_value: Option<Decimal>,
    /// Number of days after the due date before a late fee is applied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub late_fee_grace_days: Option<i32>,
    /// How many months of rent the deposit equals (typically 1–3).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deposit_months: Option<i32>,
    /// Maximum days within which deposit must be refunded after move-out.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deposit_refund_days: Option<i32>,
    /// Service charge config — only relevant for multi-unit properties.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service_charges: Option<Vec<ServiceChargeConfig>>,
}

// ── PropertyConfig discriminated union ───────────────────────────────────────

/// The `type` field discriminates the variant (e.g. `"single_family"`).
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PropertyConfig {
    SingleFamily {
        #[serde(skip_serializing_if = "Option::is_none")]
        floors: Option<u8>,
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

// ── Property ──────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct UnitType {
    pub id: Uuid,
    pub name: String,
    /// Optional category label, e.g. `"studio"`, `"1br"`, `"penthouse"`.
    /// Free-form string so agencies can define their own taxonomy.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit_type: Option<String>,
    pub bedrooms: i16,
    pub bathrooms: i16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_sqm: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub photos: Option<Vec<String>>,
    pub base_rent: Option<Decimal>,
    pub base_deposit: Option<Decimal>,
    pub quantity: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit_numbers: Option<Vec<String>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Property {
    pub id: Uuid,
    pub agency_id: Uuid,
    pub slug: String,
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
    /// Per-property policy settings: payment methods, agent commission,
    /// late-fee overrides, deposit rules, service charges.
    pub policies: Option<PropertyPolicies>,
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
    pub slug: String,
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
    pub policies: Option<PropertyPolicies>,
}

pub struct UpdatePropertyCommand {
    pub id: Uuid,
    pub agency_id: Uuid,
    pub slug: Option<String>,
    pub name: Option<String>,
    pub address: Option<String>,
    pub city: Option<String>,
    pub work_order_prefix: Option<String>,
    /// When `Some`, replaces the entire policies blob. `None` = leave unchanged.
    pub policies: Option<PropertyPolicies>,
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

/// Convert an arbitrary string into a lowercase URL slug.
pub fn slugify(input: &str) -> String {
    let mut result = String::new();
    let mut last_was_hyphen = false;

    for ch in input.trim().chars() {
        let normalized = ch.to_ascii_lowercase();
        if normalized.is_ascii_alphanumeric() {
            result.push(normalized);
            last_was_hyphen = false;
        } else if !last_was_hyphen && !result.is_empty() {
            result.push('-');
            last_was_hyphen = true;
        }
    }

    let slug = result.trim_matches('-').to_string();
    if slug.is_empty() {
        "property".to_string()
    } else {
        slug
    }
}

/// Guarantee uniqueness within an agency by suffixing `-2`, `-3`, and so on.
pub fn unique_slug(base: &str, existing_slugs: &[String]) -> String {
    if !existing_slugs.iter().any(|slug| slug == base) {
        return base.to_string();
    }

    let mut n = 2u32;
    loop {
        let candidate = format!("{base}-{n}");
        if !existing_slugs.iter().any(|slug| slug == &candidate) {
            return candidate;
        }
        n += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::{slugify, unique_slug};

    #[test]
    fn slugify_normalizes_spaces_and_symbols() {
        assert_eq!(slugify("Westlands Heights"), "westlands-heights");
        assert_eq!(slugify("The  Residences @ Kilimani!"), "the-residences-kilimani");
    }

    #[test]
    fn slugify_trims_and_falls_back_when_empty() {
        assert_eq!(slugify("  ---  "), "property");
        assert_eq!(slugify("  Riverside Plaza  "), "riverside-plaza");
    }

    #[test]
    fn unique_slug_appends_numeric_suffixes() {
        let existing = vec![
            "westlands-heights".to_string(),
            "westlands-heights-2".to_string(),
        ];

        assert_eq!(unique_slug("westlands-heights", &existing), "westlands-heights-3");
        assert_eq!(unique_slug("kilimani-court", &existing), "kilimani-court");
    }
}
