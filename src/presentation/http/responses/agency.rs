use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::{agency::{AgencyIntegration, AgencySettings}, enums::AgencyStatus};

#[derive(Serialize, ToSchema)]
pub struct AgencyResponse {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub schema_name: String,
    pub fga_store_id: Option<String>,
    pub status: AgencyStatus,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PatchAgencySettingsResponse {
    pub message: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AgencySettingsResponse {
    pub branding: serde_json::Value,
    pub communication: serde_json::Value,
    pub workflows: serde_json::Value,
    pub ui: serde_json::Value,
    pub portal: serde_json::Value,
    pub onboarding: serde_json::Value,
    pub extra: serde_json::Value,
}

impl From<&AgencySettings> for AgencySettingsResponse {
    fn from(s: &AgencySettings) -> Self {
        let v = serde_json::to_value(s).unwrap_or_default();
        Self {
            branding: v["branding"].clone(),
            communication: v["communication"].clone(),
            workflows: v["workflows"].clone(),
            ui: v["ui"].clone(),
            portal: v["portal"].clone(),
            onboarding: v["onboarding"].clone(),
            extra: v["extra"].clone(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AgencyIntegrationResponse {
    pub provider_type: String,
    pub provider_key: String,
    pub is_active: bool,
    pub settings: serde_json::Value,
    pub credentials: &'static str,
}

impl From<AgencyIntegration> for AgencyIntegrationResponse {
    fn from(i: AgencyIntegration) -> Self {
        Self {
            provider_type: i.provider_type,
            provider_key: i.provider_key,
            is_active: i.is_active,
            settings: i.settings,
            credentials: "[redacted]",
        }
    }
}
