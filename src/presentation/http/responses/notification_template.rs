use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::notification_template::NotificationTemplate;

#[derive(Debug, Serialize, ToSchema)]
pub struct NotificationTemplateResponse {
    pub id: Uuid,
    pub property_id: Option<Uuid>,
    pub channel: String,
    pub event_key: String,
    pub locale: String,
    pub subject: Option<String>,
    pub body: String,
}

impl From<NotificationTemplate> for NotificationTemplateResponse {
    fn from(t: NotificationTemplate) -> Self {
        Self {
            id: t.id,
            property_id: t.property_id,
            channel: t.channel,
            event_key: t.event_key,
            locale: t.locale,
            subject: t.subject,
            body: t.body,
        }
    }
}
