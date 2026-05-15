// ── List caretakers ───────────────────────────────────────────────────────────

use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::maintenance_repository::MaintenanceRepository},
    domain::maintenance::{Caretaker, UpdateCaretakerCommand},
};

pub struct ListCaretakersUseCase {
    pub repo: Arc<dyn MaintenanceRepository>,
}

impl ListCaretakersUseCase {
    pub fn new(repo: Arc<dyn MaintenanceRepository>) -> Self {
        Self { repo }
    }

    /// Returns caretakers, optionally narrowed to a single property.
    /// Only active caretakers are returned by default; the repo layer
    /// filters on `is_active = true` unless the caller is an admin view.
    pub async fn execute(
        &self,
        property_id: Option<Uuid>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Caretaker>, AppError> {
        let limit = limit.clamp(1, 200);
        let offset = offset.max(0);
        self.repo.find_caretakers(property_id, limit, offset).await
    }
}

// ── Update caretaker ──────────────────────────────────────────────────────────

pub struct UpdateCaretakerUseCase {
    pub repo: Arc<dyn MaintenanceRepository>,
}

impl UpdateCaretakerUseCase {
    pub fn new(repo: Arc<dyn MaintenanceRepository>) -> Self {
        Self { repo }
    }
}

pub struct UpdateCaretakerInput {
    pub id: Uuid,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub is_active: Option<bool>,
}

impl UpdateCaretakerUseCase {
    pub async fn execute(&self, input: UpdateCaretakerInput) -> Result<Caretaker, AppError> {
        // Trim whitespace from string fields before persisting.
        let first_name = input
            .first_name
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let last_name = input
            .last_name
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        // Verify the caretaker exists before attempting update.
        self.repo
            .find_caretaker_by_id(input.id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("caretaker {}", input.id)))?;

        self.repo
            .update_caretaker(UpdateCaretakerCommand {
                id: input.id,
                first_name,
                last_name,
                phone: input.phone,
                email: input.email,
                is_active: input.is_active,
            })
            .await
    }
}
