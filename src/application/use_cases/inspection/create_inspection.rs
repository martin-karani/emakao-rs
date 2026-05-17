use std::sync::Arc;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        ports::inspection_repository::{CreateInspectionCommand, InspectionRepository},
    },
    domain::{enums::InspectionType, inspection::Inspection},
};

pub struct CreateInspectionUseCase {
    pub repo: Arc<dyn InspectionRepository>,
}

pub struct CreateInspectionInput {
    pub property_id: Uuid,
    pub unit_id: Uuid,
    pub agreement_id: Option<Uuid>,
    pub inspection_type: InspectionType,
    pub scheduled_at: OffsetDateTime,
    pub created_by: Uuid,
}

impl CreateInspectionUseCase {
    pub fn new(repo: Arc<dyn InspectionRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, input: CreateInspectionInput) -> Result<Inspection, AppError> {
        if input.scheduled_at < OffsetDateTime::now_utc() - time::Duration::minutes(5) {
            return Err(AppError::Validation(
                "scheduled_at must not be in the past".into(),
            ));
        }

        let inspection = self
            .repo
            .create(CreateInspectionCommand {
                property_id: input.property_id,
                unit_id: input.unit_id,
                agreement_id: input.agreement_id,
                inspection_type: input.inspection_type,
                scheduled_at: input.scheduled_at,
                created_by: input.created_by,
            })
            .await?;

        tracing::info!(
            inspection_id = %inspection.id,
            unit_id       = %inspection.unit_id,
            "inspection scheduled"
        );

        Ok(inspection)
    }
}
