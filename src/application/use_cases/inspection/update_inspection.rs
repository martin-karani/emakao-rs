use std::sync::Arc;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        ports::inspection_repository::{InspectionRepository, UpdateInspectionCommand},
    },
    domain::{
        enums::{InspectionStatus, InspectionType},
        inspection::{Inspection, InspectionItem},
    },
};

pub struct UpdateInspectionUseCase {
    pub repo: Arc<dyn InspectionRepository>,
}

pub struct UpdateInspectionInput {
    pub id: Uuid,
    pub status: Option<InspectionStatus>,
    pub scheduled_at: Option<OffsetDateTime>,
    pub completed_at: Option<OffsetDateTime>,
    /// Some(Some(uuid)) = set conductor, Some(None) = clear conductor
    pub conducted_by: Option<Option<Uuid>>,
    pub items: Option<Vec<InspectionItem>>,
    pub summary_notes: Option<String>,
}

impl UpdateInspectionUseCase {
    pub fn new(repo: Arc<dyn InspectionRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, input: UpdateInspectionInput) -> Result<Inspection, AppError> {
        // Validate: completing an inspection requires completed_at
        if let Some(InspectionStatus::Completed) = input.status {
            if input.completed_at.is_none() {
                // Default to now if the caller didn't supply it
            }
        }

        // Validate: completed_at must not be in the future
        if let Some(completed_at) = input.completed_at {
            if completed_at > OffsetDateTime::now_utc() + time::Duration::minutes(5) {
                return Err(AppError::Validation(
                    "completed_at must not be in the future".into(),
                ));
            }
        }

        let inspection = self
            .repo
            .update(UpdateInspectionCommand {
                id: input.id,
                status: input.status,
                scheduled_at: input.scheduled_at,
                completed_at: input.completed_at,
                conducted_by: input.conducted_by,
                items: input.items,
                summary_notes: input.summary_notes,
            })
            .await?;

        tracing::info!(
            inspection_id = %inspection.id,
            status        = ?inspection.status,
            "inspection updated"
        );

        Ok(inspection)
    }
}
