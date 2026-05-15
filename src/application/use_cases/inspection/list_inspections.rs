use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{
        errors::AppError,
        ports::inspection_repository::{InspectionFilter, InspectionRepository},
    },
    domain::{
        enums::{InspectionStatus, InspectionType},
        inspection::Inspection,
    },
};

pub struct ListInspectionsUseCase {
    pub repo: Arc<dyn InspectionRepository>,
}

pub struct ListInspectionsInput {
    pub property_id: Option<Uuid>,
    pub unit_id: Option<Uuid>,
    pub agreement_id: Option<Uuid>,
    pub status: Option<InspectionStatus>,
    pub inspection_type: Option<InspectionType>,
    pub limit: i64,
    pub offset: i64,
}

impl ListInspectionsUseCase {
    pub fn new(repo: Arc<dyn InspectionRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, input: ListInspectionsInput) -> Result<Vec<Inspection>, AppError> {
        let limit = input.limit.clamp(1, 100);
        let offset = input.offset.max(0);

        self.repo
            .find_all(InspectionFilter {
                property_id: input.property_id,
                unit_id: input.unit_id,
                agreement_id: input.agreement_id,
                status: input.status,
                inspection_type: input.inspection_type,
                limit,
                offset,
            })
            .await
    }
}
