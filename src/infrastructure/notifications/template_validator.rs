// src/infrastructure/notifications/template_validator.rs
//
// Implements the `TemplateValidator` port defined in the application layer
// using the MiniJinja `Environment` already held in AppState.

use minijinja::Environment;

use crate::application::{
    errors::AppError, use_cases::notification_template::upsert_template::TemplateValidator,
};

pub struct MiniJinjaValidator {
    env: Environment<'static>,
}

impl MiniJinjaValidator {
    pub fn new(env: Environment<'static>) -> Self {
        Self { env }
    }
}

impl TemplateValidator for MiniJinjaValidator {
    fn validate(&self, body: &str) -> Result<(), AppError> {
        self.env
            .template_from_str(body)
            .map(|_| ())
            .map_err(|e| AppError::Validation(format!("invalid MiniJinja template: {e}")))
    }
}
