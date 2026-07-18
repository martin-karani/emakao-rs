
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::feedback_repository::FeedbackRepository},
    domain::feedback::{Feedback, FeedbackStatus},
};

pub struct UpdateFeedbackStatusUseCase {
    pub repo: Arc<dyn FeedbackRepository>,
}

impl UpdateFeedbackStatusUseCase {
    pub fn new(repo: Arc<dyn FeedbackRepository>) -> Self {
        Self { repo }
    }
}

pub struct UpdateFeedbackStatusInput {
    pub agency_id: Uuid,
    pub feedback_id: Uuid,
    pub status: FeedbackStatus,
}

impl UpdateFeedbackStatusUseCase {
    pub async fn execute(&self, input: UpdateFeedbackStatusInput) -> Result<Feedback, AppError> {
        self.repo.update_status(crate::application::ports::feedback_repository::UpdateFeedbackStatusCommand {
            feedback_id: input.feedback_id,
            agency_id: input.agency_id,
            status: input.status,
        }).await
    }
}
