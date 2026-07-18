use async_trait::async_trait;
use sqlx::{PgPool, Row};
use std::str::FromStr;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        ports::feedback_repository::{
            CreateFeedbackCommand, CreateFeedbackReplyCommand, FeedbackRepository,
            UpdateFeedbackStatusCommand,
        },
    },
    domain::feedback::{Feedback, FeedbackReply, FeedbackStatus, FeedbackType, SatisfactionRating},
};

#[derive(Debug, sqlx::FromRow)]
struct FeedbackRow {
    id: Uuid,
    agency_id: Uuid,
    user_id: Uuid,
    feedback_type: String,
    satisfaction: Option<String>,
    status: String,
    subject: String,
    description: String,
    email: Option<String>,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

impl TryFrom<FeedbackRow> for Feedback {
    type Error = AppError;

    fn try_from(row: FeedbackRow) -> Result<Self, Self::Error> {
        let feedback_type = FeedbackType::from_str(&row.feedback_type)
            .map_err(|_| AppError::Validation("Invalid feedback type".to_string()))?;

        let satisfaction = row
            .satisfaction
            .and_then(|s| SatisfactionRating::from_str(&s).ok());

        let status = FeedbackStatus::from_str(&row.status)
            .map_err(|_| AppError::Validation("Invalid feedback status".to_string()))?;

        Ok(Feedback {
            id: row.id,
            agency_id: row.agency_id,
            user_id: row.user_id,
            feedback_type,
            satisfaction,
            status,
            subject: row.subject,
            description: row.description,
            email: row.email,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

#[derive(Debug, sqlx::FromRow)]
struct FeedbackReplyRow {
    id: Uuid,
    feedback_id: Uuid,
    user_id: Uuid,
    message: String,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

impl TryFrom<FeedbackReplyRow> for FeedbackReply {
    type Error = AppError;

    fn try_from(row: FeedbackReplyRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            feedback_id: row.feedback_id,
            user_id: row.user_id,
            message: row.message,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

pub struct PgFeedbackRepo {
    pool: PgPool,
}

impl From<PgPool> for PgFeedbackRepo {
    fn from(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl FeedbackRepository for PgFeedbackRepo {
    async fn create(&self, cmd: CreateFeedbackCommand) -> Result<Feedback, AppError> {
        let id = Uuid::new_v4();
        let now = OffsetDateTime::now_utc();
        let status_str = FeedbackStatus::Open.to_string();

        let feedback_type_str = cmd.feedback_type.to_string();
        let satisfaction_str = cmd.satisfaction.map(|s| s.to_string());

        let row = sqlx::query(
            r#"
            INSERT INTO feedback (id, agency_id, user_id, feedback_type, satisfaction, status, subject, description, email, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING id, agency_id, user_id, feedback_type, satisfaction, status, subject, description, email, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(cmd.agency_id)
        .bind(cmd.user_id)
        .bind(feedback_type_str)
        .bind(satisfaction_str)
        .bind(status_str)
        .bind(cmd.subject)
        .bind(cmd.description)
        .bind(cmd.email)
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        let feedback_row = FeedbackRow {
            id: row.try_get("id")?,
            agency_id: row.try_get("agency_id")?,
            user_id: row.try_get("user_id")?,
            feedback_type: row.try_get("feedback_type")?,
            satisfaction: row.try_get("satisfaction")?,
            status: row.try_get("status")?,
            subject: row.try_get("subject")?,
            description: row.try_get("description")?,
            email: row.try_get("email")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        };

        feedback_row.try_into()
    }

    async fn list_for_agency(
        &self,
        agency_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Feedback>, AppError> {
        let rows = sqlx::query(
            r#"
            SELECT id, agency_id, user_id, feedback_type, satisfaction, status, subject, description, email, created_at, updated_at
            FROM feedback
            WHERE agency_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(agency_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let mut feedbacks = Vec::with_capacity(rows.len());
        for row in rows {
            let feedback_row = FeedbackRow {
                id: row.try_get("id")?,
                agency_id: row.try_get("agency_id")?,
                user_id: row.try_get("user_id")?,
                feedback_type: row.try_get("feedback_type")?,
                satisfaction: row.try_get("satisfaction")?,
                status: row.try_get("status")?,
                subject: row.try_get("subject")?,
                description: row.try_get("description")?,
                email: row.try_get("email")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            };
            feedbacks.push(feedback_row.try_into()?);
        }

        Ok(feedbacks)
    }

    async fn get_by_id(&self, feedback_id: Uuid, agency_id: Uuid) -> Result<Feedback, AppError> {
        let row = sqlx::query(
            r#"
            SELECT id, agency_id, user_id, feedback_type, satisfaction, status, subject, description, email, created_at, updated_at
            FROM feedback
            WHERE id = $1 AND agency_id = $2
            "#,
        )
        .bind(feedback_id)
        .bind(agency_id)
        .fetch_one(&self.pool)
        .await?;

        let feedback_row = FeedbackRow {
            id: row.try_get("id")?,
            agency_id: row.try_get("agency_id")?,
            user_id: row.try_get("user_id")?,
            feedback_type: row.try_get("feedback_type")?,
            satisfaction: row.try_get("satisfaction")?,
            status: row.try_get("status")?,
            subject: row.try_get("subject")?,
            description: row.try_get("description")?,
            email: row.try_get("email")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        };

        feedback_row.try_into()
    }

    async fn update_status(&self, cmd: UpdateFeedbackStatusCommand) -> Result<Feedback, AppError> {
        let status_str = cmd.status.to_string();
        let row = sqlx::query(
            r#"
            UPDATE feedback
            SET status = $1
            WHERE id = $2 AND agency_id = $3
            RETURNING id, agency_id, user_id, feedback_type, satisfaction, status, subject, description, email, created_at, updated_at
            "#,
        )
        .bind(status_str)
        .bind(cmd.feedback_id)
        .bind(cmd.agency_id)
        .fetch_one(&self.pool)
        .await?;

        let feedback_row = FeedbackRow {
            id: row.try_get("id")?,
            agency_id: row.try_get("agency_id")?,
            user_id: row.try_get("user_id")?,
            feedback_type: row.try_get("feedback_type")?,
            satisfaction: row.try_get("satisfaction")?,
            status: row.try_get("status")?,
            subject: row.try_get("subject")?,
            description: row.try_get("description")?,
            email: row.try_get("email")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        };

        feedback_row.try_into()
    }

    async fn create_reply(
        &self,
        cmd: CreateFeedbackReplyCommand,
    ) -> Result<FeedbackReply, AppError> {
        let id = Uuid::new_v4();
        let now = OffsetDateTime::now_utc();

        let row = sqlx::query(
            r#"
            INSERT INTO feedback_replies (id, feedback_id, user_id, message, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, feedback_id, user_id, message, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(cmd.feedback_id)
        .bind(cmd.user_id)
        .bind(cmd.message)
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        let reply_row = FeedbackReplyRow {
            id: row.try_get("id")?,
            feedback_id: row.try_get("feedback_id")?,
            user_id: row.try_get("user_id")?,
            message: row.try_get("message")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        };

        reply_row.try_into()
    }

    async fn list_replies(
        &self,
        feedback_id: Uuid,
        agency_id: Uuid,
    ) -> Result<Vec<FeedbackReply>, AppError> {
        let rows = sqlx::query(
            r#"
            SELECT fr.id, fr.feedback_id, fr.user_id, fr.message, fr.created_at, fr.updated_at
            FROM feedback_replies fr
            JOIN feedback f ON fr.feedback_id = f.id
            WHERE fr.feedback_id = $1 AND f.agency_id = $2
            ORDER BY fr.created_at ASC
            "#,
        )
        .bind(feedback_id)
        .bind(agency_id)
        .fetch_all(&self.pool)
        .await?;

        let mut replies = Vec::with_capacity(rows.len());
        for row in rows {
            let reply_row = FeedbackReplyRow {
                id: row.try_get("id")?,
                feedback_id: row.try_get("feedback_id")?,
                user_id: row.try_get("user_id")?,
                message: row.try_get("message")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            };
            replies.push(reply_row.try_into()?);
        }

        Ok(replies)
    }
}
