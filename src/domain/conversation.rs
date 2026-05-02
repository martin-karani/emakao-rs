use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: Uuid,
    pub agency_id: Uuid,
    pub participant_ids: Vec<Uuid>,
    pub subject: Option<String>,
    pub created_at: OffsetDateTime,
}

pub struct CreateConversationCommand {
    pub agency_id: Uuid,
    pub participant_ids: Vec<Uuid>,
    pub subject: Option<String>,
}
