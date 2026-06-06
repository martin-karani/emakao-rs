use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomRole {
    pub id: Uuid,
    pub agency_id: Uuid,
    pub name: String,
    pub permissions: HashSet<String>,
    pub is_system: bool,
    pub created_at: OffsetDateTime,
}

impl CustomRole {
    pub fn system_roles(agency_id: Uuid) -> Vec<Self> {
        vec![
            Self {
                id: Uuid::nil(), // Placeholder, will be generated or ignored if just for listing
                agency_id,
                name: "agency_owner".to_string(),
                permissions: ["*".to_string()].into_iter().collect(),
                is_system: true,
                created_at: OffsetDateTime::now_utc(),
            },
            Self {
                id: Uuid::nil(),
                agency_id,
                name: "manager".to_string(),
                permissions: [
                    "properties:read",
                    "properties:write",
                    "units:read",
                    "units:write",
                    "agreements:read",
                    "agreements:write",
                    "ledger:read",
                    "ledger:write",
                    "payments:read",
                    "payments:write",
                    "maintenance:read",
                    "maintenance:write",
                    "vendors:read",
                    "vendors:write",
                    "owners:read",
                    "owners:write",
                    "residents:read",
                    "residents:write",
                    "inspections:read",
                    "inspections:write",
                    "documents:read",
                    "documents:write",
                    "analytics:read",
                ]
                .into_iter()
                .map(String::from)
                .collect(),
                is_system: true,
                created_at: OffsetDateTime::now_utc(),
            },
            Self {
                id: Uuid::nil(),
                agency_id,
                name: "agent".to_string(),
                permissions: [
                    "properties:read",
                    "units:read",
                    "agreements:read",
                    "ledger:read",
                    "maintenance:read",
                    "maintenance:write",
                    "residents:read",
                    "inspections:read",
                    "inspections:write",
                    "documents:read",
                    "documents:write",
                ]
                .into_iter()
                .map(String::from)
                .collect(),
                is_system: true,
                created_at: OffsetDateTime::now_utc(),
            },
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionDefinition {
    pub resource: String,
    pub action: String,
    pub description: String,
}

impl PermissionDefinition {
    pub fn all() -> Vec<Self> {
        vec![
            // Roles & Permissions
            Self::new("roles", "read", "Can view roles and their permissions"),
            Self::new("roles", "write", "Can create, update, and delete roles"),
            // Agency Settings
            Self::new("agency_settings", "read", "Can view agency settings"),
            Self::new(
                "agency_settings",
                "write",
                "Can update agency settings and workflow rules",
            ),
            // Staff
            Self::new("staff", "read", "Can view staff members"),
            Self::new("staff", "write", "Can invite and manage staff members"),
            // Properties
            Self::new("properties", "read", "Can view properties"),
            Self::new("properties", "write", "Can create and manage properties"),
            // Units
            Self::new("units", "read", "Can view units"),
            Self::new("units", "write", "Can create and manage units"),
            // Agreements
            Self::new("agreements", "read", "Can view lease agreements"),
            Self::new(
                "agreements",
                "write",
                "Can create and manage lease agreements",
            ),
            // Finance & Ledger
            Self::new("ledger", "read", "Can view financial ledgers"),
            Self::new("ledger", "write", "Can post charges and credits to ledgers"),
            // Payments
            Self::new("payments", "read", "Can view payments and claims"),
            Self::new("payments", "write", "Can process and approve payments"),
            // Maintenance
            Self::new("maintenance", "read", "Can view work orders"),
            Self::new("maintenance", "write", "Can create and manage work orders"),
            // Vendors
            Self::new("vendors", "read", "Can view vendors"),
            Self::new("vendors", "write", "Can invite and manage vendors"),
            // Owners
            Self::new("owners", "read", "Can view property owners"),
            Self::new("owners", "write", "Can onboard and manage owners"),
            // Residents
            Self::new("residents", "read", "Can view residents"),
            Self::new("residents", "write", "Can invite and manage residents"),
            // Inspections
            Self::new("inspections", "read", "Can view inspections"),
            Self::new(
                "inspections",
                "write",
                "Can schedule and perform inspections",
            ),
            // Documents
            Self::new("documents", "read", "Can view documents"),
            Self::new("documents", "write", "Can upload and manage documents"),
            // Analytics
            Self::new("analytics", "read", "Can view reports and analytics"),
            // Tax
            Self::new("tax", "read", "Can view tax obligations"),
            Self::new("tax", "write", "Can file and manage tax compliance"),
            // Utilities
            Self::new("utility", "read", "Can view utility meters and bills"),
            Self::new("utility", "write", "Can record readings and generate bills"),
        ]
    }

    fn new(resource: &str, action: &str, description: &str) -> Self {
        Self {
            resource: resource.to_string(),
            action: action.to_string(),
            description: description.to_string(),
        }
    }
}
