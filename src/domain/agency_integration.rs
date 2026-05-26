// src/domain/agency_integration.rs

use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

/// Redacted summary of a provider integration.
///
/// Credentials are stored AES-256-GCM encrypted and are **never** included
/// in this type — the `credentials` column is write-only from the application
/// layer's perspective.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AgencyIntegration {
    pub agency_id: Uuid,
    /// e.g. "payment" | "sms" | "email" | "storage"
    pub provider_type: String,
    /// e.g. "mpesa" | "africas_talking" | "sendgrid"
    pub provider_key: String,
    pub is_active: bool,
    /// Non-secret config: shortcodes, sender IDs, bucket names.
    pub settings: serde_json::Value,
}
