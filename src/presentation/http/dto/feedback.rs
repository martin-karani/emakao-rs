use garde::Validate;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::feedback::{FeedbackStatus, FeedbackType, SatisfactionRating};

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateFeedbackDto {
    #[garde(skip)]
    pub feedback_type: FeedbackType,

    #[garde(skip)]
    pub satisfaction: Option<SatisfactionRating>,

    #[garde(length(min = 5, max = 200))]
    pub subject: String,

    #[garde(length(min = 20, max = 5000))]
    pub description: String,

    #[garde(skip)]
    pub email: Option<String>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdateFeedbackStatusDto {
    #[garde(skip)]
    pub status: FeedbackStatus,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateFeedbackReplyDto {
    #[garde(length(min = 1, max = 5000))]
    pub message: String,
}
