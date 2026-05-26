// src/infrastructure/db/agency_integration_repository_sqlx.rs
//
// Encryption responsibility: this repository receives plain-text credentials
// from the use case and encrypts them with AES-256-GCM before writing.
// The application layer never sees or touches `encrypt_jsonb` directly.

use std::sync::Arc;

use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        ports::agency_integration_repository::{
            AgencyIntegrationRepository, UpsertIntegrationCommand,
        },
    },
    domain::agency_integration::AgencyIntegration,
    infrastructure::crypto::encrypt_jsonb,
};

pub struct PgAgencyIntegrationRepo {
    pool: PgPool,
    /// AES-256 key used to encrypt credential payloads at rest.
    enc_key: Arc<[u8; 32]>,
}

impl PgAgencyIntegrationRepo {
    pub fn new(pool: PgPool, enc_key: Arc<[u8; 32]>) -> Self {
        Self { pool, enc_key }
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

// ── Implementation ────────────────────────────────────────────────────────────

#[async_trait]
impl AgencyIntegrationRepository for PgAgencyIntegrationRepo {
    async fn list(&self, agency_id: Uuid) -> Result<Vec<AgencyIntegration>, AppError> {
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

    async fn upsert(&self, cmd: UpsertIntegrationCommand) -> Result<(), AppError> {
        // Encrypt before writing — the plain-text credentials must not reach
        // the database.
        let encrypted = encrypt_jsonb(&cmd.credentials, &self.enc_key)
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

    async fn deactivate(
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
