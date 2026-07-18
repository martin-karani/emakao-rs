use std::sync::Arc;

use crate::application::{errors::AppError, ports::notification_repository::NotificationRepository};

pub struct MarkNotificationReadInput {
    pub id: uuid::Uuid,
    pub user_id: uuid::Uuid,
}

pub struct MarkNotificationReadUseCase {
    repo: Arc<dyn NotificationRepository>,
}

impl MarkNotificationReadUseCase {
    pub fn new(repo: Arc<dyn NotificationRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, input: MarkNotificationReadInput) -> Result<(), AppError> {
        self.repo.mark_read(input.id, input.user_id).await
    }
}
