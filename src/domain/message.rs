use crate::domain::enums::SenderType;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub sender_id: Uuid,
    pub sender_type: SenderType,
    pub body: String,
    pub created_at: OffsetDateTime,
}

pub struct CreateMessageCommand {
    pub conversation_id: Uuid,
    pub sender_id: Uuid,
    pub sender_type: SenderType,
    pub body: String,
}
