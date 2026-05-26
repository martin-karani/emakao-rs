// src/application/use_cases/notification_template/list_templates.rs

use std::sync::Arc;

use crate::{
    application::{
        errors::AppError,
        ports::notification_template_repository::{
            ListTemplatesFilter, NotificationTemplateRepository,
        },
    },
    domain::notification_template::NotificationTemplate,
};

pub struct ListTemplatesInput {
    pub agency_id: uuid::Uuid,
    pub channel: Option<String>,
    pub event_key: Option<String>,
}

pub struct ListTemplatesUseCase {
    repo: Arc<dyn NotificationTemplateRepository>,
}

impl ListTemplatesUseCase {
    pub fn new(repo: Arc<dyn NotificationTemplateRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(
        &self,
        input: ListTemplatesInput,
    ) -> Result<Vec<NotificationTemplate>, AppError> {
        self.repo
            .list(ListTemplatesFilter {
                agency_id: input.agency_id,
                channel: input.channel,
                event_key: input.event_key,
            })
            .await
    }
}
