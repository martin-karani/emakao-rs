// src/infrastructure/kra/gava_connect.rs
//
// Client for KRA's GavaConnect API (developer.go.ke).
//
// Live today (16 APIs):
//   • PIN Checker — /kra/pin-checker/v1/verify
//   • TCC Checker — /kra/tcc/v1/check
//   • NIL Return  — /kra/nil-return/v1/file
//
// Coming soon (register now, wire later):
//   • eRITS  — /kra/erits/v1/return      (rental income MRI filing)
//   • eTIMS  — /kra/etims/v1/invoice     (e-invoice submission)
//
// Authentication: OAuth2 client_credentials.
// Base URL: https://developer.go.ke  (sandbox: https://sandbox.developer.go.ke)

use std::sync::Arc;
use time::OffsetDateTime;

use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::domain::tax::{EritsReturnAck, EritsReturnPayload, KraPinVerificationResult};

// ── Config ────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct GavaConnectConfig {
    pub base_url: String, // "https://developer.go.ke"
    pub consumer_key: String,
    pub consumer_secret: String,
    /// Toggle for eRITS integration (set false until API docs are finalised).
    pub erits_enabled: bool,
    /// Toggle for eTIMS integration.
    pub etims_enabled: bool,
}

impl GavaConnectConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            base_url: std::env::var("KRA_GAVA_BASE_URL")
                .unwrap_or_else(|_| "https://developer.go.ke".into()),
            consumer_key: std::env::var("KRA_GAVA_CONSUMER_KEY")?,
            consumer_secret: std::env::var("KRA_GAVA_CONSUMER_SECRET")?,
            erits_enabled: std::env::var("KRA_ERITS_ENABLED")
                .map(|v| v == "true")
                .unwrap_or(false),
            etims_enabled: std::env::var("KRA_ETIMS_ENABLED")
                .map(|v| v == "true")
                .unwrap_or(false),
        })
    }
}

// ── Token cache ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct CachedToken {
    access_token: String,
    expires_at: OffsetDateTime,
}

// ── Client ────────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct GavaConnectClient {
    config: GavaConnectConfig,
    http: Client,
    token_cache: Arc<RwLock<Option<CachedToken>>>,
}

impl GavaConnectClient {
    pub fn new(config: GavaConnectConfig) -> Self {
        let http = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("emakao/1.0 (+https://emakao.co.ke)")
            .build()
            .expect("failed to build reqwest client");

        Self {
            config,
            http,
            token_cache: Arc::new(RwLock::new(None)),
        }
    }

    // ── Auth ─────────────────────────────────────────────────────────────────

    async fn access_token(&self) -> anyhow::Result<String> {
        // Check cache first (read lock)
        {
            let cache = self.token_cache.read().await;
            if let Some(ref t) = *cache {
                if t.expires_at > OffsetDateTime::now_utc() + time::Duration::seconds(60) {
                    return Ok(t.access_token.clone());
                }
            }
        }

        // Fetch new token (write lock)
        let mut cache = self.token_cache.write().await;

        #[derive(Deserialize)]
        struct TokenResponse {
            access_token: String,
            expires_in: i64, // seconds
        }

        let resp: TokenResponse = self
            .http
            .post(format!("{}/oauth2/token", self.config.base_url))
            .basic_auth(
                &self.config.consumer_key,
                Some(&self.config.consumer_secret),
            )
            .form(&[("grant_type", "client_credentials")])
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let token = CachedToken {
            access_token: resp.access_token.clone(),
            expires_at: OffsetDateTime::now_utc() + time::Duration::seconds(resp.expires_in),
        };

        *cache = Some(token);
        Ok(resp.access_token)
    }

    // ── PIN verification (LIVE TODAY) ─────────────────────────────────────────

    pub async fn verify_pin(&self, kra_pin: &str) -> anyhow::Result<KraPinVerificationResult> {
        #[derive(Serialize)]
        struct Req<'a> {
            kra_pin: &'a str,
        }

        #[derive(Deserialize)]
        struct Resp {
            valid: bool,
            taxpayer_name: Option<String>,
            obligations: Option<Vec<String>>,
        }

        let token = self.access_token().await?;

        let resp: Resp = self
            .http
            .get(format!(
                "{}/kra/pin-checker/v1/verify",
                self.config.base_url
            ))
            .bearer_auth(&token)
            .query(&[("pin", kra_pin)])
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        Ok(KraPinVerificationResult {
            kra_pin: kra_pin.to_string(),
            taxpayer_name: resp.taxpayer_name,
            pin_valid: resp.valid,
            tcc_valid: None, // checked separately
            obligations: resp.obligations.unwrap_or_default(),
            verified_at: OffsetDateTime::now_utc(),
        })
    }

    // ── TCC check (LIVE TODAY) ────────────────────────────────────────────────

    pub async fn check_tcc(&self, kra_pin: &str) -> anyhow::Result<bool> {
        #[derive(Deserialize)]
        struct Resp {
            tcc_valid: bool,
        }

        let token = self.access_token().await?;

        let resp: Resp = self
            .http
            .get(format!("{}/kra/tcc/v1/check", self.config.base_url))
            .bearer_auth(&token)
            .query(&[("pin", kra_pin)])
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        Ok(resp.tcc_valid)
    }

    // ── eRITS MRI return filing (FEATURE-FLAGGED — erits_enabled) ─────────────

    pub async fn file_mri_return(
        &self,
        payload: &EritsReturnPayload,
    ) -> anyhow::Result<EritsReturnAck> {
        if !self.config.erits_enabled {
            // Phase 1 fallback: log but do not call KRA; staff files on iTax.
            tracing::warn!(
                owner_pin   = %payload.owner_kra_pin,
                period      = %payload.tax_period,
                "eRITS integration disabled — obligation must be filed manually on iTax"
            );
            return Err(anyhow::anyhow!(
                "eRITS integration not yet enabled; file manually on iTax"
            ));
        }

        #[derive(Serialize)]
        struct EritsRequest<'a> {
            owner_pin: &'a str,
            property_ref: &'a str,
            tenant_pin: Option<&'a str>,
            period_year: i32,
            period_month: u8,
            gross_rent_kes: String,
            tax_kes: String,
            is_nil_return: bool,
        }

        #[derive(Deserialize)]
        struct EritsResponse {
            ack_number: String,
            prn: String,
            accepted_at: String,
            due_date: String,
        }

        let token = self.access_token().await?;

        let resp: EritsResponse = self
            .http
            .post(format!("{}/kra/erits/v1/return", self.config.base_url))
            .bearer_auth(&token)
            .json(&EritsRequest {
                owner_pin: &payload.owner_kra_pin,
                property_ref: &payload.property_id_kra,
                tenant_pin: payload.tenant_kra_pin.as_deref(),
                period_year: payload.tax_period.year,
                period_month: payload.tax_period.month,
                gross_rent_kes: payload.gross_rent_kes.to_string(),
                tax_kes: payload.tax_kes.to_string(),
                is_nil_return: payload.is_nil_return,
            })
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        // Parse dates returned as strings
        let accepted_at = OffsetDateTime::parse(
            &resp.accepted_at,
            &time::format_description::well_known::Rfc3339,
        )?;
        let due_date = time::Date::parse(
            &resp.due_date,
            &time::format_description::parse("[year]-[month]-[day]").unwrap(),
        )?;

        Ok(EritsReturnAck {
            ack_number: resp.ack_number,
            prn: resp.prn,
            accepted_at,
            due_date,
        })
    }
}
