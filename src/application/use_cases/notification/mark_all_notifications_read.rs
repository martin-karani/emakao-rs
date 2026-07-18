use std::sync::Arc;

use crate::application::{errors::AppError, ports::notification_repository::NotificationRepository};

pub struct MarkAllNotificationsReadInput {
    pub user_id: uuid::Uuid,
}

pub struct MarkAllNotificationsReadUseCase {
    repo: Arc<dyn NotificationRepository>,
}

impl MarkAllNotificationsReadUseCase {
    pub fn new(repo: Arc<dyn NotificationRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, input: MarkAllNotificationsReadInput) -> Result<(), AppError> {
        self.repo.mark_all_read(input.user_id).await
    }
}
