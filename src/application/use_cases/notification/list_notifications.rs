use std::sync::Arc;

use crate::{
    application::{errors::AppError, ports::notification_repository::NotificationRepository},
    domain::notification::Notification,
};

pub struct ListNotificationsInput {
    pub user_id: uuid::Uuid,
    pub limit: Option<i64>,
}

pub struct ListNotificationsUseCase {
    repo: Arc<dyn NotificationRepository>,
}

impl ListNotificationsUseCase {
    pub fn new(repo: Arc<dyn NotificationRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(
        &self,
        input: ListNotificationsInput,
    ) -> Result<Vec<Notification>, AppError> {
        let limit = input.limit.unwrap_or(20).min(100);
        self.repo.list(input.user_id, limit).await
    }
}
