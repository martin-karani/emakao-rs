use crate::application::errors::AppError;
use crate::domain::message::{CreateMessageCommand, Message};
use async_trait::async_trait;
use uuid::Uuid;

#[async_trait]
pub trait MessageRepository: Send + Sync + 'static {
    async fn create(&self, cmd: CreateMessageCommand) -> Result<Message, AppError>;
    async fn list_by_conversation(
        &self,
        conversation_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Message>, AppError>;
}
