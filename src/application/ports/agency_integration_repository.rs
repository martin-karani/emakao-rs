use async_trait::async_trait;
use uuid::Uuid;

use crate::{application::errors::AppError, domain::agency_integration::AgencyIntegration};

// ── Commands ──────────────────────────────────────────────────────────────────

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

// ── Port ──────────────────────────────────────────────────────────────────────

#[async_trait]
pub trait AgencyIntegrationRepository: Send + Sync + 'static {
    /// Return all integrations for an agency. Credentials are never included.
    async fn list(&self, agency_id: Uuid) -> Result<Vec<AgencyIntegration>, AppError>;

    /// Upsert credentials + settings and mark the integration active.
    /// Credentials are encrypted by the implementation before storage.
    async fn upsert(&self, cmd: UpsertIntegrationCommand) -> Result<(), AppError>;

    /// Set `is_active = false` for the given (provider_type, provider_key) pair.
    /// Silently succeeds if the row does not exist.
    async fn deactivate(
        &self,
        agency_id: Uuid,
        provider_type: &str,
        provider_key: &str,
    ) -> Result<(), AppError>;
}
