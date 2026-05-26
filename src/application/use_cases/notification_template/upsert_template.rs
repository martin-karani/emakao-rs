// src/application/use_cases/notification_template/upsert_template.rs
//
// This use case owns the rule: "a template body must be a syntactically
// valid MiniJinja expression before it is persisted."
//
// The `TemplateValidator` port keeps that validation dependency abstract so
// the use case stays infrastructure-free and straightforwardly testable.

use std::sync::Arc;

use uuid::Uuid;

use crate::application::{
    errors::AppError,
    ports::notification_template_repository::{
        NotificationTemplateRepository, UpsertTemplateCommand,
    },
};

// ── Validator port ────────────────────────────────────────────────────────────

/// Validates a MiniJinja template string without rendering it.
/// Implemented by the MiniJinja `Environment` in the infrastructure layer.
pub trait TemplateValidator: Send + Sync + 'static {
    fn validate(&self, body: &str) -> Result<(), AppError>;
}

// ── Input ─────────────────────────────────────────────────────────────────────

pub struct UpsertTemplateInput {
    pub agency_id: Uuid,
    pub channel: String,
    pub event_key: String,
    pub locale: String,
    pub subject: Option<String>,
    pub body: String,
}

// ── Use case ──────────────────────────────────────────────────────────────────

pub struct UpsertTemplateUseCase {
    repo: Arc<dyn NotificationTemplateRepository>,
    validator: Arc<dyn TemplateValidator>,
}

impl UpsertTemplateUseCase {
    pub fn new(
        repo: Arc<dyn NotificationTemplateRepository>,
        validator: Arc<dyn TemplateValidator>,
    ) -> Self {
        Self { repo, validator }
    }

    pub async fn execute(&self, input: UpsertTemplateInput) -> Result<(), AppError> {
        // Business rule: body must parse as a valid MiniJinja template.
        self.validator.validate(&input.body)?;

        self.repo
            .upsert(UpsertTemplateCommand {
                agency_id: input.agency_id,
                channel: input.channel,
                event_key: input.event_key,
                locale: input.locale,
                subject: input.subject,
                body: input.body,
            })
            .await
    }
}
