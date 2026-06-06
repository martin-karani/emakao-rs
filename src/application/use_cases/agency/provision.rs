//
// Provisions a complete new agency in one atomic sequence:
//   1. Insert the agencies row (platform DB)
//   2. Provision the tenant Postgres schema (run migrations)
//   3. Create an OpenFGA store
//   4. Seed the default authorization model
//   5. Persist the FGA store_id
//   6. Seed the standard chart of accounts   ← NEW
//
// Step 6 uses `PgAccountingRepo` directly because `ProvisionAgencyUseCase`
// already imports from infrastructure (`AgencyPoolManager`).  This is the
// only place in the application layer where that boundary is crossed for
// provisioning-time bootstrapping.

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
    infrastructure::db::{
        accounting_repository_sqlx::PgAccountingRepo, pool::AgencyPoolManager,
        role_repository_sqlx::PgRoleRepo,
    },
};

// Re-export so callers don't have to import the accounting port directly.
use crate::application::ports::{
    accounting_repository::AccountingRepository as _, role_repository::RoleRepository as _,
};

pub struct ProvisionAgencyInput {
    pub name: String,
    pub slug: String,
    pub country_code: String,
    pub currency_code: String,
}

pub struct ProvisionAgencyUseCase {
    pub agency_repo: Arc<dyn AgencyRepository>,
    pub openfga: Arc<dyn OpenFgaPort>,
    pub pool_manager: Arc<AgencyPoolManager>,
    pub default_model: serde_json::Value,
}

impl ProvisionAgencyUseCase {
    pub fn new(
        agency_repo: Arc<dyn AgencyRepository>,
        pool_manager: Arc<AgencyPoolManager>,
        openfga: Arc<dyn OpenFgaPort>,
    ) -> Self {
        // Load the default model from a resource file or embedded JSON
        let default_model =
            serde_json::from_str(include_str!("../../../../resources/fga/default_model.json"))
                .expect("failed to parse default FGA model");

        Self {
            agency_repo,
            pool_manager,
            openfga,
            default_model,
        }
    }

    pub async fn execute(&self, input: ProvisionAgencyInput) -> Result<Agency, AppError> {
        let schema_name = format!("agency_{}", input.slug.replace('-', "_").to_lowercase());

        tracing::info!(slug = %input.slug, "provisioning agency");

        // ── Step 1: Insert agencies row ────────────────────────────────────────
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
                if e.to_string().contains("duplicate key")
                    || e.to_string().contains("unique constraint")
                {
                    AppError::Conflict(format!("agency with slug '{}' already exists", input.slug))
                } else {
                    e
                }
            })?;

        // ── Step 2: Provision Postgres schema ──────────────────────────────────
        self.pool_manager
            .provision_new_schema(&schema_name)
            .await
            .map_err(|e| {
                tracing::error!(error = %format!("{e:#}"), "schema provisioning failed");
                AppError::InternalServer(format!("schema provisioning failed: {e:#}"))
            })?;
        // ── Step 3: Create OpenFGA store ───────────────────────────────────────
        let store_id = self.openfga.create_store(&input.slug).await?;

        // ── Step 4: Seed default authorization model ───────────────────────────
        let _model_id = self
            .openfga
            .write_auth_model(&store_id, &self.default_model)
            .await?;

        tracing::info!(agency_id = %agency.id, store_id = %store_id, "FGA model written");

        // ── Step 5: Persist FGA store_id ──────────────────────────────────────
        self.agency_repo
            .save_fga_store_id(agency.id, &store_id)
            .await?;

        // ── Step 6: Seed the standard chart of accounts ────────────────────────
        // Get the tenant pool for the newly created schema and insert the
        // 18 system accounts defined in `domain::accounting::system_accounts()`.
        let tenant_pool = self
            .pool_manager
            .for_agency(agency.id)
            .await
            .map_err(|e| AppError::InternalServer(format!("tenant pool open failed: {e}")))?;

        let accounting_repo = PgAccountingRepo::new(tenant_pool.clone());
        accounting_repo
            .seed_system_accounts(agency.id)
            .await
            .map_err(|e| {
                // Non-fatal: log and continue — the agency is provisioned; CoA can
                // be seeded manually if this fails (e.g. migration not yet applied).
                tracing::warn!(agency_id = %agency.id, error = %e, "CoA seeding failed (non-fatal)");
                e
            })
            .ok();

        // ── Step 7: Seed system roles ──────────────────────────────────────────
        let role_repo = PgRoleRepo::new(tenant_pool.clone());
        role_repo
            .seed_system_roles(agency.id)
            .await
            .map_err(|e| {
                tracing::warn!(agency_id = %agency.id, error = %e, "role seeding failed (non-fatal)");
                e
            })
            .ok();

        let provisioned = Agency {
            fga_store_id: Some(store_id),
            ..agency
        };

        tracing::info!(agency_id = %provisioned.id, "agency provisioned ✓");
        Ok(provisioned)
    }
}
