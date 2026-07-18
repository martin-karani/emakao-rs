use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub id: Uuid,
    pub user_id: Uuid,
    pub title: String,
    pub body: String,
    pub is_read: bool,
    pub created_at: OffsetDateTime,
}

pub struct CreateNotificationCommand {
    pub user_id: Uuid,
    pub title: String,
    pub body: String,
}
