use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    application::errors::AppError,
    domain::agency::{Agency, AgencyIntegration, ResolvedAgency},
};

pub struct CreateAgencyCommand {
    pub name: String,
    /// URL-safe identifier, e.g. "acme-realty".
    pub slug: String,
    /// Postgres schema name, e.g. "agency_acme_realty".
    /// Derived from slug in the use case; stored here for migration targeting.
    pub schema_name: String,
    pub country_code: String,
    pub currency_code: String,
}

pub struct UpsertIntegrationCommand {
    pub agency_id: Uuid,
    pub provider_type: String,
    pub provider_key: String,
    /// **Plain-text** credentials (e.g. API keys).
    /// The repository implementation is responsible for encrypting these before
    /// writing to `agency_integrations.credentials` (AES-256-GCM).
    pub credentials: serde_json::Value,
    /// Non-secret config: shortcodes, sender IDs, bucket names.
    pub settings: serde_json::Value,
}

#[async_trait]
pub trait AgencyRepository: Send + Sync + 'static {
    async fn create(&self, cmd: CreateAgencyCommand) -> Result<Agency, AppError>;

    /// Persist the OpenFGA `store_id` obtained during provisioning.
    async fn save_fga_store_id(&self, agency_id: Uuid, store_id: &str) -> Result<(), AppError>;

    /// Look up an agency by slug (used by admin handlers).
    async fn find_by_slug(&self, slug: &str) -> Result<Option<Agency>, AppError>;

    /// Look up a slim agency by slug (used by auth use cases).
    async fn find_agency_by_slug(&self, slug: &str) -> Result<Option<ResolvedAgency>, AppError>;

    /// Look up a slim agency by id (used by auth use cases).
    async fn find_agency_by_id(&self, id: Uuid) -> Result<Option<ResolvedAgency>, AppError>;

    /// Check if a contact exists for an agency with a specific role.
    async fn contact_exists_for_agency(
        &self,
        agency_id: Uuid,
        contact: &str,
        contact_type: &str,
        role: &str,
    ) -> Result<bool, AppError>;

    // ── Integration methods ───────────────────────────────────────────────────────

    /// Return all integrations for an agency. Credentials are never included.
    async fn list_integrations(&self, agency_id: Uuid) -> Result<Vec<AgencyIntegration>, AppError>;

    /// Upsert credentials + settings and mark the integration active.
    /// Credentials are encrypted by the implementation before storage.
    async fn upsert_integration(&self, cmd: UpsertIntegrationCommand) -> Result<(), AppError>;

    /// Set `is_active = false` for the given (provider_type, provider_key) pair.
    /// Silently succeeds if the row does not exist.
    async fn deactivate_integration(
        &self,
        agency_id: Uuid,
        provider_type: &str,
        provider_key: &str,
    ) -> Result<(), AppError>;
}
