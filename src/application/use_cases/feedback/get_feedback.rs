
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::feedback_repository::FeedbackRepository},
    domain::feedback::Feedback,
};

pub struct GetFeedbackUseCase {
    pub repo: Arc<dyn FeedbackRepository>,
}

impl GetFeedbackUseCase {
    pub fn new(repo: Arc<dyn FeedbackRepository>) -> Self {
        Self { repo }
    }
}

pub struct GetFeedbackInput {
    pub agency_id: Uuid,
    pub feedback_id: Uuid,
}

impl GetFeedbackUseCase {
    pub async fn execute(&self, input: GetFeedbackInput) -> Result<Feedback, AppError> {
        self.repo.get_by_id(input.feedback_id, input.agency_id).await
    }
}
