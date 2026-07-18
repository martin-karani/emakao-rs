use crate::domain::feedback::{
    Feedback, FeedbackReply, FeedbackStatus, FeedbackType, SatisfactionRating,
};
use serde::Serialize;
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, ToSchema)]
pub struct FeedbackResponse {
    pub id: Uuid,
    pub agency_id: Uuid,
    pub user_id: Uuid,
    pub feedback_type: FeedbackType,
    pub satisfaction: Option<SatisfactionRating>,
    pub status: FeedbackStatus,
    pub subject: String,
    pub description: String,
    pub email: Option<String>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl From<Feedback> for FeedbackResponse {
    fn from(feedback: Feedback) -> Self {
        Self {
            id: feedback.id,
            agency_id: feedback.agency_id,
            user_id: feedback.user_id,
            feedback_type: feedback.feedback_type,
            satisfaction: feedback.satisfaction,
            status: feedback.status,
            subject: feedback.subject,
            description: feedback.description,
            email: feedback.email,
            created_at: feedback.created_at,
            updated_at: feedback.updated_at,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct FeedbackReplyResponse {
    pub id: Uuid,
    pub feedback_id: Uuid,
    pub user_id: Uuid,
    pub message: String,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl From<FeedbackReply> for FeedbackReplyResponse {
    fn from(reply: FeedbackReply) -> Self {
        Self {
            id: reply.id,
            feedback_id: reply.feedback_id,
            user_id: reply.user_id,
            message: reply.message,
            created_at: reply.created_at,
            updated_at: reply.updated_at,
        }
    }
}
