// src/infrastructure/providers/registry.rs
//
// ProviderRegistry holds one live provider instance per agency per type.
// Providers are loaded from `agency_integrations` at startup and on any
// integration CRUD operation.
//
// Credentials are decrypted with the platform AES-256-GCM key before use.
// The encrypted bytes are stored in `agency_integrations.credentials` (JSONB).

use std::sync::Arc;

use dashmap::DashMap;
use sqlx::PgPool;
use uuid::Uuid;

use super::traits::{EmailProvider, PaymentProvider, SmsProvider, StorageProvider};

// Concrete provider implementations (stubs — fill in from your existing impl).
use crate::infrastructure::providers::{
    africas_talking::AfricasTalkingProvider, mpesa::MpesaProvider, sendgrid::SendGridProvider,
    twilio::TwilioProvider,
};

// ── Multi-provider SMS router ─────────────────────────────────────────────────

pub struct SmsRouter {
    /// Per-agency provider override.
    pub agency: DashMap<Uuid, Arc<dyn SmsProvider>>,
    /// Platform default (Africa's Talking) — used when no agency override exists.
    pub default: Arc<dyn SmsProvider>,
}

impl SmsRouter {
    /// Return the agency-specific provider, or the platform default.
    pub fn for_agency(&self, agency_id: Uuid) -> Arc<dyn SmsProvider> {
        self.agency
            .get(&agency_id)
            .map(|p| Arc::clone(&*p))
            .unwrap_or_else(|| Arc::clone(&self.default))
    }

    pub fn set(&self, agency_id: Uuid, provider: Arc<dyn SmsProvider>) {
        self.agency.insert(agency_id, provider);
    }

    pub fn remove(&self, agency_id: Uuid) {
        self.agency.remove(&agency_id);
    }
}

// ── Registry ──────────────────────────────────────────────────────────────────

pub struct ProviderRegistry {
    pub sms: SmsRouter,
    pub email: DashMap<Uuid, Arc<dyn EmailProvider>>,
    pub payment: DashMap<Uuid, Arc<dyn PaymentProvider>>,
    pub storage: DashMap<Uuid, Arc<dyn StorageProvider>>,
}

impl ProviderRegistry {
    pub fn new(default_sms: Arc<dyn SmsProvider>) -> Self {
        Self {
            sms: SmsRouter {
                agency: DashMap::new(),
                default: default_sms,
            },
            email: DashMap::new(),
            payment: DashMap::new(),
            storage: DashMap::new(),
        }
    }

    /// Load (or reload) all active integrations for one agency.
    /// Call at startup for every agency, and after any integration CRUD.
    pub async fn load_agency(
        &self,
        agency_id: Uuid,
        pool: &PgPool,
        enc_key: &[u8; 32],
    ) -> anyhow::Result<()> {
        // First, remove stale entries so a deactivated provider is no longer used.
        self.sms.agency.remove(&agency_id);
        self.email.remove(&agency_id);
        self.payment.remove(&agency_id);
        self.storage.remove(&agency_id);

        let rows = sqlx::query(
            r#"
            SELECT provider_type, provider_key, credentials, settings
            FROM agency_integrations
            WHERE agency_id = $1 AND is_active = true
            "#,
        )
        .bind(agency_id)
        .fetch_all(pool)
        .await?;

        for row in rows {
            use sqlx::Row;
            let provider_type: String = row.try_get("provider_type")?;
            let provider_key: String = row.try_get("provider_key")?;
            let creds_raw: serde_json::Value = row.try_get("credentials")?;
            let settings: Option<serde_json::Value> = row.try_get("settings")?;

            let creds = crate::infrastructure::crypto::decrypt_jsonb(&creds_raw, enc_key)?;

            match (provider_type.as_str(), provider_key.as_str()) {
                ("sms", "africas_talking") => {
                    self.sms.set(
                        agency_id,
                        Arc::new(AfricasTalkingProvider::from_creds(creds)?),
                    );
                }
                ("sms", "twilio") => {
                    self.sms
                        .set(agency_id, Arc::new(TwilioProvider::from_creds(creds)?));
                }
                ("payment", "mpesa") => {
                    self.payment.insert(
                        agency_id,
                        Arc::new(MpesaProvider::from_creds(creds, settings.unwrap_or(serde_json::json!({})))?),
                    );
                }
                ("email", "sendgrid") => {
                    self.email
                        .insert(agency_id, Arc::new(SendGridProvider::from_creds(creds)?));
                }
                (pt, pk) => {
                    tracing::warn!(
                        agency = %agency_id,
                        "unknown integration: {pt}/{pk} — skipped"
                    );
                }
            }
        }

        Ok(())
    }
}
