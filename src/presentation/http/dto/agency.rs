use garde::Validate;
use serde::Deserialize;
use utoipa::ToSchema;

#[derive(Deserialize, Validate, ToSchema)]
pub struct CreateAgencyDto {
    #[garde(length(min = 2, max = 120))]
    pub name: String,

    /// Lowercase, hyphen-separated, e.g. "acme-realty".
    #[garde(pattern(r"^[a-z0-9]+(?:-[a-z0-9]+)*$"), length(min = 2, max = 63))]
    pub slug: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PatchAgencySettingsDto {
    /// JSON object merged into the agency settings document.
    pub patch: serde_json::Value,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpsertAgencyIntegrationDto {
    pub credentials: serde_json::Value,
    #[serde(default)]
    pub settings: serde_json::Value,
}
