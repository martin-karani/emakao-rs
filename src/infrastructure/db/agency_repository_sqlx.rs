use std::sync::Arc;

use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        ports::agency_repository::{AgencyRepository, CreateAgencyCommand, UpsertIntegrationCommand},
    },
    domain::agency::{Agency, AgencyIntegration, ResolvedAgency},
    infrastructure::crypto::encrypt_jsonb,
};

pub struct PgAgencyRepo {
    /// Always the platform pool (emakao_platform, public schema).
    pool: PgPool,
    /// AES-256 key used to encrypt integration credentials at rest.
    enc_key: Option<Arc<[u8; 32]>>,
}

impl PgAgencyRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool, enc_key: None }
    }

    pub fn new_with_encryption(pool: PgPool, enc_key: Arc<[u8; 32]>) -> Self {
        Self { pool, enc_key: Some(enc_key) }
    }
}

// ── Internal row ──────────────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct IntegrationRow {
    agency_id: Uuid,
    provider_type: String,
    provider_key: String,
    is_active: bool,
    settings: serde_json::Value,
}

impl From<IntegrationRow> for AgencyIntegration {
    fn from(r: IntegrationRow) -> Self {
        Self {
            agency_id: r.agency_id,
            provider_type: r.provider_type,
            provider_key: r.provider_key,
            is_active: r.is_active,
            settings: r.settings,
        }
    }
}

#[async_trait]
impl AgencyRepository for PgAgencyRepo {
    async fn create(&self, cmd: CreateAgencyCommand) -> Result<Agency, AppError> {
        let row = sqlx::query(
            r#"
            INSERT INTO agencies (name, slug, schema_name, country_code, currency_code)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, name, slug, schema_name, fga_store_id, status
            "#,
        )
        .bind(&cmd.name)
        .bind(&cmd.slug)
        .bind(&cmd.schema_name)
        .bind(&cmd.country_code)
        .bind(&cmd.currency_code)
        .fetch_one(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        use sqlx::Row;
        Ok(Agency {
            id: row.get("id"),
            name: row.get("name"),
            slug: row.get("slug"),
            schema_name: row.get("schema_name"),
            fga_store_id: row.get("fga_store_id"),
            status: row.get("status"),
            country_code: cmd.country_code,
            currency_code: cmd.currency_code,
        })
    }

    async fn save_fga_store_id(&self, agency_id: Uuid, store_id: &str) -> Result<(), AppError> {
        sqlx::query("UPDATE agencies SET fga_store_id = $1 WHERE id = $2")
            .bind(store_id)
            .bind(agency_id)
            .execute(&self.pool)
            .await
            .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;
        Ok(())
    }

    async fn find_by_slug(&self, slug: &str) -> Result<Option<Agency>, AppError> {
        let row = sqlx::query(
            r#"
            SELECT id, name, slug, schema_name, fga_store_id, status, country_code, currency_code
            FROM   agencies
            WHERE  slug = $1
            "#,
        )
        .bind(slug)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        use sqlx::Row;
        Ok(row.map(|r| Agency {
            id: r.get("id"),
            name: r.get("name"),
            slug: r.get("slug"),
            schema_name: r.get("schema_name"),
            fga_store_id: r.get("fga_store_id"),
            status: r.get("status"),
            country_code: r.get("country_code"),
            currency_code: r.get("currency_code"),
        }))
    }

    async fn find_agency_by_slug(&self, slug: &str) -> Result<Option<ResolvedAgency>, AppError> {
        let row = sqlx::query(
            r#"
            SELECT id, name, slug, schema_name, fga_store_id
            FROM agencies WHERE slug = $1 AND status::text = 'active'
            "#,
        )
        .bind(slug)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        use sqlx::Row;
        Ok(row.map(|r| ResolvedAgency {
            id: r.get("id"),
            name: r.get("name"),
            slug: r.get("slug"),
            schema_name: r.get("schema_name"),
            fga_store_id: r.get("fga_store_id"),
        }))
    }

    async fn find_agency_by_id(&self, id: Uuid) -> Result<Option<ResolvedAgency>, AppError> {
        let row = sqlx::query(
            r#"
            SELECT id, name, slug, schema_name, fga_store_id
            FROM agencies WHERE id = $1 AND status::text = 'active'
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        use sqlx::Row;
        Ok(row.map(|r| ResolvedAgency {
            id: r.get("id"),
            name: r.get("name"),
            slug: r.get("slug"),
            schema_name: r.get("schema_name"),
            fga_store_id: r.get("fga_store_id"),
        }))
    }

    async fn contact_exists_for_agency(
        &self,
        agency_id: Uuid,
        contact: &str,
        contact_type: &str,
        role: &str,
    ) -> Result<bool, AppError> {
        let exists: Option<bool> = sqlx::query_scalar(
            r#"
            SELECT EXISTS (
                SELECT 1
                FROM portal_user_index pui
                JOIN user_agency_roles uar ON uar.id = pui.membership_id
                WHERE pui.contact = $1
                  AND pui.contact_type = $2::contact_type
                  AND uar.agency_id = $3
                  AND uar.role = $4::user_role
            )
            "#,
        )
        .bind(contact)
        .bind(contact_type)
        .bind(agency_id)
        .bind(role)
        .fetch_one(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        Ok(exists.unwrap_or(false))
    }

    // ── Integration methods ───────────────────────────────────────────────────────

    async fn list_integrations(&self, agency_id: Uuid) -> Result<Vec<AgencyIntegration>, AppError> {
        // `credentials` is intentionally excluded — it is encrypted and should
        // never travel outside the infrastructure layer.
        let rows = sqlx::query_as::<_, IntegrationRow>(
            r#"
            SELECT agency_id, provider_type, provider_key, is_active, settings
            FROM   agency_integrations
            WHERE  agency_id = $1
            ORDER  BY provider_type, provider_key
            "#,
        )
        .bind(agency_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(rows.into_iter().map(AgencyIntegration::from).collect())
    }

    async fn upsert_integration(&self, cmd: UpsertIntegrationCommand) -> Result<(), AppError> {
        let enc_key = self.enc_key.as_ref().ok_or_else(|| {
            AppError::InternalServer("encryption key not configured for agency repository".to_string())
        })?;

        // Encrypt before writing — the plain-text credentials must not reach
        // the database.
        let encrypted = encrypt_jsonb(&cmd.credentials, enc_key)
            .map_err(|e| AppError::InternalServer(format!("credential encryption failed: {e}")))?;

        sqlx::query(
            r#"
            INSERT INTO agency_integrations
                (agency_id, provider_type, provider_key, credentials, settings, is_active)
            VALUES
                ($1, $2, $3, $4, $5, true)
            ON CONFLICT (agency_id, provider_type, provider_key)
            DO UPDATE SET
                credentials = EXCLUDED.credentials,
                settings    = EXCLUDED.settings,
                is_active   = true,
                updated_at  = now()
            "#,
        )
        .bind(cmd.agency_id)
        .bind(cmd.provider_type)
        .bind(cmd.provider_key)
        .bind(encrypted)
        .bind(cmd.settings)
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(())
    }

    async fn deactivate_integration(
        &self,
        agency_id: Uuid,
        provider_type: &str,
        provider_key: &str,
    ) -> Result<(), AppError> {
        sqlx::query(
            r#"
            UPDATE agency_integrations
            SET    is_active  = false,
                   updated_at = now()
            WHERE  agency_id     = $1
              AND  provider_type = $2
              AND  provider_key  = $3
            "#,
        )
        .bind(agency_id)
        .bind(provider_type)
        .bind(provider_key)
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(())
    }
}
