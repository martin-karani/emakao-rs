// src/domain/notification_template.rs

use serde::Serialize;
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

/// A per-agency, per-channel MiniJinja template for notification events.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct NotificationTemplate {
    pub id: Uuid,
    pub agency_id: Uuid,
    /// "sms" | "email" | "whatsapp"
    pub channel: String,
    /// e.g. "rent.overdue", "lease.expiry_notice"
    pub event_key: String,
    /// BCP-47 locale code, defaults to "en".
    pub locale: String,
    /// Subject line — email only, `None` for SMS / WhatsApp.
    pub subject: Option<String>,
    /// Raw MiniJinja template string. Validated before storage.
    pub body: String,
    pub created_at: OffsetDateTime,
}
