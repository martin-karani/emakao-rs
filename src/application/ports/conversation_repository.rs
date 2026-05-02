use crate::application::errors::AppError;
use crate::domain::conversation::{Conversation, CreateConversationCommand};
use async_trait::async_trait;
use uuid::Uuid;

#[async_trait]
pub trait ConversationRepository: Send + Sync + 'static {
    async fn create(&self, cmd: CreateConversationCommand) -> Result<Conversation, AppError>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Conversation>, AppError>;
}
