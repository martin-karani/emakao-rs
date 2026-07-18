
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::feedback_repository::FeedbackRepository},
    domain::feedback::FeedbackReply,
};

pub struct CreateFeedbackReplyUseCase {
    pub repo: Arc<dyn FeedbackRepository>,
}

impl CreateFeedbackReplyUseCase {
    pub fn new(repo: Arc<dyn FeedbackRepository>) -> Self {
        Self { repo }
    }
}

pub struct CreateFeedbackReplyInput {
    pub feedback_id: Uuid,
    pub user_id: Uuid,
    pub message: String,
}

impl CreateFeedbackReplyUseCase {
    pub async fn execute(&self, input: CreateFeedbackReplyInput) -> Result<FeedbackReply, AppError> {
        let message = input.message.trim().to_string();
        if message.is_empty() {
            return Err(AppError::Validation("Message must not be empty".into()));
        }

        self.repo.create_reply(crate::application::ports::feedback_repository::CreateFeedbackReplyCommand {
            feedback_id: input.feedback_id,
            user_id: input.user_id,
            message,
        }).await
    }
}
