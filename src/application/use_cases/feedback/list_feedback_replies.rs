
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::feedback_repository::FeedbackRepository},
    domain::feedback::FeedbackReply,
};

pub struct ListFeedbackRepliesUseCase {
    pub repo: Arc<dyn FeedbackRepository>,
}

impl ListFeedbackRepliesUseCase {
    pub fn new(repo: Arc<dyn FeedbackRepository>) -> Self {
        Self { repo }
    }
}

pub struct ListFeedbackRepliesInput {
    pub agency_id: Uuid,
    pub feedback_id: Uuid,
}

impl ListFeedbackRepliesUseCase {
    pub async fn execute(&self, input: ListFeedbackRepliesInput) -> Result<Vec<FeedbackReply>, AppError> {
        self.repo.list_replies(input.feedback_id, input.agency_id).await
    }
}
