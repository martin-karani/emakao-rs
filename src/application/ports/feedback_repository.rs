use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    application::errors::AppError,
    domain::feedback::{Feedback, FeedbackReply, FeedbackStatus, FeedbackType, SatisfactionRating},
};

pub struct CreateFeedbackCommand {
    pub agency_id: Uuid,
    pub user_id: Uuid,
    pub feedback_type: FeedbackType,
    pub satisfaction: Option<SatisfactionRating>,
    pub subject: String,
    pub description: String,
    pub email: Option<String>,
}

pub struct UpdateFeedbackStatusCommand {
    pub feedback_id: Uuid,
    pub agency_id: Uuid,
    pub status: FeedbackStatus,
}

pub struct CreateFeedbackReplyCommand {
    pub feedback_id: Uuid,
    pub user_id: Uuid,
    pub message: String,
}

#[async_trait]
pub trait FeedbackRepository: Send + Sync + 'static {
    async fn create(&self, cmd: CreateFeedbackCommand) -> Result<Feedback, AppError>;
    async fn list_for_agency(
        &self,
        agency_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Feedback>, AppError>;
    async fn get_by_id(&self, feedback_id: Uuid, agency_id: Uuid) -> Result<Feedback, AppError>;
    async fn update_status(&self, cmd: UpdateFeedbackStatusCommand) -> Result<Feedback, AppError>;
    async fn create_reply(
        &self,
        cmd: CreateFeedbackReplyCommand,
    ) -> Result<FeedbackReply, AppError>;
    async fn list_replies(
        &self,
        feedback_id: Uuid,
        agency_id: Uuid,
    ) -> Result<Vec<FeedbackReply>, AppError>;
}
