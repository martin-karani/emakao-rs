use async_trait::async_trait;
use uuid::Uuid;

use crate::{application::errors::AppError, domain::agency::Agency};

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

#[async_trait]
pub trait AgencyRepository: Send + Sync + 'static {
    async fn create(&self, cmd: CreateAgencyCommand) -> Result<Agency, AppError>;

    /// Persist the OpenFGA `store_id` obtained during provisioning.
    async fn save_fga_store_id(&self, agency_id: Uuid, store_id: &str) -> Result<(), AppError>;

    /// Look up an agency by slug (used by admin handlers).
    async fn find_by_slug(&self, slug: &str) -> Result<Option<Agency>, AppError>;
}
