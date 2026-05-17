use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{errors::AppError, ports::inspection_repository::InspectionRepository},
    domain::inspection::Inspection,
};

pub struct GetInspectionUseCase {
    pub repo: Arc<dyn InspectionRepository>,
}

impl GetInspectionUseCase {
    pub fn new(repo: Arc<dyn InspectionRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, id: Uuid) -> Result<Inspection, AppError> {
        self.repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("inspection {id}")))
    }
}
