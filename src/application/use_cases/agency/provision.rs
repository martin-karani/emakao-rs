use std::sync::Arc;

use crate::{
    application::{
        errors::AppError,
        ports::{
            agency_repository::{AgencyRepository, CreateAgencyCommand},
            openfga_port::OpenFgaPort,
        },
    },
    domain::agency::Agency,
    infrastructure::db::pool::AgencyPoolManager,
};

pub struct ProvisionAgencyInput {
    pub name: String,
    /// URL-safe slug, e.g. "acme-realty".  Must be unique across all agencies.
    pub slug: String,
    pub country_code: String,
    pub currency_code: String,
}

pub struct ProvisionAgencyUseCase {
    pub agency_repo: Arc<dyn AgencyRepository>,
    pub openfga: Arc<dyn OpenFgaPort>,
    pub pool_manager: Arc<AgencyPoolManager>,
    /// Loaded once at startup from `resources/fga/default_model.json`.
    pub default_model: serde_json::Value,
}

impl ProvisionAgencyUseCase {
    pub async fn execute(&self, input: ProvisionAgencyInput) -> Result<Agency, AppError> {
        // ── Derive schema name ─────────────────────────────────────────────
        // Postgres identifiers must be lowercase, alphanumeric + underscores.
        // We prefix with "agency_" and replace hyphens with underscores.
        let schema_name = format!("agency_{}", input.slug.replace('-', "_").to_lowercase());

        // ── Step 1: Insert agencies row ────────────────────────────────────
        tracing::info!(slug = %input.slug, "provisioning agency");

        let agency = self
            .agency_repo
            .create(CreateAgencyCommand {
                name: input.name.clone(),
                slug: input.slug.clone(),
                schema_name: schema_name.clone(),
                country_code: input.country_code,
                currency_code: input.currency_code,
            })
            .await
            .map_err(|e| {
                // surface duplicate slug as a Conflict instead of a raw DB error
                if e.to_string().contains("duplicate key")
                    || e.to_string().contains("unique constraint")
                {
                    AppError::Conflict(format!("agency with slug '{}' already exists", input.slug))
                } else {
                    e
                }
            })?;

        // ── Step 2: Provision Postgres schema ──────────────────────────────
        self.pool_manager
            .provision_new_schema(&schema_name)
            .await
            .map_err(|e| AppError::InternalServer(format!("schema provisioning failed: {e}")))?;

        // ── Step 3: Create OpenFGA store ───────────────────────────────────
        // Use the slug as the store name (human-readable in the OpenFGA UI).
        let store_id = self.openfga.create_store(&input.slug).await?;

        // ── Step 4: Seed the default authorization model ───────────────────
        let _model_id = self
            .openfga
            .write_auth_model(&store_id, &self.default_model)
            .await?;

        tracing::info!(
            agency_id  = %agency.id,
            store_id   = %store_id,
            "default authorization model written"
        );

        // ── Step 5: Persist store_id ───────────────────────────────────────
        self.agency_repo
            .save_fga_store_id(agency.id, &store_id)
            .await?;

        // ── Step 6: Return the fully-populated agency ──────────────────────
        let provisioned = Agency {
            fga_store_id: Some(store_id),
            ..agency
        };

        tracing::info!(agency_id = %provisioned.id, "agency provisioned ✓");
        Ok(provisioned)
    }
}
