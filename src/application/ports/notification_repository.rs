use crate::application::errors::AppError;
use crate::domain::notification::{CreateNotificationCommand, Notification};
use async_trait::async_trait;
use uuid::Uuid;

#[async_trait]
pub trait NotificationRepository: Send + Sync + 'static {
    /// Fetch the most recent notifications for a user (newest first).
    async fn list(
        &self,
        user_id: Uuid,
        limit: i64,
    ) -> Result<Vec<Notification>, AppError>;

    /// Insert a new notification row.
    async fn create(&self, cmd: CreateNotificationCommand) -> Result<Notification, AppError>;

    /// Mark a single notification as read (only if it belongs to this user).
    async fn mark_read(&self, id: Uuid, user_id: Uuid) -> Result<(), AppError>;

    /// Mark every unread notification for this user as read.
    async fn mark_all_read(&self, user_id: Uuid) -> Result<(), AppError>;

    /// Count unread notifications for a user.
    async fn unread_count(&self, user_id: Uuid) -> Result<i64, AppError>;
}
