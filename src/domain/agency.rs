// src/domain/agency.rs

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct Agency {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub schema_name: String,
    pub fga_store_id: Option<String>,
    pub status: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PortalType {
    Staff,
    Resident,
    Owner,
}

impl std::fmt::Display for PortalType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Staff => write!(f, "staff"),
            Self::Resident => write!(f, "resident"),
            Self::Owner => write!(f, "owner"),
        }
    }
}

/// Injected by `tenant_resolver` middleware; available in every handler.
/// Feature/limit decisions must use `ResolvedSubscription`, not this struct.
#[derive(Clone, Debug)]
pub struct ResolvedAgency {
    pub id: Uuid,
    pub schema_name: String,
    pub portal_type: PortalType,
    pub fga_store_id: Option<String>,
}
