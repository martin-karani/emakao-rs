// src/infrastructure/providers/traits.rs
//
// Core provider traits.  Every concrete provider implements one of these.
// The registry (registry.rs) holds the live instances keyed by agency_id.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

// ── SMS ───────────────────────────────────────────────────────────────────────

#[async_trait]
pub trait SmsProvider: Send + Sync {
    /// Stable key used to identify this provider in the DB.
    fn key(&self) -> &'static str;

    /// Send a single SMS.  Returns the provider's message ID on success.
    async fn send(&self, to: &str, body: &str, sender_id: Option<&str>) -> anyhow::Result<String>;
}

// ── Email ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailMessage {
    pub to: String,
    pub subject: String,
    pub body: String,
    /// Optional plain-text fallback; HTML assumed if `None`.
    pub text: Option<String>,
    pub reply_to: Option<String>,
}

#[async_trait]
pub trait EmailProvider: Send + Sync {
    fn key(&self) -> &'static str;
    /// Returns the provider's message ID on success.
    async fn send(&self, msg: EmailMessage) -> anyhow::Result<String>;
}

// ── Payment ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StkRequest {
    pub phone: String,
    pub amount_kes: rust_decimal::Decimal,
    pub account_ref: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StkResponse {
    pub checkout_request_id: String,
    pub merchant_request_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PaymentStatus {
    Pending,
    Success,
    Failed,
    Cancelled,
    Unknown,
}

#[async_trait]
pub trait PaymentProvider: Send + Sync {
    fn key(&self) -> &'static str;
    async fn initiate_stk(&self, req: StkRequest) -> anyhow::Result<StkResponse>;
    async fn query_status(&self, checkout_id: &str) -> anyhow::Result<PaymentStatus>;
    async fn verify_receipt(&self, receipt_number: &str, claim_id: uuid::Uuid, agency_id: uuid::Uuid) -> anyhow::Result<bool>;
}

// ── Storage ───────────────────────────────────────────────────────────────────

#[async_trait]
pub trait StorageProvider: Send + Sync {
    fn key(&self) -> &'static str;
    /// Upload bytes under `key` and return the public URL.
    async fn upload(&self, key: &str, data: &[u8], content_type: &str) -> anyhow::Result<String>;
    async fn download(&self, key: &str) -> anyhow::Result<Vec<u8>>;
    async fn delete(&self, key: &str) -> anyhow::Result<()>;
}
