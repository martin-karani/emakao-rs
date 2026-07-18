use crate::{
    application::{errors::AppError, ports::notification_repository::NotificationRepository},
    domain::notification::{CreateNotificationCommand, Notification},
};
use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

pub struct PgNotificationRepo {
    pool: PgPool,
}

impl PgNotificationRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl From<PgPool> for PgNotificationRepo {
    fn from(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct NotificationRow {
    id: Uuid,
    user_id: Uuid,
    title: String,
    body: String,
    is_read: bool,
    created_at: time::OffsetDateTime,
}

impl From<NotificationRow> for Notification {
    fn from(r: NotificationRow) -> Self {
        Self {
            id: r.id,
            user_id: r.user_id,
            title: r.title,
            body: r.body,
            is_read: r.is_read,
            created_at: r.created_at,
        }
    }
}

#[async_trait]
impl NotificationRepository for PgNotificationRepo {
    async fn list(&self, user_id: Uuid, limit: i64) -> Result<Vec<Notification>, AppError> {
        let rows = sqlx::query_as::<_, NotificationRow>(
            r#"SELECT id, user_id, title, body, is_read, created_at
               FROM notifications
               WHERE user_id = $1
               ORDER BY created_at DESC
               LIMIT $2"#,
        )
        .bind(user_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::InternalServer(e.to_string()))?;

        Ok(rows.into_iter().map(Notification::from).collect())
    }

    async fn create(&self, cmd: CreateNotificationCommand) -> Result<Notification, AppError> {
        let row = sqlx::query_as::<_, NotificationRow>(
            r#"INSERT INTO notifications (id, user_id, title, body)
               VALUES ($1, $2, $3, $4)
               RETURNING id, user_id, title, body, is_read, created_at"#,
        )
        .bind(Uuid::new_v4())
        .bind(cmd.user_id)
        .bind(cmd.title)
        .bind(cmd.body)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::InternalServer(e.to_string()))?;

        Ok(Notification::from(row))
    }

    async fn mark_read(&self, id: Uuid, user_id: Uuid) -> Result<(), AppError> {
        sqlx::query(
            r#"UPDATE notifications
               SET is_read = TRUE
               WHERE id = $1 AND user_id = $2"#,
        )
        .bind(id)
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::InternalServer(e.to_string()))?;

        Ok(())
    }

    async fn mark_all_read(&self, user_id: Uuid) -> Result<(), AppError> {
        sqlx::query(
            r#"UPDATE notifications
               SET is_read = TRUE
               WHERE user_id = $1 AND is_read = FALSE"#,
        )
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::InternalServer(e.to_string()))?;

        Ok(())
    }

    async fn unread_count(&self, user_id: Uuid) -> Result<i64, AppError> {
        let count: (i64,) = sqlx::query_as(
            r#"SELECT COUNT(*) FROM notifications WHERE user_id = $1 AND is_read = FALSE"#,
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::InternalServer(e.to_string()))?;

        Ok(count.0)
    }
}
