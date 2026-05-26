use std::sync::Arc;

use uuid::Uuid;

use crate::application::{
    errors::AppError, ports::notification_template_repository::NotificationTemplateRepository,
};

pub struct DeleteTemplateInput {
    pub agency_id: Uuid,
    pub channel: String,
    pub event_key: String,
    pub locale: String,
}

pub struct DeleteTemplateUseCase {
    repo: Arc<dyn NotificationTemplateRepository>,
}

impl DeleteTemplateUseCase {
    pub fn new(repo: Arc<dyn NotificationTemplateRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, input: DeleteTemplateInput) -> Result<(), AppError> {
        self.repo
            .delete(
                input.agency_id,
                &input.channel,
                &input.event_key,
                &input.locale,
            )
            .await
    }
}
