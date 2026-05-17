use std::sync::Arc;

use anyhow::{Context, Result};
use dashmap::DashMap;
use sqlx::{
    postgres::{PgConnectOptions, PgPoolOptions},
    ConnectOptions, PgPool,
};
use tracing::log::LevelFilter;
use uuid::Uuid;

static AGENCY_MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("migrations/agency");

const POOL_IDLE_TIMEOUT_SECS: u64 = 300;

/// Injected into every authenticated request by `resolve_agency_context` middleware.
#[derive(Clone)]
pub struct AgencyPool(pub PgPool);

/// Manages database pools for the platform and all agency schemas.
#[derive(Clone)]
pub struct AgencyPoolManager {
    platform_pool: PgPool,
    agency_pools: Arc<DashMap<String, PgPool>>,
    agency_database_url: String,
    max_connections_per_agency: u32,
}

impl AgencyPoolManager {
    pub fn new(platform_pool: PgPool, agency_database_url: String) -> Self {
        Self {
            platform_pool,
            agency_pools: Arc::new(DashMap::new()),
            agency_database_url,
            max_connections_per_agency: 10,
        }
    }

    /// Returns the platform pool (emakao_platform, public schema).
    pub fn platform(&self) -> &PgPool {
        &self.platform_pool
    }

    pub fn cached_agency_pools(&self) -> usize {
        self.agency_pools.len()
    }

    /// Returns (or lazily creates) a schema-scoped agency pool.
    /// Called by `resolve_agency_context` on every authenticated request.
    pub async fn for_tenant(&self, schema_name: &str) -> Result<PgPool> {
        if let Some(pool) = self.agency_pools.get(schema_name) {
            return Ok(pool.clone());
        }

        let pool = self
            .build_tenant_pool(schema_name)
            .await
            .with_context(|| format!("could not build pool for schema '{schema_name}'"))?;

        self.agency_pools
            .insert(schema_name.to_owned(), pool.clone());
        tracing::debug!(schema = schema_name, "agency pool created");
        Ok(pool)
    }

    /// Resolves `schema_name` for `agency_id` from the platform DB, then
    /// returns the schema-scoped pool.
    pub async fn for_agency(&self, agency_id: Uuid) -> Result<PgPool> {
        let row = sqlx::query("SELECT schema_name FROM agencies WHERE id = $1")
            .bind(agency_id)
            .fetch_optional(&self.platform_pool)
            .await
            .context("failed to look up agency schema")?
            .ok_or_else(|| anyhow::anyhow!("agency {agency_id} not found"))?;

        use sqlx::Row;
        let schema_name: String = row.get("schema_name");
        self.for_tenant(&schema_name).await
    }

    /// Call this exactly once during agency provisioning, before the first
    /// `for_tenant` call so that tables exist when the pool is used.
    pub async fn provision_new_schema(&self, schema_name: &str) -> Result<()> {
        let schema = schema_name.to_owned();

        let opts: PgConnectOptions = self
            .agency_database_url
            .parse()
            .context("invalid AGENCY_DATABASE_URL")?;

        let provision_pool = PgPoolOptions::new()
            .max_connections(2)
            .after_connect({
                let s = schema.clone();
                move |conn, _| {
                    let s = s.clone();
                    Box::pin(async move {
                        // Idempotent: CREATE SCHEMA IF NOT EXISTS
                        sqlx::query(&format!("CREATE SCHEMA IF NOT EXISTS \"{s}\""))
                            .execute(&mut *conn)
                            .await?;
                        sqlx::query(&format!("SET search_path TO \"{s}\", public"))
                            .execute(&mut *conn)
                            .await?;
                        Ok(())
                    })
                }
            })
            .connect_with(opts)
            .await
            .with_context(|| format!("could not open provisioning pool for schema '{schema}'"))?;

        AGENCY_MIGRATOR
            .run(&provision_pool)
            .await
            .with_context(|| format!("agency migration failed for schema '{schema}'"))?;

        provision_pool.close().await;
        tracing::info!(schema = schema_name, "agency schema provisioned ✓");
        Ok(())
    }

    async fn build_tenant_pool(&self, schema_name: &str) -> Result<PgPool> {
        let schema = schema_name.to_owned();

        let opts: PgConnectOptions = self
            .agency_database_url
            .parse()
            .context("invalid AGENCY_DATABASE_URL")?;

        let opts = opts
            .log_statements(LevelFilter::Debug)
            .log_slow_statements(LevelFilter::Warn, std::time::Duration::from_secs(1));

        PgPoolOptions::new()
            .max_connections(self.max_connections_per_agency)
            .idle_timeout(std::time::Duration::from_secs(POOL_IDLE_TIMEOUT_SECS))
            .after_connect(move |conn, _| {
                let s = schema.clone();
                Box::pin(async move {
                    sqlx::query(&format!("SET search_path TO \"{s}\", public"))
                        .execute(&mut *conn)
                        .await?;
                    Ok(())
                })
            })
            .connect_with(opts)
            .await
            .context("PgPoolOptions::connect_with failed")
    }
}
