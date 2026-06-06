// src/application/ports/notification_template_repository.rs

use async_trait::async_trait;
use uuid::Uuid;

use crate::{application::errors::AppError, domain::notification_template::NotificationTemplate};

// ── Query / Commands ──────────────────────────────────────────────────────────

pub struct ListTemplatesFilter {
    pub agency_id: Uuid,
    pub property_id: Option<Uuid>,
    pub channel: Option<String>,
    pub event_key: Option<String>,
}

pub struct UpsertTemplateCommand {
    pub agency_id: Uuid,
    pub property_id: Option<Uuid>,
    pub channel: String,
    pub event_key: String,
    pub locale: String,
    pub subject: Option<String>,
    pub body: String,
}

// ── Port ──────────────────────────────────────────────────────────────────────

#[async_trait]
pub trait NotificationTemplateRepository: Send + Sync + 'static {
    /// List templates matching the optional channel / event_key filters.
    async fn list(
        &self,
        filter: ListTemplatesFilter,
    ) -> Result<Vec<NotificationTemplate>, AppError>;

    /// Insert or update (ON CONFLICT … DO UPDATE) a template.
    async fn upsert(&self, cmd: UpsertTemplateCommand) -> Result<(), AppError>;

    /// Delete a specific (channel, event_key, locale) triple.
    /// Silently succeeds if the row does not exist.
    async fn delete(
        &self,
        agency_id: Uuid,
        property_id: Option<Uuid>,
        channel: &str,
        event_key: &str,
        locale: &str,
    ) -> Result<(), AppError>;
}
