use crate::{
    application::{errors::AppError, ports::conversation_repository::ConversationRepository},
    domain::conversation::{Conversation, CreateConversationCommand},
};
use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

pub struct PgConversationRepo {
    pool: PgPool,
}

impl PgConversationRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl From<PgPool> for PgConversationRepo {
    fn from(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct ConversationRow {
    id: Uuid,
    agency_id: Uuid,
    participant_ids: Vec<Uuid>,
    subject: Option<String>,
    created_at: time::OffsetDateTime,
}

impl From<ConversationRow> for Conversation {
    fn from(r: ConversationRow) -> Self {
        Self {
            id: r.id,
            agency_id: r.agency_id,
            participant_ids: r.participant_ids,
            subject: r.subject,
            created_at: r.created_at,
        }
    }
}

#[async_trait]
impl ConversationRepository for PgConversationRepo {
    async fn create(&self, cmd: CreateConversationCommand) -> Result<Conversation, AppError> {
        let row = sqlx::query_as::<_, ConversationRow>(
            r#"INSERT INTO conversations (id, agency_id, participant_ids, subject)
               VALUES ($1, $2, $3, $4)
               RETURNING id, agency_id, participant_ids, subject, created_at"#,
        )
        .bind(Uuid::new_v4())
        .bind(cmd.agency_id)
        .bind(&cmd.participant_ids)
        .bind(cmd.subject)
        .fetch_one(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        Ok(Conversation::from(row))
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Conversation>, AppError> {
        let row = sqlx::query_as::<_, ConversationRow>(
            "SELECT id, agency_id, participant_ids, subject, created_at FROM conversations WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e: sqlx::Error| AppError::InternalServer(e.to_string()))?;

        Ok(row.map(Conversation::from))
    }
}
