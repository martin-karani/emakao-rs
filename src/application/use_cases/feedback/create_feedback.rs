
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        ports::feedback_repository::{CreateFeedbackCommand, FeedbackRepository},
    },
    domain::feedback::{Feedback, FeedbackType, SatisfactionRating},
};

pub struct CreateFeedbackUseCase {
    pub repo: Arc<dyn FeedbackRepository>,
}

impl CreateFeedbackUseCase {
    pub fn new(repo: Arc<dyn FeedbackRepository>) -> Self {
        Self { repo }
    }
}

pub struct CreateFeedbackInput {
    pub agency_id: Uuid,
    pub user_id: Uuid,
    pub feedback_type: FeedbackType,
    pub satisfaction: Option<SatisfactionRating>,
    pub subject: String,
    pub description: String,
    pub email: Option<String>,
}

impl CreateFeedbackUseCase {
    pub async fn execute(&self, input: CreateFeedbackInput) -> Result<Feedback, AppError> {
        let subject = input.subject.trim().to_string();
        if subject.is_empty() {
            return Err(AppError::Validation("Subject must not be empty".into()));
        }

        let description = input.description.trim().to_string();
        if description.is_empty() {
            return Err(AppError::Validation("Description must not be empty".into()));
        }

        let feedback = self
            .repo
            .create(CreateFeedbackCommand {
                agency_id: input.agency_id,
                user_id: input.user_id,
                feedback_type: input.feedback_type,
                satisfaction: input.satisfaction,
                subject,
                description,
                email: input.email,
            })
            .await?;

        Ok(feedback)
    }
}

