use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::maintenance_repository::MaintenanceRepository},
    domain::maintenance::{Caretaker, CreateCaretakerCommand},
};

pub struct CreateCaretakerUseCase {
    pub repo: Arc<dyn MaintenanceRepository>,
}

impl CreateCaretakerUseCase {
    pub fn new(repo: Arc<dyn MaintenanceRepository>) -> Self {
        Self { repo }
    }
}

pub struct CreateCaretakerInput {
    pub property_id: Uuid,
    pub created_by: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub phone: Option<String>,
    pub email: Option<String>,
}

impl CreateCaretakerUseCase {
    pub async fn execute(&self, input: CreateCaretakerInput) -> Result<Caretaker, AppError> {
        let first_name = input.first_name.trim().to_string();
        let last_name = input.last_name.trim().to_string();

        if first_name.is_empty() {
            return Err(AppError::Validation("first_name must not be empty".into()));
        }
        if last_name.is_empty() {
            return Err(AppError::Validation("last_name must not be empty".into()));
        }
        // At least one contact method so we can reach them later.
        if input.phone.is_none() && input.email.is_none() {
            return Err(AppError::Validation(
                "caretaker must have at least a phone number or email address".into(),
            ));
        }

        let caretaker = self
            .repo
            .create_caretaker(CreateCaretakerCommand {
                property_id: input.property_id,
                created_by: input.created_by,
                first_name,
                last_name,
                phone: input.phone,
                email: input.email,
            })
            .await?;

        tracing::info!(
            caretaker_id = %caretaker.id,
            property_id  = %caretaker.property_id,
            "caretaker created"
        );

        Ok(caretaker)
    }
}
