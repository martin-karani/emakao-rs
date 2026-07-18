
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::feedback_repository::FeedbackRepository},
    domain::feedback::Feedback,
};

pub struct ListFeedbackUseCase {
    pub repo: Arc<dyn FeedbackRepository>,
}

impl ListFeedbackUseCase {
    pub fn new(repo: Arc<dyn FeedbackRepository>) -> Self {
        Self { repo }
    }
}

pub struct ListFeedbackInput {
    pub agency_id: Uuid,
    pub limit: i64,
    pub offset: i64,
}

impl ListFeedbackUseCase {
    pub async fn execute(&self, input: ListFeedbackInput) -> Result<Vec<Feedback>, AppError> {
        self.repo.list_for_agency(input.agency_id, input.limit, input.offset).await
    }
}
