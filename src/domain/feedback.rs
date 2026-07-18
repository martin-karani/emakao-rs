use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum FeedbackType {
    Bug,
    Feature,
    Improvement,
    Other,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub enum SatisfactionRating {
    VeryDissatisfied,
    Dissatisfied,
    Neutral,
    Satisfied,
    VerySatisfied,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub enum FeedbackStatus {
    Open,
    InProgress,
    Resolved,
    Closed,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct Feedback {
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

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct FeedbackReply {
    pub id: Uuid,
    pub feedback_id: Uuid,
    pub user_id: Uuid,
    pub message: String,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

// Add conversion from/to string for DB conversions for serde/json
impl std::fmt::Display for FeedbackType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FeedbackType::Bug => write!(f, "bug"),
            FeedbackType::Feature => write!(f, "feature"),
            FeedbackType::Improvement => write!(f, "improvement"),
            FeedbackType::Other => write!(f, "other"),
        }
    }
}

impl std::str::FromStr for FeedbackType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "bug" => Ok(FeedbackType::Bug),
            "feature" => Ok(FeedbackType::Feature),
            "improvement" => Ok(FeedbackType::Improvement),
            "other" => Ok(FeedbackType::Other),
            _ => Err(()),
        }
    }
}

impl std::fmt::Display for SatisfactionRating {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SatisfactionRating::VeryDissatisfied => write!(f, "very-dissatisfied"),
            SatisfactionRating::Dissatisfied => write!(f, "dissatisfied"),
            SatisfactionRating::Neutral => write!(f, "neutral"),
            SatisfactionRating::Satisfied => write!(f, "satisfied"),
            SatisfactionRating::VerySatisfied => write!(f, "very-satisfied"),
        }
    }
}

impl std::str::FromStr for SatisfactionRating {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "very-dissatisfied" => Ok(SatisfactionRating::VeryDissatisfied),
            "dissatisfied" => Ok(SatisfactionRating::Dissatisfied),
            "neutral" => Ok(SatisfactionRating::Neutral),
            "satisfied" => Ok(SatisfactionRating::Satisfied),
            "very-satisfied" => Ok(SatisfactionRating::VerySatisfied),
            _ => Err(()),
        }
    }
}

impl std::fmt::Display for FeedbackStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FeedbackStatus::Open => write!(f, "open"),
            FeedbackStatus::InProgress => write!(f, "in-progress"),
            FeedbackStatus::Resolved => write!(f, "resolved"),
            FeedbackStatus::Closed => write!(f, "closed"),
        }
    }
}

impl std::str::FromStr for FeedbackStatus {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "open" => Ok(FeedbackStatus::Open),
            "in-progress" => Ok(FeedbackStatus::InProgress),
            "resolved" => Ok(FeedbackStatus::Resolved),
            "closed" => Ok(FeedbackStatus::Closed),
            _ => Err(()),
        }
    }
}
