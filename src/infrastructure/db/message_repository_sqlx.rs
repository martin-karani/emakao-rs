use crate::{
    application::{errors::AppError, ports::message_repository::MessageRepository},
    domain::message::{CreateMessageCommand, Message},
};
use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

pub struct PgMessageRepo {
    pool: PgPool,
}

impl PgMessageRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl From<PgPool> for PgMessageRepo {
    fn from(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct MessageRow {
    id: Uuid,
    conversation_id: Uuid,
    sender_id: Uuid,
    sender_type: String,
    body: String,
    created_at: time::OffsetDateTime,
}

impl From<MessageRow> for Message {
    fn from(r: MessageRow) -> Self {
        Self {
            id: r.id,
            conversation_id: r.conversation_id,
            sender_id: r.sender_id,
            sender_type: r.sender_type,
            body: r.body,
            created_at: r.created_at,
        }
    }
}

#[async_trait]
impl MessageRepository for PgMessageRepo {
    async fn create(&self, cmd: CreateMessageCommand) -> Result<Message, AppError> {
        let row = sqlx::query_as!(
            MessageRow,
            r#"INSERT INTO messages (id, conversation_id, sender_id, sender_type, body)
               VALUES ($1, $2, $3, $4, $5)
               RETURNING id, conversation_id, sender_id, sender_type, body, created_at"#,
            Uuid::new_v4(),
            cmd.conversation_id,
            cmd.sender_id,
            cmd.sender_type,
            cmd.body,
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(Message::from(row))
    }

    async fn list_by_conversation(
        &self,
        conversation_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Message>, AppError> {
        let rows = sqlx::query_as!(
            MessageRow,
            "SELECT id, conversation_id, sender_id, sender_type, body, created_at
             FROM messages WHERE conversation_id = $1 ORDER BY created_at ASC LIMIT $2 OFFSET $3",
            conversation_id,
            limit,
            offset
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(Message::from).collect())
    }
}
